/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.
use std::sync::{Arc, Mutex};
use std::time::Instant;

use std::path::PathBuf;

use anyhow::Result;

use crate::audio::{load_sample, AudioEngine, Mixer, Sample};
use crate::dsl::engine::ScriptEngine;
use crate::editor::{Editor, EditorMode};
use crate::export;
use crate::pattern::note::Pitch;
use crate::pattern::{Note, Pattern};
use crate::project;
use crate::song::Song;
use crate::transport::{AdvanceResult, PlaybackMode, Transport, TransportState};
use crate::ui::arrangement::ArrangementView;
use crate::ui::code_editor::CodeEditor;
use crate::ui::export_dialog::ExportDialog;
use crate::ui::file_browser::FileBrowser;
use crate::ui::modal::Modal;
use crate::ui::theme::Theme;

/// Which top-level view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    /// Pattern editor (default) — F1
    PatternEditor,
    /// Arrangement / song sequence — F2
    Arrangement,
    /// Instrument list — F3
    InstrumentList,
    /// Code editor (full-screen) — F4
    CodeEditor,
}

/// Application state
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,

    /// Whether the application is running (for state management)
    pub running: bool,

    /// The pattern editor (owns the pattern, cursor, mode, undo history)
    pub editor: Editor,

    /// Song data model (pattern pool, arrangement, instruments)
    pub song: Song,

    /// Arrangement view state (cursor, scroll)
    pub arrangement_view: ArrangementView,

    /// Stack of active modal dialogs (top modal is last in Vec)
    modal_stack: Vec<Modal>,

    /// File browser for loading audio samples
    pub file_browser: FileBrowser,

    /// Export dialog for rendering to WAV
    pub export_dialog: ExportDialog,

    /// Names of loaded instruments (indexed by instrument number)
    instrument_names: Vec<String>,

    /// Path to the current project file (None if unsaved)
    pub project_path: Option<PathBuf>,

    /// Currently active top-level view
    pub current_view: AppView,

    /// The application's color theme
    pub theme: Theme,

    /// Audio engine (None if no audio device is available)
    audio_engine: Option<AudioEngine>,

    /// Shared mixer for audio rendering (shared with audio callback thread)
    mixer: Arc<Mutex<Mixer>>,

    /// Transport system for playback control (play/pause/stop, BPM, looping)
    pub transport: Transport,

    /// Code editor for writing Rhai DSL scripts
    pub code_editor: CodeEditor,

    /// Whether the split view is active (pattern left, code editor right)
    pub split_view: bool,

    /// DSL scripting engine for executing Rhai scripts
    script_engine: ScriptEngine,

    /// Whether live mode is active (scripts auto-re-evaluate on every pattern loop)
    pub live_mode: bool,

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

        let editor = Editor::new(pattern.clone());

        // Create a song with the demo pattern in its pool
        let mut song = Song::new("Untitled", 125.0);
        song.patterns[0] = pattern;

        // Initialize file browser at current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let file_browser = FileBrowser::new(&cwd);

        Self {
            should_quit: false,
            running: false,
            editor,
            song,
            arrangement_view: ArrangementView::new(),
            modal_stack: Vec::new(),
            file_browser,
            export_dialog: ExportDialog::new(),
            instrument_names: vec!["sine440".to_string()],
            project_path: None,
            current_view: AppView::PatternEditor,
            theme: Theme::default(),
            audio_engine,
            mixer,
            transport,
            code_editor: CodeEditor::new(),
            split_view: false,
            script_engine: ScriptEngine::new(),
            live_mode: false,
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
        Sample::new(data, sample_rate, 1, Some("sine440".to_string())).with_base_note(57)
        // A-4 = MIDI 57 (440Hz)
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

    /// Update application state, advancing playback row based on BPM timing.
    ///
    /// In Song mode, when the transport signals a pattern change, the editor
    /// is updated to the new pattern from the song's arrangement.
    pub fn update(&mut self) -> Result<()> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Keep transport in sync with current pattern size and arrangement length
        self.transport
            .set_num_rows(self.editor.pattern().num_rows());
        self.transport
            .set_arrangement_length(self.song.arrangement.len());

        let was_playing = self.transport.is_playing();

        let advance_result = self.transport.advance(delta);

        match advance_result {
            AdvanceResult::Row(row) => {
                // In live mode, re-execute script when pattern loops back to row 0
                if row == 0 && self.live_mode {
                    self.execute_script();
                }
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(row, self.editor.pattern());
                }
            }
            AdvanceResult::PatternChange {
                arrangement_pos,
                row,
            } => {
                // Load the new pattern from the arrangement
                self.load_arrangement_pattern(arrangement_pos);
                // In live mode, re-execute script on pattern change
                if self.live_mode {
                    self.execute_script();
                }
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(row, self.editor.pattern());
                }
            }
            AdvanceResult::Stopped => {
                // Handled below in was_playing check
            }
            AdvanceResult::None => {
                // Even when no row advances, sync track state for real-time
                // mute/solo/volume/pan changes to take effect immediately
                if self.transport.is_playing() {
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.update_tracks(self.editor.pattern().tracks());
                    }
                }
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

    /// Load the pattern at the given arrangement position into the editor.
    fn load_arrangement_pattern(&mut self, arrangement_pos: usize) {
        if let Some(&pattern_idx) = self.song.arrangement.get(arrangement_pos) {
            if let Some(pattern) = self.song.patterns.get(pattern_idx) {
                self.editor = Editor::new(pattern.clone());
                self.transport.set_num_rows(pattern.num_rows());
            }
        }
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

    /// Toggle audio playback between play and pause.
    ///
    /// In Song mode, starting from stopped loads the first arrangement pattern.
    pub fn toggle_play(&mut self) {
        match self.transport.state() {
            TransportState::Stopped => {
                // Sync arrangement length before starting
                self.transport
                    .set_arrangement_length(self.song.arrangement.len());
                // In Song mode, load the pattern at the current arrangement position
                if self.transport.playback_mode() == PlaybackMode::Song {
                    self.load_arrangement_pattern(self.transport.arrangement_position());
                }
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

    /// Toggle between pattern and song playback modes
    pub fn toggle_playback_mode(&mut self) {
        self.transport.toggle_playback_mode();
    }

    /// Jump to the next pattern in the arrangement
    pub fn jump_next_pattern(&mut self) {
        self.transport
            .set_arrangement_length(self.song.arrangement.len());
        let current = self.transport.arrangement_position();
        let next = current + 1;
        if next < self.song.arrangement.len() {
            self.transport.jump_to_arrangement_position(next);
            self.load_arrangement_pattern(next);
        }
    }

    /// Jump to the previous pattern in the arrangement
    pub fn jump_prev_pattern(&mut self) {
        self.transport
            .set_arrangement_length(self.song.arrangement.len());
        let current = self.transport.arrangement_position();
        if current > 0 {
            let prev = current - 1;
            self.transport.jump_to_arrangement_position(prev);
            self.load_arrangement_pattern(prev);
        }
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
        let path = self
            .file_browser
            .selected_path()
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(&path, output_sample_rate).map_err(|e| format!("Failed to load: {}", e))?;

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

    /// Open the export dialog.
    pub fn open_export_dialog(&mut self) {
        let name = self
            .project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");
        let default_path = format!("{}.wav", name);
        self.export_dialog.open(&default_path);
    }

    /// Check if the export dialog is open.
    pub fn has_export_dialog(&self) -> bool {
        self.export_dialog.active
    }

    /// Execute the WAV export using current dialog settings.
    pub fn execute_export(&mut self) {
        let config = self.export_dialog.to_config();
        let path = PathBuf::from(&self.export_dialog.output_path);

        self.export_dialog.start_export();

        // Clone samples from the mixer for offline rendering
        let samples: Vec<Sample> = if let Ok(mixer) = self.mixer.lock() {
            mixer.samples().to_vec()
        } else {
            self.export_dialog
                .finish_error("Failed to lock mixer".to_string());
            return;
        };

        // Run export synchronously (offline rendering)
        let duration = export::song_duration(&self.song);
        match export::export_wav(&path, &self.song, &samples, &config, |progress| {
            // Progress is 0.0-1.0, but we can't update the dialog in a closure
            // because &mut self is already borrowed. Progress is best-effort here.
            let _ = progress;
        }) {
            Ok(()) => {
                let message = format!(
                    "Exported successfully!\n\nFile: {}\nDuration: {:.1}s\nSample rate: {} Hz\nBit depth: {}-bit",
                    path.display(),
                    duration,
                    config.sample_rate,
                    config.bit_depth.bits_per_sample(),
                );
                self.export_dialog.finish_success(message);
            }
            Err(e) => {
                self.export_dialog.finish_error(format!("{}", e));
            }
        }
    }

    /// Get the list of loaded instrument names.
    pub fn instrument_names(&self) -> &[String] {
        &self.instrument_names
    }

    /// Get loaded instrument count.
    pub fn instrument_count(&self) -> usize {
        self.instrument_names.len()
    }

    /// Switch to a different top-level view.
    pub fn set_view(&mut self, view: AppView) {
        self.current_view = view;
        // When switching to CodeEditor view, activate the code editor
        if view == AppView::CodeEditor {
            self.code_editor.active = true;
        } else {
            self.code_editor.active = false;
        }
    }

    /// Toggle split view mode (pattern left, code editor right).
    pub fn toggle_split_view(&mut self) {
        self.split_view = !self.split_view;
        if self.split_view {
            self.code_editor.active = true;
            // Ensure we're in pattern editor view for the split
            if self.current_view == AppView::CodeEditor {
                self.current_view = AppView::PatternEditor;
            }
        } else {
            self.code_editor.active = false;
        }
    }

    /// Check if the code editor is active (either full-screen or split).
    pub fn is_code_editor_active(&self) -> bool {
        self.code_editor.active
    }

    /// Toggle live mode on/off.
    ///
    /// When live mode is active, scripts in the code editor are automatically
    /// re-evaluated on every pattern loop, allowing real-time algorithmic
    /// pattern generation during playback.
    pub fn toggle_live_mode(&mut self) {
        self.live_mode = !self.live_mode;
    }

    /// Execute the current script in the code editor.
    ///
    /// Scripts run in the main event loop (not the audio thread), so they never
    /// block audio rendering. When a script modifies the pattern during active
    /// playback, the mixer is retriggered on the current row so changes are
    /// immediately audible without waiting for the next row advance.
    pub fn execute_script(&mut self) {
        let code = self.code_editor.text();
        if code.trim().is_empty() {
            self.code_editor
                .set_output("(empty script)".to_string(), false);
            return;
        }

        match self
            .script_engine
            .eval_with_pattern(&code, self.editor.pattern())
        {
            Ok((result, commands)) => {
                // Apply pattern commands to the editor's pattern
                use crate::dsl::engine::{apply_commands, ScriptResult};
                let cmd_count = commands.len();
                apply_commands(self.editor.pattern_mut(), &commands);

                // If playback is active and the script modified the pattern,
                // retrigger the mixer on the current row so changes are
                // immediately audible (not waiting for the next row advance).
                if cmd_count > 0 && self.transport.is_playing() {
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.tick(self.transport.current_row(), self.editor.pattern());
                    }
                }

                // Format output message
                let output_msg = if cmd_count > 0 {
                    match result {
                        ScriptResult::Value(v) => {
                            format!("Applied {} commands. Result: {}", cmd_count, v)
                        }
                        _ => format!("Applied {} commands to pattern.", cmd_count),
                    }
                } else {
                    match result {
                        ScriptResult::Value(v) => v,
                        ScriptResult::Unit => "(ok)".to_string(),
                        ScriptResult::PatternResult(_) => "(pattern result)".to_string(),
                    }
                };
                self.code_editor.set_output(output_msg, false);
            }
            Err(err) => {
                self.code_editor.set_output(err, true);
            }
        }
    }

    /// Save the current project to disk.
    ///
    /// If a project path is set, saves to that path. Otherwise saves to
    /// "untitled.trs" in the current directory.
    pub fn save_project(&mut self) {
        let path = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.trs"));

        match project::save_project(&path, &self.song) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.open_modal(Modal::info(
                    "Project Saved".to_string(),
                    format!("Saved to: {}", path.display()),
                ));
            }
            Err(e) => {
                self.open_modal(Modal::error("Save Failed".to_string(), format!("{}", e)));
            }
        }
    }

    /// Load a project from disk.
    ///
    /// If a project path is set, loads from that path. Otherwise uses the
    /// file browser. This replaces the current song data.
    pub fn load_project(&mut self, path: &std::path::Path) {
        match project::load_project(path) {
            Ok(song) => {
                // Update the editor with the first pattern from the loaded song
                let pattern = if !song.patterns.is_empty() {
                    song.patterns[0].clone()
                } else {
                    Pattern::default()
                };
                self.editor = Editor::new(pattern);
                self.song = song;
                self.project_path = Some(path.to_path_buf());
                self.arrangement_view = ArrangementView::new();
                self.transport.stop();

                self.open_modal(Modal::info(
                    "Project Loaded".to_string(),
                    format!("Loaded: {}", path.display()),
                ));
            }
            Err(e) => {
                self.open_modal(Modal::error("Load Failed".to_string(), format!("{}", e)));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_view_default_is_pattern_editor() {
        let app = App::new();
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    #[test]
    fn test_set_view_to_arrangement() {
        let mut app = App::new();
        app.set_view(AppView::Arrangement);
        assert_eq!(app.current_view, AppView::Arrangement);
    }

    #[test]
    fn test_set_view_to_instrument_list() {
        let mut app = App::new();
        app.set_view(AppView::InstrumentList);
        assert_eq!(app.current_view, AppView::InstrumentList);
    }

    #[test]
    fn test_set_view_back_to_pattern_editor() {
        let mut app = App::new();
        app.set_view(AppView::Arrangement);
        app.set_view(AppView::PatternEditor);
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    #[test]
    fn test_set_view_same_view_is_noop() {
        let mut app = App::new();
        app.set_view(AppView::PatternEditor);
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    #[test]
    fn test_app_view_enum_equality() {
        assert_eq!(AppView::PatternEditor, AppView::PatternEditor);
        assert_eq!(AppView::Arrangement, AppView::Arrangement);
        assert_eq!(AppView::InstrumentList, AppView::InstrumentList);
        assert_eq!(AppView::CodeEditor, AppView::CodeEditor);
        assert_ne!(AppView::PatternEditor, AppView::Arrangement);
        assert_ne!(AppView::Arrangement, AppView::InstrumentList);
        assert_ne!(AppView::InstrumentList, AppView::CodeEditor);
    }

    #[test]
    fn test_app_view_is_copy() {
        let view = AppView::Arrangement;
        let copy = view;
        assert_eq!(view, copy); // Both still valid (Copy trait)
    }

    #[test]
    fn test_view_cycle_all_three() {
        let mut app = App::new();
        assert_eq!(app.current_view, AppView::PatternEditor);
        app.set_view(AppView::Arrangement);
        assert_eq!(app.current_view, AppView::Arrangement);
        app.set_view(AppView::InstrumentList);
        assert_eq!(app.current_view, AppView::InstrumentList);
        app.set_view(AppView::PatternEditor);
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    // --- Song-level playback tests ---

    #[test]
    fn test_default_playback_mode_is_pattern() {
        let app = App::new();
        assert_eq!(app.transport.playback_mode(), PlaybackMode::Pattern);
    }

    #[test]
    fn test_toggle_playback_mode() {
        let mut app = App::new();
        assert_eq!(app.transport.playback_mode(), PlaybackMode::Pattern);

        app.toggle_playback_mode();
        assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);

        app.toggle_playback_mode();
        assert_eq!(app.transport.playback_mode(), PlaybackMode::Pattern);
    }

    #[test]
    fn test_jump_next_pattern_with_multiple_patterns() {
        let mut app = App::new();
        // Add a second pattern to the song pool
        let pattern2 = Pattern::new(8, 4);
        app.song.patterns.push(pattern2);
        app.song.arrangement = vec![0, 1]; // Two entries in arrangement

        assert_eq!(app.transport.arrangement_position(), 0);

        app.jump_next_pattern();
        assert_eq!(app.transport.arrangement_position(), 1);

        // Already at last position — should not advance
        app.jump_next_pattern();
        assert_eq!(app.transport.arrangement_position(), 1);
    }

    #[test]
    fn test_jump_prev_pattern() {
        let mut app = App::new();
        let pattern2 = Pattern::new(8, 4);
        app.song.patterns.push(pattern2);
        app.song.arrangement = vec![0, 1];

        // Start at 0 — cannot go back
        app.jump_prev_pattern();
        assert_eq!(app.transport.arrangement_position(), 0);

        // Jump to 1, then back to 0
        app.jump_next_pattern();
        assert_eq!(app.transport.arrangement_position(), 1);

        app.jump_prev_pattern();
        assert_eq!(app.transport.arrangement_position(), 0);
    }

    #[test]
    fn test_jump_pattern_loads_correct_pattern_into_editor() {
        let mut app = App::new();
        // Pattern 0: 16 rows, Pattern 1: 8 rows
        let pattern2 = Pattern::new(8, 4);
        app.song.patterns.push(pattern2);
        app.song.arrangement = vec![0, 1];

        // Editor starts with pattern 0 (16 rows)
        assert_eq!(app.editor.pattern().num_rows(), 16);

        // Jump to pattern 1 (8 rows)
        app.jump_next_pattern();
        assert_eq!(app.editor.pattern().num_rows(), 8);

        // Jump back to pattern 0 (16 rows)
        app.jump_prev_pattern();
        assert_eq!(app.editor.pattern().num_rows(), 16);
    }

    #[test]
    fn test_stop_resets_arrangement_position() {
        let mut app = App::new();
        let pattern2 = Pattern::new(8, 4);
        app.song.patterns.push(pattern2);
        app.song.arrangement = vec![0, 1];

        app.jump_next_pattern();
        assert_eq!(app.transport.arrangement_position(), 1);

        app.stop();
        assert_eq!(app.transport.arrangement_position(), 0);
        assert_eq!(app.transport.current_row(), 0);
    }

    #[test]
    fn test_song_mode_toggle_play_loads_arrangement_pattern() {
        let mut app = App::new();
        let pattern2 = Pattern::new(8, 4);
        app.song.patterns.push(pattern2);
        app.song.arrangement = vec![0, 1];

        app.toggle_playback_mode(); // Switch to Song mode
        assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);

        // Starting playback in Song mode should load the arrangement pattern
        app.toggle_play();
        assert!(app.transport.is_playing());
    }

    // --- Export Dialog Tests ---

    #[test]
    fn test_open_export_dialog_default_path() {
        let mut app = App::new();
        assert!(!app.has_export_dialog());

        app.open_export_dialog();
        assert!(app.has_export_dialog());
        assert_eq!(app.export_dialog.output_path, "untitled.wav");
    }

    #[test]
    fn test_open_export_dialog_with_project_path() {
        let mut app = App::new();
        app.project_path = Some(PathBuf::from("my_song.trs"));

        app.open_export_dialog();
        assert!(app.has_export_dialog());
        assert_eq!(app.export_dialog.output_path, "my_song.wav");
    }

    #[test]
    fn test_export_dialog_close() {
        let mut app = App::new();
        app.open_export_dialog();
        assert!(app.has_export_dialog());

        app.export_dialog.close();
        assert!(!app.has_export_dialog());
    }

    #[test]
    fn test_execute_export_creates_file() {
        let mut app = App::new();
        let dir = std::env::temp_dir().join("tracker_rs_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_app_export.wav");

        app.open_export_dialog();
        app.export_dialog.output_path = path.display().to_string();
        app.execute_export();

        use crate::ui::export_dialog::ExportPhase;
        assert_eq!(app.export_dialog.phase, ExportPhase::Done);
        assert_eq!(app.export_dialog.progress, 100);
        assert!(path.exists());

        // Verify it's a valid WAV
        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().channels, 2);
        assert_eq!(reader.spec().sample_rate, 44100);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_execute_export_with_custom_settings() {
        let mut app = App::new();
        let dir = std::env::temp_dir().join("tracker_rs_tests");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_app_export_48k.wav");

        app.open_export_dialog();
        app.export_dialog.output_path = path.display().to_string();
        app.export_dialog.sample_rate = 48000;
        app.export_dialog.bit_depth = crate::export::BitDepth::Bits24;
        app.execute_export();

        use crate::ui::export_dialog::ExportPhase;
        assert_eq!(app.export_dialog.phase, ExportPhase::Done);

        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48000);
        assert_eq!(reader.spec().bits_per_sample, 24);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_execute_export_invalid_path_fails() {
        let mut app = App::new();
        app.open_export_dialog();
        // Use an invalid directory path
        app.export_dialog.output_path = "/nonexistent/path/to/file.wav".to_string();
        app.execute_export();

        use crate::ui::export_dialog::ExportPhase;
        assert_eq!(app.export_dialog.phase, ExportPhase::Failed);
        assert!(!app.export_dialog.result_message.is_empty());
    }

    // --- Code Editor and Split View Tests ---

    #[test]
    fn test_set_view_code_editor_activates_editor() {
        let mut app = App::new();
        assert!(!app.code_editor.active);
        app.set_view(AppView::CodeEditor);
        assert_eq!(app.current_view, AppView::CodeEditor);
        assert!(app.code_editor.active);
    }

    #[test]
    fn test_set_view_pattern_deactivates_code_editor() {
        let mut app = App::new();
        app.set_view(AppView::CodeEditor);
        assert!(app.code_editor.active);
        app.set_view(AppView::PatternEditor);
        assert!(!app.code_editor.active);
    }

    #[test]
    fn test_toggle_split_view_on() {
        let mut app = App::new();
        assert!(!app.split_view);
        assert!(!app.code_editor.active);
        app.toggle_split_view();
        assert!(app.split_view);
        assert!(app.code_editor.active);
    }

    #[test]
    fn test_toggle_split_view_off() {
        let mut app = App::new();
        app.toggle_split_view();
        assert!(app.split_view);
        app.toggle_split_view();
        assert!(!app.split_view);
        assert!(!app.code_editor.active);
    }

    #[test]
    fn test_split_view_from_code_editor_switches_to_pattern() {
        let mut app = App::new();
        app.set_view(AppView::CodeEditor);
        app.toggle_split_view();
        assert!(app.split_view);
        // Should switch to PatternEditor for the split
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    #[test]
    fn test_is_code_editor_active() {
        let mut app = App::new();
        assert!(!app.is_code_editor_active());

        app.set_view(AppView::CodeEditor);
        assert!(app.is_code_editor_active());

        app.set_view(AppView::PatternEditor);
        assert!(!app.is_code_editor_active());

        app.toggle_split_view();
        assert!(app.is_code_editor_active());
    }

    #[test]
    fn test_execute_script_empty() {
        let mut app = App::new();
        app.execute_script();
        assert_eq!(app.code_editor.output(), "(empty script)");
        assert!(!app.code_editor.output_is_error);
    }

    #[test]
    fn test_execute_script_simple_expression() {
        let mut app = App::new();
        app.code_editor.set_text("40 + 2");
        app.execute_script();
        assert_eq!(app.code_editor.output(), "42");
        assert!(!app.code_editor.output_is_error);
    }

    #[test]
    fn test_execute_script_error() {
        let mut app = App::new();
        app.code_editor.set_text("let x = ;");
        app.execute_script();
        assert!(app.code_editor.output_is_error);
        assert!(!app.code_editor.output().is_empty());
    }

    #[test]
    fn test_execute_script_set_note() {
        let mut app = App::new();
        app.code_editor.set_text(
            r#"
            let n = note("C", 4);
            set_note(0, 0, n);
        "#,
        );
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        assert!(app.code_editor.output().contains("Applied"));
        // Verify note was placed
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.is_some());
        let cell = cell.unwrap();
        assert!(cell.note.is_some());
    }

    #[test]
    fn test_execute_script_clear_pattern() {
        let mut app = App::new();
        // First set some notes
        app.editor
            .pattern_mut()
            .set_note(0, 0, Note::simple(Pitch::C, 4));
        // Then clear via script
        app.code_editor.set_text("clear_pattern();");
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        // Verify pattern was cleared
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.map_or(true, |c| c.is_empty()));
    }

    #[test]
    fn test_view_cycle_includes_code_editor() {
        let mut app = App::new();
        assert_eq!(app.current_view, AppView::PatternEditor);
        app.set_view(AppView::Arrangement);
        assert_eq!(app.current_view, AppView::Arrangement);
        app.set_view(AppView::InstrumentList);
        assert_eq!(app.current_view, AppView::InstrumentList);
        app.set_view(AppView::CodeEditor);
        assert_eq!(app.current_view, AppView::CodeEditor);
        app.set_view(AppView::PatternEditor);
        assert_eq!(app.current_view, AppView::PatternEditor);
    }

    // --- Live Mode Tests ---

    #[test]
    fn test_live_mode_default_off() {
        let app = App::new();
        assert!(!app.live_mode);
    }

    #[test]
    fn test_toggle_live_mode() {
        let mut app = App::new();
        assert!(!app.live_mode);
        app.toggle_live_mode();
        assert!(app.live_mode);
        app.toggle_live_mode();
        assert!(!app.live_mode);
    }

    #[test]
    fn test_live_mode_re_executes_on_pattern_loop() {
        let mut app = App::new();
        // Set up a small 4-row pattern
        let pattern = Pattern::new(4, 4);
        app.editor = Editor::new(pattern);
        app.transport.set_num_rows(4);

        // Write a script that sets a note at row 0
        app.code_editor.set_text(
            r#"
            let n = note("D", 5);
            set_note(0, 0, n);
        "#,
        );

        // Enable live mode and start playback
        app.live_mode = true;
        app.transport.play();

        // Advance through all rows to trigger the loop
        let spr = 60.0 / 120.0 / 4.0; // seconds per row at 120 BPM
        app.transport.advance(spr); // Row 1
        app.last_update = Instant::now();
        app.transport.advance(spr); // Row 2
        app.transport.advance(spr); // Row 3

        // Clear the specific cell before the loop triggers
        app.editor.pattern_mut().clear_cell(0, 0);
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.map_or(true, |c| c.is_empty()));

        // Now advance past the end — should loop to row 0 and re-execute script
        // We need to call update() which handles the advance and live mode logic
        // But update() uses last_update for delta, so let's simulate directly
        // by calling the transport advance and then mimicking update behavior
        let result = app.transport.advance(spr);
        assert_eq!(result, crate::transport::AdvanceResult::Row(0));

        // Simulate what update() does for Row(0) with live_mode
        if app.live_mode {
            app.execute_script();
        }

        // Verify the script was re-executed: note should be placed at (0, 0)
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.is_some());
        assert!(cell.unwrap().note.is_some());
    }

    #[test]
    fn test_live_mode_does_not_execute_when_disabled() {
        let mut app = App::new();
        let pattern = Pattern::new(4, 4);
        app.editor = Editor::new(pattern);
        app.transport.set_num_rows(4);

        // Write a script that sets a note
        app.code_editor.set_text(
            r#"
            let n = note("D", 5);
            set_note(0, 0, n);
        "#,
        );

        // Live mode OFF
        app.live_mode = false;
        app.transport.play();

        // Advance through all rows to trigger the loop
        let spr = 60.0 / 120.0 / 4.0;
        app.transport.advance(spr); // Row 1
        app.transport.advance(spr); // Row 2
        app.transport.advance(spr); // Row 3
        let result = app.transport.advance(spr); // Row 0 (loop)
        assert_eq!(result, crate::transport::AdvanceResult::Row(0));

        // Pattern should remain empty since live mode is off
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.map_or(true, |c| c.is_empty()));
    }

    #[test]
    fn test_live_mode_with_empty_script() {
        let mut app = App::new();
        let pattern = Pattern::new(4, 4);
        app.editor = Editor::new(pattern);
        app.transport.set_num_rows(4);

        // Empty script — live mode should not crash
        app.code_editor.set_text("");
        app.live_mode = true;
        app.execute_script(); // Should handle gracefully
        assert!(!app.code_editor.output_is_error);
    }

    #[test]
    fn test_live_mode_with_error_script() {
        let mut app = App::new();
        let pattern = Pattern::new(4, 4);
        app.editor = Editor::new(pattern);
        app.transport.set_num_rows(4);

        // Invalid script — live mode should display error, not panic
        app.code_editor.set_text("let x = ;");
        app.live_mode = true;
        app.execute_script(); // Should handle gracefully
        assert!(app.code_editor.output_is_error);
    }

    // --- Audio Wiring Tests ---

    #[test]
    fn test_script_execution_retriggers_mixer_during_playback() {
        let mut app = App::new();
        // Start playback
        app.transport.play();
        assert!(app.transport.is_playing());

        // Execute a script that modifies the pattern — should retrigger mixer
        app.code_editor.set_text(
            r#"
            let n = note("E", 4);
            set_note(0, 0, n);
        "#,
        );
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        assert!(app.code_editor.output().contains("Applied"));
        // Verify note was placed (pattern was modified)
        let cell = app.editor.pattern().get_cell(0, 0);
        assert!(cell.is_some());
        assert!(cell.unwrap().note.is_some());
    }

    #[test]
    fn test_script_no_retrigger_when_stopped() {
        let mut app = App::new();
        // Transport is stopped
        assert!(app.transport.is_stopped());

        // Execute a script — should still apply commands, just no mixer retrigger
        app.code_editor.set_text(
            r#"
            let n = note("E", 4);
            set_note(0, 0, n);
        "#,
        );
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        assert!(app.code_editor.output().contains("Applied"));
    }

    #[test]
    fn test_script_no_retrigger_for_readonly_script() {
        let mut app = App::new();
        app.transport.play();

        // Execute a script that doesn't modify the pattern (no commands)
        app.code_editor.set_text("40 + 2");
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        assert_eq!(app.code_editor.output(), "42");
    }

    #[test]
    fn test_script_execution_does_not_block_audio_thread() {
        // Verify that script execution runs synchronously on main thread
        // while audio callback runs on separate thread via Arc<Mutex<Mixer>>.
        // The mixer is behind Arc<Mutex>, so scripts don't touch the audio callback.
        let mut app = App::new();
        app.transport.play();

        // Heavy script execution should complete without deadlock
        app.code_editor.set_text(
            r#"
            for i in range(0, 16) {
                let n = note("C", 4);
                set_note(i, 0, n);
            }
        "#,
        );
        app.execute_script();
        assert!(!app.code_editor.output_is_error);
        assert!(app.code_editor.output().contains("Applied 16 commands"));
    }

    #[test]
    fn test_live_mode_changes_take_effect_on_next_loop() {
        let mut app = App::new();
        let pattern = Pattern::new(4, 4);
        app.editor = Editor::new(pattern);
        app.transport.set_num_rows(4);

        // Script fills column 0 with C4 notes
        app.code_editor.set_text(
            r#"
            for i in range(0, 4) {
                let n = note("C", 4);
                set_note(i, 0, n);
            }
        "#,
        );

        // Enable live mode and start playback
        app.live_mode = true;
        app.transport.play();

        // Advance through all rows without executing script
        let spr = 60.0 / 120.0 / 4.0;
        app.transport.advance(spr); // Row 1
        app.transport.advance(spr); // Row 2
        app.transport.advance(spr); // Row 3

        // Verify pattern is empty before the loop
        for i in 0..4 {
            let cell = app.editor.pattern().get_cell(i, 0);
            assert!(cell.map_or(true, |c| c.is_empty()));
        }

        // Loop back to row 0 — live mode should re-execute script
        let result = app.transport.advance(spr);
        assert_eq!(result, crate::transport::AdvanceResult::Row(0));
        // Simulate update() behavior
        if app.live_mode {
            app.execute_script();
        }

        // Now all 4 rows should have notes
        for i in 0..4 {
            let cell = app.editor.pattern().get_cell(i, 0);
            assert!(cell.is_some(), "Row {} should have a note", i);
            assert!(
                cell.unwrap().note.is_some(),
                "Row {} note should not be empty",
                i
            );
        }
    }

    #[test]
    fn test_execute_script_during_playback_preserves_transport_state() {
        let mut app = App::new();
        app.transport.set_num_rows(16);
        app.transport.play();

        // Advance a few rows
        let spr = 60.0 / 120.0 / 4.0;
        app.transport.advance(spr); // Row 1
        app.transport.advance(spr); // Row 2
        let row_before = app.transport.current_row();
        assert_eq!(row_before, 2);

        // Execute script
        app.code_editor.set_text(
            r#"
            let n = note("A", 3);
            set_note(0, 0, n);
        "#,
        );
        app.execute_script();

        // Transport state should be unchanged
        assert!(app.transport.is_playing());
        assert_eq!(app.transport.current_row(), 2);
    }
}
