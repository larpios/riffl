//! Transport system for playback control
//!
//! Manages play/stop/pause state, BPM timing, row advancement,
//! pattern looping, and song-level arrangement sequencing for the tracker.

/// Transport playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
}

/// Playback mode: single pattern loop or full song arrangement
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    PatternChange { arrangement_pos: usize, row: usize },
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

    /// Lines per beat (1-255)
    lpb: u32,

    /// Ticks per row (1-255)
    tpl: u32,

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

    /// Loop region: (start_row, end_row) inclusive. None = no loop region set.
    loop_region: Option<(usize, usize)>,

    /// Whether the loop region is enforced during playback
    loop_region_active: bool,

    /// Row index for pattern loop (E60 effect).
    pattern_loop_row: Option<usize>,

    /// Remaining loop iterations (E6x effect).
    pattern_loop_count: u8,

    /// Extra rows to delay (EEx effect).
    pattern_delay: u8,

    /// Whether a jump was just applied, so the next advance should not increment.
    just_jumped: bool,
}

/// Minimum allowed BPM
const MIN_BPM: f64 = 20.0;
/// Maximum allowed BPM
const MAX_BPM: f64 = 999.0;
/// Minimum allowed LPB/TPL
const MIN_LPB: u32 = 1;
const MAX_LPB: u32 = 255;
const MIN_TPL: u32 = 1;
const MAX_TPL: u32 = 255;

impl Transport {
    /// Create a new Transport with default settings
    pub fn new() -> Self {
        Self {
            state: TransportState::Stopped,
            bpm: 120.0,
            lpb: 4,
            tpl: 6,
            current_row: 0,
            arrangement_position: 0,
            loop_enabled: false,
            playback_mode: PlaybackMode::Song,
            tick_accumulator: 0.0,
            num_rows: 64,
            arrangement_length: 1,
            loop_region: None,
            loop_region_active: false,
            pattern_loop_row: None,
            pattern_loop_count: 0,
            pattern_delay: 0,
            just_jumped: false,
        }
    }

    /// Advance the transport by the given delta time, returning all row changes that occurred.
    ///
    /// This is a wrapper around `advance()` that handles multiple row increments
    /// if the delta time is large enough to span more than one row. This ensures
    /// that effects (like BPM changes) on intermediate rows are not skipped.
    pub fn advance_iter(&mut self, delta_time: f64) -> Vec<AdvanceResult> {
        let mut results = Vec::new();
        
        // Use a small safety limit to prevent infinite loops if seconds_per_row is zero
        let mut iterations = 0;
        let max_iterations = 1000;
        
        loop {
            // Only add the full delta to the accumulator on the first iteration
            let delta = if iterations == 0 { delta_time } else { 0.0 };
            let res = self.advance(delta);
            
            if res == AdvanceResult::None {
                break;
            }
            
            results.push(res);
            
            if res == AdvanceResult::Stopped {
                break;
            }
            
            iterations += 1;
            if iterations >= max_iterations {
                break;
            }
        }
        
        results
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

        // If a jump was just applied (e.g. from an effect in the previous tick),
        // return the jumped-to row immediately. This ensures that the first row
        // of a jump is triggered without waiting for another row duration.
        if self.just_jumped {
            self.just_jumped = false;
            return AdvanceResult::Row(self.current_row);
        }

        let seconds_per_row = self.seconds_per_row();

        if self.tick_accumulator >= seconds_per_row {
            // Handle Pattern Delay (EEx)
            if self.pattern_delay > 0 {
                self.pattern_delay -= 1;
                self.tick_accumulator -= seconds_per_row;
                return AdvanceResult::None;
            }

            self.tick_accumulator -= seconds_per_row;

            let next_row = self.current_row + 1;

            // Loop region takes precedence: wrap at loop_end back to loop_start
            if self.loop_region_active {
                if let Some((loop_start, loop_end)) = self.loop_region {
                    let loop_end = loop_end.min(self.num_rows.saturating_sub(1));
                    let loop_start = loop_start.min(loop_end);
                    if self.current_row >= loop_end {
                        self.current_row = loop_start;
                        return AdvanceResult::Row(loop_start);
                    } else {
                        self.current_row = next_row;
                        return AdvanceResult::Row(next_row);
                    }
                }
            }

            if next_row >= self.num_rows {
                // End of current pattern — behavior depends on playback mode
                match self.playback_mode {
                    PlaybackMode::Pattern => {
                        if self.loop_enabled {
                            self.current_row = 0;
                            // Note: ProTracker typically keeps E60 loop point within the pattern?
                            // For simplicity, we'll keep it for now but we could reset it.
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
                                // Reset pattern loop state on pattern change
                                self.pattern_loop_row = None;
                                self.pattern_loop_count = 0;
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
                            // Reset pattern loop state on pattern change
                            self.pattern_loop_row = None;
                            self.pattern_loop_count = 0;
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
                self.pattern_loop_row = None;
                self.pattern_loop_count = 0;
                self.pattern_delay = 0;
                self.just_jumped = false;
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

    /// Start playback from the given row index (clamped to valid range).
    ///
    /// Unlike `play()`, this always begins at the specified row regardless of
    /// current state — useful for "Play From Cursor" where the user wants
    /// playback to begin at the edit cursor position.
    pub fn play_from(&mut self, start_row: usize) {
        let clamped = start_row.min(self.num_rows.saturating_sub(1));
        self.current_row = clamped;
        self.tick_accumulator = 0.0;
        self.pattern_loop_row = None;
        self.pattern_loop_count = 0;
        self.pattern_delay = 0;
        self.just_jumped = false;
        self.state = TransportState::Playing;
    }

    /// Stop playback and reset position to the beginning
    pub fn stop(&mut self) {
        self.state = TransportState::Stopped;
        self.current_row = 0;
        self.arrangement_position = 0;
        self.tick_accumulator = 0.0;
        self.pattern_loop_row = None;
        self.pattern_loop_count = 0;
        self.pattern_delay = 0;
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

    /// Set the lines per beat, clamped to the valid range (1-255)
    pub fn set_lpb(&mut self, lpb: u32) {
        self.lpb = lpb.clamp(MIN_LPB, MAX_LPB);
    }

    /// Get the current lines per beat
    pub fn lpb(&self) -> u32 {
        self.lpb
    }

    /// Set the ticks per line (speed), clamped to the valid range (1-255)
    pub fn set_tpl(&mut self, tpl: u32) {
        self.tpl = tpl.clamp(MIN_TPL, MAX_TPL);
    }

    /// Get the current ticks per line
    pub fn tpl(&self) -> u32 {
        self.tpl
    }

    /// Get the current transport state
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// Get the current row position
    pub fn current_row(&self) -> usize {
        self.current_row
    }

    /// Set current row
    pub fn set_row(&mut self, row: usize) {
        self.current_row = row.min(self.num_rows.saturating_sub(1));
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

    /// Set the loop region (start and end row, inclusive).
    /// start is clamped to [0, num_rows-1] and end is clamped to [start, num_rows-1].
    pub fn set_loop_region(&mut self, start: usize, end: usize) {
        let start = start.min(self.num_rows.saturating_sub(1));
        let end = end.clamp(start, self.num_rows.saturating_sub(1));
        self.loop_region = Some((start, end));
    }

    /// Clear the loop region (deactivates loop region as a side effect)
    pub fn clear_loop_region(&mut self) {
        self.loop_region = None;
        self.loop_region_active = false;
    }

    /// Get the loop region (start, end) if set
    pub fn loop_region(&self) -> Option<(usize, usize)> {
        self.loop_region
    }

    /// Whether the loop region is currently active (enforced during playback)
    pub fn loop_region_active(&self) -> bool {
        self.loop_region_active
    }

    /// Toggle whether the loop region is active.
    /// Has no effect if no loop region is set.
    pub fn toggle_loop_region_active(&mut self) {
        if self.loop_region.is_some() {
            self.loop_region_active = !self.loop_region_active;
        }
    }

    /// Set loop region active state directly
    pub fn set_loop_region_active(&mut self, active: bool) {
        self.loop_region_active = active && self.loop_region.is_some();
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
            self.just_jumped = true;
            true
        } else {
            false
        }
    }

    /// Jump to the next arrangement position at a specific row (PatternBreak Dxx).
    ///
    /// Advances the arrangement position by one and sets the starting row to `row`.
    /// Wraps to position 0 if at the last arrangement slot (when looping).
    /// Returns true if the jump was applied, false if the arrangement has only one slot.
    pub fn pattern_break(&mut self, row: usize) -> bool {
        let next_pos = self.arrangement_position + 1;
        let next_pos = if next_pos >= self.arrangement_length {
            if self.loop_enabled {
                0
            } else {
                return false;
            }
        } else {
            next_pos
        };
        self.arrangement_position = next_pos;
        self.current_row = row.min(self.num_rows.saturating_sub(1));
        self.tick_accumulator = 0.0;
        self.just_jumped = true;
        true
    }

    /// Set the pattern loop row (E60).
    pub fn set_pattern_loop_row(&mut self, row: Option<usize>) {
        self.pattern_loop_row = row;
    }

    /// Set the pattern loop start point to the current row (E60).
    pub fn set_pattern_loop_start(&mut self) {
        self.pattern_loop_row = Some(self.current_row);
    }

    /// Get the pattern loop row.
    pub fn pattern_loop_row(&self) -> Option<usize> {
        self.pattern_loop_row
    }

    /// Set the pattern loop count (E6x).
    pub fn set_pattern_loop_count(&mut self, count: u8) {
        self.pattern_loop_count = count;
    }

    /// Get the pattern loop count.
    pub fn pattern_loop_count(&self) -> u8 {
        self.pattern_loop_count
    }

    /// Handle a pattern loop jump if the count > 0 (E6x).
    /// Returns the target row if the jump should occur, else None.
    pub fn handle_pattern_loop(&mut self, count: u8) -> Option<usize> {
        if self.pattern_loop_count == 0 {
            // First time hitting this loop command in the current cycle
            self.pattern_loop_count = count;
        } else {
            // Subsequent times
            self.pattern_loop_count -= 1;
        }

        if self.pattern_loop_count > 0 {
            let target = self.pattern_loop_row.unwrap_or(0);
            self.current_row = target;
            self.tick_accumulator = 0.0;
            self.just_jumped = true;
            Some(target)
        } else {
            // Loop finished
            self.pattern_loop_row = None;
            None
        }
    }

    /// Trigger a pattern loop jump if the count > 0.
    /// Returns the target row if the jump should occur, else None.
    #[deprecated(note = "Use handle_pattern_loop instead")]
    pub fn trigger_pattern_loop(&mut self) -> Option<usize> {
        if self.pattern_loop_count > 0 {
            self.pattern_loop_count -= 1;
            let target = self.pattern_loop_row.unwrap_or(0);
            self.current_row = target;
            self.tick_accumulator = 0.0;
            self.just_jumped = true;
            if self.pattern_loop_count == 0 {
                self.pattern_loop_row = None;
            }
            return Some(target);
        }
        None
    }

    /// Set the pattern delay (EEx).
    pub fn set_pattern_delay(&mut self, delay: u8) {
        self.pattern_delay = delay;
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

    /// Calculate seconds per row using tick-based timing (standard for MOD/FT2/Furnace).
    /// Formula: seconds per row = (2.5 / BPM) * TPL
    /// At 120 BPM, TPL 6: (2.5 / 120) * 6 = 0.125s (125ms)
    fn seconds_per_row(&self) -> f64 {
        (2.5 / self.bpm) * self.tpl as f64
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
        assert!(!transport.loop_enabled());
        assert!(transport.is_stopped());
        assert_eq!(transport.playback_mode(), PlaybackMode::Song);
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
        let spr = (2.5 / 120.0) * 6.0; // 0.125s at 120 BPM, speed 6
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

        // At 120 BPM, TPL 6: each row lasts 0.125s
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
        transport.set_playback_mode(PlaybackMode::Pattern);
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
        transport.set_playback_mode(PlaybackMode::Pattern);
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
        assert!(!transport.loop_enabled());

        transport.toggle_loop();
        assert!(transport.loop_enabled());

        transport.toggle_loop();
        assert!(!transport.loop_enabled());
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
        assert_eq!(
            transport.advance(spr),
            AdvanceResult::PatternChange {
                arrangement_pos: 1,
                row: 0,
            }
        );
        assert_eq!(transport.arrangement_position(), 1);
        assert_eq!(transport.current_row(), 0);

        // Pattern 1: rows 0-3
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));

        // End of pattern 1 → advance to pattern 2
        assert_eq!(
            transport.advance(spr),
            AdvanceResult::PatternChange {
                arrangement_pos: 2,
                row: 0,
            }
        );
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
        assert_eq!(
            transport.advance(spr),
            AdvanceResult::PatternChange {
                arrangement_pos: 1,
                row: 0,
            }
        );
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
        // End of arrangement → loop back to pattern 0
        assert_eq!(
            transport.advance(spr),
            AdvanceResult::PatternChange {
                arrangement_pos: 0,
                row: 0,
            }
        );
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

        // Advance should stay on row 0
        assert_eq!(transport.advance(spr), AdvanceResult::Row(0));
        // Then proceed to 1
        assert_eq!(transport.advance(spr), AdvanceResult::Row(1));
    }

    #[test]
    fn test_toggle_playback_mode() {
        let mut transport = Transport::new();
        assert_eq!(transport.playback_mode(), PlaybackMode::Song);

        transport.toggle_playback_mode();
        assert_eq!(transport.playback_mode(), PlaybackMode::Pattern);

        transport.toggle_playback_mode();
        assert_eq!(transport.playback_mode(), PlaybackMode::Song);
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
        assert_eq!(
            transport.advance(spr),
            AdvanceResult::PatternChange {
                arrangement_pos: 0,
                row: 0,
            }
        );
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

    #[test]
    fn test_pattern_break_advances_arrangement_position() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_arrangement_length(3);
        transport.play();

        assert!(transport.pattern_break(4));
        assert_eq!(transport.arrangement_position(), 1);
        assert_eq!(transport.current_row(), 4);

        // Advance should stay on row 4 because of just_jumped
        let spr = 0.125;
        assert_eq!(transport.advance(spr), AdvanceResult::Row(4));
        // Next advance should go to 5
        assert_eq!(transport.advance(spr), AdvanceResult::Row(5));
    }

    #[test]
    fn test_pattern_break_wraps_with_loop() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_arrangement_length(2);
        transport.set_loop_enabled(true);
        transport.jump_to_arrangement_position(1);

        // At last position, loop enabled → wraps to 0
        assert!(transport.pattern_break(0));
        assert_eq!(transport.arrangement_position(), 0);
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_pattern_break_fails_no_loop_at_end() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_arrangement_length(2);
        transport.set_loop_enabled(false);
        transport.jump_to_arrangement_position(1);

        // At last position, loop disabled → returns false
        assert!(!transport.pattern_break(0));
        assert_eq!(transport.arrangement_position(), 1); // unchanged
    }

    #[test]
    fn test_pattern_break_clamps_row_to_num_rows() {
        let mut transport = Transport::new();
        transport.set_num_rows(8);
        transport.set_arrangement_length(2);

        // Row 99 should be clamped to num_rows - 1 = 7
        assert!(transport.pattern_break(99));
        assert_eq!(transport.current_row(), 7);
    }

    #[test]
    fn test_play_from_starts_at_given_row() {
        let mut transport = Transport::new();
        transport.set_num_rows(64);
        transport.play_from(16);
        assert!(transport.is_playing());
        assert_eq!(transport.current_row(), 16);
    }

    #[test]
    fn test_play_from_clamps_to_last_row() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.play_from(999);
        assert!(transport.is_playing());
        assert_eq!(transport.current_row(), 15);
    }

    #[test]
    fn test_play_from_zero_behaves_like_play() {
        let mut transport = Transport::new();
        transport.set_num_rows(64);
        transport.play_from(0);
        assert!(transport.is_playing());
        assert_eq!(transport.current_row(), 0);
    }

    #[test]
    fn test_play_from_overrides_paused_position() {
        let mut transport = Transport::new();
        transport.set_num_rows(64);
        transport.play();
        transport.pause();
        // play_from should override the paused position and start playing
        transport.play_from(32);
        assert!(transport.is_playing());
        assert_eq!(transport.current_row(), 32);
    }

    // --- Loop region tests ---

    #[test]
    fn test_loop_region_set_and_get() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        assert!(transport.loop_region().is_none());

        transport.set_loop_region(4, 8);
        assert_eq!(transport.loop_region(), Some((4, 8)));
    }

    #[test]
    fn test_loop_region_clear() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_loop_region(2, 6);
        transport.set_loop_region_active(true);
        transport.clear_loop_region();
        assert!(transport.loop_region().is_none());
        assert!(!transport.loop_region_active());
    }

    #[test]
    fn test_loop_region_toggle_active_only_when_region_set() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        // No region set — toggle should be a no-op
        transport.toggle_loop_region_active();
        assert!(!transport.loop_region_active());

        // Set region then toggle
        transport.set_loop_region(0, 7);
        transport.toggle_loop_region_active();
        assert!(transport.loop_region_active());
        transport.toggle_loop_region_active();
        assert!(!transport.loop_region_active());
    }

    #[test]
    fn test_loop_region_wraps_playback_at_end() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(16);
        transport.set_loop_region(2, 4);
        transport.set_loop_region_active(true);
        transport.play_from(2);

        let spr = 0.125;
        assert_eq!(transport.advance(spr), AdvanceResult::Row(3));
        assert_eq!(transport.advance(spr), AdvanceResult::Row(4));
        // At loop end (4), next advance should wrap back to loop start (2)
        assert_eq!(transport.advance(spr), AdvanceResult::Row(2));
        assert!(transport.is_playing());
        assert_eq!(transport.current_row(), 2);
    }

    #[test]
    fn test_loop_region_inactive_does_not_affect_advance() {
        let mut transport = Transport::new();
        transport.set_bpm(120.0);
        transport.set_num_rows(8);
        transport.set_loop_region(2, 4);
        // loop_region_active is false by default
        transport.set_loop_enabled(true);
        transport.set_playback_mode(PlaybackMode::Pattern);
        transport.play();

        let spr = 0.125;
        // Should advance normally past row 4, looping at num_rows
        for _ in 0..7 {
            transport.advance(spr);
        }
        assert_eq!(transport.current_row(), 7);
        assert_eq!(transport.advance(spr), AdvanceResult::Row(0)); // Normal pattern wrap
    }

    #[test]
    fn test_loop_region_clamped_to_num_rows() {
        let mut transport = Transport::new();
        transport.set_num_rows(8);
        transport.set_loop_region(3, 99); // end beyond num_rows
        assert_eq!(transport.loop_region(), Some((3, 7))); // clamped to 7
    }

    #[test]
    fn test_loop_region_start_after_end_swapped() {
        let mut transport = Transport::new();
        transport.set_num_rows(16);
        transport.set_loop_region(10, 4); // start > end — end clamped up to start
        assert_eq!(transport.loop_region(), Some((10, 10))); // both become start
    }

    #[test]
    fn test_pattern_loop_logic() {
        let mut transport = Transport::new();
        transport.set_num_rows(32);
        transport.play();

        // 1. Set loop start at row 10 (E60)
        transport.set_row(10);
        transport.set_pattern_loop_start();
        assert_eq!(transport.pattern_loop_row(), Some(10));

        // 2. Hit loop command at row 20, loop 1 time (E61)
        transport.set_row(20);
        let target = transport.handle_pattern_loop(1);
        assert_eq!(target, Some(10));
        assert_eq!(transport.current_row(), 10);
        assert_eq!(transport.pattern_loop_count(), 1);

        // 3. Advance should stay on row 10 because of just_jumped
        let spr = 0.125;
        assert_eq!(transport.advance(spr), AdvanceResult::Row(10));

        // 4. Hit loop command at row 20 again
        transport.set_row(20);
        let target = transport.handle_pattern_loop(1);
        assert_eq!(target, None); // Finished loop
        assert_eq!(transport.pattern_loop_row(), None); // State reset
        assert_eq!(transport.pattern_loop_count(), 0);

        // 5. Advance should proceed to row 21
        assert_eq!(transport.advance(spr), AdvanceResult::Row(21));
    }
}
