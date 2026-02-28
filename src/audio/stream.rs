//! Audio stream management

use crate::audio::error::{AudioError, AudioResult};
use crate::audio::device::AudioDevice;

/// Configuration for an audio stream
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u16,
    /// Buffer size in frames
    pub buffer_size: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            sample_rate: 48000,  // Default to 48kHz
            channels: 2,          // Default to stereo
            buffer_size: 256,     // Default to 256 frames
        }
    }
}

/// Builder for constructing an AudioStream with custom configuration
pub struct StreamBuilder {
    config: StreamConfig,
    device: Option<AudioDevice>,
}

impl StreamBuilder {
    /// Create a new StreamBuilder with default configuration
    pub fn new() -> Self {
        StreamBuilder {
            config: StreamConfig::default(),
            device: None,
        }
    }

    /// Set the sample rate in Hz
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.config.sample_rate = rate;
        self
    }

    /// Set the number of channels
    pub fn channels(mut self, channels: u16) -> Self {
        self.config.channels = channels;
        self
    }

    /// Set the buffer size in frames
    pub fn buffer_size(mut self, size: u32) -> Self {
        self.config.buffer_size = size;
        self
    }

    /// Set the audio device to use
    pub fn device(mut self, device: AudioDevice) -> Self {
        self.device = Some(device);
        self
    }

    /// Build the AudioStream with the configured settings
    pub fn build(self) -> AudioResult<AudioStream> {
        // Validate configuration
        if self.config.sample_rate == 0 {
            return Err(AudioError::UnsupportedConfig("Sample rate must be greater than 0".to_string()));
        }
        if self.config.channels == 0 {
            return Err(AudioError::UnsupportedConfig("Channel count must be greater than 0".to_string()));
        }
        if self.config.buffer_size == 0 {
            return Err(AudioError::UnsupportedConfig("Buffer size must be greater than 0".to_string()));
        }

        // Use default device if not specified
        let device = match self.device {
            Some(d) => d,
            None => crate::audio::device::default_device()?,
        };

        Ok(AudioStream {
            config: self.config,
            device,
            stream: None,
        })
    }
}

impl Default for StreamBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an audio stream
pub struct AudioStream {
    config: StreamConfig,
    device: AudioDevice,
    stream: Option<cpal::Stream>,
}

impl AudioStream {
    /// Create a new AudioStream with default configuration
    pub fn new() -> AudioResult<Self> {
        StreamBuilder::new().build()
    }

    /// Create a StreamBuilder for custom configuration
    pub fn builder() -> StreamBuilder {
        StreamBuilder::new()
    }

    /// Get the stream configuration
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }

    /// Get the audio device
    pub fn device(&self) -> &AudioDevice {
        &self.device
    }
}

impl Default for AudioStream {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioStream")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stream() {
        // Test building stream with default configuration
        let stream = AudioStream::builder().build();

        match stream {
            Ok(s) => {
                // Verify default configuration
                assert_eq!(s.config().sample_rate, 48000);
                assert_eq!(s.config().channels, 2);
                assert_eq!(s.config().buffer_size, 256);
                println!("Built stream with default config: {:?}", s.config());
            }
            Err(AudioError::NoDefaultDevice) => {
                // This is acceptable in CI/test environments without audio hardware
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error building stream: {:?}", e);
            }
        }

        // Test building stream with custom configuration
        let custom_stream = AudioStream::builder()
            .sample_rate(44100)
            .channels(1)
            .buffer_size(512)
            .build();

        match custom_stream {
            Ok(s) => {
                // Verify custom configuration
                assert_eq!(s.config().sample_rate, 44100);
                assert_eq!(s.config().channels, 1);
                assert_eq!(s.config().buffer_size, 512);
                println!("Built stream with custom config: {:?}", s.config());
            }
            Err(AudioError::NoDefaultDevice) => {
                // This is acceptable in CI/test environments without audio hardware
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error building custom stream: {:?}", e);
            }
        }
    }

    #[test]
    fn test_stream_config_validation() {
        // Test invalid sample rate
        let result = AudioStream::builder()
            .sample_rate(0)
            .build();
        assert!(result.is_err());
        if let Err(AudioError::UnsupportedConfig(msg)) = result {
            assert!(msg.contains("Sample rate"));
        }

        // Test invalid channels
        let result = AudioStream::builder()
            .channels(0)
            .build();
        assert!(result.is_err());
        if let Err(AudioError::UnsupportedConfig(msg)) = result {
            assert!(msg.contains("Channel count"));
        }

        // Test invalid buffer size
        let result = AudioStream::builder()
            .buffer_size(0)
            .build();
        assert!(result.is_err());
        if let Err(AudioError::UnsupportedConfig(msg)) = result {
            assert!(msg.contains("Buffer size"));
        }
    }

    #[test]
    fn test_default_stream_config() {
        let config = StreamConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.buffer_size, 256);
    }

    #[test]
    fn test_builder_fluent_api() {
        // Test that builder methods return self for chaining
        let builder = StreamBuilder::new()
            .sample_rate(44100)
            .channels(1)
            .buffer_size(1024);

        // Verify configuration was set correctly
        let stream = builder.build();

        match stream {
            Ok(s) => {
                assert_eq!(s.config().sample_rate, 44100);
                assert_eq!(s.config().channels, 1);
                assert_eq!(s.config().buffer_size, 1024);
            }
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }
}
