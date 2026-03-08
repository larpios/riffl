//! Send/return bus routing for shared effects processing.

use crate::audio::dsp::{DspProcessor, ProcessSpec, RampedParam};

/// Default number of send buses.
pub const DEFAULT_NUM_BUSES: usize = 4;

/// Ramp time for bus return gain changes.
const BUS_RAMP_SECS: f32 = 0.005;

/// Single stereo send bus with optional insert effects.
pub struct SendBus {
    /// Pre-allocated stereo interleaved scratch buffer for accumulating sends.
    buffer: Vec<f32>,
    /// Insert effects chain applied to the bus signal.
    effects: Vec<Box<dyn DspProcessor>>,
    /// Bus return level (mixed back into master output).
    return_gain: RampedParam,
    /// Number of frames this buffer is sized for.
    max_frames: usize,
}

impl SendBus {
    /// Create a new send bus.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            effects: Vec::new(),
            return_gain: RampedParam::new(1.0),
            max_frames: 0,
        }
    }

    /// Prepare the bus for processing.
    pub fn prepare(&mut self, spec: ProcessSpec) {
        self.max_frames = spec.max_block_frames;
        self.buffer
            .resize(spec.max_block_frames * spec.channels, 0.0);
        self.return_gain.set_sample_rate(spec.sample_rate);
        for effect in &mut self.effects {
            effect.prepare(spec);
        }
    }

    /// Clear the internal accumulation buffer for the active frame range.
    pub fn clear_buffer(&mut self, num_frames: usize) {
        let sample_count = (num_frames.saturating_mul(2)).min(self.buffer.len());
        self.buffer[..sample_count].fill(0.0);
    }

    /// Accumulate a stereo sample into this bus with send level applied.
    pub fn accumulate(&mut self, frame: usize, left: f32, right: f32, send_level: f32) {
        let idx = frame.saturating_mul(2);
        if idx + 1 >= self.buffer.len() {
            return;
        }
        self.buffer[idx] += left * send_level;
        self.buffer[idx + 1] += right * send_level;
    }

    /// Process all insert effects in order.
    pub fn process_effects(&mut self) {
        for effect in &mut self.effects {
            effect.process_block(&mut self.buffer, 2);
        }
    }

    /// Mix this bus back into output using smoothed return gain.
    pub fn mix_into(&mut self, output: &mut [f32], num_frames: usize) {
        let max_frames_from_buffer = self.buffer.len() / 2;
        let max_frames_from_output = output.len() / 2;
        let frames = num_frames
            .min(max_frames_from_buffer)
            .min(max_frames_from_output);

        for frame in 0..frames {
            let gain = self.return_gain.next();
            let idx = frame * 2;
            output[idx] += self.buffer[idx] * gain;
            output[idx + 1] += self.buffer[idx + 1] * gain;
        }
    }

    /// Set the bus return gain target.
    pub fn set_return_gain(&mut self, gain: f32) {
        self.return_gain.set(gain, BUS_RAMP_SECS);
    }

    /// Get the current return gain target value.
    pub fn return_gain(&self) -> f32 {
        self.return_gain.target()
    }

    /// Add an insert effect to this bus.
    ///
    /// This should be called from a non-audio thread.
    pub fn add_effect(&mut self, effect: Box<dyn DspProcessor>) {
        self.effects.push(effect);
    }

    /// Clear all insert effects.
    ///
    /// This should be called from a non-audio thread.
    pub fn clear_effects(&mut self) {
        self.effects.clear();
    }

    /// Reset bus state and all insert effects.
    pub fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
        self.buffer.fill(0.0);
    }
}

impl Default for SendBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Collection of send buses for shared effects routing.
pub struct BusSystem {
    buses: Vec<SendBus>,
}

impl BusSystem {
    /// Create a bus system with a fixed number of buses.
    pub fn new(num_buses: usize) -> Self {
        Self {
            buses: (0..num_buses).map(|_| SendBus::new()).collect(),
        }
    }

    /// Prepare all buses.
    pub fn prepare(&mut self, spec: ProcessSpec) {
        for bus in &mut self.buses {
            bus.prepare(spec);
        }
    }

    /// Clear all bus buffers for the active frame range.
    pub fn clear_all(&mut self, num_frames: usize) {
        for bus in &mut self.buses {
            bus.clear_buffer(num_frames);
        }
    }

    /// Accumulate one sample into the selected bus.
    pub fn accumulate(
        &mut self,
        bus_index: usize,
        frame: usize,
        left: f32,
        right: f32,
        send_level: f32,
    ) {
        if let Some(bus) = self.buses.get_mut(bus_index) {
            bus.accumulate(frame, left, right, send_level);
        }
    }

    /// Process all buses then mix them into output.
    pub fn process_and_mix(&mut self, output: &mut [f32], num_frames: usize) {
        for bus in &mut self.buses {
            bus.process_effects();
            bus.mix_into(output, num_frames);
        }
    }

    /// Number of configured buses.
    pub fn num_buses(&self) -> usize {
        self.buses.len()
    }

    /// Mutable access to a bus for configuration.
    pub fn bus_mut(&mut self, index: usize) -> Option<&mut SendBus> {
        self.buses.get_mut(index)
    }

    /// Reset all buses.
    pub fn reset(&mut self) {
        for bus in &mut self.buses {
            bus.reset();
        }
    }
}

impl Default for BusSystem {
    fn default() -> Self {
        Self::new(DEFAULT_NUM_BUSES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Debug)]
    struct MultiplyEffect {
        factor: f32,
        reset_count: Arc<AtomicUsize>,
    }

    impl DspProcessor for MultiplyEffect {
        fn prepare(&mut self, _spec: ProcessSpec) {}

        fn process_block(&mut self, buffer: &mut [f32], _channels: usize) {
            for sample in buffer {
                *sample *= self.factor;
            }
        }

        fn reset(&mut self) {
            self.reset_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn spec() -> ProcessSpec {
        ProcessSpec {
            sample_rate: 48_000.0,
            max_block_frames: 8,
            channels: 2,
        }
    }

    #[test]
    fn test_send_bus_default() {
        let bus = SendBus::default();
        assert_eq!(bus.buffer.len(), 0);
        assert_eq!(bus.effects.len(), 0);
        assert_eq!(bus.return_gain(), 1.0);
        assert_eq!(bus.max_frames, 0);
    }

    #[test]
    fn test_send_bus_accumulate_and_mix() {
        let mut bus = SendBus::new();
        bus.prepare(spec());
        bus.clear_buffer(2);
        bus.accumulate(0, 1.0, -1.0, 0.5);
        bus.accumulate(1, 0.25, 0.75, 1.0);

        let mut out = vec![0.0; 4];
        bus.mix_into(&mut out, 2);

        assert!((out[0] - 0.5).abs() < 1e-6);
        assert!((out[1] + 0.5).abs() < 1e-6);
        assert!((out[2] - 0.25).abs() < 1e-6);
        assert!((out[3] - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_send_bus_clear_buffer() {
        let mut bus = SendBus::new();
        bus.prepare(spec());
        bus.accumulate(0, 1.0, 1.0, 1.0);
        bus.accumulate(1, 1.0, 1.0, 1.0);
        bus.clear_buffer(2);

        assert_eq!(bus.buffer[0], 0.0);
        assert_eq!(bus.buffer[1], 0.0);
        assert_eq!(bus.buffer[2], 0.0);
        assert_eq!(bus.buffer[3], 0.0);
    }

    #[test]
    fn test_send_bus_return_gain() {
        let mut bus = SendBus::new();
        bus.prepare(spec());
        bus.accumulate(0, 1.0, 1.0, 1.0);

        let mut out = vec![0.0; 2];
        bus.mix_into(&mut out, 1);
        let before = out[0];

        bus.set_return_gain(0.0);
        for _ in 0..256 {
            let _ = bus.return_gain.next();
        }

        out.fill(0.0);
        bus.mix_into(&mut out, 1);
        let after = out[0];

        assert!(after < before);
    }

    #[test]
    fn test_bus_system_default() {
        let buses = BusSystem::default();
        assert_eq!(buses.num_buses(), DEFAULT_NUM_BUSES);
    }

    #[test]
    fn test_bus_system_accumulate_bounds() {
        let mut buses = BusSystem::new(1);
        buses.prepare(spec());
        buses.clear_all(1);
        buses.accumulate(999, 0, 1.0, 1.0, 1.0);
        let mut out = vec![0.0; 2];
        buses.process_and_mix(&mut out, 1);
        assert_eq!(out, vec![0.0, 0.0]);
    }

    #[test]
    fn test_bus_system_process_and_mix() {
        let mut buses = BusSystem::new(1);
        buses.prepare(spec());
        buses.clear_all(1);

        let reset_count = Arc::new(AtomicUsize::new(0));
        buses
            .bus_mut(0)
            .expect("bus 0 exists")
            .add_effect(Box::new(MultiplyEffect {
                factor: 2.0,
                reset_count,
            }));

        buses.accumulate(0, 0, 0.5, -0.5, 1.0);
        let mut out = vec![0.0; 2];
        buses.process_and_mix(&mut out, 1);

        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[1] + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_send_bus_reset() {
        let mut bus = SendBus::new();
        bus.prepare(spec());

        let reset_count = Arc::new(AtomicUsize::new(0));
        bus.add_effect(Box::new(MultiplyEffect {
            factor: 1.0,
            reset_count: Arc::clone(&reset_count),
        }));

        bus.accumulate(0, 1.0, 1.0, 1.0);
        bus.reset();

        assert_eq!(reset_count.load(Ordering::Relaxed), 1);
        assert!(bus.buffer.iter().all(|&v| v == 0.0));
    }
}
