//! Error types for audio operations

use std::fmt;

/// Custom error type for audio operations
#[derive(Debug)]
pub enum AudioError {
    /// Error from the cpal library
    CpalError(cpal::BuildStreamError),
    /// Device not found
    DeviceNotFound,
    /// No default device available
    NoDefaultDevice,
    /// Unsupported configuration
    UnsupportedConfig(String),
    /// Stream error
    StreamError(String),
    /// Error loading a sample file
    LoadError(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::CpalError(e) => write!(f, "cpal error: {}", e),
            AudioError::DeviceNotFound => write!(f, "audio device not found"),
            AudioError::NoDefaultDevice => write!(f, "no default audio device available"),
            AudioError::UnsupportedConfig(msg) => write!(f, "unsupported configuration: {}", msg),
            AudioError::StreamError(msg) => write!(f, "stream error: {}", msg),
            AudioError::LoadError(msg) => write!(f, "sample load error: {}", msg),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<cpal::BuildStreamError> for AudioError {
    fn from(err: cpal::BuildStreamError) -> Self {
        AudioError::CpalError(err)
    }
}

/// Type alias for Result with AudioError
pub type AudioResult<T> = Result<T, AudioError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_not_found_error() {
        let err = AudioError::DeviceNotFound;
        assert_eq!(err.to_string(), "audio device not found");
    }

    #[test]
    fn test_no_default_device_error() {
        let err = AudioError::NoDefaultDevice;
        assert_eq!(err.to_string(), "no default audio device available");
    }

    #[test]
    fn test_unsupported_config_error() {
        let msg = "48kHz not supported";
        let err = AudioError::UnsupportedConfig(msg.to_string());
        assert_eq!(
            err.to_string(),
            "unsupported configuration: 48kHz not supported"
        );
    }

    #[test]
    fn test_stream_error() {
        let msg = "buffer underrun";
        let err = AudioError::StreamError(msg.to_string());
        assert_eq!(err.to_string(), "stream error: buffer underrun");
    }

    #[test]
    fn test_audio_result_ok() {
        let result: AudioResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_audio_result_err() {
        let result: AudioResult<i32> = Err(AudioError::DeviceNotFound);
        assert!(result.is_err());
    }
}
