use super::App;
use std::time::Instant;
use riffl_core::transport::{PlaybackMode, TransportState};

use riffl_core::pattern::{Pattern, note::NoteEvent};

impl App {
    /// Toggle audio playback between play and pause.
    ///
    /// In Song mode, starting from stopped loads the first arrangement pattern.
    pub fn toggle_play(&mut self) {
        match self.transport.state() {
            TransportState::Stopped => {
                // Restore the original tempo in case pattern effects (Txx/Axx) modified
                // it during the previous playback run.
                self.song.bpm = self.initial_bpm;
                self.song.tpl = self.initial_tpl;
                self.transport.set_bpm(self.initial_bpm);
                self.transport.set_tpl(self.initial_tpl);
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.update_tempo(self.initial_bpm);
                    mixer.set_tpl(self.initial_tpl);
                }

                // Sync arrangement length before starting
                self.transport
                    .set_arrangement_length(self.song.arrangement.len());
                // In Song mode, load the pattern at the current arrangement position
                if self.transport.playback_mode() == PlaybackMode::Song {
                    self.load_arrangement_pattern(self.transport.arrangement_position());
                }
                
                // When starting from stopped, we perform a chase if the starting row is not 0
                if self.transport.current_row() > 0 {
                    self.chase_notes();
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
                // NOTE: We no longer call mixer.stop_all() here to allow "true pause"
                // where voices pick up where they left off.
            }
            TransportState::Paused => {
                // If the user moved the cursor while paused, Space resumes from the cursor
                // if they are in follow mode, or just resumes from where it was.
                // To support "consistent sound when moving playhead", we check if we should jump.
                let cursor_row = self.editor.cursor_row();
                if !self.follow_mode && self.transport.current_row() != cursor_row {
                    self.play_from_cursor();
                    return;
                }

                self.transport.play();
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
        
        // When playing from cursor, chase notes from previous rows so it sounds correct
        if start_row > 0 {
            self.chase_notes();
        }

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.tick(self.transport.current_row(), self.editor.pattern());
        }
        if let Some(ref mut engine) = self.audio_engine {
            let _ = engine.start();
        }
    }

    /// Scan backwards from the current row to find the most recent note on each
    /// channel and trigger them. This ensures that jumping to the middle of a
    /// pattern doesn't result in silence.
    pub fn chase_notes(&mut self) {
        let current_row = self.transport.current_row();
        let pattern = self.editor.pattern();
        let num_channels = pattern.num_channels();
        
        if let Ok(mut mixer) = self.mixer.lock() {
            // First, clear everything so we have a clean slate for the chase
            mixer.stop_all();
            
            // Prepare a single row that contains the "last known state" for each channel
            let mut chase_row_cells = vec![riffl_core::pattern::row::Cell::default(); num_channels];
            
            for ch in 0..num_channels {
                for r in (0..current_row).rev() {
                    if let Some(row) = pattern.get_row(r) {
                        if let Some(cell) = row.get(ch) {
                            if let Some(NoteEvent::On(_)) = &cell.note {
                                chase_row_cells[ch] = cell.clone();
                                break;
                            } else if let Some(NoteEvent::Off) | Some(NoteEvent::Cut) = &cell.note {
                                // Channel was explicitly stopped/cut
                                break;
                            }
                        }
                    }
                }
            }
            
            // Create a temporary pattern with this single chase row
            let mut chase_pattern = Pattern::new(1, num_channels);
            chase_pattern.set_row(0, chase_row_cells);
            
            // CRITICAL: Copy track state (volume, pan, mute) so the chase row 
            // is rendered with the correct channel settings.
            for (i, track) in pattern.tracks().iter().enumerate() {
                if i < num_channels {
                    chase_pattern.tracks_mut()[i] = track.clone();
                }
            }
            
            // Tick the chase row to trigger the voices
            mixer.tick(0, &chase_pattern);
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
    pub(super) fn sync_mixer_instruments(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_instruments(self.song.instruments.clone());
        }
    }

    pub(super) fn sync_mixer_tracks(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.update_tracks(&self.song.tracks);
        }
    }

    pub(super) fn sync_mixer_global_volume(&self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_global_volume(self.song.global_volume);
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
            use riffl_core::pattern::pattern::{MAX_ROW_COUNT, MIN_ROW_COUNT};
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
            use riffl_core::pattern::row::Cell;
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

    /// Synchronize the number of channels inside the audio mixer with the current pattern.
    pub fn sync_mixer_channels(&mut self) {
        let num_channels = self.editor.pattern().num_channels();
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_num_channels(num_channels);
        }
        if let Ok(mut gm) = self.glicol_mixer.lock() {
            gm.set_num_channels(num_channels);
        }
    }
}

use riffl_core::audio::TransportCommand;

impl App {
    /// Apply transport commands produced by effect processing (Fxx, Bxx, Dxx).
    ///
    /// These commands are returned by `mixer.tick()` when pattern effects fire:
    /// - `SetBpm`: Update tempo on both transport and mixer effect processor.
    /// - `PositionJump (Bxx)`: Jump to arrangement position; loads new pattern in Song mode.
    /// - `PatternBreak (Dxx)`: Advance to next arrangement entry at the given row.
    pub(super) fn apply_effect_transport_commands(&mut self, commands: Vec<TransportCommand>) {
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
                TransportCommand::SetTpl(tpl) => {
                    self.transport.set_tpl(tpl);
                    self.song.tpl = tpl;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.set_tpl(tpl);
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
                TransportCommand::PatternLoop(sub_param) => {
                    if sub_param == 0 {
                        // E60: set loop point
                        self.transport.set_pattern_loop_start();
                    } else {
                        // E6x (x>0): jump back to loop point x times
                        if let Some(target) = self.transport.handle_pattern_loop(sub_param) {
                            if self.follow_mode {
                                self.editor.go_to_row(target);
                            }
                        }
                    }
                }
                TransportCommand::PatternDelay(delay) => {
                    // EEx: pattern delay
                    self.transport.set_pattern_delay(delay);
                }
                TransportCommand::ScriptTrigger { channel, param } => {
                    // Zxx: custom effect command for Rhai script triggering.
                    // Store for the app layer to process (e.g., invoke a registered macro).
                    self.pending_script_triggers.push((channel, param));
                }
            }
        }
    }
}
