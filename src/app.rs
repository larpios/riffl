/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;

use crate::audio::{AudioEngine, Mixer, Sample, load_sample};
use crate::editor::{Editor, EditorMode};
use crate::pattern::note::Pitch;
use crate::pattern::{Note, Pattern};
use crate::transport::{Transport, TransportState};
use crate::ui::file_browser::FileBrowser;
use crate::ui::modal::Modal;
use crate::ui::theme::Theme;

/// Application state
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,

    /// Whether the application is running (for state management)
    pub running: bool,

    /// The pattern editor (owns the pattern, cursor, mode, undo history)
    pub editor: Editor,

    /// Stack of active modal dialogs (top modal is last in Vec)
    modal_stack: Vec<Modal>,

    /// File browser for loading audio samples
    pub file_browser: FileBrowser,

    /// Names of loaded instruments (indexed by instrument number)
    instrument_names: Vec<String>,

    /// The application's color theme
    pub theme: Theme,

    /// Audio engine (None if no audio device is available)
    audio_engine: Option<AudioEngine>,

    /// Shared mixer for audio rendering (shared with audio callback thread)
    mixer: Arc<Mutex<Mixer>>,

    /// Transport system for playback control (play/pause/stop, BPM, looping)
    pub transport: Transport,

    /// Timestamp of the last update call (for delta time calculation)
    last_update: Instant,
}

impl App {
    /// Create a new App instance with demo pattern and audio engine
    pub fn new() -> Self {
        // Try to create audio engine to get output sample rate
        let audio_engine = AudioEngine::new().ok();
        let output_sample_rate = audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        // Generate a demo sine wave sample at 440Hz, 0.25s duration
        let demo_sample = Self::generate_sine_sample(440.0, 0.25, 44100);

        // Create a demo pattern: C4, E4, G4, C5 across 16 rows
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(4, 0, Note::simple(Pitch::E, 4));
        pattern.set_note(8, 0, Note::simple(Pitch::G, 4));
        pattern.set_note(12, 0, Note::simple(Pitch::C, 5));

        // Create mixer with engine's output sample rate
        let mixer = Arc::new(Mutex::new(Mixer::new(
            vec![demo_sample],
            pattern.num_channels(),
            output_sample_rate,
        )));

        // Create transport and sync with pattern size
        let mut transport = Transport::new();
        transport.set_num_rows(pattern.num_rows());

        let editor = Editor::new(pattern);

        // Initialize file browser at current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let file_browser = FileBrowser::new(&cwd);

        Self {
            should_quit: false,
            running: false,
            editor,
            modal_stack: Vec::new(),
            file_browser,
            instrument_names: vec!["sine440".to_string()],
            theme: Theme::default(),
            audio_engine,
            mixer,
            transport,
            last_update: Instant::now(),
        }
    }

    /// Generate a sine wave sample at the given frequency and duration.
    /// The base_note is set to A-4 (MIDI 57) since the demo sine is at 440Hz.
    fn generate_sine_sample(freq: f32, duration_secs: f32, sample_rate: u32) -> Sample {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut data = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            data.push((2.0 * std::f32::consts::PI * freq * t).sin());
        }
        Sample::new(data, sample_rate, 1, Some("sine440".to_string()))
            .with_base_note(57) // A-4 = MIDI 57 (440Hz)
    }

    /// Initialize the application and set up the audio callback
    pub fn init(&mut self) -> Result<()> {
        self.running = true;

        // Set up audio callback that renders from the shared mixer
        if let Some(ref mut engine) = self.audio_engine {
            let mixer = self.mixer.clone();
            let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
                if let Ok(mut m) = mixer.lock() {
                    m.render(data);
                } else {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }
            }));

            if engine.set_callback(callback).is_err() {
                self.audio_engine = None;
            }
        }

        Ok(())
    }

    /// Update application state, advancing playback row based on BPM timing
    pub fn update(&mut self) -> Result<()> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Keep transport in sync with pattern size
        self.transport.set_num_rows(self.editor.pattern().num_rows());

        let was_playing = self.transport.is_playing();

        if let Some(row) = self.transport.advance(delta) {
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.tick(row, self.editor.pattern());
            }
        } else if self.transport.is_playing() {
            // Even when no row advances, sync track state for real-time
            // mute/solo/volume/pan changes to take effect immediately
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.update_tracks(self.editor.pattern().tracks());
            }
        }

        // Handle auto-stop (loop disabled, reached end)
        if was_playing && self.transport.is_stopped() {
            if let Some(ref mut engine) = self.audio_engine {
                let _ = engine.pause();
            }
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.stop_all();
            }
        }

        Ok(())
    }

    /// Check if the application should continue running
    pub fn should_run(&self) -> bool {
        self.running && !self.should_quit
    }

    /// Get the current editor mode
    pub fn editor_mode(&self) -> EditorMode {
        self.editor.mode()
    }

    /// Handle application quit with audio cleanup
    pub fn quit(&mut self) {
        if !self.transport.is_stopped() {
            self.transport.stop();
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.stop_all();
            }
        }
        if let Some(ref mut engine) = self.audio_engine {
            engine.stop();
        }
        self.should_quit = true;
        self.running = false;
    }

    /// Toggle audio playback between play and pause
    pub fn toggle_play(&mut self) {
        match self.transport.state() {
            TransportState::Stopped => {
                self.transport.play();
                // Trigger first row
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(self.transport.current_row(), self.editor.pattern());
                }
                if let Some(ref mut engine) = self.audio_engine {
                    let _ = engine.start();
                }
            }
            TransportState::Playing => {
                self.transport.pause();
                if let Some(ref mut engine) = self.audio_engine {
                    let _ = engine.pause();
                }
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.stop_all();
                }
            }
            TransportState::Paused => {
                self.transport.play();
                // Resume — trigger current row
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(self.transport.current_row(), self.editor.pattern());
                }
                if let Some(ref mut engine) = self.audio_engine {
                    let _ = engine.start();
                }
            }
        }
    }

    /// Stop playback and reset position to row 0
    pub fn stop(&mut self) {
        self.transport.stop();
        if let Some(ref mut engine) = self.audio_engine {
            let _ = engine.pause();
        }
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.stop_all();
        }
    }

    /// Adjust BPM by a delta value
    pub fn adjust_bpm(&mut self, delta: f64) {
        self.transport.adjust_bpm(delta);
    }

    /// Toggle loop mode on/off
    pub fn toggle_loop(&mut self) {
        self.transport.toggle_loop();
    }

    /// Toggle mute on the current track (channel under cursor)
    pub fn toggle_mute_current_track(&mut self) {
        let ch = self.editor.cursor_channel();
        if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
            track.toggle_mute();
        }
    }

    /// Toggle solo on the current track (channel under cursor)
    pub fn toggle_solo_current_track(&mut self) {
        let ch = self.editor.cursor_channel();
        if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
            track.toggle_solo();
        }
    }

    /// Open a modal dialog by adding it to the modal stack
    pub fn open_modal(&mut self, modal: Modal) {
        self.modal_stack.push(modal);
    }

    /// Close the current modal dialog by removing it from the modal stack
    pub fn close_modal(&mut self) -> Option<Modal> {
        self.modal_stack.pop()
    }

    /// Get the currently active modal dialog, if any
    pub fn current_modal(&self) -> Option<&Modal> {
        self.modal_stack.last()
    }

    /// Check if any modal is currently open
    pub fn has_modal(&self) -> bool {
        !self.modal_stack.is_empty()
    }

    /// Open the file browser overlay
    pub fn open_file_browser(&mut self) {
        self.file_browser.open();
    }

    /// Close the file browser overlay
    pub fn close_file_browser(&mut self) {
        self.file_browser.close();
    }

    /// Check if the file browser is open
    pub fn has_file_browser(&self) -> bool {
        self.file_browser.active
    }

    /// Load the currently selected file from the file browser.
    /// Returns Ok(instrument_index) on success, or an error message.
    pub fn load_selected_sample(&mut self) -> Result<usize, String> {
        let path = self.file_browser.selected_path()
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self.audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample = load_sample(&path, output_sample_rate)
            .map_err(|e| format!("Failed to load: {}", e))?;

        let name = sample.name().unwrap_or("unknown").to_string();

        let idx = if let Ok(mut mixer) = self.mixer.lock() {
            let idx = mixer.add_sample(sample);
            idx
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        self.instrument_names.push(name);
        Ok(idx)
    }

    /// Get the list of loaded instrument names.
    pub fn instrument_names(&self) -> &[String] {
        &self.instrument_names
    }

    /// Get loaded instrument count.
    pub fn instrument_count(&self) -> usize {
        self.instrument_names.len()
    }

    /// Open a test modal (for demonstration purposes)
    pub fn open_test_modal(&mut self) {
        let modal = Modal::info(
            "Welcome to Tracker RS".to_string(),
            "This is a test modal dialog!\n\nYou can:\n• Press 'm' to open this modal\n• Press ESC to close it\n• Stack multiple modals\n\nModal dialogs are perfect for showing messages,\nconfirmations, and help text.".to_string()
        ).with_size(70, 50);

        self.open_modal(modal);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
