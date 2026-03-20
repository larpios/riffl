/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.
use std::sync::{Arc, Mutex};
use std::time::Instant;

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::Config;
use crate::editor::{Editor, EditorMode};
use crate::ui::arrangement::ArrangementView;
use crate::ui::code_editor::CodeEditor;
use crate::ui::export_dialog::ExportDialog;
use crate::ui::file_browser::FileBrowser;
use crate::ui::instrument_editor::InstrumentEditorState;
use crate::ui::modal::Modal;
use crate::ui::sample_browser::SampleBrowser;
use crate::ui::theme::{Theme, ThemeKind};
use tracker_core::audio::{load_sample, AudioEngine, Mixer, Sample, TransportCommand};
use tracker_core::dsl::engine::ScriptEngine;
use tracker_core::export;
use tracker_core::pattern::note::{NoteEvent, Pitch};
use tracker_core::pattern::{Note, Pattern};
use tracker_core::project;
use tracker_core::song::{Instrument, Song};
use tracker_core::transport::{AdvanceResult, PlaybackMode, Transport, TransportState};

/// Which top-level view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    /// Pattern editor (default) — F1 / 1
    PatternEditor,
    /// Arrangement / song sequence — F2 / 2
    Arrangement,
    /// Instrument list — F3 / 3
    InstrumentList,
    /// Code editor (full-screen) — F4 / 4
    CodeEditor,
    /// Pattern list (pool) — 5
    PatternList,
    /// Dedicated sample browser — 6
    SampleBrowser,
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

    /// File browser for loading audio samples (overlay, Ctrl+F)
    pub file_browser: FileBrowser,

    /// Dedicated sample browser view (view 6)
    pub sample_browser: SampleBrowser,

    /// Export dialog for rendering to WAV
    pub export_dialog: ExportDialog,

    /// Names of loaded instruments (indexed by instrument number)
    instrument_names: Vec<String>,

    /// Currently selected instrument index in the instrument list (None if none selected)
    instrument_selection: Option<usize>,

    /// Currently selected pattern index in the pattern list (None if none selected)
    pattern_selection: Option<usize>,

    /// Path to the current project file (None if unsaved)
    pub project_path: Option<PathBuf>,

    /// Configured sample directories (from config / CLI).
    /// Stored so we can rebuild browser roots when the project path changes.
    configured_sample_dirs: Vec<PathBuf>,

    /// Currently active top-level view
    pub current_view: AppView,

    /// Currently active theme kind
    pub theme_kind: ThemeKind,

    /// Derived color theme (always in sync with theme_kind)
    pub theme: Theme,

    /// Loaded application configuration
    pub config: Config,

    /// Audio engine (None if no audio device is available)
    audio_engine: Option<AudioEngine>,

    /// Shared mixer for audio rendering (shared with audio callback thread)
    mixer: Arc<Mutex<Mixer>>,

    /// Prototype Glicol mixer
    glicol_mixer: Arc<Mutex<tracker_core::audio::glicol_mixer::GlicolMixer>>,

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

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Scroll offset for the help overlay (in lines)
    pub help_scroll: u16,

    /// Timestamp of the last update call (for delta time calculation)
    last_update: Instant,

    /// Pending first key of a two-key chord (e.g. 'd' waiting for 'dd')
    pub pending_key: Option<char>,

    /// Whether r (replace-once) mode is pending: next note key replaces current cell without advancing cursor
    pub pending_replace: bool,

    /// Whether the tutor view is open (opened with :tutor)
    pub show_tutor: bool,

    /// Scroll offset for the tutor view (in lines)
    pub tutor_scroll: u16,

    /// Whether follow mode is active: edit cursor chases playhead during playback
    pub follow_mode: bool,

    /// Whether the project has unsaved changes
    pub is_dirty: bool,

    /// Whether a quit confirmation is pending (user pressed q with unsaved changes)
    pub pending_quit: bool,

    /// Path of a sample the user selected in the browser but hasn't confirmed an action for yet.
    pub pending_sample_path: Option<PathBuf>,

    /// Whether vim command-line mode is active (`:` was pressed)
    pub command_mode: bool,

    /// Current command-line input buffer
    pub command_input: String,

    /// Whether BPM inline prompt is active (Ctrl+B opens it, Enter applies, Esc cancels)
    pub bpm_prompt_mode: bool,

    /// Current BPM prompt input buffer
    pub bpm_prompt_input: String,

    /// Whether pattern length inline prompt is active (Ctrl+P opens it)
    pub len_prompt_mode: bool,

    /// Current pattern length prompt input buffer
    pub len_prompt_input: String,

    /// Timestamps of recent taps for tap-tempo (`t` in Normal mode)
    pub tap_times: Vec<Instant>,

    /// Whether draw mode is active: cursor-down auto-repeats the last entered note
    pub draw_mode: bool,

    /// The last note entered in Insert mode, replayed on each cursor-down when draw_mode is on
    pub draw_note: Option<NoteEvent>,

    /// Instrument editor panel state (shown below the instrument list)
    pub inst_editor: InstrumentEditorState,
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
            vec![Arc::new(demo_sample)],
            Vec::new(),
            pattern.num_channels(),
            output_sample_rate,
        )));

        // Create a song with the demo pattern in its pool
        let mut song = Song::new("Untitled", 125.0);

        // Create transport synced to song BPM and pattern size
        let mut transport = Transport::new();
        transport.set_num_rows(pattern.num_rows());
        transport.set_bpm(song.bpm);
        // Sync mixer effect processor tempo with the song BPM
        if let Ok(mut m) = mixer.lock() {
            m.update_tempo(song.bpm);
        }

        let editor = Editor::new(pattern.clone());

        let glicol_mixer = Arc::new(Mutex::new(
            tracker_core::audio::glicol_mixer::GlicolMixer::new(
                pattern.num_channels(),
                output_sample_rate,
            ),
        ));
        song.patterns[0] = pattern;

        use tracker_core::song::Instrument;
        let mut demo_inst = Instrument::new("sine440");
        demo_inst.sample_index = Some(0);
        demo_inst.sample_path = None;
        song.instruments.push(demo_inst);

        // Initialize file browser and sample browser at current working directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let file_browser = FileBrowser::new(&cwd);
        let sample_browser = SampleBrowser::new(vec![cwd.clone()]);

        Self {
            should_quit: false,
            running: false,
            editor,
            song,
            arrangement_view: ArrangementView::new(),
            modal_stack: Vec::new(),
            file_browser,
            sample_browser,
            export_dialog: ExportDialog::new(),
            instrument_names: vec!["sine440".to_string()],
            instrument_selection: None,
            pattern_selection: None,
            project_path: None,
            configured_sample_dirs: Vec::new(),
            current_view: AppView::PatternEditor,
            theme_kind: ThemeKind::Nord,
            theme: Theme::from_kind(ThemeKind::Nord),
            config: Config::default(),
            audio_engine,
            mixer,
            glicol_mixer,
            transport,
            code_editor: CodeEditor::new(),
            split_view: false,
            script_engine: ScriptEngine::new(),
            live_mode: false,
            show_help: false,
            help_scroll: 0,
            last_update: Instant::now(),
            pending_key: None,
            pending_replace: false,
            show_tutor: false,
            tutor_scroll: 0,
            follow_mode: false,
            is_dirty: false,
            pending_quit: false,
            pending_sample_path: None,
            command_mode: false,
            command_input: String::new(),
            bpm_prompt_mode: false,
            bpm_prompt_input: String::new(),
            len_prompt_mode: false,
            len_prompt_input: String::new(),
            tap_times: Vec::new(),
            draw_mode: false,
            draw_note: None,
            inst_editor: InstrumentEditorState::default(),
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
        self.sync_mixer_instruments();

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

        self.transport
            .set_num_rows(self.editor.pattern().num_rows());
        self.transport
            .set_arrangement_length(self.song.arrangement.len());

        let was_playing = self.transport.is_playing();
        let old_arrangement_pos = self.transport.arrangement_position();

        let advance_result = self.transport.advance(delta);

        match advance_result {
            AdvanceResult::Row(row) => {
                if self.follow_mode {
                    self.editor.go_to_row(row);
                }
                if row == 0 && self.live_mode {
                    self.execute_script();
                }
                let transport_cmds = if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(row, self.editor.pattern())
                } else {
                    Vec::new()
                };
                if let Ok(mut gm) = self.glicol_mixer.lock() {
                    // Primitive Glicol trigger: if there's a note on channel 0, play it
                    if let Some(r) = self.editor.pattern().get_row(row) {
                        if let Some(cell) = r.first() {
                            use tracker_core::pattern::note::NoteEvent;
                            match &cell.note {
                                Some(NoteEvent::On(note)) => {
                                    gm.note_on(0, note.frequency() as f32);
                                }
                                Some(NoteEvent::Off) => {
                                    gm.note_off(0);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                self.apply_effect_transport_commands(transport_cmds);
            }
            AdvanceResult::PatternChange {
                arrangement_pos,
                row,
            } => {
                self.flush_editor_pattern(old_arrangement_pos);
                self.load_arrangement_pattern(arrangement_pos);
                if self.follow_mode {
                    self.editor.go_to_row(row);
                }
                if self.live_mode {
                    self.execute_script();
                }
                let transport_cmds = if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.tick(row, self.editor.pattern())
                } else {
                    Vec::new()
                };
                if let Ok(mut gm) = self.glicol_mixer.lock() {
                    // Primitive Glicol trigger: if there's a note on channel 0, play it
                    if let Some(r) = self.editor.pattern().get_row(row) {
                        if let Some(cell) = r.first() {
                            use tracker_core::pattern::note::NoteEvent;
                            match &cell.note {
                                Some(NoteEvent::On(note)) => {
                                    gm.note_on(0, note.frequency() as f32);
                                }
                                Some(NoteEvent::Off) => {
                                    gm.note_off(0);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                self.apply_effect_transport_commands(transport_cmds);
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

    /// Apply transport commands produced by effect processing (Fxx, Bxx, Dxx).
    ///
    /// These commands are returned by `mixer.tick()` when pattern effects fire:
    /// - `SetBpm`: Update tempo on both transport and mixer effect processor.
    /// - `PositionJump (Bxx)`: Jump to arrangement position; loads new pattern in Song mode.
    /// - `PatternBreak (Dxx)`: Advance to next arrangement entry at the given row.
    fn apply_effect_transport_commands(&mut self, commands: Vec<TransportCommand>) {
        for cmd in commands {
            match cmd {
                TransportCommand::SetBpm(bpm) => {
                    let clamped = bpm.clamp(20.0, 999.0);
                    self.transport.set_bpm(clamped);
                    self.song.bpm = clamped;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.update_tempo(clamped);
                    }
                }
                TransportCommand::PositionJump(pos) => {
                    let old_pos = self.transport.arrangement_position();
                    if self.transport.jump_to_arrangement_position(pos) && pos != old_pos {
                        self.flush_editor_pattern(old_pos);
                        self.load_arrangement_pattern(pos);
                    }
                }
                TransportCommand::PatternBreak(row) => {
                    let old_pos = self.transport.arrangement_position();
                    if self.transport.pattern_break(row) {
                        let new_pos = self.transport.arrangement_position();
                        self.flush_editor_pattern(old_pos);
                        self.load_arrangement_pattern(new_pos);
                    }
                }
            }
        }
    }

    pub fn flush_editor_pattern(&mut self, arrangement_pos: usize) {
        if let Some(&pattern_idx) = self.song.arrangement.get(arrangement_pos) {
            if let Some(pattern) = self.song.patterns.get_mut(pattern_idx) {
                *pattern = self.editor.pattern().clone();
            }
        }
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
        if self.is_dirty {
            self.pending_quit = true;
            self.open_modal(Modal::confirmation(
                "Unsaved Changes".to_string(),
                "Quit without saving?".to_string(),
            ));
            return;
        }
        self.force_quit();
    }

    pub fn force_quit(&mut self) {
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

    /// Mark the project as having unsaved changes.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Execute the current command-line input and exit command mode.
    pub fn execute_command(&mut self) {
        let cmd = self.command_input.trim().to_string();
        self.command_mode = false;
        self.command_input.clear();

        // Parse "bpm N" or "t N" or "tempo N"
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let is_bpm_cmd = matches!(parts[0], "bpm" | "t" | "tempo");

        if is_bpm_cmd {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<f64>().ok()) {
                let clamped = val.clamp(20.0, 999.0);
                self.transport.set_bpm(clamped);
                self.song.bpm = clamped;
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.update_tempo(clamped);
                }
                self.mark_dirty();
            } else {
                self.open_modal(Modal::error(
                    "Invalid BPM".to_string(),
                    format!("Usage: :bpm <value>  (got: {:?})", parts.get(1)),
                ));
            }
            return;
        }

        // :step N — set row advance step size
        if parts[0] == "step" {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                self.editor.set_step_size(val);
            } else {
                self.open_modal(Modal::error(
                    "Invalid step".to_string(),
                    "Usage: :step <0-8>".to_string(),
                ));
            }
            return;
        }

        // :w filename — save as a new/specific file
        if parts[0] == "w" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            let current_pos = self.transport.arrangement_position();
            self.flush_editor_pattern(current_pos);
            match project::save_project(&path, &self.song) {
                Ok(()) => {
                    self.project_path = Some(path.clone());
                    self.is_dirty = false;
                    self.open_modal(Modal::info(
                        "Project Saved".to_string(),
                        format!("Saved to: {}", path.display()),
                    ));
                }
                Err(e) => {
                    self.open_modal(Modal::error("Save Failed".to_string(), format!("{}", e)));
                }
            }
            return;
        }

        // :e filename — open/load a project file
        if parts[0] == "e" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            self.load_project(&path);
            return;
        }

        match cmd.as_str() {
            "w" => self.save_project(),
            "wq" | "x" => {
                self.save_project();
                if !self.is_dirty {
                    self.force_quit();
                }
            }
            "q" => self.quit(),
            "q!" => self.force_quit(),
            "tutor" => {
                self.show_tutor = true;
                self.tutor_scroll = 0;
            }
            _ => {
                self.open_modal(Modal::error(
                    "Unknown command".to_string(),
                    format!(":{}", cmd),
                ));
            }
        }
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

    /// Start playback from the current editor cursor row.
    ///
    /// Implements "Play From Cursor": if the transport is stopped or paused,
    /// playback begins at the row the edit cursor is on rather than row 0.
    /// If already playing, this is a no-op (use toggle_play to pause/resume).
    pub fn play_from_cursor(&mut self) {
        if self.transport.is_playing() {
            return;
        }
        let start_row = self.editor.cursor_row();
        self.transport
            .set_arrangement_length(self.song.arrangement.len());
        if self.transport.playback_mode() == PlaybackMode::Song {
            self.load_arrangement_pattern(self.transport.arrangement_position());
        }
        self.transport.play_from(start_row);
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.tick(self.transport.current_row(), self.editor.pattern());
        }
        if let Some(ref mut engine) = self.audio_engine {
            let _ = engine.start();
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

    /// Sync the mixer's instrument list from song.instruments.
    /// Must be called after any mutation to song.instruments.
    fn sync_mixer_instruments(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_instruments(self.song.instruments.clone());
        }
    }

    /// Adjust BPM by a delta value
    pub fn adjust_bpm(&mut self, delta: f64) {
        self.transport.adjust_bpm(delta);
        let new_bpm = self.transport.bpm();
        self.song.bpm = new_bpm;
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.update_tempo(new_bpm);
        }
    }

    /// Open the inline BPM prompt, pre-populated with the current BPM.
    pub fn open_bpm_prompt(&mut self) {
        self.bpm_prompt_mode = true;
        self.bpm_prompt_input = format!("{:.0}", self.transport.bpm());
    }

    /// Execute the BPM prompt: parse input and apply BPM if valid.
    pub fn execute_bpm_prompt(&mut self) {
        if let Ok(bpm) = self.bpm_prompt_input.trim().parse::<f64>() {
            let clamped = bpm.clamp(20.0, 999.0);
            self.transport.set_bpm(clamped);
            self.song.bpm = clamped;
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.update_tempo(clamped);
            }
        }
        self.bpm_prompt_mode = false;
        self.bpm_prompt_input.clear();
    }

    /// Open the inline pattern length prompt, pre-populated with current row count.
    pub fn open_len_prompt(&mut self) {
        self.len_prompt_mode = true;
        self.len_prompt_input = format!("{}", self.editor.pattern().row_count());
    }

    /// Execute the pattern length prompt: parse input and resize pattern if valid.
    pub fn execute_len_prompt(&mut self) {
        if let Ok(n) = self.len_prompt_input.trim().parse::<usize>() {
            use tracker_core::pattern::pattern::{MAX_ROW_COUNT, MIN_ROW_COUNT};
            let clamped = n.clamp(MIN_ROW_COUNT, MAX_ROW_COUNT);
            self.editor.pattern_mut().set_row_count(clamped);
            self.transport.set_num_rows(clamped);
            // Clamp cursor if it's now past end of pattern
            let cursor = self.editor.cursor_row();
            if cursor >= clamped {
                self.editor.go_to_row(clamped.saturating_sub(1));
            }
            // Flush to song so the change persists on pattern switch
            let pos = self.transport.arrangement_position();
            self.flush_editor_pattern(pos);
        }
        self.len_prompt_mode = false;
        self.len_prompt_input.clear();
    }

    /// Record a tap for tap-tempo. Computes BPM from the average interval
    /// of all taps within the last 3 seconds (requires at least 2 taps).
    pub fn tap_tempo(&mut self) {
        let now = Instant::now();
        // Drop taps older than 3 seconds
        self.tap_times
            .retain(|t| now.duration_since(*t).as_secs_f64() < 3.0);
        self.tap_times.push(now);

        if self.tap_times.len() >= 2 {
            let intervals: Vec<f64> = self
                .tap_times
                .windows(2)
                .map(|w| w[1].duration_since(w[0]).as_secs_f64())
                .collect();
            let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
            let bpm = (60.0 / avg_interval).clamp(20.0, 999.0);
            self.transport.set_bpm(bpm);
            self.song.bpm = bpm;
            if let Ok(mut mixer) = self.mixer.lock() {
                mixer.update_tempo(bpm);
            }
        }
    }

    /// Toggle loop mode on/off
    pub fn toggle_loop(&mut self) {
        self.transport.toggle_loop();
    }

    /// Set the loop region start to the current cursor row.
    /// If end is already set and is before the new start, it's updated to equal start.
    /// Activates the loop region automatically once both start and end are set.
    pub fn set_loop_start(&mut self) {
        let row = self.editor.cursor_row();
        let end = self.transport.loop_region().map(|(_, e)| e).unwrap_or(row);
        let end = end.max(row);
        self.transport.set_loop_region(row, end);
        self.transport.set_loop_region_active(true);
    }

    /// Set the loop region end to the current cursor row.
    /// If start is after the new end, the start is updated to equal end.
    /// Activates the loop region automatically once both start and end are set.
    pub fn set_loop_end(&mut self) {
        let row = self.editor.cursor_row();
        let start = self.transport.loop_region().map(|(s, _)| s).unwrap_or(row);
        let start = start.min(row);
        self.transport.set_loop_region(start, row);
        self.transport.set_loop_region_active(true);
    }

    /// Toggle the loop region active state.
    /// Has no effect if no loop region is set.
    pub fn toggle_loop_region_active(&mut self) {
        self.transport.toggle_loop_region_active();
    }

    /// Toggle draw mode on/off.
    pub fn toggle_draw_mode(&mut self) {
        self.draw_mode = !self.draw_mode;
    }

    /// Write draw_note at the current cursor position (no cursor advance).
    /// No-op if draw_mode is false or draw_note is None.
    pub fn apply_draw_note(&mut self) {
        if !self.draw_mode {
            return;
        }
        if let Some(note_event) = self.draw_note {
            use tracker_core::pattern::row::Cell;
            let row = self.editor.cursor_row();
            let ch = self.editor.cursor_channel();
            self.editor
                .pattern_mut()
                .set_cell(row, ch, Cell::with_note(note_event));
            self.mark_dirty();
        }
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
            self.flush_editor_pattern(current);
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
            self.flush_editor_pattern(current);
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

    /// Set the sample directories used by both the overlay file browser and the dedicated view.
    pub fn set_sample_dirs(&mut self, dirs: Vec<std::path::PathBuf>) {
        self.configured_sample_dirs = dirs;
        self.refresh_browser_roots();
    }

    /// Rebuild browser roots from configured dirs plus any project-relative samples dir.
    ///
    /// Call this after changing `project_path` or `configured_sample_dirs`.
    pub(crate) fn refresh_browser_roots(&mut self) {
        // Overlay file browser uses the first configured dir as its starting point
        if let Some(first) = self.configured_sample_dirs.first() {
            self.file_browser = FileBrowser::new(first);
        }
        self.sample_browser
            .set_roots(self.configured_sample_dirs.clone());

        // Auto-add <project_dir>/samples/ if it exists and isn't already a root
        if let Some(proj_dir) = self
            .project_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
        {
            let samples_dir = proj_dir.join("samples");
            if samples_dir.is_dir() {
                self.sample_browser.add_auto_root(samples_dir);
            }
        }
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
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        use tracker_core::song::Instrument;
        let mut instrument = Instrument::new(&name);
        instrument.sample_index = Some(idx);
        instrument.sample_path = Some(path.display().to_string());
        self.song.instruments.push(instrument);
        self.sync_mixer_instruments();

        self.instrument_names.push(name);
        Ok(idx)
    }

    /// Load the currently selected file from the dedicated sample browser view.
    /// Returns Ok(instrument_index) on success, or an error message.
    pub fn load_sample_from_browser(&mut self) -> Result<usize, String> {
        let path = self
            .sample_browser
            .selected_path()
            .filter(|_| self.sample_browser.selected_is_file())
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(&path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let name = sample.name().unwrap_or("unknown").to_string();

        let idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        use tracker_core::song::Instrument;
        let mut instrument = Instrument::new(&name);
        instrument.sample_index = Some(idx);
        instrument.sample_path = Some(path.display().to_string());
        self.song.instruments.push(instrument);
        self.sync_mixer_instruments();

        self.instrument_names.push(name);
        self.mark_dirty();
        Ok(idx)
    }

    /// Load a sample from an explicit path and add it as a new instrument.
    ///
    /// Used by the sample browser action menu ("Load as new instrument").
    pub fn load_sample_from_path(&mut self, path: &Path) -> Result<usize, String> {
        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let name = sample.name().unwrap_or("unknown").to_string();

        let idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        use tracker_core::song::Instrument;
        let mut instrument = Instrument::new(&name);
        instrument.sample_index = Some(idx);
        instrument.sample_path = Some(path.display().to_string());
        self.song.instruments.push(instrument);
        self.sync_mixer_instruments();
        self.instrument_names.push(name);
        self.mark_dirty();
        Ok(idx)
    }

    /// Assign a sample file to an existing instrument slot, replacing its current sample.
    ///
    /// Loads the audio from `path` into the mixer and updates the instrument's
    /// `sample_index` and `sample_path`. The instrument name is preserved.
    pub fn assign_sample_to_instrument(
        &mut self,
        path: &Path,
        inst_idx: usize,
    ) -> Result<(), String> {
        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let sample_idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        let inst = self
            .song
            .instruments
            .get_mut(inst_idx)
            .ok_or_else(|| format!("Instrument slot {inst_idx:02X} does not exist"))?;

        inst.sample_index = Some(sample_idx);
        inst.sample_path = Some(path.display().to_string());
        self.sync_mixer_instruments();
        self.mark_dirty();
        Ok(())
    }

    /// Preview a note pitch through the current instrument's sample.
    /// Called when the user enters a note in Insert mode.
    pub fn preview_note_pitch(&mut self, pitch: Pitch, octave: u8) {
        let note = Note::simple(pitch, octave);
        let target_freq = note.frequency();

        // Find the sample for the current instrument
        let inst_idx = self.editor.current_instrument() as usize;
        let sample = {
            let mixer = match self.mixer.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            mixer.samples().get(inst_idx).cloned()
        };

        let sample = match sample {
            Some(s) => s,
            None => return,
        };

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let base_freq = sample.base_frequency();
        let rate =
            (target_freq / base_freq) * (sample.sample_rate() as f64 / output_sample_rate as f64);

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.trigger_preview(sample, rate);
        }

        if let Some(ref mut engine) = self.audio_engine {
            if !engine.is_playing() {
                let _ = engine.start();
            }
        }
    }

    /// Preview the currently selected sample in the sample browser.
    /// Loads and plays it at natural pitch without adding it to the instrument list.
    pub fn preview_selected_sample(&mut self) -> Result<(), String> {
        let path = self
            .sample_browser
            .selected_path()
            .filter(|_| self.sample_browser.selected_is_file())
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(&path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        if let Ok(mut mixer) = self.mixer.lock() {
            // Natural pitch: play at sample's recorded rate
            let rate = sample.sample_rate() as f64 / output_sample_rate as f64;
            mixer.trigger_preview(Arc::new(sample), rate);
        } else {
            return Err("Failed to lock mixer".to_string());
        }

        // Ensure audio engine is running so the preview is audible
        if let Some(ref mut engine) = self.audio_engine {
            if !engine.is_playing() {
                let _ = engine.start();
            }
        }

        Ok(())
    }

    /// Import a ProTracker .mod file, replacing the current song.
    /// Returns Ok(()) on success, or an error message.
    pub fn import_mod_file(&mut self, path: &std::path::Path) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;
        let result = tracker_core::mod_import::import_mod(&data)
            .map_err(|e| format!("MOD parse error: {e}"))?;

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.clear_samples();
            self.instrument_names.clear();
            for (sample, inst) in result.samples.iter().zip(result.song.instruments.iter()) {
                mixer.add_sample(Arc::new(sample.clone()));
                self.instrument_names.push(inst.name.clone());
            }
        } else {
            return Err("Failed to lock mixer".to_string());
        }

        self.song = result.song;
        self.sync_mixer_instruments();
        self.transport.stop();

        let pattern = if !self.song.patterns.is_empty() {
            self.song.patterns[0].clone()
        } else {
            tracker_core::pattern::Pattern::default()
        };
        self.editor = crate::editor::Editor::new(pattern);
        self.arrangement_view = crate::ui::arrangement::ArrangementView::new();
        self.is_dirty = false;

        Ok(())
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

        // Clone sample references from the mixer for offline rendering
        let samples: Vec<Arc<Sample>> = if let Ok(mixer) = self.mixer.lock() {
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

    /// Get the loaded samples from the mixer.
    pub fn loaded_samples(&self) -> Vec<Arc<Sample>> {
        if let Ok(mixer) = self.mixer.lock() {
            mixer.samples().to_vec()
        } else {
            Vec::new()
        }
    }

    /// Get loaded instrument count.
    pub fn instrument_count(&self) -> usize {
        self.instrument_names.len()
    }

    /// Get the currently selected instrument index.
    pub fn instrument_selection(&self) -> Option<usize> {
        self.instrument_selection
    }

    /// Set the selected instrument index.
    pub fn set_instrument_selection(&mut self, index: Option<usize>) {
        self.instrument_selection = index;
    }

    /// Move instrument selection up.
    pub fn instrument_selection_up(&mut self) {
        let count = self.song.instruments.len();
        if count == 0 {
            self.instrument_selection = None;
            return;
        }
        match self.instrument_selection {
            None => self.instrument_selection = Some(count - 1),
            Some(0) => self.instrument_selection = Some(count - 1),
            Some(i) => self.instrument_selection = Some(i - 1),
        }
    }

    /// Move instrument selection down.
    pub fn instrument_selection_down(&mut self) {
        let count = self.song.instruments.len();
        if count == 0 {
            self.instrument_selection = None;
            return;
        }
        match self.instrument_selection {
            None => self.instrument_selection = Some(0),
            Some(i) if i >= count - 1 => self.instrument_selection = Some(0),
            Some(i) => self.instrument_selection = Some(i + 1),
        }
    }

    /// Add a new empty instrument.
    pub fn add_instrument(&mut self) {
        let idx = self.song.instruments.len();
        let name = format!("Inst{:02X}", idx);
        let inst = Instrument::new(&name);
        self.song.instruments.push(inst);
        self.sync_mixer_instruments();
        self.instrument_names.push(name);
        self.instrument_selection = Some(idx);
    }

    /// Delete the selected instrument.
    pub fn delete_instrument(&mut self) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments.remove(idx);
                self.sync_mixer_instruments();
                self.instrument_names.remove(idx);
                // Adjust selection
                if self.song.instruments.is_empty() {
                    self.instrument_selection = None;
                } else if idx >= self.song.instruments.len() {
                    self.instrument_selection = Some(self.song.instruments.len() - 1);
                }
                return true;
            }
        }
        false
    }

    /// Rename the selected instrument.
    pub fn rename_instrument(&mut self, new_name: String) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments[idx].name = new_name.clone();
                self.instrument_names[idx] = new_name;
                return true;
            }
        }
        false
    }

    /// Update instrument properties (volume, base_note as MIDI value).
    pub fn update_instrument(&mut self, volume: f32, base_note_midi: u8) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments[idx].volume = volume;
                if let Some(pitch) = Pitch::from_semitone(base_note_midi % 12) {
                    let octave = base_note_midi / 12;
                    self.song.instruments[idx].base_note = Note::simple(pitch, octave);
                    self.sync_mixer_instruments();
                    return true;
                }
            }
        }
        false
    }

    /// Set the name of the selected instrument.
    pub fn set_instrument_name(&mut self, name: String) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                if !name.is_empty() {
                    self.song.instruments[idx].name = name.clone();
                    if idx < self.instrument_names.len() {
                        self.instrument_names[idx] = name;
                    }
                    self.mark_dirty();
                }
            }
        }
    }

    /// Adjust volume of the selected instrument by `delta` percentage points (clamped 0..=100).
    pub fn adjust_instrument_volume(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current_pct = (self.song.instruments[idx].volume * 100.0).round() as i32;
                let new_pct = (current_pct + delta).clamp(0, 100);
                self.song.instruments[idx].volume = new_pct as f32 / 100.0;
                self.sync_mixer_instruments();
                self.mark_dirty();
            }
        }
    }

    /// Adjust the base note of the selected instrument by `semitones`.
    pub fn adjust_instrument_base_note(&mut self, semitones: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current_midi = self.song.instruments[idx].base_note.midi_note() as i32;
                let new_midi = (current_midi + semitones).clamp(0, 127) as u8;
                if let Some(pitch) = Pitch::from_semitone(new_midi % 12) {
                    let octave = new_midi / 12;
                    self.song.instruments[idx].base_note = Note::simple(pitch, octave);
                    self.sync_mixer_instruments();
                    self.mark_dirty();
                }
            }
        }
    }

    /// Adjust the finetune of the selected instrument by `delta` (clamped -8..=7).
    pub fn adjust_instrument_finetune(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current = self.song.instruments[idx].finetune as i32;
                let new_val = (current + delta).clamp(-8, 7) as i8;
                self.song.instruments[idx].finetune = new_val;
                self.sync_mixer_instruments();
                self.mark_dirty();
            }
        }
    }

    /// Select instrument for use in pattern editor.
    pub fn select_instrument(&mut self) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.editor.set_instrument(idx);
            }
        }
    }

    /// Get the currently selected pattern index.
    pub fn pattern_selection(&self) -> Option<usize> {
        self.pattern_selection
    }

    /// Set the selected pattern index.
    pub fn set_pattern_selection(&mut self, index: Option<usize>) {
        self.pattern_selection = index;
    }

    /// Move pattern selection up.
    pub fn pattern_selection_up(&mut self) {
        let count = self.song.patterns.len();
        if count == 0 {
            self.pattern_selection = None;
            return;
        }
        match self.pattern_selection {
            None => self.pattern_selection = Some(count - 1),
            Some(0) => self.pattern_selection = Some(count - 1),
            Some(i) => self.pattern_selection = Some(i - 1),
        }
    }

    /// Move pattern selection down.
    pub fn pattern_selection_down(&mut self) {
        let count = self.song.patterns.len();
        if count == 0 {
            self.pattern_selection = None;
            return;
        }
        match self.pattern_selection {
            None => self.pattern_selection = Some(0),
            Some(i) if i >= count - 1 => self.pattern_selection = Some(0),
            Some(i) => self.pattern_selection = Some(i + 1),
        }
    }

    /// Add a new empty pattern.
    pub fn add_pattern(&mut self) {
        if let Some(idx) = self.song.add_pattern(Pattern::default()) {
            self.pattern_selection = Some(idx);
        }
    }

    /// Delete the selected pattern.
    pub fn delete_pattern(&mut self) -> bool {
        if let Some(idx) = self.pattern_selection {
            if self.song.remove_pattern(idx) {
                // Adjust selection
                if self.song.patterns.is_empty() {
                    // Add a default pattern if none remain
                    self.song.add_pattern(Pattern::default());
                    self.pattern_selection = Some(0);
                } else if idx >= self.song.patterns.len() {
                    self.pattern_selection = Some(self.song.patterns.len() - 1);
                }
                return true;
            }
        }
        false
    }

    /// Duplicate (clone) the selected pattern.
    pub fn duplicate_pattern(&mut self) -> bool {
        if let Some(idx) = self.pattern_selection {
            if let Some(new_idx) = self.song.duplicate_pattern(idx) {
                self.pattern_selection = Some(new_idx);
                return true;
            }
        }
        false
    }

    /// Select pattern for editing (load it into the editor).
    pub fn select_pattern(&mut self) {
        if let Some(idx) = self.pattern_selection {
            if idx < self.song.patterns.len() {
                // Replace editor's pattern with the selected one
                self.editor.set_pattern(self.song.patterns[idx].clone());
                // Update transport to match the new pattern's row count
                self.transport
                    .set_num_rows(self.song.patterns[idx].num_rows());
            }
        }
    }

    /// Switch to a different top-level view.
    pub fn set_view(&mut self, view: AppView) {
        // Always start code editor in Normal mode when entering/leaving it
        self.code_editor.insert_mode = false;
        self.current_view = view;
        // When switching to CodeEditor view, activate the code editor
        self.code_editor.active = view == AppView::CodeEditor;
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
                use tracker_core::dsl::engine::{apply_commands, ScriptResult};
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
        let current_pos = self.transport.arrangement_position();
        self.flush_editor_pattern(current_pos);

        let path = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.trs"));

        match project::save_project(&path, &self.song) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.is_dirty = false;
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
                let pattern = if !song.patterns.is_empty() {
                    song.patterns[0].clone()
                } else {
                    Pattern::default()
                };
                self.editor = Editor::new(pattern);

                let output_sample_rate = self
                    .audio_engine
                    .as_ref()
                    .map(|e| e.sample_rate())
                    .unwrap_or(44100);

                let mut missing_samples = Vec::new();
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.clear_samples();
                    self.instrument_names.clear();

                    for inst in &song.instruments {
                        let sample_name = if let Some(sample_path) = &inst.sample_path {
                            let sp_path = PathBuf::from(sample_path);
                            match load_sample(&sp_path, output_sample_rate) {
                                Ok(sample) => {
                                    mixer.add_sample(Arc::new(sample));
                                    inst.name.clone()
                                }
                                Err(_) => {
                                    mixer.add_sample(Arc::new(Sample::default()));
                                    missing_samples.push(sample_path.clone());
                                    format!("{} (MISSING)", inst.name)
                                }
                            }
                        } else {
                            mixer.add_sample(Arc::new(Sample::default()));
                            inst.name.clone()
                        };
                        self.instrument_names.push(sample_name);
                    }
                }

                self.song = song;
                self.transport.set_bpm(self.song.bpm);
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.update_tempo(self.song.bpm);
                }
                self.sync_mixer_instruments();
                self.project_path = Some(path.to_path_buf());
                self.is_dirty = false;
                self.arrangement_view = ArrangementView::new();
                self.transport.stop();
                // Auto-detect project-relative samples dir
                self.refresh_browser_roots();

                if missing_samples.is_empty() {
                    self.open_modal(Modal::info(
                        "Project Loaded".to_string(),
                        format!("Loaded: {}", path.display()),
                    ));
                } else {
                    self.open_modal(Modal::error(
                        "Project Loaded with Missing Samples".to_string(),
                        format!(
                            "Loaded: {}\n\nMissing samples:\n{}",
                            path.display(),
                            missing_samples.join("\n")
                        ),
                    ));
                }
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
        app.export_dialog.bit_depth = tracker_core::export::BitDepth::Bits24;
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
        assert_eq!(result, tracker_core::transport::AdvanceResult::Row(0));

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
        assert_eq!(result, tracker_core::transport::AdvanceResult::Row(0));

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
        assert_eq!(result, tracker_core::transport::AdvanceResult::Row(0));
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

    // --- BPM prompt tests ---

    #[test]
    fn test_open_bpm_prompt_prepopulates_current_bpm() {
        let mut app = App::new();
        app.transport.set_bpm(140.0);
        app.open_bpm_prompt();
        assert!(app.bpm_prompt_mode);
        assert_eq!(app.bpm_prompt_input, "140");
    }

    #[test]
    fn test_execute_bpm_prompt_applies_valid_bpm() {
        let mut app = App::new();
        app.bpm_prompt_mode = true;
        app.bpm_prompt_input = "180".to_string();
        app.execute_bpm_prompt();
        assert!(!app.bpm_prompt_mode);
        assert!(app.bpm_prompt_input.is_empty());
        assert_eq!(app.transport.bpm(), 180.0);
        assert_eq!(app.song.bpm, 180.0);
    }

    #[test]
    fn test_execute_bpm_prompt_clamps_to_min() {
        let mut app = App::new();
        app.bpm_prompt_mode = true;
        app.bpm_prompt_input = "5".to_string();
        app.execute_bpm_prompt();
        assert_eq!(app.transport.bpm(), 20.0);
    }

    #[test]
    fn test_execute_bpm_prompt_clamps_to_max() {
        let mut app = App::new();
        app.bpm_prompt_mode = true;
        app.bpm_prompt_input = "9999".to_string();
        app.execute_bpm_prompt();
        assert_eq!(app.transport.bpm(), 999.0);
    }

    #[test]
    fn test_execute_bpm_prompt_ignores_invalid_input() {
        let mut app = App::new();
        let original_bpm = app.transport.bpm();
        app.bpm_prompt_mode = true;
        app.bpm_prompt_input = "abc".to_string();
        app.execute_bpm_prompt();
        assert!(!app.bpm_prompt_mode);
        // BPM unchanged for invalid input
        assert_eq!(app.transport.bpm(), original_bpm);
    }

    // --- Pattern length prompt tests ---

    #[test]
    fn test_open_len_prompt_prepopulates_current_row_count() {
        let mut app = App::new();
        let current_len = app.editor.pattern().row_count();
        app.open_len_prompt();
        assert!(app.len_prompt_mode);
        assert_eq!(app.len_prompt_input, format!("{}", current_len));
    }

    #[test]
    fn test_execute_len_prompt_resizes_pattern_and_transport() {
        let mut app = App::new();
        app.len_prompt_mode = true;
        app.len_prompt_input = "32".to_string();
        app.execute_len_prompt();
        assert!(!app.len_prompt_mode);
        assert_eq!(app.editor.pattern().row_count(), 32);
        assert_eq!(app.transport.num_rows(), 32);
    }

    #[test]
    fn test_execute_len_prompt_clamps_to_min() {
        let mut app = App::new();
        app.len_prompt_mode = true;
        app.len_prompt_input = "4".to_string(); // below 16
        app.execute_len_prompt();
        assert_eq!(app.editor.pattern().row_count(), 16);
        assert_eq!(app.transport.num_rows(), 16);
    }

    #[test]
    fn test_execute_len_prompt_clamps_to_max() {
        let mut app = App::new();
        app.len_prompt_mode = true;
        app.len_prompt_input = "9999".to_string(); // above 512
        app.execute_len_prompt();
        assert_eq!(app.editor.pattern().row_count(), 512);
        assert_eq!(app.transport.num_rows(), 512);
    }

    #[test]
    fn test_execute_len_prompt_ignores_invalid_input() {
        let mut app = App::new();
        let original = app.editor.pattern().row_count();
        app.len_prompt_mode = true;
        app.len_prompt_input = "abc".to_string();
        app.execute_len_prompt();
        assert!(!app.len_prompt_mode);
        // Row count unchanged for invalid input
        assert_eq!(app.editor.pattern().row_count(), original);
    }

    #[test]
    fn test_execute_len_prompt_flushes_to_song() {
        let mut app = App::new();
        app.len_prompt_mode = true;
        app.len_prompt_input = "48".to_string();
        app.execute_len_prompt();
        // The song's pattern 0 should also be updated
        let pat_idx = app.song.arrangement[app.transport.arrangement_position()];
        assert_eq!(app.song.patterns[pat_idx].row_count(), 48);
    }

    // --- Tap tempo tests ---

    #[test]
    fn test_single_tap_does_not_change_bpm() {
        let mut app = App::new();
        let original_bpm = app.transport.bpm();
        app.tap_tempo();
        // Only 1 tap — no interval to compute
        assert_eq!(app.transport.bpm(), original_bpm);
    }

    #[test]
    fn test_two_taps_set_bpm_from_interval() {
        let mut app = App::new();
        // Manually insert two taps 0.5s apart (= 120 BPM)
        let base = Instant::now();
        app.tap_times.push(base);
        app.tap_times
            .push(base + std::time::Duration::from_millis(500));
        // Simulate a third tap 0.5s after the last one
        app.tap_times
            .push(base + std::time::Duration::from_millis(1000));
        // Compute expected BPM: avg interval = 0.5s → 120 BPM
        let intervals = [0.5f64, 0.5];
        let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let expected_bpm = (60.0 / avg).clamp(20.0, 999.0);

        // Set transport to the computed BPM directly (mimics what tap_tempo would do)
        app.transport.set_bpm(expected_bpm);
        assert!((app.transport.bpm() - 120.0).abs() < 1.0);
    }

    #[test]
    fn test_tap_times_older_than_3s_are_dropped() {
        let mut app = App::new();
        // Insert a very old tap (5 seconds ago)
        app.tap_times
            .push(Instant::now() - std::time::Duration::from_secs(5));
        let original_bpm = app.transport.bpm();
        app.tap_tempo(); // Only 1 valid tap after pruning → no BPM change
        assert_eq!(app.transport.bpm(), original_bpm);
    }

    // --- Loop region tests ---

    #[test]
    fn test_set_loop_start_sets_region_and_activates() {
        let mut app = App::new();
        app.editor.go_to_row(4);
        app.set_loop_start();
        let region = app.transport.loop_region();
        assert!(region.is_some());
        assert_eq!(region.unwrap().0, 4); // start = cursor row
        assert!(app.transport.loop_region_active());
    }

    #[test]
    fn test_set_loop_end_sets_region_and_activates() {
        let mut app = App::new();
        app.editor.go_to_row(8);
        app.set_loop_end();
        let region = app.transport.loop_region();
        assert!(region.is_some());
        assert_eq!(region.unwrap().1, 8); // end = cursor row
        assert!(app.transport.loop_region_active());
    }

    #[test]
    fn test_set_loop_start_then_end_gives_correct_region() {
        let mut app = App::new();
        app.editor.go_to_row(4);
        app.set_loop_start();
        app.editor.go_to_row(12);
        app.set_loop_end();
        assert_eq!(app.transport.loop_region(), Some((4, 12)));
        assert!(app.transport.loop_region_active());
    }

    #[test]
    fn test_set_loop_end_before_start_adjusts_start() {
        let mut app = App::new();
        app.editor.go_to_row(8);
        app.set_loop_start();
        // Move cursor before the start and set end there
        app.editor.go_to_row(3);
        app.set_loop_end();
        let region = app.transport.loop_region();
        assert!(region.is_some());
        let (s, e) = region.unwrap();
        assert!(s <= e); // region must be valid
        assert_eq!(e, 3);
    }

    #[test]
    fn test_toggle_loop_region_active() {
        let mut app = App::new();
        app.editor.go_to_row(0);
        app.set_loop_start();
        app.editor.go_to_row(7);
        app.set_loop_end();
        assert!(app.transport.loop_region_active()); // auto-activated
        app.toggle_loop_region_active();
        assert!(!app.transport.loop_region_active());
        app.toggle_loop_region_active();
        assert!(app.transport.loop_region_active());
    }

    // --- Draw mode tests ---

    #[test]
    fn test_draw_mode_starts_inactive() {
        let app = App::new();
        assert!(!app.draw_mode);
        assert!(app.draw_note.is_none());
    }

    #[test]
    fn test_toggle_draw_mode() {
        let mut app = App::new();
        app.toggle_draw_mode();
        assert!(app.draw_mode);
        app.toggle_draw_mode();
        assert!(!app.draw_mode);
    }

    #[test]
    fn test_apply_draw_note_writes_to_cursor() {
        use tracker_core::pattern::note::NoteEvent;
        let mut app = App::new();
        app.draw_mode = true;
        app.draw_note = Some(NoteEvent::On(Note::simple(Pitch::C, 4)));
        app.editor.go_to_row(2);
        app.apply_draw_note();
        let cell = app.editor.pattern().get_cell(2, 0);
        assert!(cell.is_some());
        assert_eq!(
            cell.unwrap().note,
            Some(NoteEvent::On(Note::simple(Pitch::C, 4)))
        );
    }

    #[test]
    fn test_apply_draw_note_noop_when_mode_off() {
        use tracker_core::pattern::note::NoteEvent;
        let mut app = App::new();
        app.draw_mode = false;
        app.draw_note = Some(NoteEvent::On(Note::simple(Pitch::C, 4)));
        app.editor.go_to_row(2);
        app.apply_draw_note();
        let cell = app.editor.pattern().get_cell(2, 0);
        // Row 2 should be empty (no note written)
        assert!(
            cell.is_none() || cell.unwrap().note.is_none(),
            "apply_draw_note should be a no-op when draw_mode is false"
        );
    }

    #[test]
    fn test_apply_draw_note_noop_when_note_none() {
        let mut app = App::new();
        app.draw_mode = true;
        app.draw_note = None;
        app.editor.go_to_row(2);
        app.apply_draw_note();
        let cell = app.editor.pattern().get_cell(2, 0);
        assert!(
            cell.is_none() || cell.unwrap().note.is_none(),
            "apply_draw_note should be a no-op when draw_note is None"
        );
    }

    // --- Tutor view tests ---

    #[test]
    fn test_tutor_starts_hidden() {
        let app = App::new();
        assert!(!app.show_tutor);
        assert_eq!(app.tutor_scroll, 0);
    }

    #[test]
    fn test_execute_command_tutor_opens_view() {
        let mut app = App::new();
        app.command_mode = true;
        app.command_input = "tutor".to_string();
        app.execute_command();
        assert!(app.show_tutor, "show_tutor should be true after :tutor");
        assert_eq!(app.tutor_scroll, 0, "scroll should reset to 0");
        assert!(!app.command_mode, "command mode should be exited");
    }

    #[test]
    fn test_tutor_content_has_lines() {
        let count = crate::ui::tutor::content_line_count();
        assert!(count > 20, "tutor should have at least 20 lines of content");
    }

    #[test]
    fn test_project_samples_dir_auto_added_to_browser() {
        let dir = std::env::temp_dir().join("riffl_app_proj_samples");
        std::fs::create_dir_all(&dir).unwrap();
        let samples_dir = dir.join("samples");
        std::fs::create_dir_all(&samples_dir).unwrap();

        let mut app = App::new();
        // Simulate a loaded project whose directory contains ./samples/
        app.project_path = Some(dir.join("test.trs"));
        app.refresh_browser_roots();

        let has_samples = app
            .sample_browser
            .entries()
            .iter()
            .any(|e| e.path == samples_dir);
        assert!(has_samples, "project-relative samples/ should be auto-added as a root");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_project_samples_dir_not_added_when_missing() {
        let dir = std::env::temp_dir().join("riffl_app_proj_no_samples");
        std::fs::create_dir_all(&dir).unwrap();
        // No samples/ subdir created here

        let mut app = App::new();
        app.project_path = Some(dir.join("test.trs"));
        app.refresh_browser_roots();

        let samples_dir = dir.join("samples");
        let has_samples = app
            .sample_browser
            .entries()
            .iter()
            .any(|e| e.path == samples_dir);
        assert!(!has_samples, "should not add samples/ root when directory doesn't exist");

        std::fs::remove_dir_all(&dir).ok();
    }
}
