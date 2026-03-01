/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;

use crate::audio::{AudioEngine, Mixer, Sample};
use crate::pattern::{Note, Pattern, Pitch};
use crate::ui::modal::Modal;
use crate::ui::theme::Theme;

/// Application state
///
/// The App struct is the central state manager for the TUI application.
/// It coordinates between different subsystems (UI, input, etc.) and
/// maintains the application's runtime state.
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,

    /// Whether the application is running (for state management)
    pub running: bool,

    /// Cursor X position (for vim-style navigation)
    pub cursor_x: u16,

    /// Cursor Y position (for vim-style navigation)
    pub cursor_y: u16,

    /// Stack of active modal dialogs (top modal is last in Vec)
    modal_stack: Vec<Modal>,

    /// The application's color theme
    pub theme: Theme,

    /// Audio engine (None if no audio device is available)
    audio_engine: Option<AudioEngine>,

    /// Shared mixer for audio rendering (shared with audio callback thread)
    mixer: Arc<Mutex<Mixer>>,

    /// The current pattern being played/edited
    pub pattern: Pattern,

    /// Whether audio is currently playing
    pub is_playing: bool,

    /// Current playback row position
    pub current_row: usize,

    /// Tempo in beats per minute
    pub bpm: f64,

    /// Timestamp of the last row advance (for BPM timing)
    last_row_time: Instant,
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

        Self {
            should_quit: false,
            running: false,
            cursor_x: 0,
            cursor_y: 0,
            modal_stack: Vec::new(),
            theme: Theme::default(),
            audio_engine,
            mixer,
            pattern,
            is_playing: false,
            current_row: 0,
            bpm: 120.0,
            last_row_time: Instant::now(),
        }
    }

    /// Generate a sine wave sample at the given frequency and duration
    fn generate_sine_sample(freq: f32, duration_secs: f32, sample_rate: u32) -> Sample {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut data = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            data.push((2.0 * std::f32::consts::PI * freq * t).sin());
        }
        Sample::new(data, sample_rate, 1, Some("sine440".to_string()))
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
        if self.is_playing {
            // seconds_per_row = 60 / (bpm * rows_per_beat), with 4 rows per beat
            let seconds_per_row = 15.0 / self.bpm;
            let elapsed = self.last_row_time.elapsed().as_secs_f64();

            if elapsed >= seconds_per_row {
                self.last_row_time = Instant::now();
                self.current_row = (self.current_row + 1) % self.pattern.num_rows();
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(self.current_row, &self.pattern);
                }
            }
        }
        Ok(())
    }

    /// Check if the application should continue running
    ///
    /// # Returns
    /// true if the app should keep running, false if it should exit
    pub fn should_run(&self) -> bool {
        self.running && !self.should_quit
    }

    /// Handle application quit with audio cleanup
    pub fn quit(&mut self) {
        // Stop audio before quitting
        if self.is_playing {
            self.is_playing = false;
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

    /// Toggle audio playback on/off
    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.is_playing = false;
            if let Some(ref mut engine) = self.audio_engine {
                let _ = engine.pause();
            }
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.stop_all();
            }
        } else {
            self.is_playing = true;
            self.current_row = 0;
            self.last_row_time = Instant::now();
            // Tick the first row to trigger initial notes
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.tick(self.current_row, &self.pattern);
            }
            if let Some(ref mut engine) = self.audio_engine {
                let _ = engine.start();
            }
        }
    }

    /// Move cursor left (vim: h)
    ///
    /// Decrements the cursor X position unless already at the leftmost position.
    /// This implements vim-style h key navigation.
    pub fn move_left(&mut self) {
        self.cursor_x = self.cursor_x.saturating_sub(1);
    }

    /// Move cursor down (vim: j)
    ///
    /// Increments the cursor Y position unless already at the maximum.
    /// This implements vim-style j key navigation.
    /// Bounded to a maximum of 9 for the demo grid.
    pub fn move_down(&mut self) {
        if self.cursor_y < 9 {
            self.cursor_y = self.cursor_y.saturating_add(1);
        }
    }

    /// Move cursor up (vim: k)
    ///
    /// Decrements the cursor Y position unless already at the topmost position.
    /// This implements vim-style k key navigation.
    pub fn move_up(&mut self) {
        self.cursor_y = self.cursor_y.saturating_sub(1);
    }

    /// Move cursor right (vim: l)
    ///
    /// Increments the cursor X position unless already at the maximum.
    /// This implements vim-style l key navigation.
    /// Bounded to a maximum of 9 for the demo grid.
    pub fn move_right(&mut self) {
        if self.cursor_x < 9 {
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    /// Open a modal dialog by adding it to the modal stack
    ///
    /// This pushes a new modal onto the stack, making it the currently
    /// active modal. The modal will be rendered on top of the current UI.
    ///
    /// # Arguments
    /// * `modal` - The modal dialog to open
    ///
    /// # Example
    /// ```no_run
    /// app.open_modal(Modal::info(
    ///     "Welcome".to_string(),
    ///     "Welcome to Tracker RS!".to_string()
    /// ));
    /// ```
    pub fn open_modal(&mut self, modal: Modal) {
        self.modal_stack.push(modal);
    }

    /// Close the current modal dialog by removing it from the modal stack
    ///
    /// This pops the top modal from the stack. If there are multiple modals
    /// stacked, the next one becomes active. If there are no modals, this
    /// is a no-op.
    ///
    /// # Returns
    /// The closed modal, or None if there were no modals open
    pub fn close_modal(&mut self) -> Option<Modal> {
        self.modal_stack.pop()
    }

    /// Get the currently active modal dialog, if any
    ///
    /// This returns a reference to the top modal on the stack without
    /// removing it. Returns None if there are no modals open.
    ///
    /// # Returns
    /// A reference to the current modal, or None if no modal is open
    pub fn current_modal(&self) -> Option<&Modal> {
        self.modal_stack.last()
    }

    /// Check if any modal is currently open
    ///
    /// # Returns
    /// true if at least one modal is open, false otherwise
    pub fn has_modal(&self) -> bool {
        !self.modal_stack.is_empty()
    }

    /// Open a test modal (for demonstration purposes)
    ///
    /// This is a convenience method for testing modal functionality.
    /// It opens a welcome modal with sample content.
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
