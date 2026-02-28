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

/// Represents an audio device
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
        self.device
            .name()
            .map_err(|_| AudioError::DeviceNotFound)
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
    let default_name = default_device
        .as_ref()
        .and_then(|d| d.name().ok());

    let mut devices = Vec::new();

    let output_devices = host
        .output_devices()
        .map_err(|_| AudioError::DeviceNotFound)?;

    for device in output_devices {
        if let Ok(name) = device.name() {
            let is_default = default_name.as_ref().map_or(false, |dn| dn == &name);
            devices.push(DeviceInfo {
                name,
                is_default,
            });
        }
    }

    Ok(devices)
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
}
