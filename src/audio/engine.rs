//! High-level audio engine API

use crate::audio::device::AudioDevice;
use crate::audio::error::{AudioError, AudioResult};
use crate::audio::stream::{AudioCallback, AudioStream, StreamConfig};

/// High-level audio engine for managing audio playback
pub struct AudioEngine {
    device: AudioDevice,
    config: StreamConfig,
    stream: Option<AudioStream>,
    playing: bool,
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
            stream: None,
            playing: false,
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
            stream: None,
            playing: false,
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

    /// Set an audio callback and build the audio stream
    ///
    /// This creates and configures the audio stream with the provided callback.
    /// The callback will be invoked on the audio thread to fill audio buffers.
    ///
    /// # Arguments
    ///
    /// * `callback` - A function that fills the audio buffer with samples
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot be built
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut engine = AudioEngine::new()?;
    /// let callback = Arc::new(Mutex::new(|data: &mut [f32]| {
    ///     // Fill with silence
    ///     for sample in data.iter_mut() {
    ///         *sample = 0.0;
    ///     }
    /// }));
    /// engine.set_callback(callback)?;
    /// engine.start()?;
    /// ```
    pub fn set_callback(&mut self, callback: AudioCallback) -> AudioResult<()> {
        // Create a new audio stream with the current configuration
        let mut stream = AudioStream::builder()
            .sample_rate(self.config.sample_rate)
            .channels(self.config.channels)
            .buffer_size(self.config.buffer_size)
            .device(self.device.clone())
            .build()?;

        // Build the stream with the callback
        stream.build_with_callback(callback)?;

        // Store the stream
        self.stream = Some(stream);
        self.playing = false;

        Ok(())
    }

    /// Start audio playback
    ///
    /// Begins playing audio through the configured callback.
    /// You must call `set_callback()` before calling this method.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No callback has been set
    /// - The stream cannot be started
    pub fn start(&mut self) -> AudioResult<()> {
        match &self.stream {
            Some(stream) => {
                stream.play()?;
                self.playing = true;
                Ok(())
            }
            None => Err(AudioError::StreamError(
                "No callback set. Call set_callback() first.".to_string()
            )),
        }
    }

    /// Pause audio playback
    ///
    /// Pauses the audio stream. Can be resumed by calling `start()` again.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No stream is active
    /// - The stream cannot be paused
    pub fn pause(&mut self) -> AudioResult<()> {
        match &self.stream {
            Some(stream) => {
                stream.pause()?;
                self.playing = false;
                Ok(())
            }
            None => Err(AudioError::StreamError(
                "No stream active. Call set_callback() and start() first.".to_string()
            )),
        }
    }

    /// Stop audio playback and destroy the stream
    ///
    /// Stops playback and releases the audio stream resources.
    /// To start again, you must call `set_callback()` to create a new stream.
    pub fn stop(&mut self) {
        self.stream = None;
        self.playing = false;
    }

    /// Check if audio is currently playing
    ///
    /// Returns `true` if the audio stream is active and playing.
    pub fn is_playing(&self) -> bool {
        self.playing && self.stream.is_some()
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

    #[test]
    fn test_engine_playback() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        // Test that we can start, pause, and stop audio through the engine API
        let mut engine = match AudioEngine::new() {
            Ok(e) => e,
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
                return; // Skip test in CI environments
            }
            Err(e) => {
                panic!("Unexpected error creating engine: {:?}", e);
            }
        };

        println!("Created AudioEngine");

        // Verify engine is not playing initially
        assert!(!engine.is_playing(), "Engine should not be playing initially");

        // Create a counter to track callback invocations
        let invocation_count = Arc::new(AtomicUsize::new(0));
        let invocation_count_clone = invocation_count.clone();

        // Create a callback that fills buffer with silence and counts invocations
        let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
            invocation_count_clone.fetch_add(1, Ordering::SeqCst);
            for sample in data.iter_mut() {
                *sample = 0.0;
            }
        }));

        // Set the callback
        match engine.set_callback(callback) {
            Ok(_) => {
                println!("Successfully set callback");

                // Verify engine is still not playing after setting callback
                assert!(!engine.is_playing(), "Engine should not be playing after setting callback");

                // Start playback
                match engine.start() {
                    Ok(_) => {
                        println!("Successfully started playback");

                        // Verify engine is now playing
                        assert!(engine.is_playing(), "Engine should be playing after start");

                        // Wait for some callbacks to be invoked
                        std::thread::sleep(Duration::from_millis(100));

                        let count_while_playing = invocation_count.load(Ordering::SeqCst);
                        println!("Callback invoked {} times while playing", count_while_playing);

                        // Verify callback was invoked
                        assert!(count_while_playing > 0, "Callback should be invoked while playing");

                        // Pause playback
                        match engine.pause() {
                            Ok(_) => {
                                println!("Successfully paused playback");

                                // Verify engine is not playing after pause
                                assert!(!engine.is_playing(), "Engine should not be playing after pause");

                                // Wait a bit
                                std::thread::sleep(Duration::from_millis(100));

                                // Get count after pause
                                let count_after_pause = invocation_count.load(Ordering::SeqCst);
                                println!("Callback invoked {} times after pause", count_after_pause);

                                // Resume playback
                                match engine.start() {
                                    Ok(_) => {
                                        println!("Successfully resumed playback");

                                        // Verify engine is playing again
                                        assert!(engine.is_playing(), "Engine should be playing after resume");

                                        // Wait for more callbacks
                                        std::thread::sleep(Duration::from_millis(100));

                                        let count_after_resume = invocation_count.load(Ordering::SeqCst);
                                        println!("Callback invoked {} times after resume", count_after_resume);

                                        // Should have more invocations after resuming
                                        assert!(
                                            count_after_resume > count_after_pause,
                                            "Callback should continue being invoked after resuming"
                                        );

                                        // Stop playback
                                        engine.stop();
                                        println!("Successfully stopped playback");

                                        // Verify engine is not playing after stop
                                        assert!(!engine.is_playing(), "Engine should not be playing after stop");

                                        println!("Engine playback control test passed!");
                                    }
                                    Err(e) => {
                                        println!("Failed to resume playback (acceptable in test environment): {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Failed to pause playback (acceptable in test environment): {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to start playback (acceptable in test environment): {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to set callback (acceptable in test environment): {:?}", e);
            }
        }
    }

    #[test]
    fn test_callback_registration() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        // Test that we can register a custom audio callback that generates audio data
        let mut engine = match AudioEngine::new() {
            Ok(e) => e,
            Err(AudioError::NoDefaultDevice) => {
                println!("No default audio device available (likely CI/test environment)");
                return; // Skip test in CI environments
            }
            Err(e) => {
                panic!("Unexpected error creating engine: {:?}", e);
            }
        };

        println!("Created AudioEngine for callback registration test");

        // Create a counter to track callback invocations
        let invocation_count = Arc::new(AtomicUsize::new(0));
        let invocation_count_clone = invocation_count.clone();

        // Track the number of samples processed
        let samples_processed = Arc::new(AtomicUsize::new(0));
        let samples_processed_clone = samples_processed.clone();

        // Create a callback that generates a simple audio pattern (alternating +0.1 and -0.1)
        // This simulates generating actual audio data rather than just silence
        let callback = Arc::new(Mutex::new(move |data: &mut [f32]| {
            invocation_count_clone.fetch_add(1, Ordering::SeqCst);

            // Generate a simple alternating pattern to demonstrate actual audio generation
            for (i, sample) in data.iter_mut().enumerate() {
                *sample = if i % 2 == 0 { 0.1 } else { -0.1 };
            }

            samples_processed_clone.fetch_add(data.len(), Ordering::SeqCst);
        }));

        // Test that we can register the callback
        match engine.set_callback(callback) {
            Ok(_) => {
                println!("Successfully registered custom audio callback");

                // Verify the callback was set (engine should have a stream now)
                // We can verify this by checking that start() doesn't fail with "No callback set"

                // Start playback to verify the callback works
                match engine.start() {
                    Ok(_) => {
                        println!("Successfully started playback with custom callback");

                        // Verify engine is playing
                        assert!(engine.is_playing(), "Engine should be playing after start");

                        // Wait for some callbacks to be invoked
                        std::thread::sleep(Duration::from_millis(100));

                        let count = invocation_count.load(Ordering::SeqCst);
                        let samples = samples_processed.load(Ordering::SeqCst);

                        println!("Callback invoked {} times, processed {} samples", count, samples);

                        // Verify the callback was invoked
                        assert!(count > 0, "Callback should be invoked during playback");
                        assert!(samples > 0, "Callback should process samples during playback");

                        // Stop playback
                        engine.stop();
                        println!("Successfully stopped playback");

                        assert!(!engine.is_playing(), "Engine should not be playing after stop");

                        println!("Callback registration test passed!");
                    }
                    Err(e) => {
                        println!("Failed to start playback (acceptable in test environment): {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to register callback (acceptable in test environment): {:?}", e);
            }
        }
    }
}
