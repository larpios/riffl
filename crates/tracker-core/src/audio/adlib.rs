//! Adlib/OPL chip synthesis for S3M files.
//!
//! Implements FM synthesis using the opl-emu crate for Adlib instruments
//! in Scream Tracker 3 modules.

#![allow(unused)]

use opl::chip::Chip;

const OPL_CHANNELS: usize = 9;

pub struct AdlibSynthesizer {
    chip: Chip,
    sample_rate: u32,
    buffer: Vec<f32>,
}

impl AdlibSynthesizer {
    pub fn new(sample_rate: u32) -> Self {
        let mut chip = Chip::new(sample_rate);
        chip.setup();
        Self {
            chip,
            sample_rate,
            buffer: Vec::with_capacity(1024),
        }
    }

    pub fn init(&mut self, registers: &[u8]) {
        if registers.len() >= 256 {
            for (i, &reg) in registers.iter().enumerate().take(256) {
                self.chip.write_reg(i as u32, reg);
            }
        }
    }

    pub fn render_samples(&mut self, num_samples: usize) -> &[f32] {
        self.buffer.clear();
        self.buffer.resize(num_samples, 0.0);

        // Generate samples into temporary i32 buffer
        let mut temp_buffer = vec![0i32; num_samples];
        self.chip.generate_block_2(num_samples, &mut temp_buffer);

        // Convert i32 samples to f32 in range [-1.0, 1.0]
        // OPL output is 16-bit signed
        for (i, &sample) in temp_buffer.iter().enumerate() {
            self.buffer[i] = sample as f32 / 32768.0;
        }

        &self.buffer
    }

    pub fn note_on(&mut self, channel: usize, note: u8, velocity: u8) {
        if channel < OPL_CHANNELS {
            let freq = calculate_note_frequency(note);
            let reg_offset = channel * 3;
            self.chip
                .write_reg(0xa0 + channel as u32, (freq & 0xff) as u8);
            self.chip
                .write_reg(0xb0 + channel as u32, ((freq >> 8) | 0x20) as u8);
            self.chip
                .write_reg(0x83 + reg_offset as u32, velocity.saturating_mul(2));
        }
    }

    pub fn note_off(&mut self, channel: usize) {
        if channel < OPL_CHANNELS {
            self.chip.write_reg(0xb0 + channel as u32, 0);
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
