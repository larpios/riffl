//! Audio playback and sample management
//!
//! This module handles audio output, sample loading, and playback control.

pub mod channel_strip;
pub mod device;
pub mod dsp;
pub mod effect_processor;
pub mod engine;
pub mod error;
pub mod loader;
pub mod mixer;
pub mod sample;
pub mod stream;

pub use channel_strip::ChannelStrip;
pub use device::{AudioDevice, DeviceInfo};
pub use dsp::{DspProcessor, ProcessSpec, RampedParam};
pub use effect_processor::{TrackerEffectProcessor, TransportCommand, VoiceRenderState};
pub use engine::AudioEngine;
pub use error::{AudioError, AudioResult};
pub use loader::load_sample;
pub use mixer::Mixer;
pub use sample::{Sample, C4_MIDI};
pub use stream::{AudioCallback, AudioStream, StreamConfig};
