//! Chip-oriented sample conversion utilities.
//!
//! These helpers derive low-resolution chip-friendly payloads from high-resolution
//! PCM sample data so the UI can expose how an edited sample maps onto tighter
//! console-era constraints.

use serde::{Deserialize, Serialize};

use crate::audio::sample::Sample;

/// Number of 4-bit entries emitted for the wavetable preview.
pub const CHIP_WAVETABLE_LEN: usize = 32;

/// Default number of DPCM bytes emitted for the preview payload.
pub const CHIP_DPCM_BYTES: usize = 64;

/// Serializable chip conversion results for a sample-backed instrument.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChipRenderData {
    /// Source frame count before conversion.
    pub source_frames: usize,
    /// 32-entry 4-bit wavetable preview.
    pub wavetable_2a03: Vec<u8>,
    /// Packed DPCM-style 1-bit delta stream preview.
    pub dpcm: Vec<u8>,
    /// Mean absolute reconstruction error for the wavetable conversion.
    pub wavetable_error: f32,
    /// Mean absolute reconstruction error for the DPCM conversion.
    pub dpcm_error: f32,
}

impl ChipRenderData {
    /// Build chip conversion previews from a PCM sample.
    pub fn from_sample(sample: &Sample) -> Self {
        let mono = downmix_sample(sample);
        if mono.is_empty() {
            return Self::default();
        }

        let wavetable_2a03 = sample_to_wavetable(&mono, CHIP_WAVETABLE_LEN);
        let dpcm = sample_to_dpcm(&mono, CHIP_DPCM_BYTES);
        let wavetable_error = wavetable_error(&mono, &wavetable_2a03);
        let dpcm_error = dpcm_error(&mono, &dpcm);

        Self {
            source_frames: mono.len(),
            wavetable_2a03,
            dpcm,
            wavetable_error,
            dpcm_error,
        }
    }
}

fn downmix_sample(sample: &Sample) -> Vec<f32> {
    let channels = sample.channels() as usize;
    if channels == 0 {
        return Vec::new();
    }

    sample
        .data()
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

fn sample_to_wavetable(samples: &[f32], output_len: usize) -> Vec<u8> {
    if samples.is_empty() || output_len == 0 {
        return Vec::new();
    }

    let mut wavetable = Vec::with_capacity(output_len);
    for idx in 0..output_len {
        let start = idx * samples.len() / output_len;
        let end = ((idx + 1) * samples.len() / output_len)
            .max(start + 1)
            .min(samples.len());
        let avg = samples[start..end].iter().copied().sum::<f32>() / (end - start) as f32;
        let quantized = (((avg.clamp(-1.0, 1.0) + 1.0) * 7.5).round() as i32).clamp(0, 15);
        wavetable.push(quantized as u8);
    }
    wavetable
}

fn sample_to_dpcm(samples: &[f32], output_bytes: usize) -> Vec<u8> {
    if samples.is_empty() || output_bytes == 0 {
        return Vec::new();
    }

    let total_bits = output_bytes * 8;
    let mut payload = Vec::with_capacity(output_bytes);
    let mut level = 0.0f32;

    for byte_idx in 0..output_bytes {
        let mut byte = 0u8;
        for bit_idx in 0..8 {
            let sample_idx = (byte_idx * 8 + bit_idx) * samples.len() / total_bits;
            let target = samples[sample_idx.min(samples.len() - 1)];
            if target >= level {
                byte |= 1 << bit_idx;
                level = (level + (2.0 / 63.0)).min(1.0);
            } else {
                level = (level - (2.0 / 63.0)).max(-1.0);
            }
        }
        payload.push(byte);
    }

    payload
}

fn wavetable_error(source: &[f32], wavetable: &[u8]) -> f32 {
    if source.is_empty() || wavetable.is_empty() {
        return 0.0;
    }

    let mut error_sum = 0.0;
    for (idx, &sample) in source.iter().enumerate() {
        let wave_idx = idx * wavetable.len() / source.len();
        let reconstructed = (wavetable[wave_idx] as f32 / 7.5) - 1.0;
        error_sum += (sample - reconstructed).abs();
    }
    error_sum / source.len() as f32
}

fn dpcm_error(source: &[f32], payload: &[u8]) -> f32 {
    if source.is_empty() || payload.is_empty() {
        return 0.0;
    }

    let total_bits = payload.len() * 8;
    let mut reconstruction = Vec::with_capacity(total_bits);
    let mut level = 0.0f32;

    for &byte in payload {
        for bit_idx in 0..8 {
            let bit = (byte >> bit_idx) & 1;
            if bit == 1 {
                level = (level + (2.0 / 63.0)).min(1.0);
            } else {
                level = (level - (2.0 / 63.0)).max(-1.0);
            }
            reconstruction.push(level);
        }
    }

    let mut error_sum = 0.0;
    for (idx, &sample) in source.iter().enumerate() {
        let recon_idx = idx * total_bits / source.len();
        error_sum += (sample - reconstruction[recon_idx]).abs();
    }
    error_sum / source.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample() -> Sample {
        let data: Vec<f32> = (0..256)
            .map(|idx| ((idx as f32 / 255.0) * std::f32::consts::TAU).sin())
            .collect();
        Sample::new(data, 44_100, 1, Some("chip".to_string()))
    }

    #[test]
    fn test_chip_render_data_from_sample() {
        let sample = make_sample();
        let chip = ChipRenderData::from_sample(&sample);

        assert_eq!(chip.source_frames, 256);
        assert_eq!(chip.wavetable_2a03.len(), CHIP_WAVETABLE_LEN);
        assert_eq!(chip.dpcm.len(), CHIP_DPCM_BYTES);
        assert!(chip.wavetable_2a03.iter().all(|&n| n <= 15));
        assert!(chip.wavetable_error >= 0.0);
        assert!(chip.dpcm_error >= 0.0);
    }

    #[test]
    fn test_chip_render_data_empty_sample() {
        let sample = Sample::new(Vec::new(), 44_100, 1, Some("empty".to_string()));
        let chip = ChipRenderData::from_sample(&sample);

        assert_eq!(chip, ChipRenderData::default());
    }
}
