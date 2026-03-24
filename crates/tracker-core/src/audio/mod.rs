//! Audio playback and sample management
//!
//! This module handles audio output, sample loading, and playback control.

#[cfg(feature = "adlib")]
pub mod adlib;
pub mod bus;
pub mod channel_strip;
pub mod chip;
pub mod device;
pub mod dsp;
pub mod effect_processor;
pub mod effects;
pub mod engine;
pub mod error;
pub mod glicol_mixer;
pub mod loader;
pub mod mixer;
pub mod pitch;
pub mod sample;
pub mod stream;

pub use bus::{BusSystem, SendBus, DEFAULT_NUM_BUSES};
pub use channel_strip::ChannelStrip;
pub use chip::{ChipRenderData, CHIP_DPCM_BYTES, CHIP_WAVETABLE_LEN};
pub use device::{AudioDevice, DeviceInfo};
pub use dsp::{DspProcessor, ProcessSpec, RampedParam};
pub use effect_processor::{TrackerEffectProcessor, TransportCommand, VoiceRenderState};
pub use effects::{BiquadCoefs, BiquadFilter, DelayLine};
pub use engine::AudioEngine;
pub use error::{AudioError, AudioResult};
pub use loader::load_sample;
pub use mixer::Mixer;
pub use pitch::SlideMode;
pub use sample::{LoopMode, Sample, C4_MIDI};
pub use stream::{AudioCallback, AudioStream, StreamConfig};
