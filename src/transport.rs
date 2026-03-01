/// Transport system for playback control
///
/// Manages play/stop/pause state, BPM timing, row advancement,
/// pattern looping, and song-level arrangement sequencing for the tracker.

/// Transport playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
}

/// Playback mode: single pattern loop or full song arrangement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    /// Loop the current pattern only
    Pattern,
    /// Play through the arrangement sequence (pattern after pattern)
    Song,
}

/// Result of a transport advance, describing what happened
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceResult {
    /// No row change occurred (not enough time elapsed or not playing)
    None,
    /// Advanced to a new row within the current pattern
    Row(usize),
    /// Advanced to a new pattern in the arrangement (arrangement_index, first_row)
    PatternChange {
        arrangement_pos: usize,
        row: usize,
    },
    /// Playback stopped (reached end of arrangement without loop)
    Stopped,
}

/// Transport controls for sequencer playback
#[derive(Debug)]
pub struct Transport {
    /// Current playback state
    state: TransportState,

    /// Tempo in beats per minute (clamped to 20-999)
    bpm: f64,

    /// Current row position within the pattern
    current_row: usize,

    /// Current arrangement position (index into the arrangement Vec)
    arrangement_position: usize,

    /// Whether the pattern/song loops when reaching the end
    loop_enabled: bool,

    /// Playback mode: pattern-only or song arrangement
    playback_mode: PlaybackMode,

    /// Accumulated time for BPM-based row advancement
    tick_accumulator: f64,

    /// Total number of rows in the current pattern
    num_rows: usize,

    /// Length of the arrangement sequence (set by App)
    arrangement_length: usize,
}

/// Minimum allowed BPM
const MIN_BPM: f64 = 20.0;
/// Maximum allowed BPM
const MAX_BPM: f64 = 999.0;
/// Rows per beat (speed 6 in tracker terms = 4 rows per beat)
const ROWS_PER_BEAT: f64 = 4.0;

impl Transport {
    /// Create a new Transport with default settings
    pub fn new() -> Self {
        Self {
            state: TransportState::Stopped,
            bpm: 120.0,
            current_row: 0,
            arrangement_position: 0,
            loop_enabled: true,
            playback_mode: PlaybackMode::Pattern,
            tick_accumulator: 0.0,
            num_rows: 64,
            arrangement_length: 1,
        }
    }

    /// Advance the transport by the given delta time in seconds.
    ///
    /// Returns an `AdvanceResult` describing what happened:
    /// - `None`: no row change
    /// - `Row(idx)`: advanced to a new row in the current pattern
    /// - `PatternChange { arrangement_pos, row }`: moved to next pattern in arrangement
    /// - `Stopped`: playback ended (arrangement finished, no loop)
    pub fn advance(&mut self, delta_time: f64) -> AdvanceResult {
        if self.state != TransportState::Playing {
            return AdvanceResult::None;
        }

        self.tick_accumulator += delta_time;
        let seconds_per_row = self.seconds_per_row();

        if self.tick_accumulator >= seconds_per_row {
            self.tick_accumulator -= seconds_per_row;

            // Prevent accumulator from building up too much
            if self.tick_accumulator > seconds_per_row {
                self.tick_accumulator = 0.0;
            }

            let next_row = self.current_row + 1;
            if next_row >= self.num_rows {
                // End of current pattern — behavior depends on playback mode
                match self.playback_mode {
                    PlaybackMode::Pattern => {
                        if self.loop_enabled {
                            self.current_row = 0;
                            return AdvanceResult::Row(0);
                        } else {
                            self.state = TransportState::Stopped;
                            self.current_row = 0;
                            self.tick_accumulator = 0.0;
                            return AdvanceResult::Stopped;
                        }
                    }
                    PlaybackMode::Song => {
                        let next_pos = self.arrangement_position + 1;
                        if next_pos >= self.arrangement_length {
                            // End of arrangement
                            if self.loop_enabled {
                                self.arrangement_position = 0;
                                self.current_row = 0;
                                return AdvanceResult::PatternChange {
                                    arrangement_pos: 0,
                                    row: 0,
                                };
                            } else {
                                self.state = TransportState::Stopped;
                                self.current_row = 0;
                                self.arrangement_position = 0;
                                self.tick_accumulator = 0.0;
                                return AdvanceResult::Stopped;
                            }
                        } else {
                            // Move to next pattern in arrangement
                            self.arrangement_position = next_pos;
                            self.current_row = 0;
                            return AdvanceResult::PatternChange {
                                arrangement_pos: next_pos,
                                row: 0,
                            };
                        }
                    }
                }
            } else {
                self.current_row = next_row;
                return AdvanceResult::Row(next_row);
            }
        }

        AdvanceResult::None
    }

    /// Start playback from the current position (or from the beginning if stopped)
    pub fn play(&mut self) {
        match self.state {
            TransportState::Stopped => {
                self.current_row = 0;
                self.tick_accumulator = 0.0;
                self.state = TransportState::Playing;
            }
            TransportState::Paused => {
                // Resume from current position
                self.state = TransportState::Playing;
            }
            TransportState::Playing => {
                // Already playing, no-op
            }
        }
    }

    /// Stop playback and reset position to the beginning
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.current_row = 0;
        self.arrangement_position = 0;
        self.tick_accumulator = 0.0;
    }

    /// Pause playback at the current position
    pub fn pause(&mut self) {
        if self.state == TransportState::Playing {
            self.state = TransportState::Paused;
        }
    }

    /// Toggle between play and pause states.
    /// If stopped, starts playing from the beginning.
    pub fn toggle_play_pause(&mut self) {
        match self.state {
            TransportState::Stopped => self.play(),
            TransportState::Playing => self.pause(),
            TransportState::Paused => self.play(),
        }
    }

    /// Set the BPM, clamped to the valid range (20-999)
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(MIN_BPM, MAX_BPM);
    }

    /// Adjust BPM by a delta value, clamped to the valid range
    pub fn adjust_bpm(&mut self, delta: f64) {
        self.set_bpm(self.bpm + delta);
    }

    /// Get the current BPM
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Get the current transport state
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Get the current row position
    pub fn current_row(&self) -> usize {
        self.current_row
    }

    /// Get the current arrangement position (index into the arrangement Vec)
    pub fn arrangement_position(&self) -> usize {
        self.arrangement_position
    }

    /// Get the current pattern index (alias for arrangement_position for backward compat)
    pub fn current_pattern(&self) -> usize {
        self.arrangement_position
    }

    /// Get the playback mode
    pub fn playback_mode(&self) -> PlaybackMode {
        self.playback_mode
    }

    /// Set the playback mode
    pub fn set_playback_mode(&mut self, mode: PlaybackMode) {
        self.playback_mode = mode;
    }

    /// Toggle between pattern and song playback modes
    pub fn toggle_playback_mode(&mut self) {
        self.playback_mode = match self.playback_mode {
            PlaybackMode::Pattern => PlaybackMode::Song,
            PlaybackMode::Song => PlaybackMode::Pattern,
        };
    }

    /// Check if loop mode is enabled
    pub fn loop_enabled(&self) -> bool {
        self.loop_enabled
    }

    /// Set loop mode
    pub fn set_loop_enabled(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
    }

    /// Toggle loop mode
    pub fn toggle_loop(&mut self) {
        self.loop_enabled = !self.loop_enabled;
    }

    /// Set the number of rows in the current pattern
    pub fn set_num_rows(&mut self, num_rows: usize) {
        self.num_rows = num_rows.max(1);
        if self.current_row >= self.num_rows {
            self.current_row = 0;
        }
    }

    /// Get the number of rows
    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    /// Set the arrangement length (number of entries in the song arrangement)
    pub fn set_arrangement_length(&mut self, length: usize) {
        self.arrangement_length = length.max(1);
        if self.arrangement_position >= self.arrangement_length {
            self.arrangement_position = 0;
        }
    }

    /// Get the arrangement length
    pub fn arrangement_length(&self) -> usize {
        self.arrangement_length
    }

    /// Jump to a specific arrangement position, resetting the row to 0.
    /// Returns true if the position was valid.
    pub fn jump_to_arrangement_position(&mut self, pos: usize) -> bool {
        if pos < self.arrangement_length {
            self.arrangement_position = pos;
            self.current_row = 0;
            self.tick_accumulator = 0.0;
            true
        } else {
            false
        }
    }

    /// Check if the transport is currently playing
    pub fn is_playing(&self) -> bool {
        self.state == TransportState::Playing
    }

    /// Check if the transport is stopped
    pub fn is_stopped(&self) -> bool {
        self.state == TransportState::Stopped
    }

    /// Check if the transport is paused
    pub fn is_paused(&self) -> bool {
        self.state == TransportState::Paused
    }

    /// Calculate seconds per row based on current BPM.
    /// At 120 BPM with 4 rows per beat: 60 / 120 / 4 = 0.125s (125ms)
    fn seconds_per_row(&self) -> f64 {
        60.0 / self.bpm / ROWS_PER_BEAT
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let transport = Transport::new();
        assert_eq!(transport.state(), TransportState::Stopped);
        assert_eq!(transport.bpm(), 120.0);
        assert_eq!(transport.current_row(), 0);
        assert!(transport.loop_enabled());
        assert!(transport.is_stopped());
        assert_eq!(transport.playback_mode(), PlaybackMode::Pattern);
        assert_eq!(transport.arrangement_position(), 0);
    }

    #[test]
    fn test_state_transitions_stopped_playing_paused() {
        let mut transport = Transport::new();

        // Stopped -> Playing
        transport.play();
        assert_eq!(transport.state(), TransportState::Playing);
        assert!(transport.is_playing());

        // Playing -> Paused
        transport.pause();
        assert_eq!(transport.state(), TransportState::Paused);
        assert!(transport.is_paused());

        // Paused -> Playing (resume)
        transport.play();
        assert_eq!(transport.state(), TransportState::Playing);

        // Playing -> Stopped
        transport.stop();
        assert_eq!(transport.state(), TransportState::Stopped);
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_toggle_play_pause() {
        let mut transport = Transport::new();

        // Stopped -> Playing
        transport.toggle_play_pause();
        assert!(transport.is_playing());

        // Playing -> Paused
        transport.toggle_play_pause();
        assert!(transport.is_paused());

        // Paused -> Playing
        transport.toggle_play_pause();
        assert!(transport.is_playing());
    }

    #[test]
    fn test_stop_resets_position() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.play();

        // Advance a few rows
        let spr = 60.0 / 120.0 / 4.0; // 0.125s at 120 BPM
        transport.advance(spr);
        assert_eq!(transport.current_row(), 1);

        transport.stop();
        assert_eq!(transport.current_row(), 0);
        assert!(transport.is_stopped());
    }

    #[test]
    fn test_bpm_timing_accuracy() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(16);
        transport.play();

        // At 120 BPM, 4 rows/beat: each row lasts 0.125s
        let spr = 0.125;

        // Not enough time — should not advance
        assert_eq!(transport.advance(0.05), AdvanceResult::None);

        // Enough time to advance (0.05 + 0.08 = 0.13 > 0.125)
        assert_eq!(transport.advance(0.08), AdvanceResult::Row(1));

        // Advance again after full row period
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
    }

    #[test]
    fn test_row_wrapping_with_loop() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(4);
        transport.set_loop_enabled(true);
        transport.play();

        let spr = 0.125;

        // Advance through all 4 rows (0 -> 1 -> 2 -> 3 -> 0)
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(0)); // Wraps back to 0
        assert!(transport.is_playing()); // Still playing
    }

    #[test]
    fn test_stop_at_end_without_loop() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(4);
        transport.set_loop_enabled(false);
        transport.play();

        let spr = 0.125;

        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));

        // At end of pattern without loop — should stop
        assert_eq!(transport.advance(spr), AdvanceResult::Stopped);
        assert!(transport.is_stopped());
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_bpm_range_clamping() {
        let mut transport = Transport::new();

        transport.set_bpm(10.0);
        assert_eq!(transport.bpm(), 20.0); // Clamped to minimum

        transport.set_bpm(1500.0);
        assert_eq!(transport.bpm(), 999.0); // Clamped to maximum

        transport.set_bpm(200.0);
        assert_eq!(transport.bpm(), 200.0); // Within range
    }

    #[test]
    fn test_adjust_bpm() {
        let mut transport = Transport::new();
        assert_eq!(transport.bpm(), 120.0);

        transport.adjust_bpm(10.0);
        assert_eq!(transport.bpm(), 130.0);

        transport.adjust_bpm(-20.0);
        assert_eq!(transport.bpm(), 110.0);

        // Clamping at boundaries
        transport.set_bpm(995.0);
        transport.adjust_bpm(10.0);
        assert_eq!(transport.bpm(), 999.0);
    }

    #[test]
    fn test_pause_only_when_playing() {
        let mut transport = Transport::new();

        // Pause when stopped should remain stopped
        transport.pause();
        assert!(transport.is_stopped());

        transport.play();
        transport.pause();
        assert!(transport.is_paused());
    }

    #[test]
    fn test_play_idempotent_when_playing() {
        let mut transport = Transport::new();
        transport.play();
        assert!(transport.is_playing());

        // Calling play() again should be a no-op
        transport.play();
        assert!(transport.is_playing());
    }

    #[test]
    fn test_advance_when_not_playing() {
        let mut transport = Transport::new();
        // Stopped
        assert_eq!(transport.advance(1.0), AdvanceResult::None);

        // Paused
        transport.play();
        transport.pause();
        assert_eq!(transport.advance(1.0), AdvanceResult::None);
    }

    #[test]
    fn test_toggle_loop() {
        let mut transport = Transport::new();
        assert!(transport.loop_enabled());

        transport.toggle_loop();
        assert!(!transport.loop_enabled());

        transport.toggle_loop();
        assert!(transport.loop_enabled());
    }

    #[test]
    fn test_set_num_rows_clamps_current_row() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.play();

        // Advance to row 10
        let spr = 0.125;
        for _ in 0..10 {
            transport.advance(spr);
        }
        assert_eq!(transport.current_row(), 10);

        // Shrink pattern — current_row should reset
        transport.set_num_rows(4);
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_different_bpm_timing() {
        // At 240 BPM, 4 rows/beat: each row lasts 0.0625s
        let mut transport = Transport::new();
        transport.set_bpm(240.0);
        transport.set_num_rows(16);
        transport.play();

        assert_eq!(transport.advance(0.06), AdvanceResult::None); // Not quite enough
        assert_eq!(transport.advance(0.01), AdvanceResult::Row(1)); // 0.07 > 0.0625
    }

    // --- Song-level playback tests ---

    #[test]
    fn test_song_mode_advances_through_arrangement() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(4); // 4 rows per pattern
        transport.set_arrangement_length(3); // 3 patterns in arrangement
        transport.set_playback_mode(PlaybackMode::Song);
        transport.set_loop_enabled(false);
        transport.play();

        let spr = 0.125;

        // Pattern 0: rows 0-3
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));

        // End of pattern 0 → advance to pattern 1
        assert_eq!(transport.advance(spr), AdvanceResult::PatternChange {
            arrangement_pos: 1,
            row: 0,
        });
        assert_eq!(transport.arrangement_position(), 1);
        assert_eq!(transport.current_row(), 0);

        // Pattern 1: rows 0-3
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));

        // End of pattern 1 → advance to pattern 2
        assert_eq!(transport.advance(spr), AdvanceResult::PatternChange {
            arrangement_pos: 2,
            row: 0,
        });
        assert_eq!(transport.arrangement_position(), 2);

        // Pattern 2: rows 0-3
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));

        // End of arrangement without loop → stop
        assert_eq!(transport.advance(spr), AdvanceResult::Stopped);
        assert!(transport.is_stopped());
    }

    #[test]
    fn test_song_mode_loops_arrangement() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(2); // 2 rows per pattern for brevity
        transport.set_arrangement_length(2); // 2 patterns
        transport.set_playback_mode(PlaybackMode::Song);
        transport.set_loop_enabled(true);
        transport.play();

        let spr = 0.125;

        // Pattern 0
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        // End of pattern 0 → pattern 1
        assert_eq!(transport.advance(spr), AdvanceResult::PatternChange {
            arrangement_pos: 1,
            row: 0,
        });
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        // End of arrangement → loop back to pattern 0
        assert_eq!(transport.advance(spr), AdvanceResult::PatternChange {
            arrangement_pos: 0,
            row: 0,
        });
        assert!(transport.is_playing());
        assert_eq!(transport.arrangement_position(), 0);
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_song_mode_stop_resets_arrangement_position() {
        let mut transport = Transport::new();
        transport.set_num_rows(4);
        transport.set_arrangement_length(3);
        transport.set_playback_mode(PlaybackMode::Song);
        transport.play();

        let spr = 0.125;
        // Advance past first pattern
        for _ in 0..4 {
            transport.advance(spr);
        }
        assert_eq!(transport.arrangement_position(), 1);

        transport.stop();
        assert_eq!(transport.arrangement_position(), 0);
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_jump_to_arrangement_position() {
        let mut transport = Transport::new();
        transport.set_arrangement_length(5);

        assert!(transport.jump_to_arrangement_position(3));
        assert_eq!(transport.arrangement_position(), 3);
        assert_eq!(transport.current_row(), 0);

        // Out of bounds
        assert!(!transport.jump_to_arrangement_position(5));
        assert_eq!(transport.arrangement_position(), 3); // unchanged
    }

    #[test]
    fn test_jump_to_arrangement_position_resets_row() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_arrangement_length(4);
        transport.play();

        let spr = 0.125;
        // Advance some rows
        transport.advance(spr);
        transport.advance(spr);
        assert_eq!(transport.current_row(), 2);

        // Jump to different arrangement position
        transport.jump_to_arrangement_position(2);
        assert_eq!(transport.current_row(), 0);
        assert_eq!(transport.arrangement_position(), 2);
    }

    #[test]
    fn test_toggle_playback_mode() {
        let mut transport = Transport::new();
        assert_eq!(transport.playback_mode(), PlaybackMode::Pattern);

        transport.toggle_playback_mode();
        assert_eq!(transport.playback_mode(), PlaybackMode::Song);

        transport.toggle_playback_mode();
        assert_eq!(transport.playback_mode(), PlaybackMode::Pattern);
    }

    #[test]
    fn test_set_arrangement_length_clamps_position() {
        let mut transport = Transport::new();
        transport.set_arrangement_length(5);
        transport.jump_to_arrangement_position(4);
        assert_eq!(transport.arrangement_position(), 4);

        // Shrink arrangement — position should reset
        transport.set_arrangement_length(2);
        assert_eq!(transport.arrangement_position(), 0);
    }

    #[test]
    fn test_pattern_mode_ignores_arrangement() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(4);
        transport.set_arrangement_length(3);
        transport.set_playback_mode(PlaybackMode::Pattern);
        transport.set_loop_enabled(true);
        transport.play();

        let spr = 0.125;

        // Advance through one pattern
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));
        // Loops within pattern, does NOT advance to next arrangement entry
        assert_eq!(transport.advance(spr), AdvanceResult::Row(0));
        assert_eq!(transport.arrangement_position(), 0); // unchanged
    }

    #[test]
    fn test_song_mode_single_pattern_arrangement_loops() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(2);
        transport.set_arrangement_length(1);
        transport.set_playback_mode(PlaybackMode::Song);
        transport.set_loop_enabled(true);
        transport.play();

        let spr = 0.125;

        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        // End of single-entry arrangement → loops back
        assert_eq!(transport.advance(spr), AdvanceResult::PatternChange {
            arrangement_pos: 0,
            row: 0,
        });
        assert!(transport.is_playing());
    }

    #[test]
    fn test_song_mode_single_pattern_no_loop_stops() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(2);
        transport.set_arrangement_length(1);
        transport.set_playback_mode(PlaybackMode::Song);
        transport.set_loop_enabled(false);
        transport.play();

        let spr = 0.125;

        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Stopped);
        assert!(transport.is_stopped());
    }
}
