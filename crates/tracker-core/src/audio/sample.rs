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
    /// Default volume multiplier for this sample (0.0 to 1.0).
    pub volume: f32,
    /// Fine-tune adjustment in cents (-100 to +100).
    pub finetune: i32,
    /// Sustain loop mode (IT format). Loops while key is held, plays through on release.
    pub sustain_loop_mode: LoopMode,
    /// Start of sustain loop in frames.
    pub sustain_loop_start: usize,
    /// End of sustain loop in frames (inclusive).
    pub sustain_loop_end: usize,
    /// Slice points for beat-sliced playback.
    pub slices: Vec<Slice>,
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
            volume: 1.0,
            finetune: 0,
            sustain_loop_mode: LoopMode::NoLoop,
            sustain_loop_start: 0,
            sustain_loop_end: 0,
            slices: Vec::new(),
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

    /// Set sustain loop points and mode (IT format).
    /// The sustain loop is active while the key is held; on key release the
    /// sample plays through to its end or enters the regular loop.
    pub fn with_sustain_loop(mut self, mode: LoopMode, start: usize, end: usize) -> Self {
        self.sustain_loop_mode = mode;
        self.sustain_loop_start = start;
        self.sustain_loop_end = end;
        self
    }

    /// Set the fine-tune value for the sample in cents.
    pub fn with_finetune(mut self, finetune: i32) -> Self {
        self.finetune = finetune;
        self
    }

    /// Set the default volume for the sample (0.0 to 1.0).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
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

    /// Get the loop mode and boundaries, if looping is enabled.
    pub fn loop_info(&self) -> Option<(LoopMode, usize, usize)> {
        if self.loop_mode == LoopMode::NoLoop || self.loop_end <= self.loop_start {
            None
        } else {
            Some((self.loop_mode, self.loop_start, self.loop_end))
        }
    }

    /// Get a reference to the raw audio data buffer.
    pub fn data_ref(&self) -> &[f32] {
        &self.data
    }

    /// Get a mutable reference to the raw audio data buffer.
    pub fn data_mut(&mut self) -> &mut Vec<f32> {
        &mut self.data
    }

    /// Set a sample value at the given frame (left channel for stereo).
    pub fn set_sample(&mut self, frame: usize, value: f32) {
        if frame >= self.frame_count() {
            return;
        }
        let channels = self.channels as usize;
        let idx = frame * channels;
        if idx < self.data.len() {
            self.data[idx] = value.clamp(-1.0, 1.0);
        }
    }

    /// Get the header/overhead bytes stored alongside the sample.
    /// Returns 0 for in-memory samples (no header overhead).
    pub fn header_size(&self) -> u32 {
        0
    }
}

/// A slice point within a sample, defining a sub-region that can be triggered independently.
///
/// Used for sample-based beat slicing (e.g., breakbeats, drum loops). Each slice
/// represents a segment of the parent sample that can be triggered by note number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slice {
    /// Start position in frames (inclusive).
    pub start_frame: usize,
    /// End position in frames (exclusive).
    pub end_frame: usize,
}

impl Slice {
    pub fn new(start_frame: usize, end_frame: usize) -> Self {
        Self {
            start_frame,
            end_frame: end_frame.max(start_frame),
        }
    }

    /// Length of this slice in frames.
    pub fn len(&self) -> usize {
        self.end_frame.saturating_sub(self.start_frame)
    }

    /// Whether this slice has zero length.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Sample {
    /// Get the slice list for this sample.
    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }

    /// Set manual slice points.
    pub fn set_slices(&mut self, slices: Vec<Slice>) {
        self.slices = slices;
    }

    /// Generate evenly-spaced slices by dividing the sample into `count` equal parts.
    pub fn slice_even(&mut self, count: usize) {
        if count == 0 || self.frame_count() == 0 {
            self.slices.clear();
            return;
        }
        let total = self.frame_count();
        let slice_len = total / count;
        self.slices = (0..count)
            .map(|i| {
                let start = i * slice_len;
                let end = if i == count - 1 {
                    total
                } else {
                    start + slice_len
                };
                Slice::new(start, end)
            })
            .collect();
    }

    /// Add a single manual slice point, splitting at the given frame.
    /// If there are no existing slices, creates two slices (before and after the point).
    pub fn add_slice_point(&mut self, frame: usize) {
        let total = self.frame_count();
        if frame == 0 || frame >= total {
            return;
        }

        if self.slices.is_empty() {
            self.slices.push(Slice::new(0, frame));
            self.slices.push(Slice::new(frame, total));
            return;
        }

        // Find the slice that contains this frame and split it
        if let Some(idx) = self
            .slices
            .iter()
            .position(|s| frame > s.start_frame && frame < s.end_frame)
        {
            let old_end = self.slices[idx].end_frame;
            self.slices[idx].end_frame = frame;
            self.slices.insert(idx + 1, Slice::new(frame, old_end));
        }
    }

    /// Remove a slice by index, merging it with the next slice.
    pub fn remove_slice(&mut self, index: usize) {
        if self.slices.len() <= 1 || index >= self.slices.len() {
            return;
        }
        if index + 1 < self.slices.len() {
            self.slices[index + 1].start_frame = self.slices[index].start_frame;
        }
        self.slices.remove(index);
    }

    /// Get the slice at the given index, if it exists.
    pub fn get_slice(&self, index: usize) -> Option<&Slice> {
        self.slices.get(index)
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
            volume: 1.0,
            finetune: 0,
            sustain_loop_mode: LoopMode::NoLoop,
            sustain_loop_start: 0,
            sustain_loop_end: 0,
            slices: Vec::new(),
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

    #[test]
    fn test_sample_finetune_property() {
        let sample = Sample::new(vec![0.0; 100], 44100, 1, None).with_finetune(50);
        assert_eq!(sample.finetune, 50);
    }

    #[test]
    fn test_slice_new() {
        let s = Slice::new(100, 200);
        assert_eq!(s.start_frame, 100);
        assert_eq!(s.end_frame, 200);
        assert_eq!(s.len(), 100);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_slice_inverted_range() {
        let s = Slice::new(200, 100);
        assert_eq!(s.end_frame, 200); // clamped to start
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }

    #[test]
    fn test_slice_even() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(4);
        assert_eq!(sample.slices().len(), 4);
        assert_eq!(sample.slices()[0].start_frame, 0);
        assert_eq!(sample.slices()[0].end_frame, 250);
        assert_eq!(sample.slices()[1].start_frame, 250);
        assert_eq!(sample.slices()[1].end_frame, 500);
        assert_eq!(sample.slices()[3].start_frame, 750);
        assert_eq!(sample.slices()[3].end_frame, 1000);
    }

    #[test]
    fn test_slice_even_zero() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(0);
        assert!(sample.slices().is_empty());
    }

    #[test]
    fn test_add_slice_point_empty() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.add_slice_point(500);
        assert_eq!(sample.slices().len(), 2);
        assert_eq!(sample.slices()[0], Slice::new(0, 500));
        assert_eq!(sample.slices()[1], Slice::new(500, 1000));
    }

    #[test]
    fn test_add_slice_point_split() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(2); // [0,500) [500,1000)
        sample.add_slice_point(250);
        assert_eq!(sample.slices().len(), 3);
        assert_eq!(sample.slices()[0], Slice::new(0, 250));
        assert_eq!(sample.slices()[1], Slice::new(250, 500));
        assert_eq!(sample.slices()[2], Slice::new(500, 1000));
    }

    #[test]
    fn test_add_slice_point_boundary_ignored() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.add_slice_point(0); // at start -- ignored
        assert!(sample.slices().is_empty());
        sample.add_slice_point(1000); // at end -- ignored
        assert!(sample.slices().is_empty());
    }

    #[test]
    fn test_remove_slice() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(4); // [0,250) [250,500) [500,750) [750,1000)
        sample.remove_slice(1); // remove [250,500), merge into next
        assert_eq!(sample.slices().len(), 3);
        assert_eq!(sample.slices()[1], Slice::new(250, 750));
    }

    #[test]
    fn test_remove_slice_single_noop() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(1);
        sample.remove_slice(0); // can't remove the only slice
        assert_eq!(sample.slices().len(), 1);
    }

    #[test]
    fn test_get_slice() {
        let mut sample = Sample::new(vec![0.0; 1000], 44100, 1, None);
        sample.slice_even(3);
        assert!(sample.get_slice(0).is_some());
        assert!(sample.get_slice(2).is_some());
        assert!(sample.get_slice(3).is_none());
    }
}
