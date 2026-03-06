//! Audio sample data representation
//!
//! This module provides the Sample struct which represents loaded audio data
//! in memory. Samples can be loaded from various formats (WAV, FLAC, OGG) and
//! are stored in a format ready for playback.

/// MIDI note number for C-4 (standard tracker base pitch).
pub const C4_MIDI: u8 = 48;

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
}

impl Sample {
    /// Create a new Sample instance with default base note C-4.
    pub fn new(data: Vec<f32>, sample_rate: u32, channels: u16, name: Option<String>) -> Self {
        Self {
            data,
            sample_rate,
            channels,
            name,
            base_note: C4_MIDI,
        }
    }

    /// Create a new Sample with an explicit base note (MIDI note number).
    pub fn with_base_note(mut self, base_note: u8) -> Self {
        self.base_note = base_note;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_frequency_a4() {
        // A4 in tracker base (C4=48): C4_MIDI + 9 semitones
        let sample = Sample::default().with_base_note(C4_MIDI + 9);
        let freq = sample.base_frequency();
        assert!(
            (freq - 440.0).abs() < 1e-5,
            "A4 frequency should be approximately 440.0 Hz (within 1e-5), got {}",
            freq
        );
    }

    #[test]
    fn test_base_frequency_a5() {
        // A5 in tracker base (C4=48): C4_MIDI + 9 semitones + 1 octave
        let sample = Sample::default().with_base_note(C4_MIDI + 21);
        let freq = sample.base_frequency();
        assert!(
            (freq - 880.0).abs() < 1e-5,
            "A5 frequency should be approximately 880.0 Hz (within 1e-5), got {}",
            freq
        );
    }

    #[test]
    fn test_base_frequency_a3() {
        // A3 in tracker base (C4=48): (C4_MIDI - 12) + 9 semitones = C4_MIDI - 3
        let sample = Sample::default().with_base_note(C4_MIDI - 3);
        let freq = sample.base_frequency();
        assert!(
            (freq - 220.0).abs() < 1e-5,
            "A3 frequency should be approximately 220.0 Hz (within 1e-5), got {}",
            freq
        );
    }

    #[test]
    fn test_base_frequency_c4() {
        // Note 48 is C4 (default base note for Sample::default)
        let sample = Sample::default();
        assert_eq!(sample.base_note(), C4_MIDI, "Sample::default() should have base note C4_MIDI");
        let freq = sample.base_frequency();
        assert!(
            (freq - 261.625565).abs() < 1e-5,
            "C4 frequency should be approximately 261.63 Hz (within 1e-5), got {}",
            freq
        );
    }

    #[test]
    fn test_base_frequency_0() {
        // Extreme condition, base note 0
        let sample = Sample::default().with_base_note(0);
        let freq = sample.base_frequency();
        // (C4_MIDI + 9) semitones below A4 (440) -> 440 / (2^((C4_MIDI+9)/12))
        let expected = 440.0 * 2.0_f64.powf(-((C4_MIDI as f64 + 9.0) / 12.0));
        assert!(
            (freq - expected).abs() < 1e-5,
            "Note 0 frequency should be approximately {} (within 1e-5), got {}",
            expected,
            freq
        );
    }
}
