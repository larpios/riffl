//! Audio device enumeration and management

use crate::audio::error::{AudioError, AudioResult};
use cpal::traits::{DeviceTrait, HostTrait};

/// Information about an audio device
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Device name
    pub name: String,
    /// Whether this is the default device
    pub is_default: bool,
}

/// Supported configuration for an audio device
#[derive(Debug, Clone)]
pub struct SupportedConfig {
    /// Minimum supported sample rate
    pub min_sample_rate: u32,
    /// Maximum supported sample rate
    pub max_sample_rate: u32,
    /// Number of channels
    pub channels: u16,
}

impl SupportedConfig {
    /// Check if a specific sample rate is supported
    pub fn supports_sample_rate(&self, rate: u32) -> bool {
        rate >= self.min_sample_rate && rate <= self.max_sample_rate
    }
}

/// Represents an audio device
#[derive(Clone)]
pub struct AudioDevice {
    device: cpal::Device,
}

impl AudioDevice {
    /// Create a new AudioDevice from a cpal device
    pub fn new(device: cpal::Device) -> AudioResult<Self> {
        Ok(AudioDevice { device })
    }

    /// Get the underlying cpal device
    pub fn inner(&self) -> &cpal::Device {
        &self.device
    }

    /// Get device name
    pub fn name(&self) -> AudioResult<String> {
        self.device.name().map_err(|_| AudioError::DeviceNotFound)
    }

    /// Get supported output configurations for this device
    pub fn supported_configs(&self) -> AudioResult<Vec<SupportedConfig>> {
        let configs = self.device.supported_output_configs().map_err(|_| {
            AudioError::UnsupportedConfig("Failed to query supported configs".to_string())
        })?;

        let mut supported = Vec::new();
        for config_range in configs {
            supported.push(SupportedConfig {
                min_sample_rate: config_range.min_sample_rate().0,
                max_sample_rate: config_range.max_sample_rate().0,
                channels: config_range.channels(),
            });
        }

        Ok(supported)
    }
}

impl Default for AudioDevice {
    fn default() -> Self {
        default_device().expect("Failed to create default AudioDevice")
    }
}

/// Get the system default output device
pub fn default_device() -> AudioResult<AudioDevice> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or(AudioError::NoDefaultDevice)?;
    AudioDevice::new(device)
}

/// Enumerate all available output devices
pub fn enumerate_devices() -> AudioResult<Vec<DeviceInfo>> {
    let host = cpal::default_host();

    let default_device = host.default_output_device();
    let default_name = default_device.as_ref().and_then(|d| d.name().ok());

    let mut devices = Vec::new();

    let output_devices = host
        .output_devices()
        .map_err(|_| AudioError::DeviceNotFound)?;

    for device in output_devices {
        if let Ok(name) = device.name() {
            let is_default = default_name.as_ref().map_or(false, |dn| dn == &name);
            devices.push(DeviceInfo { name, is_default });
        }
    }

    Ok(devices)
}

/// Get an audio device by index
///
/// # Arguments
///
/// * `index` - The index of the device to retrieve (from enumerate_devices)
///
/// # Errors
///
/// Returns an error if the index is out of bounds or the device cannot be accessed
pub fn get_device_by_index(index: usize) -> AudioResult<AudioDevice> {
    let host = cpal::default_host();

    let output_devices = host
        .output_devices()
        .map_err(|_| AudioError::DeviceNotFound)?;

    let device = output_devices
        .skip(index)
        .next()
        .ok_or(AudioError::DeviceNotFound)?;

    AudioDevice::new(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        let devices = enumerate_devices();
        assert!(devices.is_ok(), "Failed to enumerate devices");

        let devices = devices.unwrap();
        // There should be at least one output device on most systems
        // However, in CI or test environments, this might not be guaranteed
        // so we just verify the function completes without error
        println!("Found {} audio devices", devices.len());

        for device in &devices {
            println!("  Device: {} (default: {})", device.name, device.is_default);
        }

        // Verify that at most one device is marked as default
        let default_count = devices.iter().filter(|d| d.is_default).count();
        assert!(default_count <= 1, "Multiple devices marked as default");
    }

    #[test]
    fn test_default_device() {
        let device = default_device();

        // In CI/test environments without audio hardware, this might fail
        // but on systems with audio devices, it should succeed
        match device {
            Ok(audio_device) => {
                // Verify we can get the device name
                let name = audio_device.name();
                assert!(name.is_ok(), "Failed to get device name");
                println!("Default device: {}", name.unwrap());
            }
            Err(AudioError::NoDefaultDevice) => {
                // This is acceptable in environments without audio hardware
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error getting default device: {:?}", e);
            }
        }
    }

    #[test]
    fn test_supported_configs() {
        let device = default_device();

        match device {
            Ok(audio_device) => {
                // Query supported configurations
                let configs_result = audio_device.supported_configs();
                if let Err(e) = configs_result {
                    // This is acceptable in environments without proper audio hardware configurations
                    println!(
                        "Failed to query supported configs (likely CI/test environment): {:?}",
                        e
                    );
                    return;
                }

                let configs = configs_result.unwrap();
                println!("Found {} supported configurations", configs.len());

                // Common sample rates to check for
                let common_rates = [44100, 48000];

                for config in &configs {
                    println!(
                        "  Config: {} channels, sample rate {}-{} Hz",
                        config.channels, config.min_sample_rate, config.max_sample_rate
                    );

                    // Check if common sample rates are supported
                    for &rate in &common_rates {
                        if config.supports_sample_rate(rate) {
                            println!("    -> Supports {} Hz", rate);
                        }
                    }
                }

                // Verify at least one configuration exists
                assert!(
                    !configs.is_empty(),
                    "Device should have at least one supported config"
                );

                // Verify sample rate ranges are valid
                for config in &configs {
                    assert!(
                        config.min_sample_rate <= config.max_sample_rate,
                        "Invalid sample rate range"
                    );
                    assert!(config.channels > 0, "Channel count must be positive");
                }
            }
            Err(AudioError::NoDefaultDevice) => {
                // This is acceptable in environments without audio hardware
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error getting default device: {:?}", e);
            }
        }
    }
}
