//! Audio sample data representation
//!
//! This module provides the Sample struct which represents loaded audio data
//! in memory. Samples can be loaded from various formats (WAV, FLAC, OGG) and
//! are stored in a format ready for playback.

/// MIDI note number for C-4 (standard tracker base pitch).
pub const C4_MIDI: u8 = 48;

/// Loop mode for sample playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    /// No loop: playback stops at the end of the sample.
    #[default]
    NoLoop,
    /// Forward loop: playback jumps back to `loop_start` when `loop_end` is reached.
    Forward,
    /// Ping-pong loop: playback reverses direction at `loop_start` and `loop_end`.
    PingPong,
}

/// Represents a loaded audio sample
#[derive(Clone, Debug)]
pub struct Sample {
    /// Raw audio data as f32 samples in range [-1.0, 1.0]
    data: Vec<f32>,
    /// Sample rate in Hz
    sample_rate: u32,
    /// Number of audio channels
    channels: u16,
    /// Optional name or filename for this sample
    name: Option<String>,
    /// MIDI note number of the sample's natural pitch (default: C-4 = 48).
    /// Playing this note will reproduce the sample at its original rate.
    base_note: u8,
    /// Loop playback mode.
    pub loop_mode: LoopMode,
    /// Start point of the loop in frames.
    pub loop_start: usize,
    /// End point of the loop in frames (inclusive).
    pub loop_end: usize,
}

impl Sample {
    /// Create a new Sample instance with default base note C-4.
    pub fn new(data: Vec<f32>, sample_rate: u32, channels: u16, name: Option<String>) -> Self {
        let frame_count = data.len() / channels as usize;
        Self {
            data,
            sample_rate,
            channels,
            name,
            base_note: C4_MIDI,
            loop_mode: LoopMode::NoLoop,
            loop_start: 0,
            loop_end: frame_count.saturating_sub(1),
        }
    }

    /// Create a new Sample with an explicit base note (MIDI note number).
    pub fn with_base_note(mut self, base_note: u8) -> Self {
        self.base_note = base_note;
        self
    }

    /// Set the loop points and mode for the sample.
    pub fn with_loop(mut self, mode: LoopMode, start: usize, end: usize) -> Self {
        self.loop_mode = mode;
        self.loop_start = start;
        self.loop_end = end;
        self
    }

    /// Get the MIDI note number of this sample's natural pitch.
    pub fn base_note(&self) -> u8 {
        self.base_note
    }

    /// Get the frequency in Hz of this sample's base note.
    pub fn base_frequency(&self) -> f64 {
        let a4_midi: i32 = 57; // A-4 = octave 4 * 12 + semitone 9
        let semitone_diff = self.base_note as i32 - a4_midi;
        440.0 * 2.0_f64.powf(semitone_diff as f64 / 12.0)
    }

    /// Get a reference to the raw audio data
    pub fn data(&self) -> &[f32] {
        &self.data
    }

    /// Get the sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of audio channels
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Get the sample name, if available
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the duration of the sample in seconds
    pub fn duration(&self) -> f64 {
        let total_frames = self.data.len() / self.channels as usize;
        total_frames as f64 / self.sample_rate as f64
    }

    /// Get the total number of sample frames
    pub fn frame_count(&self) -> usize {
        self.data.len() / self.channels as usize
    }

    /// Check if the sample is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the length of the audio data buffer
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            sample_rate: 44100,
            channels: 1,
            name: None,
            base_note: C4_MIDI,
            loop_mode: LoopMode::NoLoop,
            loop_start: 0,
            loop_end: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_duration_mono() {
        let sample_rate = 44100;
        let channels = 1;
        let data = vec![0.0; 44100]; // 1 second
        let sample = Sample::new(data, sample_rate, channels, None);
        assert_eq!(sample.duration(), 1.0);
    }

    #[test]
    fn test_sample_duration_stereo() {
        let sample_rate = 44100;
        let channels = 2;
        let data = vec![0.0; 88200]; // 1 second
        let sample = Sample::new(data, sample_rate, channels, None);
        assert_eq!(sample.duration(), 1.0);
    }

    #[test]
    fn test_sample_duration_different_rate() {
        let sample_rate = 48000;
        let channels = 1;
        let data = vec![0.0; 24000]; // 0.5 seconds
        let sample = Sample::new(data, sample_rate, channels, None);
        assert_eq!(sample.duration(), 0.5);
    }

    #[test]
    fn test_sample_duration_empty() {
        let sample = Sample::default();
        assert_eq!(sample.duration(), 0.0);
    }

    #[test]
    fn test_sample_frame_count() {
        let sample = Sample::new(vec![0.0; 100], 44100, 2, None);
        assert_eq!(sample.frame_count(), 50);
    }

    #[test]
    fn test_sample_is_empty() {
        let sample = Sample::default();
        assert!(sample.is_empty());
        let sample = Sample::new(vec![0.0], 44100, 1, None);
        assert!(!sample.is_empty());
    }

    #[test]
    fn test_sample_len() {
        let sample = Sample::new(vec![0.0; 100], 44100, 1, None);
        assert_eq!(sample.len(), 100);
    }

    #[test]
    fn test_sample_properties() {
        let data = vec![0.1, 0.2, 0.3];
        let sample = Sample::new(data.clone(), 44100, 1, Some("test".to_string()));
        assert_eq!(sample.data(), &data);
        assert_eq!(sample.sample_rate(), 44100);
        assert_eq!(sample.channels(), 1);
        assert_eq!(sample.name(), Some("test"));
        assert_eq!(sample.base_note(), C4_MIDI);
    }

    #[test]
    fn test_sample_loop_properties() {
        let sample =
            Sample::new(vec![0.0; 100], 44100, 1, None).with_loop(LoopMode::Forward, 10, 90);
        assert_eq!(sample.loop_mode, LoopMode::Forward);
        assert_eq!(sample.loop_start, 10);
        assert_eq!(sample.loop_end, 90);
    }
}
