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
use crate::ui::code_editor::{self, CodeEditor};
use crate::ui::envelope_editor::EnvelopeEditorState;
use crate::ui::export_dialog::ExportDialog;
use crate::ui::file_browser::FileBrowser;
use crate::ui::instrument_editor::InstrumentEditorState;
use crate::ui::lfo_editor::LfoEditorState;
use crate::ui::modal::Modal;
use crate::ui::sample_browser::SampleBrowser;
use crate::ui::theme::{Theme, ThemeKind};
use crate::ui::waveform_editor::WaveformEditorState;
use riffl_core::audio::{
    load_sample, AudioEngine, ChipRenderData, Mixer, Sample, TransportCommand,
};
use riffl_core::dsl::engine::ScriptEngine;
use riffl_core::export;
use riffl_core::pattern::note::{NoteEvent, Pitch};
use riffl_core::pattern::{Note, Pattern};
use riffl_core::song::{Instrument, Song};
use riffl_core::transport::{AdvanceResult, PlaybackMode, Transport, TransportState};

pub mod arrangement;
pub mod browser;
pub mod command;
pub mod instrument;
pub mod modal;
pub mod pattern;
pub mod project;
pub mod sample;
pub mod script;
#[cfg(test)]
mod tests;
pub mod transport;
pub mod view;

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
    glicol_mixer: Arc<Mutex<riffl_core::audio::glicol_mixer::GlicolMixer>>,

    /// Transport system for playback control (play/pause/stop, BPM, looping)
    pub transport: Transport,

    /// Code editor for writing Rhai DSL scripts
    pub code_editor: CodeEditor,

    /// Whether the split view is active (pattern left, code editor right)
    pub split_view: bool,

    /// Whether the instrument view is expanded (full-screen deep editing)
    pub instrument_expanded: bool,

    /// Whether the mini control panel is shown in the main view
    pub instrument_mini_panel: bool,

    /// DSL scripting engine for executing Rhai scripts
    script_engine: ScriptEngine,

    /// Whether live mode is active (scripts auto-re-evaluate on every pattern loop)
    pub live_mode: bool,

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Scroll offset for the help overlay (in lines)
    pub help_scroll: u16,

    /// Whether effect command help overlay is shown
    pub show_effect_help: bool,

    /// Scroll offset for the effect help overlay (in lines)
    pub effect_help_scroll: u16,

    /// Whether the which-key menu is shown (displays available keybindings)
    pub which_key_mode: bool,

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

    /// Envelope editor state for visual envelope editing
    pub env_editor: EnvelopeEditorState,

    /// LFO editor state for LFO configuration
    pub lfo_editor: LfoEditorState,

    /// Waveform editor state for manual sample editing
    pub waveform_editor: WaveformEditorState,

    /// Whether the sample browser has an active user-initiated preview.
    pub browser_preview_active: bool,

    /// Sample data kept for scrubbing (re-trigger from a different offset).
    browser_preview_sample: Option<Arc<Sample>>,

    /// Playback rate for the current browser preview (sample_rate / output_rate).
    browser_preview_rate: f64,

    /// Current scrub start offset in sample-native frames.
    pub browser_preview_offset_frames: usize,

    /// Pending Zxx script triggers from the last tick (channel, param).
    pub pending_script_triggers: Vec<(usize, u8)>,

    /// Cached system info for CPU/memory status bar display.
    sys_info: sysinfo::System,
    /// Cached CPU usage percentage (updated periodically).
    cached_cpu_percent: f32,
    /// Cached memory usage percentage (updated periodically).
    cached_mem_percent: f32,
    /// Last time system stats were refreshed.
    sys_info_last_update: Instant,

    /// Horizontal channel scroll offset (reset to 0 with Home/Ctrl+L)
    pub channel_scroll: usize,

    pub command_history: Vec<String>,
    pub command_history_index: Option<usize>,

    /// The BPM the song had when it was loaded (or when play was last started from
    /// the beginning). Used to restore tempo on song loop / restart so that
    /// in-pattern tempo slides (Txx/Axx) don't persist across loops.
    initial_bpm: f64,

    /// The TPL (ticks-per-line) the song had when it was loaded.
    initial_tpl: u32,
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

        // Load user configuration
        let config = if cfg!(test) {
            Config::default()
        } else {
            Config::load()
        };

        // Generate a demo sine wave sample at 440Hz, 0.25s duration
        let demo_sample = Self::generate_sine_sample(440.0, 0.25, 44100);
        let demo_chip_render = ChipRenderData::from_sample(&demo_sample);

        // Create a demo pattern: C4, E4, G4, C5 across 16 rows
        let mut pattern = Pattern::new(config.default_pattern_rows, config.default_channels);
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

        // Capture initial tempo values before `config` is moved into the struct.
        let initial_bpm = config.default_bpm;
        let initial_tpl: u32 = 6;

        // Create a song with the demo pattern in its pool
        let mut song = Song::new("Untitled", config.default_bpm);

        // Create transport synced to song BPM and pattern size
        let mut transport = Transport::new();
        transport.set_playback_mode(config.default_playback_mode);
        transport.set_loop_enabled(config.default_loop_enabled);
        transport.set_num_rows(pattern.num_rows());
        transport.set_bpm(song.bpm);
        // Sync mixer effect processor tempo with the song BPM
        if let Ok(mut m) = mixer.lock() {
            m.update_tempo(song.bpm);
            m.set_tpl(song.tpl);
            m.set_effect_mode(song.effect_mode);
            m.set_format_is_s3m(song.format_is_s3m);
            m.set_slide_mode(song.slide_mode);
        }

        let editor = Editor::new(pattern.clone());

        let glicol_mixer = Arc::new(Mutex::new(
            riffl_core::audio::glicol_mixer::GlicolMixer::new(
                pattern.num_channels(),
                output_sample_rate,
            ),
        ));
        song.patterns[0] = pattern;

        use riffl_core::song::Instrument;
        let mut demo_inst = Instrument::new("sine440");
        demo_inst.sample_index = Some(0);
        demo_inst.sample_path = None;
        demo_inst.chip_render = Some(demo_chip_render);
        song.instruments.push(demo_inst);

        // Initialize file browser at default modules dir, sample browser at sample dirs
        let default_modules = crate::config::Config::default_modules_dir();
        let file_browser = FileBrowser::new(&default_modules);
        let sample_browser = SampleBrowser::new(config.resolve_sample_dirs(None));

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
            configured_sample_dirs: config.resolve_sample_dirs(None),
            current_view: AppView::PatternEditor,
            theme_kind: config.theme_kind(),
            theme: Theme::from_kind(config.theme_kind()),
            config,
            audio_engine,
            mixer,
            glicol_mixer,
            transport,
            code_editor: CodeEditor::new(),
            split_view: false,
            instrument_expanded: false,
            instrument_mini_panel: false,
            script_engine: ScriptEngine::new(),
            live_mode: false,
            show_help: false,
            help_scroll: 0,
            show_effect_help: false,
            effect_help_scroll: 0,
            which_key_mode: false,
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
            env_editor: EnvelopeEditorState::default(),
            lfo_editor: LfoEditorState::default(),
            waveform_editor: WaveformEditorState::default(),
            browser_preview_active: false,
            browser_preview_sample: None,
            browser_preview_rate: 1.0,
            browser_preview_offset_frames: 0,
            pending_script_triggers: Vec::new(),
            sys_info: sysinfo::System::new(),
            cached_cpu_percent: 0.0,
            cached_mem_percent: 0.0,
            sys_info_last_update: Instant::now(),
            channel_scroll: 0,
            command_history: Vec::new(),
            command_history_index: None,
            initial_bpm,
            initial_tpl,
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
        let mut current_arrangement_pos = self.transport.arrangement_position();
        let advance_results = self.transport.advance_iter(delta);

        for res in advance_results {
            match res {
                AdvanceResult::Row(row) => {
                    if self.follow_mode {
                        self.editor.go_to_row(row);
                    }
                    if row == 0 && self.live_mode {
                        self.execute_script(&[]);
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
                                use riffl_core::pattern::note::NoteEvent;
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
                    // Reset BPM and TPL to initial values if we are looping back to the very start of the song
                    if arrangement_pos == 0
                        && row == 0
                        && current_arrangement_pos >= self.song.arrangement.len().saturating_sub(1)
                    {
                        self.transport.set_bpm(self.initial_bpm);
                        self.transport.set_tpl(self.initial_tpl);
                        self.song.bpm = self.initial_bpm;
                        self.song.tpl = self.initial_tpl;
                        if let Ok(mut mixer) = self.mixer.lock() {
                            mixer.update_tempo(self.initial_bpm);
                            mixer.set_tpl(self.initial_tpl);
                        }
                    }

                    let saved_cursor_row = self.editor.cursor_row();
                    let saved_cursor_channel = self.editor.cursor_channel();
                    self.flush_editor_pattern(current_arrangement_pos);
                    self.load_arrangement_pattern(arrangement_pos);
                    current_arrangement_pos = arrangement_pos;
                    if self.follow_mode {
                        self.editor.go_to_row(row);
                    } else {
                        // Preserve cursor position when not following playhead
                        self.editor.go_to_row(saved_cursor_row);
                        self.editor.set_cursor_channel(saved_cursor_channel);
                    }
                    if self.live_mode {
                        self.execute_script(&[]);
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
                                use riffl_core::pattern::note::NoteEvent;
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

        // Drain pending Zxx script triggers accumulated this tick.
        // In live mode any trigger re-runs the code editor script, passing the
        // trigger data so scripts can react to specific Zxx channel/param values.
        if !self.pending_script_triggers.is_empty() {
            let triggers: Vec<(usize, u8)> = self.pending_script_triggers.drain(..).collect();
            if self.live_mode {
                self.execute_script(&triggers);
            }
        }

        // Decay VU meter levels on every tick for visual smoothing.
        // During playback it smooths peaks; when stopped it ensures they return to zero.
        if let Ok(mut mixer) = self.mixer.lock() {
            let decay = if self.transport.is_playing() {
                0.85
            } else {
                0.70
            };
            mixer.decay_channel_levels(decay);
        }

        Ok(())
    }

    /// Return peak levels for the first `num_channels` channels as `(left, right)` pairs.
    ///
    /// Locks the mixer briefly and reads atomic peak values for each channel.
    /// Missing channels (index out of range) return `(0.0, 0.0)`.
    pub fn channel_levels(&self, num_channels: usize) -> Vec<(f32, f32)> {
        if let Ok(mixer) = self.mixer.lock() {
            (0..num_channels)
                .map(|ch| mixer.get_channel_level(ch))
                .collect()
        } else {
            vec![(0.0, 0.0); num_channels]
        }
    }

    /// Return oscilloscope waveform data for each channel.
    ///
    /// Locks the mixer briefly and reads the ring buffer waveform data.
    /// Returns empty vectors for missing channels.
    pub fn oscilloscope_data(&self, num_channels: usize) -> Vec<Vec<f32>> {
        if let Ok(mixer) = self.mixer.lock() {
            (0..num_channels)
                .map(|ch| mixer.oscilloscope_data(ch))
                .collect()
        } else {
            vec![Vec::new(); num_channels]
        }
    }

    /// Return FFT spectrum data from the master bus.
    ///
    /// Locks the mixer briefly and reads the FFT ring buffer.
    pub fn fft_data(&self) -> Vec<f32> {
        if let Ok(mixer) = self.mixer.lock() {
            mixer.fft_data()
        } else {
            Vec::new()
        }
    }

    /// Get system CPU and memory usage as (cpu_percent, memory_percent).
    pub fn system_stats(&self) -> Option<(f32, f32)> {
        Some((self.cached_cpu_percent, self.cached_mem_percent))
    }

    /// Refresh system stats (called periodically from the update loop).
    pub fn refresh_system_stats(&mut self) {
        if self.sys_info_last_update.elapsed().as_secs() < 2 {
            return;
        }
        self.sys_info_last_update = Instant::now();

        self.sys_info.refresh_memory();
        self.sys_info.refresh_cpu_all();

        let total_mem = self.sys_info.total_memory() as f64;
        let used_mem = self.sys_info.used_memory() as f64;
        self.cached_mem_percent = if total_mem > 0.0 {
            (used_mem / total_mem * 100.0) as f32
        } else {
            0.0
        };

        let cpus = self.sys_info.cpus();
        if !cpus.is_empty() {
            let total_cpu: f32 = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>();
            self.cached_cpu_percent = total_cpu / cpus.len() as f32;
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
