//! High-level audio engine API

use crate::audio::device::AudioDevice;
use crate::audio::error::{AudioError, AudioResult};
use crate::audio::stream::StreamConfig;

/// High-level audio engine for managing audio playback
pub struct AudioEngine {
    device: AudioDevice,
    config: StreamConfig,
}

impl AudioEngine {
    /// Create a new AudioEngine with default device and optimal configuration
    ///
    /// This initializes the audio engine with:
    /// - The system's default output device
    /// - Optimal configuration (48kHz, stereo, 256 frame buffer)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No default audio device is available
    /// - The device cannot be initialized
    pub fn new() -> AudioResult<Self> {
        // Get the default audio device
        let device = crate::audio::device::default_device()?;

        // Use optimal default configuration
        // 48kHz is widely supported and provides good quality
        // Stereo (2 channels) is standard for music playback
        // 256 frames provides low latency (~5ms at 48kHz)
        let config = StreamConfig::default();

        Ok(AudioEngine {
            device,
            config,
        })
    }

    /// Get the current audio device
    pub fn device(&self) -> &AudioDevice {
        &self.device
    }

    /// Get the current stream configuration
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioEngine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_init() {
        // Test that AudioEngine initializes with default device and config
        let engine = AudioEngine::new();

        match engine {
            Ok(eng) => {
                // Verify the engine has a device
                let device_name = eng.device().name();
                assert!(device_name.is_ok(), "Should be able to get device name");
                println!("AudioEngine initialized with device: {}", device_name.unwrap());

                // Verify the engine has the default configuration
                let config = eng.config();
                assert_eq!(config.sample_rate, 48000, "Should use 48kHz sample rate");
                assert_eq!(config.channels, 2, "Should use stereo (2 channels)");
                assert_eq!(config.buffer_size, 256, "Should use 256 frame buffer");

                println!("AudioEngine initialized with config: sample_rate={}Hz, channels={}, buffer_size={} frames",
                    config.sample_rate, config.channels, config.buffer_size);

                println!("AudioEngine initialization test passed!");
            }
            Err(AudioError::NoDefaultDevice) => {
                // This is acceptable in CI/test environments without audio hardware
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error initializing AudioEngine: {:?}", e);
            }
        }
    }
}
