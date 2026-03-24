//! Dual-mode pitch calculation abstraction for tracker effects.
//!
//! Handles pitch delta math for both modern linear (semitone-based) and
//! legacy Amiga period-based formats (.MOD, .S3M).

/// Amiga PAL clock constant used for period-to-frequency conversion.
pub const AMIGA_PAL_CLOCK: f64 = 3_546_894.6;

/// S3M period clock constant (8363Hz * 1712 = 14317456).
/// Actually 14317056 in most trackers.
pub const AMIGA_S3M_CLOCK: f64 = 14_317_056.0;

/// Determines how pitch slide and portamento math is computed.
///
/// - `Linear`: exponential/semitone-based steps, suitable for XM/IT and modern formats.
/// - `AmigaPeriod`: raw period arithmetic using a period clock, required for
///   accurate MOD and S3M playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SlideMode {
    /// Standard linear math: slide parameters are interpreted as 1/64th-semitone units.
    #[default]
    Linear,
    /// Amiga period math: slide parameters are raw period deltas (clock / freq).
    AmigaPeriod,
}

/// Encapsulates pitch delta calculations for both `Linear` and `AmigaPeriod` modes.
///
/// All methods take and return absolute frequencies in Hz so callers do not need
/// to handle mode-specific math themselves.
pub struct PitchCalculator;

impl PitchCalculator {
    /// Apply a one-tick pitch slide to `current_freq` and return the new frequency.
    ///
    /// # Parameters
    /// - `current_freq`: the current voice frequency in Hz.
    /// - `param_up`: slide-up speed (positive direction).
    /// - `param_down`: slide-down speed (negative direction).
    /// - `mode`: which math model to use.
    /// - `period_clock`: effective period clock for `AmigaPeriod` mode.
    ///
    /// # Mode interpretation
    /// - `Linear`: `param_up`/`param_down` are in 1/64th-semitone units per tick.
    /// - `AmigaPeriod`: `param_up`/`param_down` are raw period units per tick
    ///   (higher period = lower frequency, so `param_up` decreases the period).
    pub fn apply_slide(
        current_freq: f64,
        param_up: u8,
        param_down: u8,
        mode: SlideMode,
        period_clock: f64,
    ) -> f64 {
        if current_freq <= 0.0 {
            return current_freq;
        }

        match mode {
            SlideMode::Linear => {
                let semitones_per_tick = (param_up as f64 - param_down as f64) / 64.0;
                current_freq * 2.0_f64.powf(semitones_per_tick / 12.0)
            }
            SlideMode::AmigaPeriod => {
                // Period and frequency are inversely related: period = clock / freq.
                // Sliding "up" in pitch means decreasing the period value.
                let period = period_clock / current_freq;
                let delta = param_down as f64 - param_up as f64;
                let new_period = (period + delta).max(1.0);
                period_clock / new_period
            }
        }
    }

    /// Apply a one-tick portamento step from `current_freq` toward `target_freq`.
    ///
    /// Returns the new frequency, clamped so it never overshoots the target.
    ///
    /// # Parameters
    /// - `current_freq`: current sliding frequency in Hz.
    /// - `target_freq`: destination frequency in Hz.
    /// - `speed`: slide speed; interpretation depends on `mode`.
    /// - `mode`: which math model to use.
    /// - `period_clock`: effective period clock for `AmigaPeriod` mode.
    ///
    /// # Mode interpretation
    /// - `Linear`: `speed` is in semitone units per tick (e.g. `effect.param / 64.0`).
    ///   A ratio step of `2^(speed/12)` is applied each tick.
    /// - `AmigaPeriod`: `speed` is a raw period delta per tick (e.g. `effect.param as f64`).
    ///   The period is nudged by `speed` units toward the target period each tick.
    pub fn apply_portamento(
        current_freq: f64,
        target_freq: f64,
        speed: f64,
        mode: SlideMode,
        period_clock: f64,
    ) -> f64 {
        if speed <= 0.0 || current_freq <= 0.0 || target_freq <= 0.0 {
            return current_freq;
        }

        match mode {
            SlideMode::Linear => {
                let ratio_step = 2.0_f64.powf(speed / 12.0);
                if current_freq < target_freq {
                    (current_freq * ratio_step).min(target_freq)
                } else {
                    (current_freq / ratio_step).max(target_freq)
                }
            }
            SlideMode::AmigaPeriod => {
                let current_period = period_clock / current_freq;
                let target_period = period_clock / target_freq;
                // Higher period = lower pitch, so direction is reversed vs frequency.
                let new_period = if current_period > target_period {
                    (current_period - speed).max(target_period)
                } else {
                    (current_period + speed).min(target_period)
                };
                period_clock / new_period.max(1.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_slide_up_one_semitone() {
        // 64 units = 1 semitone per tick
        let freq = 440.0_f64;
        let new_freq =
            PitchCalculator::apply_slide(freq, 64, 0, SlideMode::Linear, AMIGA_PAL_CLOCK);
        let expected = freq * 2.0_f64.powf(1.0 / 12.0);
        assert!(
            (new_freq - expected).abs() < 0.001,
            "expected {expected}, got {new_freq}"
        );
    }

    #[test]
    fn linear_slide_down_one_semitone() {
        let freq = 440.0_f64;
        let new_freq =
            PitchCalculator::apply_slide(freq, 0, 64, SlideMode::Linear, AMIGA_PAL_CLOCK);
        let expected = freq * 2.0_f64.powf(-1.0 / 12.0);
        assert!((new_freq - expected).abs() / expected < 0.0001);
    }

    #[test]
    fn amiga_slide_up_decreases_period() {
        let freq = 440.0_f64;
        let new_freq =
            PitchCalculator::apply_slide(freq, 4, 0, SlideMode::AmigaPeriod, AMIGA_PAL_CLOCK);
        // Sliding up should raise frequency (period decreases)
        assert!(new_freq > freq, "expected higher freq, got {new_freq}");
    }

    #[test]
    fn amiga_slide_down_increases_period() {
        let freq = 440.0_f64;
        let new_freq =
            PitchCalculator::apply_slide(freq, 0, 4, SlideMode::AmigaPeriod, AMIGA_PAL_CLOCK);
        assert!(new_freq < freq, "expected lower freq, got {new_freq}");
    }

    #[test]
    fn linear_portamento_approaches_target() {
        let current = 440.0_f64;
        let target = 880.0_f64; // one octave up
        let speed = 1.0; // 1 semitone per tick
        let new_freq = PitchCalculator::apply_portamento(
            current,
            target,
            speed,
            SlideMode::Linear,
            AMIGA_PAL_CLOCK,
        );
        assert!(new_freq > current && new_freq <= target);
    }

    #[test]
    fn linear_portamento_does_not_overshoot() {
        let current = 878.0_f64;
        let target = 880.0_f64;
        let speed = 4.0; // large step, would overshoot without clamping
        let new_freq = PitchCalculator::apply_portamento(
            current,
            target,
            speed,
            SlideMode::Linear,
            AMIGA_PAL_CLOCK,
        );
        assert_eq!(new_freq, target);
    }

    #[test]
    fn amiga_portamento_approaches_target() {
        let current = 440.0_f64;
        let target = 880.0_f64;
        let speed = 10.0; // 10 period units per tick
        let new_freq = PitchCalculator::apply_portamento(
            current,
            target,
            speed,
            SlideMode::AmigaPeriod,
            AMIGA_PAL_CLOCK,
        );
        assert!(new_freq > current && new_freq <= target);
    }

    #[test]
    fn amiga_portamento_does_not_overshoot() {
        // Target is just one period unit above current
        let current_period = AMIGA_PAL_CLOCK / 440.0;
        let target_period = current_period - 1.0;
        let target = AMIGA_PAL_CLOCK / target_period;
        let speed = 100.0; // large speed, would overshoot
        let new_freq = PitchCalculator::apply_portamento(
            440.0,
            target,
            speed,
            SlideMode::AmigaPeriod,
            AMIGA_PAL_CLOCK,
        );
        assert!(
            (new_freq - target).abs() < 0.01,
            "expected {target}, got {new_freq}"
        );
    }
}
