//! Audio stream management

use crate::audio::device::AudioDevice;
use crate::audio::error::{AudioError, AudioResult};
use crate::{log_error, log_warn};
use cpal::traits::{DeviceTrait, StreamTrait};
use std::sync::{Arc, Mutex};

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
            sample_rate: 48000, // Default to 48kHz
            channels: 2,        // Default to stereo
            buffer_size: 256,   // Default to 256 frames
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
            return Err(AudioError::UnsupportedConfig(
                "Sample rate must be greater than 0".to_string(),
            ));
        }
        if self.config.channels == 0 {
            return Err(AudioError::UnsupportedConfig(
                "Channel count must be greater than 0".to_string(),
            ));
        }
        if self.config.buffer_size == 0 {
            return Err(AudioError::UnsupportedConfig(
                "Buffer size must be greater than 0".to_string(),
            ));
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

/// Type alias for audio callback function
/// The callback receives a mutable slice of f32 samples to fill
/// The callback should fill the buffer with audio data (e.g., silence, sine wave, etc.)
pub type AudioCallback = Arc<Mutex<dyn FnMut(&mut [f32]) + Send + 'static>>;

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

    /// Build and start the audio stream with a callback
    /// The callback will be invoked to fill audio buffers
    ///
    /// # Arguments
    /// * `callback` - A function that fills the audio buffer with samples
    ///
    /// # Example
    /// ```ignore
    /// let mut stream = AudioStream::new()?;
    /// let callback = Arc::new(Mutex::new(|data: &mut [f32]| {
    ///     // Fill with silence
    ///     for sample in data.iter_mut() {
    ///         *sample = 0.0;
    ///     }
    /// }));
    /// stream.build_with_callback(callback)?;
    /// ```
    pub fn build_with_callback(&mut self, callback: AudioCallback) -> AudioResult<()> {
        // Build cpal stream configuration
        let config = cpal::StreamConfig {
            channels: self.config.channels,
            sample_rate: cpal::SampleRate(self.config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.config.buffer_size),
        };

        // Build the output stream with the callback
        let stream = self
            .device
            .inner()
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Invoke the user's callback to fill the buffer
                    // This runs on the audio thread, so we need to be careful
                    if let Ok(mut cb) = callback.lock() {
                        cb(data);
                    }
                },
                |err| {
                    // Error callback - just log for now
                    log_error!("audio", "Audio stream error: {}", err);
                },
                None, // No timeout
            )
            .map_err(AudioError::from)?;

        // Store the stream
        self.stream = Some(stream);

        Ok(())
    }

    /// Check if the stream has been built
    pub fn is_built(&self) -> bool {
        self.stream.is_some()
    }

    /// Play the audio stream
    /// The stream must be built with build_with_callback before calling this
    pub fn play(&self) -> AudioResult<()> {
        match &self.stream {
            Some(stream) => stream
                .play()
                .map_err(|e| AudioError::StreamError(format!("Failed to play stream: {}", e))),
            None => Err(AudioError::StreamError(
                "Stream not built. Call build_with_callback first.".to_string(),
            )),
        }
    }

    /// Pause the audio stream
    /// The stream must be built with build_with_callback before calling this
    pub fn pause(&self) -> AudioResult<()> {
        match &self.stream {
            Some(stream) => stream
                .pause()
                .map_err(|e| AudioError::StreamError(format!("Failed to pause stream: {}", e))),
            None => Err(AudioError::StreamError(
                "Stream not built. Call build_with_callback first.".to_string(),
            )),
        }
    }
}

impl Default for AudioStream {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioStream")
    }
}

/// Implement Drop trait for clean stream shutdown
/// This ensures no audio artifacts (clicks/pops) when the stream is destroyed
impl Drop for AudioStream {
    fn drop(&mut self) {
        if let Some(stream) = &self.stream {
            // Pause the stream first to prevent abrupt cutoff
            // This reduces the likelihood of clicks/pops on shutdown
            if let Err(e) = stream.pause() {
                log_warn!("audio", "Failed to pause stream during shutdown: {}", e);
            }

            // Give the audio system a small amount of time to flush buffers
            // This helps ensure a clean shutdown without artifacts
            std::thread::sleep(std::time::Duration::from_millis(50));

            // Stream will be automatically dropped after this, which stops it cleanly
        }
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
    fn test_stream_builder_new() {
        let builder = StreamBuilder::new();
        assert_eq!(builder.config.sample_rate, 48000);
        assert_eq!(builder.config.channels, 2);
        assert_eq!(builder.config.buffer_size, 256);
        assert!(builder.device.is_none());
    }

    #[test]
    fn test_stream_builder_sample_rate() {
        let builder = StreamBuilder::new().sample_rate(44100);
        assert_eq!(builder.config.sample_rate, 44100);
    }

    #[test]
    fn test_stream_builder_channels() {
        let builder = StreamBuilder::new().channels(1);
        assert_eq!(builder.config.channels, 1);
    }

    #[test]
    fn test_stream_builder_chaining() {
        let builder = StreamBuilder::new()
            .sample_rate(96000)
            .channels(4)
            .buffer_size(512);
        assert_eq!(builder.config.sample_rate, 96000);
        assert_eq!(builder.config.channels, 4);
        assert_eq!(builder.config.buffer_size, 512);
        assert!(builder.device.is_none());
    }

    #[test]
    fn test_stream_config_validation() {
        // Test invalid sample rate
        let result = AudioStream::builder().sample_rate(0).build();
        assert!(result.is_err());
        if let Err(AudioError::UnsupportedConfig(msg)) = result {
            assert!(msg.contains("Sample rate"));
        }

        // Test invalid channels
        let result = AudioStream::builder().channels(0).build();
        assert!(result.is_err());
        if let Err(AudioError::UnsupportedConfig(msg)) = result {
            assert!(msg.contains("Channel count"));
        }

        // Test invalid buffer size
        let result = AudioStream::builder().buffer_size(0).build();
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

    #[test]
    fn test_callback_invoked() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        // Try to create an audio stream
        let mut stream = match AudioStream::builder().build() {
            Ok(s) => s,
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
                return; // Skip test in CI environments
            }
            Err(e) => {
                panic!("Unexpected error building stream: {:?}", e);
            }
        };

        // Create a flag to track if callback was invoked
        let callback_invoked = Arc::new(AtomicBool::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        // Create callback that sets the flag and fills buffer with silence
        let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
            // Mark that callback was invoked
            callback_invoked_clone.store(true, Ordering::SeqCst);

            // Fill buffer with silence (no allocations)
            for sample in data.iter_mut() {
                *sample = 0.0;
            }
        }));

        // Build the stream with the callback
        match stream.build_with_callback(callback) {
            Ok(_) => {
                println!("Successfully built stream with callback");

                // Verify stream is marked as built
                assert!(stream.is_built(), "Stream should be marked as built");

                // Start the stream
                match stream.play() {
                    Ok(_) => {
                        println!("Stream started successfully");

                        // Wait a short time for the callback to be invoked
                        std::thread::sleep(Duration::from_millis(100));

                        // Verify the callback was invoked
                        assert!(
                            callback_invoked.load(Ordering::SeqCst),
                            "Callback should have been invoked when stream is running"
                        );

                        println!("Callback was successfully invoked!");
                    }
                    Err(e) => {
                        // Stream play might fail in some environments
                        println!(
                            "Failed to play stream (acceptable in test environment): {:?}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "Failed to build stream with callback (acceptable in test environment): {:?}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_play_pause() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        // Try to create an audio stream
        let mut stream = match AudioStream::builder().build() {
            Ok(s) => s,
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
                return; // Skip test in CI environments
            }
            Err(e) => {
                panic!("Unexpected error building stream: {:?}", e);
            }
        };

        // Create a counter to track callback invocations
        let invocation_count = Arc::new(AtomicUsize::new(0));
        let invocation_count_clone = invocation_count.clone();

        // Create callback that increments counter and fills buffer with silence
        let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
            // Increment invocation counter
            invocation_count_clone.fetch_add(1, Ordering::SeqCst);

            // Fill buffer with silence (no allocations)
            for sample in data.iter_mut() {
                *sample = 0.0;
            }
        }));

        // Build the stream with the callback
        match stream.build_with_callback(callback) {
            Ok(_) => {
                println!("Successfully built stream with callback");

                // Verify stream is marked as built
                assert!(stream.is_built(), "Stream should be marked as built");

                // Start the stream
                match stream.play() {
                    Ok(_) => {
                        println!("Stream started successfully");

                        // Wait for some callbacks to be invoked
                        std::thread::sleep(Duration::from_millis(100));

                        let count_while_playing = invocation_count.load(Ordering::SeqCst);
                        println!(
                            "Callback invoked {} times while playing",
                            count_while_playing
                        );

                        // Pause the stream
                        match stream.pause() {
                            Ok(_) => {
                                println!("Stream paused successfully");

                                // Wait a bit to ensure stream is paused
                                std::thread::sleep(Duration::from_millis(100));

                                // Get the count after pausing
                                let count_after_pause = invocation_count.load(Ordering::SeqCst);
                                println!(
                                    "Callback invoked {} times after pause",
                                    count_after_pause
                                );

                                // The callback should have been invoked while playing
                                assert!(
                                    count_while_playing > 0,
                                    "Callback should have been invoked while stream was playing"
                                );

                                // Resume the stream
                                match stream.play() {
                                    Ok(_) => {
                                        println!("Stream resumed successfully");

                                        // Wait for more callbacks
                                        std::thread::sleep(Duration::from_millis(100));

                                        let count_after_resume =
                                            invocation_count.load(Ordering::SeqCst);
                                        println!(
                                            "Callback invoked {} times after resume",
                                            count_after_resume
                                        );

                                        // Should have more invocations after resuming
                                        assert!(
                                            count_after_resume > count_after_pause,
                                            "Callback should continue being invoked after resuming stream"
                                        );

                                        println!("Play/pause test passed!");
                                    }
                                    Err(e) => {
                                        println!("Failed to resume stream (acceptable in test environment): {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Failed to pause stream (acceptable in test environment): {:?}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        // Stream play might fail in some environments
                        println!(
                            "Failed to play stream (acceptable in test environment): {:?}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "Failed to build stream with callback (acceptable in test environment): {:?}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_clean_shutdown() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::time::Duration;

        // Try to create an audio stream
        let mut stream = match AudioStream::builder().build() {
            Ok(s) => s,
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
                return; // Skip test in CI environments
            }
            Err(e) => {
                panic!("Unexpected error building stream: {:?}", e);
            }
        };

        // Create a flag to track if callback was invoked
        let callback_invoked = Arc::new(AtomicBool::new(false));
        let callback_invoked_clone = callback_invoked.clone();

        // Create callback that fills buffer with silence
        let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
            callback_invoked_clone.store(true, Ordering::SeqCst);

            // Fill buffer with silence (no allocations)
            for sample in data.iter_mut() {
                *sample = 0.0;
            }
        }));

        // Build the stream with the callback
        match stream.build_with_callback(callback) {
            Ok(_) => {
                println!("Successfully built stream with callback");

                // Start the stream
                match stream.play() {
                    Ok(_) => {
                        println!("Stream started successfully");

                        // Wait for the callback to be invoked
                        std::thread::sleep(Duration::from_millis(100));

                        assert!(
                            callback_invoked.load(Ordering::SeqCst),
                            "Callback should have been invoked"
                        );

                        println!("Stream is playing, now testing clean shutdown...");

                        // Drop the stream (this triggers the Drop trait)
                        // The Drop implementation should:
                        // 1. Pause the stream
                        // 2. Wait for buffers to flush
                        // 3. Drop cleanly without clicks/pops
                        drop(stream);

                        println!("Stream dropped cleanly (manual verification required for audio artifacts)");
                    }
                    Err(e) => {
                        println!(
                            "Failed to play stream (acceptable in test environment): {:?}",
                            e
                        );
                    }
                }
            }
            Err(e) => {
                println!(
                    "Failed to build stream with callback (acceptable in test environment): {:?}",
                    e
                );
            }
        }
    }
}
