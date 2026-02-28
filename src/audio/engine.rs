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

    /// Create a new AudioEngine with a specific device and optimal configuration
    ///
    /// # Arguments
    ///
    /// * `device` - The audio device to use
    ///
    /// # Errors
    ///
    /// Returns an error if the device cannot be initialized
    pub fn with_device(device: AudioDevice) -> AudioResult<Self> {
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

    /// Get the current sample rate in Hz
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Set the sample rate in Hz
    ///
    /// Common sample rates:
    /// - 44100 Hz (CD quality)
    /// - 48000 Hz (professional audio, default)
    ///
    /// # Arguments
    ///
    /// * `rate` - The sample rate in Hz
    ///
    /// # Errors
    ///
    /// Returns an error if the sample rate is invalid (e.g., 0 Hz)
    pub fn set_sample_rate(&mut self, rate: u32) -> AudioResult<()> {
        if rate == 0 {
            return Err(AudioError::UnsupportedConfig(
                "Sample rate must be greater than 0".to_string()
            ));
        }
        self.config.sample_rate = rate;
        Ok(())
    }

    /// List all available audio output devices
    ///
    /// Returns a list of DeviceInfo containing device names and default status.
    /// This can be used to present device selection options to the user.
    ///
    /// # Errors
    ///
    /// Returns an error if device enumeration fails
    pub fn list_devices() -> AudioResult<Vec<crate::audio::device::DeviceInfo>> {
        crate::audio::device::enumerate_devices()
    }

    /// Select a specific audio device by index
    ///
    /// Changes the audio device to the one at the specified index.
    /// Use `list_devices()` to get available devices and their indices.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the device to select (from list_devices)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of bounds
    /// - The device cannot be accessed
    pub fn select_device(&mut self, index: usize) -> AudioResult<()> {
        let device = crate::audio::device::get_device_by_index(index)?;
        self.device = device;
        Ok(())
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

    #[test]
    fn test_select_device() {
        // Test that we can enumerate and select specific devices
        let devices = AudioEngine::list_devices();

        match devices {
            Ok(device_list) => {
                println!("Found {} audio devices", device_list.len());

                // Print all available devices
                for (index, device_info) in device_list.iter().enumerate() {
                    println!("  [{}] {} (default: {})",
                        index,
                        device_info.name,
                        device_info.is_default
                    );
                }

                // If we have at least one device, test device selection
                if !device_list.is_empty() {
                    // Create an engine with the default device
                    let mut engine = AudioEngine::new();

                    match engine {
                        Ok(ref mut eng) => {
                            let original_device = eng.device().name().ok();
                            println!("Original device: {:?}", original_device);

                            // Select the first device from the list
                            let select_result = eng.select_device(0);
                            assert!(select_result.is_ok(), "Should be able to select device at index 0");

                            let new_device = eng.device().name().ok();
                            println!("Selected device: {:?}", new_device);

                            // Verify the device was selected
                            assert!(new_device.is_some(), "Should have a device name after selection");

                            // If we have multiple devices, test selecting a different one
                            if device_list.len() > 1 {
                                let select_result = eng.select_device(1);
                                assert!(select_result.is_ok(), "Should be able to select device at index 1");
                                println!("Successfully selected second device");
                            }

                            // Test selecting an out-of-bounds index
                            let invalid_select = eng.select_device(999);
                            assert!(invalid_select.is_err(), "Should fail when selecting invalid index");
                            println!("Correctly rejected invalid device index");

                            println!("Device selection test passed!");
                        }
                        Err(AudioError::NoDefaultDevice) => {
                            // Can enumerate devices but no default - still acceptable in some environments
                            println!("Can enumerate devices but no default device available");
                        }
                        Err(e) => {
                            panic!("Unexpected error creating engine: {:?}", e);
                        }
                    }
                } else {
                    // No devices available - acceptable in CI environments
                    println!("No audio devices available (likely CI/test environment)");
                }
            }
            Err(e) => {
                // Device enumeration failed - acceptable in CI/test environments
                println!("Device enumeration failed (likely CI/test environment): {:?}", e);
            }
        }
    }

    #[test]
    fn test_sample_rate_config() {
        // Test that we can set and query sample rate
        let engine = AudioEngine::new();

        match engine {
            Ok(mut eng) => {
                // Verify default sample rate is 48kHz
                assert_eq!(eng.sample_rate(), 48000, "Default sample rate should be 48kHz");
                println!("Default sample rate: {}Hz", eng.sample_rate());

                // Test setting to 44.1kHz (CD quality)
                let result = eng.set_sample_rate(44100);
                assert!(result.is_ok(), "Should be able to set sample rate to 44.1kHz");
                assert_eq!(eng.sample_rate(), 44100, "Sample rate should be 44.1kHz after setting");
                println!("Set sample rate to 44.1kHz: {}Hz", eng.sample_rate());

                // Test setting to 48kHz (professional audio)
                let result = eng.set_sample_rate(48000);
                assert!(result.is_ok(), "Should be able to set sample rate to 48kHz");
                assert_eq!(eng.sample_rate(), 48000, "Sample rate should be 48kHz after setting");
                println!("Set sample rate to 48kHz: {}Hz", eng.sample_rate());

                // Test invalid sample rate (0 Hz)
                let result = eng.set_sample_rate(0);
                assert!(result.is_err(), "Should fail when setting sample rate to 0");
                if let Err(AudioError::UnsupportedConfig(msg)) = result {
                    assert!(msg.contains("Sample rate"), "Error message should mention sample rate");
                    println!("Correctly rejected invalid sample rate: {}", msg);
                }

                // Verify sample rate wasn't changed after invalid attempt
                assert_eq!(eng.sample_rate(), 48000, "Sample rate should remain 48kHz after failed set");

                println!("Sample rate configuration test passed!");
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
