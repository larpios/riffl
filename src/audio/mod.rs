//! Audio playback and sample management
//!
//! This module handles audio output, sample loading, and playback control.

pub mod device;
pub mod engine;
pub mod error;
pub mod loader;
pub mod mixer;
pub mod sample;
pub mod stream;

pub use device::{AudioDevice, DeviceInfo};
pub use engine::AudioEngine;
pub use error::{AudioError, AudioResult};
pub use loader::load_sample;
pub use mixer::Mixer;
pub use sample::{Sample, C4_MIDI};
pub use stream::{AudioCallback, AudioStream, StreamConfig};
