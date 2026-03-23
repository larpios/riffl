//! Adlib/OPL chip synthesis for S3M files.
//!
//! Implements FM synthesis using the opl-emu crate for Adlib instruments
//! in Scream Tracker 3 modules.

use opl_emu::chip::{OplChipEmu, OplEmu, OplSample};

const OPL_SAMPLE_RATE: u32 = 49716;
const OPL_CHANNELS: usize = 9;

pub struct AdlibSynthesizer {
    chip: OplEmu,
    sample_rate: u32,
    buffer: Vec<f32>,
}

impl AdlibSynthesizer {
    pub fn new(sample_rate: u32) -> Self {
        let chip = OplEmu::new();
        Self {
            chip,
            sample_rate,
            buffer: Vec::with_capacity(1024),
        }
    }

    pub fn init(&mut self, registers: &[u8]) {
        if registers.len() >= 256 {
            for (i, &reg) in registers.iter().enumerate().take(256) {
                self.chip.write_reg(i as u8, reg);
            }
        }
    }

    pub fn render_samples(&mut self, num_samples: usize) -> &[f32] {
        self.buffer.clear();
        self.buffer.reserve(num_samples);

        for _ in 0..num_samples {
            let left = self.chip.sample();
            let right = self.chip.sample();
            let stereo_sample = (left + right) as f32 / (i16::MAX as f32 * 2.0);
            self.buffer.push(stereo_sample);
        }

        &self.buffer
    }

    pub fn note_on(&mut self, channel: usize, note: u8, velocity: u8) {
        if channel < OPL_CHANNELS {
            let freq = calculate_note_frequency(note);
            let reg_offset = channel * 3;
            self.chip.write_reg(0xa0 + channel, freq & 0xff);
            self.chip.write_reg(0xb0 + channel, (freq >> 8) | 0x20);
            self.chip
                .write_reg(0x83 + reg_offset, velocity.saturating_mul(2));
        }
    }

    pub fn note_off(&mut self, channel: usize) {
        if channel < OPL_CHANNELS {
            self.chip.write_reg(0xb0 + channel, 0);
        }
    }
}

fn calculate_note_frequency(note: u8) -> u16 {
    let octave = (note / 12) as i32;
    let note_in_octave = (note % 12) as i32;
    let base_freq: f64 = 440.0 * 2.0_f64.powf((note_in_octave as f64 - 9.0) / 12.0);
    let opl_freq = (base_freq * 512.0 / 49716.0) as u16;
    ((octave as u16) << 10) | (opl_freq & 0x3FF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adlib_synthesizer_creation() {
        let synth = AdlibSynthesizer::new(48000);
        assert_eq!(synth.sample_rate, 48000);
    }

    #[test]
    fn test_render_samples() {
        let mut synth = AdlibSynthesizer::new(48000);
        let samples = synth.render_samples(100);
        assert_eq!(samples.len(), 100);
        for &sample in samples {
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_note_frequency_calculation() {
        let freq = calculate_note_frequency(60);
        assert!(freq > 0);
        assert!(freq < 0x4000);
    }
}
