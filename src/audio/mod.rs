//! Audio module providing low-latency audio playback using cpal

pub mod device;
pub mod engine;
pub mod error;
pub mod stream;

// Re-export main public API
pub use device::{AudioDevice, DeviceInfo, enumerate_devices};
pub use engine::AudioEngine;
pub use error::{AudioError, AudioResult};
pub use stream::AudioStream;
