//! Delay line implementation with fractional delay support.
//!
//! This module provides a real-time safe ring buffer delay line that implements
//! the `DspProcessor` trait. It supports multi-channel processing and sub-sample
//! accurate delays via linear interpolation.

use crate::audio::dsp::{DspProcessor, ProcessSpec};

/// A single-channel ring buffer delay line.
#[derive(Debug, Clone)]
struct SingleDelayLine {
    buffer: Vec<f32>,
    mask: usize,
    write_pos: usize,
}

impl SingleDelayLine {
    fn new(max_delay_samples: usize) -> Self {
        let size = max_delay_samples.next_power_of_two().max(2);
        Self {
            buffer: vec![0.0; size],
            mask: size - 1,
            write_pos: 0,
        }
    }

    fn push(&mut self, sample: f32) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) & self.mask;
    }

    fn read(&self, delay_samples: usize) -> f32 {
        let read_pos = (self.write_pos + self.mask + 1 - delay_samples) & self.mask;
        self.buffer[read_pos]
    }

    fn read_interpolated(&self, delay_fractional: f32) -> f32 {
        let delay_int = delay_fractional.floor() as usize;
        let fraction = delay_fractional - delay_fractional.floor();

        let s1 = self.read(delay_int);
        let s2 = self.read(delay_int + 1);

        s1 + fraction * (s2 - s1)
    }

    fn max_delay(&self) -> usize {
        self.mask
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

/// A multi-channel delay line processor.
///
/// Wraps multiple internal delay lines for stereo/multi-channel processing.
#[derive(Debug, Clone)]
pub struct DelayLine {
    lines: Vec<SingleDelayLine>,
    delay_samples: f32,
    sample_rate: f32,
    max_delay_samples: usize,
}

impl DelayLine {
    /// Creates a new delay line with the specified maximum capacity.
    ///
    /// The actual capacity will be rounded up to the next power of 2.
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            lines: vec![SingleDelayLine::new(max_delay_samples)],
            delay_samples: 0.0,
            sample_rate: 48000.0,
            max_delay_samples,
        }
    }

    /// Writes a sample to the first channel's delay line and advances its write position.
    pub fn push(&mut self, sample: f32) {
        if let Some(line) = self.lines.first_mut() {
            line.push(sample);
        }
    }

    /// Reads a sample from the first channel's delay line at the specified delay.
    pub fn read(&self, delay_samples: usize) -> f32 {
        self.lines
            .first()
            .map(|l| l.read(delay_samples))
            .unwrap_or(0.0)
    }

    /// Reads an interpolated sample from the first channel's delay line.
    pub fn read_interpolated(&self, delay_fractional: f32) -> f32 {
        self.lines
            .first()
            .map(|l| l.read_interpolated(delay_fractional))
            .unwrap_or(0.0)
    }

    /// Returns the maximum delay capacity in samples.
    pub fn max_delay(&self) -> usize {
        self.lines.first().map(|l| l.max_delay()).unwrap_or(0)
    }

    /// Returns the actual allocated buffer length (power of 2).
    pub fn len(&self) -> usize {
        self.lines.first().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns `true` if the delay line has no allocated buffer.
    pub fn is_empty(&self) -> bool {
        self.lines.first().map(|l| l.len() == 0).unwrap_or(true)
    }

    /// Sets the current delay time in seconds.
    pub fn set_delay_seconds(&mut self, seconds: f32) {
        self.delay_samples = seconds * self.sample_rate;
    }

    /// Sets the current delay time in samples.
    pub fn set_delay_samples(&mut self, samples: usize) {
        self.delay_samples = samples as f32;
    }
}

impl DspProcessor for DelayLine {
    fn prepare(&mut self, spec: ProcessSpec) {
        self.sample_rate = spec.sample_rate;
        self.lines.clear();
        self.lines
            .resize(spec.channels, SingleDelayLine::new(self.max_delay_samples));
    }

    fn process_block(&mut self, buffer: &mut [f32], channels: usize) {
        if self.lines.len() < channels {
            return;
        }

        for frame in buffer.chunks_exact_mut(channels) {
            for (ch, sample) in frame.iter_mut().enumerate() {
                let line = &mut self.lines[ch];

                // Read delayed sample
                let delayed = line.read_interpolated(self.delay_samples);

                // Write current sample
                line.push(*sample);

                // Output delayed sample
                *sample = delayed;
            }
        }
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delay_line_basic() {
        let mut delay = DelayLine::new(1024);
        delay.push(1.0);
        delay.push(2.0);
        delay.push(3.0);

        assert_eq!(delay.read(1), 3.0);
        assert_eq!(delay.read(2), 2.0);
        assert_eq!(delay.read(3), 1.0);
    }

    #[test]
    fn test_delay_line_zero_delay() {
        let mut delay = DelayLine::new(1024);
        delay.push(42.0);
        assert_eq!(delay.read(0), 0.0);
    }

    #[test]
    fn test_delay_line_interpolated() {
        let mut delay = DelayLine::new(1024);
        delay.push(1.0);
        delay.push(2.0);

        assert_eq!(delay.read_interpolated(0.5), 1.0);
        assert_eq!(delay.read_interpolated(1.5), 1.5);
    }

    #[test]
    fn test_delay_line_power_of_two() {
        let delay = DelayLine::new(1000);
        assert_eq!(delay.len(), 1024);

        let delay2 = DelayLine::new(1024);
        assert_eq!(delay2.len(), 1024);
    }

    #[test]
    fn test_delay_line_wrapping() {
        let mut delay = DelayLine::new(4);
        delay.push(1.0);
        delay.push(2.0);
        delay.push(3.0);
        delay.push(4.0);
        delay.push(5.0);

        assert_eq!(delay.read(1), 5.0);
        assert_eq!(delay.read(2), 4.0);
        assert_eq!(delay.read(3), 3.0);
        assert_eq!(delay.read(4), 2.0);
    }

    #[test]
    fn test_delay_line_reset() {
        let mut delay = DelayLine::new(1024);
        delay.push(1.0);
        delay.reset();
        assert_eq!(delay.read(1), 0.0);
    }

    #[test]
    fn test_delay_line_max_delay() {
        let mut delay = DelayLine::new(4);
        delay.push(1.0);
        delay.push(2.0);
        delay.push(3.0);

        assert_eq!(delay.max_delay(), 3);
        assert_eq!(delay.read(3), 1.0);
    }

    #[test]
    fn test_delay_process_block_stereo() {
        let mut delay = DelayLine::new(1024);
        delay.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 2,
        });
        delay.set_delay_samples(1);

        let mut buffer = vec![1.0, 2.0, 3.0, 4.0];
        delay.process_block(&mut buffer, 2);

        assert_eq!(buffer, vec![0.0, 0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_set_delay_seconds() {
        let mut delay = DelayLine::new(48000);
        delay.prepare(ProcessSpec {
            sample_rate: 48000.0,
            max_block_frames: 128,
            channels: 1,
        });

        delay.set_delay_seconds(0.5);
        assert_eq!(delay.delay_samples, 24000.0);
    }
}
