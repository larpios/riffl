//! Biquad filter implementation using Direct Form II Transposed.
//!
//! This module provides a real-time safe biquad filter that implements
//! the `DspProcessor` trait. It includes factory methods for common
//! filter types based on the Audio EQ Cookbook formulas.

use crate::audio::dsp::{DspProcessor, ProcessSpec};
use std::f32::consts::PI;

/// Coefficients for a biquad filter.
///
/// The coefficients are normalized such that `a0 = 1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoefs {
    /// Feedforward coefficient 0
    pub b0: f32,
    /// Feedforward coefficient 1
    pub b1: f32,
    /// Feedforward coefficient 2
    pub b2: f32,
    /// Feedback coefficient 1
    pub a1: f32,
    /// Feedback coefficient 2
    pub a2: f32,
}

impl Default for BiquadCoefs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoefs {
    /// Creates a lowpass filter.
    pub fn lowpass(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a highpass filter.
    pub fn highpass(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = (1.0 + cos_w0) / 2.0;
        let b1 = -(1.0 + cos_w0);
        let b2 = (1.0 + cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a bandpass filter (constant skirt gain).
    pub fn bandpass(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = q * alpha;
        let b1 = 0.0;
        let b2 = -q * alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a notch filter.
    pub fn notch(frequency: f32, q: f32, sample_rate: f32) -> Self {
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = 1.0;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a peaking EQ (bell) filter.
    pub fn bell(frequency: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a low shelf filter.
    pub fn low_shelf(frequency: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let sqrt_a = a.sqrt();

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    /// Creates a high shelf filter.
    pub fn high_shelf(frequency: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * frequency / sample_rate;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let sqrt_a = a.sqrt();

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;

        Self::normalize(b0, b1, b2, a0, a1, a2)
    }

    fn normalize(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv_a0 = 1.0 / a0;
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        }
    }
}

/// A biquad filter processor using Direct Form II Transposed.
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coefs: BiquadCoefs,
    /// Per-channel delay elements: `[z1, z2]`
    state: Vec<[f32; 2]>,
}

impl BiquadFilter {
    /// Creates a new biquad filter with the given coefficients.
    pub fn new(coefs: BiquadCoefs) -> Self {
        Self {
            coefs,
            state: Vec::new(),
        }
    }

    /// Sets the filter coefficients.
    ///
    /// This is real-time safe and does not allocate.
    pub fn set_coefficients(&mut self, coefs: BiquadCoefs) {
        self.coefs = coefs;
    }
}

impl DspProcessor for BiquadFilter {
    fn prepare(&mut self, spec: ProcessSpec) {
        self.state.clear();
        self.state.resize(spec.channels, [0.0; 2]);
    }

    fn process_block(&mut self, buffer: &mut [f32], channels: usize) {
        if self.state.len() < channels {
            return; // Not properly prepared
        }

        let b0 = self.coefs.b0;
        let b1 = self.coefs.b1;
        let b2 = self.coefs.b2;
        let a1 = self.coefs.a1;
        let a2 = self.coefs.a2;

        for frame in buffer.chunks_exact_mut(channels) {
            for (ch, sample) in frame.iter_mut().enumerate() {
                let x = *sample;
                let state = &mut self.state[ch];

                // Direct Form II Transposed
                let y = b0 * x + state[0];
                state[0] = b1 * x - a1 * y + state[1];
                state[1] = b2 * x - a2 * y;

                *sample = y;
            }
        }
    }

    fn reset(&mut self) {
        for state in &mut self.state {
            state[0] = 0.0;
            state[1] = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowpass_passes_dc() {
        let coefs = BiquadCoefs::lowpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        let mut buffer = vec![1.0; 128];
        filter.process_block(&mut buffer, 1);

        // After settling, DC should pass through
        let last_sample = buffer.last().unwrap();
        assert!((last_sample - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_lowpass_attenuates_nyquist() {
        let coefs = BiquadCoefs::lowpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        let mut buffer = vec![0.0; 128];
        for (i, sample) in buffer.iter_mut().enumerate() {
            *sample = if i % 2 == 0 { 1.0 } else { -1.0 };
        }

        filter.process_block(&mut buffer, 1);

        // Nyquist should be heavily attenuated
        let last_sample = buffer.last().unwrap();
        assert!(last_sample.abs() < 0.01);
    }

    #[test]
    fn test_highpass_blocks_dc() {
        let coefs = BiquadCoefs::highpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        let mut buffer = vec![1.0; 128];
        filter.process_block(&mut buffer, 1);

        // DC should be blocked
        let last_sample = buffer.last().unwrap();
        assert!(last_sample.abs() < 0.01);
    }

    #[test]
    fn test_highpass_passes_nyquist() {
        let coefs = BiquadCoefs::highpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        let mut buffer = vec![0.0; 128];
        for (i, sample) in buffer.iter_mut().enumerate() {
            *sample = if i % 2 == 0 { 1.0 } else { -1.0 };
        }

        filter.process_block(&mut buffer, 1);

        // Nyquist should pass through
        let last_sample = buffer.last().unwrap();
        assert!((last_sample.abs() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_reset_clears_state() {
        let coefs = BiquadCoefs::lowpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        let mut buffer = vec![1.0; 128];
        filter.process_block(&mut buffer, 1);

        assert!(filter.state[0][0].abs() > 0.0);

        filter.reset();
        assert_eq!(filter.state[0][0], 0.0);
        assert_eq!(filter.state[0][1], 0.0);
    }

    #[test]
    fn test_set_coefficients_runtime() {
        let coefs1 = BiquadCoefs::lowpass(1000.0, 0.707, 48000.0);
        let coefs2 = BiquadCoefs::highpass(1000.0, 0.707, 48000.0);

        let mut filter = BiquadFilter::new(coefs1);
        assert_eq!(filter.coefs, coefs1);

        filter.set_coefficients(coefs2);
        assert_eq!(filter.coefs, coefs2);
    }

    #[test]
    fn test_process_block_stereo() {
        let coefs = BiquadCoefs::lowpass(1000.0, 0.707, 48000.0);
        let mut filter = BiquadFilter::new(coefs);
        filter.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 2,
        });

        let mut buffer = vec![0.0; 256];
        // Channel 0 is DC, Channel 1 is Nyquist
        for i in (0..256).step_by(2) {
            buffer[i] = 1.0;
            buffer[i + 1] = if (i / 2) % 2 == 0 { 1.0 } else { -1.0 };
        }

        filter.process_block(&mut buffer, 2);

        // Channel 0 (DC) should pass
        assert!((buffer[254] - 1.0).abs() < 0.01);
        // Channel 1 (Nyquist) should be attenuated
        assert!(buffer[255].abs() < 0.01);
    }

    #[test]
    fn test_bell_filter_creates_valid_coefs() {
        let coefs = BiquadCoefs::bell(1000.0, 1.0, 6.0, 48000.0);
        assert!(coefs.b0.is_finite());
        assert!(coefs.b1.is_finite());
        assert!(coefs.b2.is_finite());
        assert!(coefs.a1.is_finite());
        assert!(coefs.a2.is_finite());
    }

    #[test]
    fn test_shelf_filters_create_valid_coefs() {
        let low_coefs = BiquadCoefs::low_shelf(100.0, 0.707, 6.0, 48000.0);
        assert!(low_coefs.b0.is_finite());
        assert!(low_coefs.b1.is_finite());
        assert!(low_coefs.b2.is_finite());
        assert!(low_coefs.a1.is_finite());
        assert!(low_coefs.a2.is_finite());

        let high_coefs = BiquadCoefs::high_shelf(10000.0, 0.707, -6.0, 48000.0);
        assert!(high_coefs.b0.is_finite());
        assert!(high_coefs.b1.is_finite());
        assert!(high_coefs.b2.is_finite());
        assert!(high_coefs.a1.is_finite());
        assert!(high_coefs.a2.is_finite());
    }
}
