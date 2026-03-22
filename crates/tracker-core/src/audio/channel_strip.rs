//! Per-channel mixing state with smoothed parameter changes.
//!
//! The channel strip replaces the old `ChannelMix` with `RampedParam`-based
//! volume, pan, mute, and solo controls that prevent clicks and pops when
//! parameters change.

use crate::audio::dsp::RampedParam;

/// Default ramp time in seconds for mixer control changes.
const MIXER_RAMP_SECS: f32 = 0.005;

/// Per-channel mixing state with smoothed parameters.
///
/// Uses `RampedParam` for volume and pan to prevent audio artifacts
/// (clicks/pops) when controls change. Mute and solo are implemented
/// as smoothed gain multipliers that ramp to 0.0 or 1.0.
#[derive(Debug, Clone)]
pub struct ChannelStrip {
    /// Track volume (0.0 to 1.0), smoothed.
    volume: RampedParam,
    /// Pan position (-1.0 = left, 0.0 = center, 1.0 = right), smoothed.
    pan: RampedParam,
    /// Mute gain (1.0 = unmuted, 0.0 = muted), smoothed.
    mute_gain: RampedParam,
    /// Solo gain (1.0 = audible, 0.0 = silenced by another channel's solo), smoothed.
    solo_gain: RampedParam,
    /// Per-bus send levels, smoothed.
    send_levels: Vec<RampedParam>,
    /// Sample rate used by all ramp parameters.
    sample_rate: f32,
}

impl ChannelStrip {
    /// Create a new channel strip with default settings (full volume, center pan, unmuted).
    pub fn new() -> Self {
        Self {
            volume: RampedParam::new(1.0),
            pan: RampedParam::new(0.0),
            mute_gain: RampedParam::new(1.0),
            solo_gain: RampedParam::new(1.0),
            send_levels: Vec::new(),
            sample_rate: 48_000.0,
        }
    }

    /// Set the sample rate for all internal ramp parameters.
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.volume.set_sample_rate(sample_rate);
        self.pan.set_sample_rate(sample_rate);
        self.mute_gain.set_sample_rate(sample_rate);
        self.solo_gain.set_sample_rate(sample_rate);
        for send in &mut self.send_levels {
            send.set_sample_rate(sample_rate);
        }
    }

    /// Ensure we have enough send level parameters for the given bus count.
    pub fn ensure_send_levels(&mut self, num_buses: usize) {
        while self.send_levels.len() < num_buses {
            let mut param = RampedParam::new(0.0);
            param.set_sample_rate(self.sample_rate);
            self.send_levels.push(param);
        }
    }

    /// Set channel control targets from track metadata.
    ///
    /// This syncs the channel strip's parameters with the track's volume,
    /// pan, mute, and solo settings, using smoothed ramps to avoid clicks.
    pub fn update_from_track(
        &mut self,
        volume: f32,
        pan: f32,
        muted: bool,
        any_soloed: bool,
        this_soloed: bool,
        send_levels: &[f32],
    ) {
        self.volume.set(volume, MIXER_RAMP_SECS);
        self.pan.set(pan, MIXER_RAMP_SECS);

        let mute_target = if muted { 0.0 } else { 1.0 };
        self.mute_gain.set(mute_target, MIXER_RAMP_SECS);

        let solo_target = if any_soloed && !this_soloed { 0.0 } else { 1.0 };
        self.solo_gain.set(solo_target, MIXER_RAMP_SECS);

        for (i, param) in self.send_levels.iter_mut().enumerate() {
            let level = send_levels.get(i).copied().unwrap_or(0.0);
            param.set(level, MIXER_RAMP_SECS);
        }
    }

    /// Compute the left and right channel gains for the current sample.
    ///
    /// Advances all internal ramp parameters by one sample.
    /// Returns (left_gain, right_gain) incorporating volume, pan law, mute, and solo.
    ///
    /// Uses equal-power panning (-3dB at center):
    /// - pan = -1.0: full left (L=1.0, R=0.0)
    /// - pan = 0.0: center (L≈0.707, R≈0.707)
    /// - pan = 1.0: full right (L=0.0, R=1.0)
    pub fn next_gains(&mut self) -> (f32, f32) {
        self.next_gains_modulated(1.0, 0.0, None)
    }

    /// Compute modulated gains incorporating per-voice envelopes and LFOs.
    ///
    /// Advances all internal ramp parameters by one sample.
    /// `mod_vol` is a multiplier for volume.
    /// `mod_pan` is an offset added to the current panning position.
    /// `pan_override` optionally overrides the track-level panning (e.g. from effects).
    pub fn next_gains_modulated(
        &mut self,
        mod_vol: f32,
        mod_pan: f32,
        pan_override: Option<f32>,
    ) -> (f32, f32) {
        let vol = self.volume.next() * mod_vol;
        let mute = self.mute_gain.next();
        let solo = self.solo_gain.next();

        // Advance the pan ramp anyway to keep it in sync
        let strip_pan = self.pan.next();
        let base_pan = pan_override.unwrap_or(strip_pan);

        let total_pan = (base_pan + mod_pan).clamp(-1.0, 1.0);
        let (pan_l, pan_r) = Self::pan_gains(total_pan);
        let combined = vol * mute * solo;

        (combined * pan_l, combined * pan_r)
    }

    /// Get the current smoothed send level for a bus, advancing the ramp.
    pub fn next_send_level(&mut self, bus_index: usize) -> f32 {
        self.send_levels
            .get_mut(bus_index)
            .map_or(0.0, RampedParam::next)
    }

    /// Override the pan position from an effect command (8xx).
    ///
    /// `pan` is in the channel strip's coordinate system: -1.0=left, 0.0=centre, 1.0=right.
    pub fn set_effect_pan(&mut self, pan: f32) {
        self.pan.set(pan.clamp(-1.0, 1.0), MIXER_RAMP_SECS);
    }

    /// Set pan position immediately without ramping (for per-frame LFOs).
    pub fn set_effect_pan_immediate(&mut self, pan: f32) {
        self.pan.set_immediate(pan.clamp(-1.0, 1.0));
    }

    /// Get the number of configured send levels.
    pub fn num_send_levels(&self) -> usize {
        self.send_levels.len()
    }

    /// Get the current left/right gains without advancing the ramp.
    pub fn current_gains(&self) -> (f32, f32) {
        let vol = self.volume.current();
        let pan = self.pan.current();
        let mute = self.mute_gain.current();
        let solo = self.solo_gain.current();

        let (pan_l, pan_r) = Self::pan_gains(pan);
        let combined = vol * mute * solo;

        (combined * pan_l, combined * pan_r)
    }

    /// Get the current volume/mute/solo gain without panning, advancing the ramp.
    /// Use this when applying custom panning (e.g., from a panning LFO).
    pub fn next_volume_gain(&mut self) -> f32 {
        let vol = self.volume.next();
        let mute = self.mute_gain.next();
        let solo = self.solo_gain.next();
        vol * mute * solo
    }

    /// Get the current pan position (in strip coordinates: -1=left, 0=center, 1=right).
    /// Does not advance any ramp parameters.
    pub fn current_pan(&self) -> f32 {
        self.pan.current()
    }

    /// Returns true if the channel is effectively silent (muted or silenced by solo).
    pub fn is_silent(&self) -> bool {
        self.mute_gain.target() == 0.0 || self.solo_gain.target() == 0.0
    }

    /// Linear pan gains from pan position (matching classic tracker behavior).
    fn pan_gains(pan: f32) -> (f32, f32) {
        let pan = pan.clamp(-1.0, 1.0);
        // Equal-power panning (-3dB center)
        // Map pan (-1.0 to 1.0) to an angle from 0 to pi/2 (90 degrees).
        // At pan = -1.0: angle = 0
        // At pan = 0.0: angle = pi/4 (45 degrees)
        // At pan = 1.0: angle = pi/2 (90 degrees)
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        (angle.cos(), angle.sin())
    }
}

impl Default for ChannelStrip {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ChannelStrip;

    fn run_to_target(strip: &mut ChannelStrip) {
        for _ in 0..320 {
            let _ = strip.next_gains();
        }
    }

    #[test]
    fn test_channel_strip_default() {
        let strip = ChannelStrip::new();
        let (left, right) = strip.current_gains();
        let center = std::f32::consts::FRAC_1_SQRT_2;
        assert!((left - center).abs() < 0.0001);
        assert!((right - center).abs() < 0.0001);
        assert!(!strip.is_silent());
    }

    #[test]
    fn test_channel_strip_update_volume() {
        let mut strip = ChannelStrip::new();
        strip.update_from_track(0.5, 0.0, false, false, false, &[]);
        run_to_target(&mut strip);

        let (left, right) = strip.current_gains();
        let center_half = 0.5 * std::f32::consts::FRAC_1_SQRT_2;
        assert!((left - center_half).abs() < 0.001);
        assert!((right - center_half).abs() < 0.001);
    }

    #[test]
    fn test_channel_strip_update_pan_left() {
        let mut strip = ChannelStrip::new();
        strip.update_from_track(1.0, -1.0, false, false, false, &[]);
        run_to_target(&mut strip);

        let (left, right) = strip.current_gains();
        assert!((left - 1.0).abs() < 0.001);
        assert!(right.abs() < 0.001);
    }

    #[test]
    fn test_channel_strip_update_pan_right() {
        let mut strip = ChannelStrip::new();
        strip.update_from_track(1.0, 1.0, false, false, false, &[]);
        run_to_target(&mut strip);

        let (left, right) = strip.current_gains();
        assert!(left.abs() < 0.001);
        assert!((right - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_channel_strip_mute() {
        let mut strip = ChannelStrip::new();
        strip.update_from_track(1.0, 0.0, true, false, false, &[]);
        run_to_target(&mut strip);

        let (left, right) = strip.current_gains();
        assert!(left.abs() < 0.001);
        assert!(right.abs() < 0.001);
        assert!(strip.is_silent());
    }

    #[test]
    fn test_channel_strip_solo() {
        let mut strip = ChannelStrip::new();
        strip.update_from_track(1.0, 0.0, false, true, false, &[]);
        run_to_target(&mut strip);

        let (left, right) = strip.current_gains();
        assert!(left.abs() < 0.001);
        assert!(right.abs() < 0.001);
        assert!(strip.is_silent());
    }

    #[test]
    fn test_channel_strip_is_silent() {
        let mut strip = ChannelStrip::new();
        assert!(!strip.is_silent());

        strip.update_from_track(1.0, 0.0, true, false, false, &[]);
        assert!(strip.is_silent());

        strip.update_from_track(1.0, 0.0, false, false, false, &[]);
        assert!(!strip.is_silent());
    }

    #[test]
    fn test_channel_strip_ramp_no_click() {
        let mut strip = ChannelStrip::new();

        let (before_l, before_r) = strip.current_gains();
        strip.update_from_track(0.0, 0.0, false, false, false, &[]);
        let (after_one_l, after_one_r) = strip.next_gains();

        assert!(after_one_l > 0.0 && after_one_l < before_l);
        assert!(after_one_r > 0.0 && after_one_r < before_r);

        run_to_target(&mut strip);
        let (final_l, final_r) = strip.current_gains();
        assert!(final_l.abs() < 0.001);
        assert!(final_r.abs() < 0.001);
    }

    #[test]
    fn test_channel_strip_send_levels_default() {
        let strip = ChannelStrip::new();
        assert_eq!(strip.num_send_levels(), 0);
    }

    #[test]
    fn test_channel_strip_ensure_send_levels() {
        let mut strip = ChannelStrip::new();
        strip.ensure_send_levels(4);
        assert_eq!(strip.num_send_levels(), 4);
    }

    #[test]
    fn test_channel_strip_send_level_ramp() {
        let mut strip = ChannelStrip::new();
        strip.set_sample_rate(48_000.0);
        strip.ensure_send_levels(1);
        strip.update_from_track(1.0, 0.0, false, false, false, &[1.0]);

        let first = strip.next_send_level(0);
        assert!(first > 0.0 && first < 1.0);

        for _ in 0..320 {
            let _ = strip.next_send_level(0);
        }

        let near_target = strip.next_send_level(0);
        assert!((near_target - 1.0).abs() < 0.001);
    }
}
