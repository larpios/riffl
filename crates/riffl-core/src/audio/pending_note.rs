//! Pending note triggers for Note Delay (EDx) effect.

/// Pending note trigger for Note Delay (EDx).
#[derive(Debug, Clone)]
pub struct PendingNote {
    pub channel: usize,
    pub instrument_index: usize,
    pub sample_index: usize,
    pub playback_rate: f64,
    pub velocity_gain: f32,
    pub hz_to_rate: f64,
    pub triggered_note_freq: f64,
    /// Effective Amiga period clock for this note trigger.
    pub period_clock: f64,
    pub offset: Option<usize>,
    pub trigger_frame: u32,
}
