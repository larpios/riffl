use super::App;
use std::time::Instant;
use tracker_core::transport::{PlaybackMode, TransportState};

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
