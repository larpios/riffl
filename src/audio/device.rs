//! Audio device enumeration and management

use crate::audio::error::{AudioError, AudioResult};

/// Represents an audio device
pub struct AudioDevice {
    // Placeholder - will be implemented in phase 2
}

impl AudioDevice {
    /// Create a new AudioDevice
    pub fn new() -> AudioResult<Self> {
        Ok(AudioDevice {})
    }
}

impl Default for AudioDevice {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioDevice")
    }
}
