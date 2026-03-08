//! DSP processing primitives for the audio engine.
//!
//! This module defines the shared processor contract and parameter smoothing
//! utilities used by channel strips and effects.

/// Configuration provided to DSP processors during preparation.
///
/// Contains all information a processor needs to pre-allocate buffers
/// and configure its internal state.
#[derive(Debug, Clone, Copy)]
pub struct ProcessSpec {
    /// Output sample rate in Hz.
    pub sample_rate: f32,
    /// Maximum number of frames per processing block.
    /// Processors should pre-allocate buffers to this size.
    pub max_block_frames: usize,
    /// Number of audio channels (typically 2 for stereo).
    pub channels: usize,
}

/// Trait for real-time audio DSP processors (effects, filters, dynamics).
///
/// All implementations must be real-time safe: `process_block` must NEVER
/// allocate memory. All buffers and state must be pre-allocated in `prepare()`.
///
/// # Lifecycle
/// 1. `prepare()` — called once before processing starts (or when config changes)
/// 2. `process_block()` — called repeatedly from the audio thread
/// 3. `reset()` — called to clear internal state (e.g., delay lines, filter history)
pub trait DspProcessor: Send {
    /// Prepare the processor for a given configuration.
    ///
    /// Called before processing begins. Implementations should pre-allocate
    /// all internal buffers and compute coefficients here.
    fn prepare(&mut self, spec: ProcessSpec);

    /// Process a block of audio in-place.
    ///
    /// `buffer` is stereo interleaved: [L0, R0, L1, R1, ...].
    /// `channels` is the number of interleaved channels (typically 2).
    ///
    /// # Real-time safety
    /// This method runs on the audio thread. It MUST NOT:
    /// - Allocate heap memory (no Vec::push, Box::new, String, etc.)
    /// - Lock mutexes
    /// - Perform I/O
    /// - Call any function that may block
    fn process_block(&mut self, buffer: &mut [f32], channels: usize);

    /// Reset internal state (filter history, delay buffers, etc.) without
    /// changing configuration. Used when playback stops or restarts.
    fn reset(&mut self);
}

/// Linear ramp parameter smoother for real-time-safe parameter changes.
///
/// Unlike exponential (one-pole) smoothers, linear ramps reach their target
/// exactly and are more predictable for mixer controls like volume and pan.
/// They also work correctly with automation systems.
///
/// # Usage
/// ```ignore
/// use tracker_core::audio::RampedParam;
///
/// let mut param = RampedParam::new(1.0);
/// param.set_sample_rate(48000.0);
/// param.set(0.5, 0.015);
/// for _ in 0..128 {
///     let value = param.next();
///     let _ = value;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RampedParam {
    current: f32,
    target: f32,
    /// Increment per sample to reach target.
    increment: f32,
    /// Remaining samples until target is reached.
    remaining_samples: u32,
    /// Sample rate for calculating ramp duration.
    sample_rate: f32,
}

impl RampedParam {
    /// Creates a new ramped parameter with an initial value.
    ///
    /// The default sample rate is `48_000.0` Hz.
    pub fn new(initial_value: f32) -> Self {
        Self {
            current: initial_value,
            target: initial_value,
            increment: 0.0,
            remaining_samples: 0,
            sample_rate: 48_000.0,
        }
    }

    /// Sets the sample rate used for converting ramp time to samples.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    /// Sets a new target value and ramp duration.
    ///
    /// If `ramp_seconds <= 0.0` or sample rate is `0.0`, the parameter snaps
    /// to the target immediately.
    pub fn set(&mut self, target: f32, ramp_seconds: f32) {
        self.target = target;

        if ramp_seconds <= 0.0 || self.sample_rate <= 0.0 {
            self.set_immediate(target);
            return;
        }

        let samples = (ramp_seconds * self.sample_rate).round() as i64;
        if samples <= 0 {
            self.set_immediate(target);
            return;
        }

        self.remaining_samples = samples as u32;
        self.increment = (self.target - self.current) / self.remaining_samples as f32;
    }

    /// Sets the parameter to a value immediately (no ramp).
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.increment = 0.0;
        self.remaining_samples = 0;
    }

    /// Advances the ramp by one sample and returns the current value.
    ///
    /// When the ramp completes, the parameter snaps exactly to target.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        self.advance_one()
    }

    fn advance_one(&mut self) -> f32 {
        if self.remaining_samples > 0 {
            self.current += self.increment;
            self.remaining_samples -= 1;

            if self.remaining_samples == 0 {
                self.current = self.target;
                self.increment = 0.0;
            }
        }

        self.current
    }

    /// Returns the current value without advancing the ramp.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Returns the current target value.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Returns `true` if a ramp is currently in progress.
    pub fn is_ramping(&self) -> bool {
        self.remaining_samples > 0
    }
}

impl Iterator for RampedParam {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.advance_one())
    }
}

#[cfg(test)]
mod tests {
    use super::{ProcessSpec, RampedParam};

    #[test]
    fn test_ramped_param_immediate() {
        let mut param = RampedParam::new(1.0);
        param.set_immediate(0.25);

        assert_eq!(param.current(), 0.25);
        assert_eq!(param.target(), 0.25);
        assert!(!param.is_ramping());
        assert_eq!(param.next(), 0.25);
    }

    #[test]
    fn test_ramped_param_ramp_basic() {
        let mut param = RampedParam::new(1.0);
        param.set_sample_rate(10.0);
        param.set(0.0, 0.5);

        for _ in 0..4 {
            let value = param.next();
            assert!(value > 0.0 && value < 1.0);
        }

        let last = param.next();
        assert_eq!(last, 0.0);
        assert_eq!(param.current(), 0.0);
    }

    #[test]
    fn test_ramped_param_ramp_reaches_target_exactly() {
        let mut param = RampedParam::new(1.0);
        param.set_sample_rate(48_000.0);
        param.set(0.3, 0.001);

        for _ in 0..48 {
            let _ = param.next();
        }

        assert_eq!(param.current(), 0.3);
        assert_eq!(param.target(), 0.3);
        assert!(!param.is_ramping());
    }

    #[test]
    fn test_ramped_param_zero_duration_snaps() {
        let mut param = RampedParam::new(0.75);
        param.set(0.1, 0.0);

        assert_eq!(param.current(), 0.1);
        assert_eq!(param.target(), 0.1);
        assert!(!param.is_ramping());
    }

    #[test]
    fn test_ramped_param_is_ramping() {
        let mut param = RampedParam::new(0.0);
        param.set_sample_rate(8.0);
        param.set(1.0, 0.5);

        assert!(param.is_ramping());

        for _ in 0..3 {
            let _ = param.next();
            assert!(param.is_ramping());
        }

        let _ = param.next();
        assert!(!param.is_ramping());
    }

    #[test]
    fn test_ramped_param_set_during_ramp() {
        let mut param = RampedParam::new(0.0);
        param.set_sample_rate(10.0);
        param.set(1.0, 1.0);

        let mid = param.next();
        assert_eq!(mid, 0.1);

        param.set(0.5, 0.4);
        assert!(param.is_ramping());

        for _ in 0..4 {
            let _ = param.next();
        }

        assert_eq!(param.current(), 0.5);
        assert_eq!(param.target(), 0.5);
        assert!(!param.is_ramping());
    }

    #[test]
    fn test_ramped_param_negative_ramp_snaps() {
        let mut param = RampedParam::new(1.0);
        param.set(0.2, -0.05);

        assert_eq!(param.current(), 0.2);
        assert_eq!(param.target(), 0.2);
        assert!(!param.is_ramping());
    }

    #[test]
    fn test_process_spec_creation() {
        let spec = ProcessSpec {
            sample_rate: 48_000.0,
            max_block_frames: 512,
            channels: 2,
        };

        assert_eq!(spec.sample_rate, 48_000.0);
        assert_eq!(spec.max_block_frames, 512);
        assert_eq!(spec.channels, 2);
    }
}
