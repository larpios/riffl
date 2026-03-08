//! Audio effects and DSP processors.
//!
//! This module provides real-time safe audio effects that implement the
//! `DspProcessor` trait. These effects can be used in channel strips or
//! as global master effects.

pub mod biquad;
pub mod delay;

pub use biquad::{BiquadCoefs, BiquadFilter};
pub use delay::DelayLine;
