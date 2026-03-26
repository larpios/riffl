//! Audio visualization and monitoring logic.
//!
//! This module handles VU meters, oscilloscope buffers, and FFT capture buffers
//! for real-time monitoring of audio signals.

use std::sync::atomic::{AtomicU32, Ordering};

/// Number of samples in each per-channel oscilloscope ring buffer.
pub const OSCILLOSCOPE_BUF_SIZE: usize = 512;

/// Number of samples in the master bus FFT capture buffer.
pub const FFT_BUF_SIZE: usize = 1024;

pub fn f32_to_u32_bits(f: f32) -> u32 {
    f.to_bits()
}

pub fn u32_bits_to_f32(bits: u32) -> f32 {
    f32::from_bits(bits)
}

pub fn atomic_max_f32(atomic: &AtomicU32, new_val: f32) {
    let new_bits = f32_to_u32_bits(new_val);
    let old_bits = atomic.load(Ordering::Relaxed);
    if new_bits > old_bits {
        atomic.store(new_bits, Ordering::Relaxed);
    }
}

/// Manages visual monitoring state for audio channels and master output.
pub struct Visualizer {
    /// Per-channel peak levels for VU meters (left, right) as atomic u32 bit patterns.
    pub channel_levels: Vec<(AtomicU32, AtomicU32)>,
    /// Per-channel oscilloscope ring buffers (mono mix of L+R).
    pub oscilloscope_bufs: Vec<Vec<f32>>,
    /// Per-channel write position into the oscilloscope ring buffer.
    pub oscilloscope_write_pos: Vec<AtomicU32>,
    /// Master bus FFT capture ring buffer (mono).
    pub fft_buf: Vec<f32>,
    /// Write position into the FFT ring buffer.
    pub fft_write_pos: AtomicU32,
}

impl Visualizer {
    /// Create a new visualizer with the given number of channels.
    pub fn new(num_channels: usize) -> Self {
        let channel_levels: Vec<(AtomicU32, AtomicU32)> = (0..num_channels)
            .map(|_| (AtomicU32::new(0), AtomicU32::new(0)))
            .collect();

        let oscilloscope_bufs: Vec<Vec<f32>> = (0..num_channels)
            .map(|_| vec![0.0f32; OSCILLOSCOPE_BUF_SIZE])
            .collect();
        let oscilloscope_write_pos: Vec<AtomicU32> =
            (0..num_channels).map(|_| AtomicU32::new(0)).collect();

        Self {
            channel_levels,
            oscilloscope_bufs,
            oscilloscope_write_pos,
            fft_buf: vec![0.0f32; FFT_BUF_SIZE],
            fft_write_pos: AtomicU32::new(0),
        }
    }

    /// Update peak levels for a channel.
    pub fn update_channel_levels(&self, ch: usize, left: f32, right: f32) {
        if ch < self.channel_levels.len() {
            atomic_max_f32(&self.channel_levels[ch].0, left);
            atomic_max_f32(&self.channel_levels[ch].1, right);
        }
    }

    /// Record a sample into a channel's oscilloscope buffer.
    pub fn record_oscilloscope_sample(&mut self, ch: usize, sample: f32) {
        if ch < self.oscilloscope_bufs.len() {
            let write_pos = self.oscilloscope_write_pos[ch].load(Ordering::Relaxed) as usize;
            self.oscilloscope_bufs[ch][write_pos] = sample;
            self.oscilloscope_write_pos[ch].store(
                ((write_pos + 1) % OSCILLOSCOPE_BUF_SIZE) as u32,
                Ordering::Relaxed,
            );
        }
    }

    pub fn record_fft_sample_mut(&mut self, sample: f32) {
        let pos = self.fft_write_pos.load(Ordering::Relaxed) as usize % FFT_BUF_SIZE;
        self.fft_buf[pos] = sample;
        self.fft_write_pos
            .store(((pos + 1) % FFT_BUF_SIZE) as u32, Ordering::Relaxed);
    }

    /// Reset all channel levels to zero.
    pub fn reset_channel_levels(&self) {
        for (l, r) in &self.channel_levels {
            l.store(0u32, Ordering::Relaxed);
            r.store(0u32, Ordering::Relaxed);
        }
    }

    /// Reset all oscilloscope buffers to zero.
    pub fn reset_oscilloscope_buffers(&mut self) {
        for buf in &mut self.oscilloscope_bufs {
            buf.fill(0.0);
        }
        for pos in &self.oscilloscope_write_pos {
            pos.store(0, Ordering::Relaxed);
        }
    }

    /// Reset the FFT capture buffer to silence.
    pub fn reset_fft_buffer(&mut self) {
        self.fft_buf.fill(0.0);
        self.fft_write_pos.store(0, Ordering::Relaxed);
    }

    /// Get the peak level for a channel (left, right).
    pub fn get_channel_level(&self, channel: usize) -> (f32, f32) {
        self.channel_levels
            .get(channel)
            .map(|(l, r)| {
                (
                    u32_bits_to_f32(l.load(Ordering::Relaxed)),
                    u32_bits_to_f32(r.load(Ordering::Relaxed)),
                )
            })
            .unwrap_or((0.0, 0.0))
    }

    /// Read the oscilloscope waveform for a channel.
    pub fn oscilloscope_data(&self, channel: usize) -> Vec<f32> {
        if let (Some(buf), Some(pos_atomic)) = (
            self.oscilloscope_bufs.get(channel),
            self.oscilloscope_write_pos.get(channel),
        ) {
            let write_pos = pos_atomic.load(Ordering::Relaxed) as usize % OSCILLOSCOPE_BUF_SIZE;
            let mut result = Vec::with_capacity(OSCILLOSCOPE_BUF_SIZE);
            for i in 0..OSCILLOSCOPE_BUF_SIZE {
                result.push(buf[(write_pos + i) % OSCILLOSCOPE_BUF_SIZE]);
            }
            result
        } else {
            vec![0.0; OSCILLOSCOPE_BUF_SIZE]
        }
    }

    /// Read the master bus FFT capture buffer in chronological order.
    pub fn fft_data(&self) -> Vec<f32> {
        let write_pos = self.fft_write_pos.load(Ordering::Relaxed) as usize % FFT_BUF_SIZE;
        let mut result = Vec::with_capacity(FFT_BUF_SIZE);
        for i in 0..FFT_BUF_SIZE {
            result.push(self.fft_buf[(write_pos + i) % FFT_BUF_SIZE]);
        }
        result
    }

    /// Decay all channel levels by the given factor (0.0 to 1.0).
    pub fn decay_channel_levels(&self, decay_factor: f32) {
        for (l, r) in &self.channel_levels {
            let current_l = u32_bits_to_f32(l.load(Ordering::Relaxed));
            let current_r = u32_bits_to_f32(r.load(Ordering::Relaxed));
            let decayed_l = current_l * decay_factor;
            let decayed_r = current_r * decay_factor;
            l.store(f32_to_u32_bits(decayed_l), Ordering::Relaxed);
            r.store(f32_to_u32_bits(decayed_r), Ordering::Relaxed);
        }
    }

    /// Resize channel-specific visualizers.
    pub fn set_num_channels(&mut self, num_channels: usize) {
        if num_channels > self.channel_levels.len() {
            for _ in self.channel_levels.len()..num_channels {
                self.channel_levels
                    .push((AtomicU32::new(0), AtomicU32::new(0)));
                self.oscilloscope_bufs
                    .push(vec![0.0f32; OSCILLOSCOPE_BUF_SIZE]);
                self.oscilloscope_write_pos.push(AtomicU32::new(0));
            }
        } else {
            self.channel_levels.truncate(num_channels);
            self.oscilloscope_bufs.truncate(num_channels);
            self.oscilloscope_write_pos.truncate(num_channels);
        }
    }
}
