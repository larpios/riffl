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
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::CpalError(e) => write!(f, "cpal error: {}", e),
            AudioError::DeviceNotFound => write!(f, "audio device not found"),
            AudioError::NoDefaultDevice => write!(f, "no default audio device available"),
            AudioError::UnsupportedConfig(msg) => write!(f, "unsupported configuration: {}", msg),
            AudioError::StreamError(msg) => write!(f, "stream error: {}", msg),
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
