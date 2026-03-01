/// Transport system for playback control
///
/// Manages play/stop/pause state, BPM timing, row advancement,
/// and pattern looping for the tracker sequencer.

/// Transport playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
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

    /// Current pattern index (for future song/order-list support)
    current_pattern: usize,

    /// Whether the pattern loops when reaching the end
    loop_enabled: bool,

    /// Accumulated time for BPM-based row advancement
    tick_accumulator: f64,

    /// Total number of rows in the current pattern
    num_rows: usize,
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
            current_pattern: 0,
            loop_enabled: true,
            tick_accumulator: 0.0,
            num_rows: 64,
        }
    }

    /// Advance the transport by the given delta time in seconds.
    ///
    /// Returns `Some(row_index)` when it's time to advance to a new row,
    /// or `None` if not enough time has elapsed.
    pub fn advance(&mut self, delta_time: f64) -> Option<usize> {
        if self.state != TransportState::Playing {
            return None;
        }

        self.tick_accumulator += delta_time;
        let seconds_per_row = self.seconds_per_row();

        if self.tick_accumulator >= seconds_per_row {
            self.tick_accumulator -= seconds_per_row;

            // Prevent accumulator from building up too much
            // (e.g., if the app was frozen for a long time)
            if self.tick_accumulator > seconds_per_row {
                self.tick_accumulator = 0.0;
            }

            let next_row = self.current_row + 1;
            if next_row >= self.num_rows {
                if self.loop_enabled {
                    self.current_row = 0;
                } else {
                    // Reached end without looping — stop playback
                    self.state = TransportState::Stopped;
                    self.current_row = 0;
                    self.tick_accumulator = 0.0;
                    return None;
                }
            } else {
                self.current_row = next_row;
            }

            Some(self.current_row)
        } else {
            None
        }
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

    /// Get the current pattern index
    pub fn current_pattern(&self) -> usize {
        self.current_pattern
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
        assert_eq!(transport.advance(0.05), None);

        // Enough time to advance (0.05 + 0.08 = 0.13 > 0.125)
        assert_eq!(transport.advance(0.08), Some(1));

        // Advance again after full row period
        assert_eq!(transport.advance(spr), Some(2));
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
        assert_eq!(transport.advance(spr), Some(1));
        assert_eq!(transport.advance(spr), Some(2));
        assert_eq!(transport.advance(spr), Some(3));
        assert_eq!(transport.advance(spr), Some(0)); // Wraps back to 0
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

        assert_eq!(transport.advance(spr), Some(1));
        assert_eq!(transport.advance(spr), Some(2));
        assert_eq!(transport.advance(spr), Some(3));

        // At end of pattern without loop — should stop
        assert_eq!(transport.advance(spr), None);
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
        assert_eq!(transport.advance(1.0), None);

        // Paused
        transport.play();
        transport.pause();
        assert_eq!(transport.advance(1.0), None);
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

        assert_eq!(transport.advance(0.06), None); // Not quite enough
        assert_eq!(transport.advance(0.01), Some(1)); // 0.07 > 0.0625
    }
}
