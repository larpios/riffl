//! Adlib/OPL chip synthesis for S3M files.
//!
//! Implements FM synthesis using the opl-emu crate for Adlib instruments
//! in Scream Tracker 3 modules.

#![allow(unused)]

use opl::chip::Chip;

const OPL_CHANNELS: usize = 9;

/// Operator offsets for OPL2/3 channels
const OP_OFFSETS: [u32; 9] = [0x00, 0x01, 0x02, 0x08, 0x09, 0x0A, 0x10, 0x11, 0x12];

pub struct AdlibSynthesizer {
    chip: Chip,
    sample_rate: u32,
    buffer: Vec<f32>,
}

impl AdlibSynthesizer {
    pub fn new(sample_rate: u32) -> Self {
        let mut chip = Chip::new(sample_rate);
        chip.setup();
        // Enable OPL3 features if possible
        chip.write_reg(0x105, 0x01); 
        Self {
            chip,
            sample_rate,
            buffer: Vec::with_capacity(1024),
        }
    }

    /// Initialize the synthesizer with raw register data
    pub fn init(&mut self, registers: &[u8]) {
        if registers.len() >= 256 {
            for (i, &reg) in registers.iter().enumerate().take(256) {
                self.chip.write_reg(i as u32, reg);
            }
        } else if registers.len() == 12 {
            // S3M Adlib instrument parameters (12 bytes)
            self.init_s3m_params(0, registers);
        }
    }

    /// Initialize a specific OPL channel with S3M instrument parameters
    pub fn init_s3m_params(&mut self, channel: usize, params: &[u8]) {
        if channel >= OPL_CHANNELS || params.len() < 11 {
            return;
        }

        let off = OP_OFFSETS[channel];
        
        // Modulator
        self.chip.write_reg(0x20 + off, params[0]);
        self.chip.write_reg(0x40 + off, params[2]);
        self.chip.write_reg(0x60 + off, params[4]);
        self.chip.write_reg(0x80 + off, params[6]);
        self.chip.write_reg(0xE0 + off, params[8]);

        // Carrier
        self.chip.write_reg(0x23 + off, params[1]);
        self.chip.write_reg(0x43 + off, params[3]);
        self.chip.write_reg(0x63 + off, params[5]);
        self.chip.write_reg(0x83 + off, params[7]);
        self.chip.write_reg(0xE3 + off, params[9]);

        // Connection/Feedback
        self.chip.write_reg(0xC0 + channel as u32, params[10]);
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
            
            // Write frequency low bits
            self.chip
                .write_reg(0xa0 + channel as u32, (freq & 0xff) as u8);
            // Write frequency high bits + block + key on bit
            self.chip
                .write_reg(0xb0 + channel as u32, ((freq >> 8) | 0x20) as u8);
            
            // Adjust volume if needed? S3M parameters already have scaling.
        }
    }

    pub fn note_off(&mut self, channel: usize) {
        if channel < OPL_CHANNELS {
            self.chip.write_reg(0xb0 + channel as u32, 0);
        }
    }
}

fn calculate_note_frequency(note: u8) -> u16 {
    // note 0..119 (C-0..B-9)
    let octave = (note / 12) as i32;
    let note_in_octave = (note % 12) as i32;
    
    // OPL F-Number calculation:
    // F-Number = freq * 2^(20-block) / 49716
    // Standard frequencies for octave 4:
    let f_numbers = [
        0x157, 0x16B, 0x181, 0x198, 0x1B0, 0x1CA,
        0x1E5, 0x202, 0x220, 0x241, 0x263, 0x287
    ];
    
    let f_num = f_numbers[note_in_octave as usize % 12];
    let block = (octave as u16).clamp(0, 7);
    
    (block << 10) | (f_num & 0x3FF)
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
        let freq = calculate_note_frequency(60); // C-5
        assert!(freq > 0);
        assert_eq!(freq >> 10, 5); // block 5
    }
}
