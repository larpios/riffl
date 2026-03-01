//! Audio sample data representation
//!
//! This module provides the Sample struct which represents loaded audio data
//! in memory. Samples can be loaded from various formats (WAV, FLAC, OGG) and
//! are stored in a format ready for playback.

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
}

impl Sample {
    /// Create a new Sample instance
    pub fn new(data: Vec<f32>, sample_rate: u32, channels: u16, name: Option<String>) -> Self {
        Self {
            data,
            sample_rate,
            channels,
            name,
        }
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
        }
    }
}
