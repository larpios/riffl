use crate::audio::bus::BusSystem;
use crate::audio::channel_strip::ChannelStrip;
use crate::audio::effect_processor::TrackerEffectProcessor;
use crate::audio::sample::{LoopMode, Sample};
use crate::song::Instrument;
use std::sync::Arc;

impl super::Mixer {
    /// Trigger a one-shot preview of the given sample.
    /// `playback_rate` = `(target_freq / base_freq) * (sample_rate / output_sample_rate)`.
    /// For natural pitch use `sample.sample_rate() as f64 / output_sample_rate as f64`.
    pub fn trigger_preview(&mut self, sample: Arc<Sample>, playback_rate: f64) {
        self.preview_pos = 0.0;
        self.preview_rate = playback_rate;
        self.preview_sample = Some(sample);
    }

    /// Stop any currently playing preview.
    pub fn stop_preview(&mut self) {
        self.preview_sample = None;
    }

    /// Returns `true` when a preview is currently active (started but not yet finished or stopped).
    pub fn is_preview_playing(&self) -> bool {
        self.preview_sample.is_some()
    }

    /// Returns `(current_frame_pos, total_frames)` for the active preview.
    ///
    /// `current_frame_pos` is the integer part of `preview_pos` (sample-native frames).
    /// `total_frames` is the frame count of the preview sample, or `0` when no preview is loaded.
    pub fn preview_pos_and_total(&self) -> (usize, usize) {
        let pos = self.preview_pos as usize;
        let total = self
            .preview_sample
            .as_ref()
            .map(|s| s.frame_count())
            .unwrap_or(0);
        (pos, total)
    }

    /// Trigger a one-shot preview starting from `start_frame` (in sample-native frames).
    /// Use `0` to play from the beginning, same as [`trigger_preview`].
    pub fn trigger_preview_at(
        &mut self,
        sample: Arc<Sample>,
        playback_rate: f64,
        start_frame: usize,
    ) {
        self.preview_pos = start_frame as f64;
        self.preview_rate = playback_rate;
        self.preview_sample = Some(sample);
    }

    /// Get the number of currently active voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| matches!(v, Some(voice) if voice.active))
            .count()
    }

    /// Add a sample to the instrument list and return its instrument index.
    pub fn add_sample(&mut self, sample: Arc<Sample>) -> usize {
        let idx = self.samples.len();
        self.samples.push(sample);
        idx
    }

    /// Replace the instrument definitions used for volume/finetune lookup.
    pub fn set_instruments(&mut self, instruments: Vec<Instrument>) {
        self.instruments = instruments;
    }

    /// Clear all loaded samples.
    pub fn clear_samples(&mut self) {
        self.samples.clear();
        self.stop_all();
    }

    /// Get the number of loaded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Get the name of a loaded sample by index.
    pub fn sample_name(&self, index: usize) -> Option<&str> {
        self.samples.get(index).and_then(|s| s.name())
    }

    /// Get a reference to the loaded samples.
    pub fn samples(&self) -> &[Arc<Sample>] {
        &self.samples
    }

    /// Replace a loaded sample by index.
    pub fn replace_sample(&mut self, index: usize, sample: Arc<Sample>) {
        if let Some(slot) = self.samples.get_mut(index) {
            *slot = sample;
        }
    }

    /// Update loop settings for a sample by index.
    pub fn set_sample_loop(
        &mut self,
        index: usize,
        mode: LoopMode,
        loop_start: usize,
        loop_end: usize,
    ) {
        if let Some(sample) = self.samples.get(index) {
            let mut s = (**sample).clone();
            s.loop_mode = mode;
            s.loop_start = loop_start;
            s.loop_end = loop_end;
            self.samples[index] = Arc::new(s);
        }
    }

    /// Stop all voices immediately and reset effect state.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            *voice = None;
        }
        self.pending_notes.clear();
        self.effect_processor.reset_all();
        self.bus_system.reset();
        self.reset_channel_levels();
        self.reset_oscilloscope_buffers();
        self.reset_fft_buffer();
    }

    /// Reset the FFT capture buffer to silence.
    pub fn reset_fft_buffer(&mut self) {
        self.visualizer.reset_fft_buffer();
    }

    /// Set the ticks per line (TPL) for the mixer and its effect processor.
    pub fn set_tpl(&mut self, tpl: u32) {
        self.tpl = tpl.max(1);
        for ch in 0..self.voices.len() {
            if let Some(state) = self.effect_processor.channel_state_mut(ch) {
                state.ticks_per_row = self.tpl as u8;
            }
        }
    }

    /// Set the global panning separation (0-128, 128 is full stereo).
    pub fn set_pan_separation(&mut self, sep: u8) {
        self.pan_separation = sep;
    }

    /// Set the panning law for the mixer.
    pub fn set_panning_law(&mut self, law: crate::song::PanningLaw) {
        self.panning_law = law;
    }

    /// Update the effect processor's tempo (frames per row).
    pub fn update_tempo(&mut self, bpm: f64) {
        self.bpm = bpm;
        self.effect_processor.update_tempo(bpm);
    }

    /// Get a reference to the effect processor.
    pub fn effect_processor(&self) -> &TrackerEffectProcessor {
        &self.effect_processor
    }

    /// Set the effect interpretation mode.
    pub fn set_effect_mode(&mut self, mode: crate::pattern::effect::EffectMode) {
        self.effect_processor.set_effect_mode(mode);
    }

    pub fn set_format_is_s3m(&mut self, is_s3m: bool) {
        self.format_is_s3m = is_s3m;
        self.effect_processor.set_use_high_res_periods(is_s3m);
    }

    /// Set the range for global volume effects (64.0 for S3M/XM, 128.0 for IT).
    pub fn set_global_volume_range(&mut self, range: f32) {
        self.effect_processor.global_volume_range = range;
    }

    /// Set the pitch slide mode for all channels.
    ///
    /// Use `SlideMode::AmigaPeriod` for MOD and S3M files.
    pub fn set_slide_mode(&mut self, mode: crate::audio::pitch::SlideMode) {
        self.effect_processor.set_slide_mode(mode);
        if mode == crate::audio::pitch::SlideMode::AmigaPeriod {
            let clock = if self.format_is_s3m {
                crate::audio::pitch::AMIGA_S3M_CLOCK
            } else {
                crate::audio::pitch::AMIGA_PAL_CLOCK
            };
            for ch in 0..self.voices.len() {
                self.effect_processor.set_period_clock(ch, clock);
            }
        }
    }

    /// Get a mutable reference to the bus system for configuration.
    pub fn bus_system_mut(&mut self) -> &mut BusSystem {
        &mut self.bus_system
    }

    /// Get a reference to the bus system.
    pub fn bus_system(&self) -> &BusSystem {
        &self.bus_system
    }

    /// Returns whether a channel is currently silenced by mute/solo state.
    pub fn is_channel_silent(&self, channel: usize) -> bool {
        self.channel_strips
            .get(channel)
            .is_some_and(ChannelStrip::is_silent)
    }

    /// Get the peak level for a channel (left, right).
    /// Returns (0.0, 0.0) for invalid channel indices.
    pub fn get_channel_level(&self, channel: usize) -> (f32, f32) {
        self.visualizer.get_channel_level(channel)
    }

    /// Reset all channel levels to zero.
    pub fn reset_channel_levels(&mut self) {
        self.visualizer.reset_channel_levels();
    }

    /// Reset all oscilloscope buffers to zero.
    pub fn reset_oscilloscope_buffers(&mut self) {
        self.visualizer.reset_oscilloscope_buffers();
    }

    /// Read the oscilloscope waveform for a channel.
    /// Returns a slice of the ring buffer in chronological order (oldest first).
    /// The returned Vec has exactly `OSCILLOSCOPE_BUF_SIZE` samples.
    pub fn oscilloscope_data(&self, channel: usize) -> Vec<f32> {
        self.visualizer.oscilloscope_data(channel)
    }

    /// Read the master bus FFT capture buffer in chronological order.
    pub fn fft_data(&self) -> Vec<f32> {
        self.visualizer.fft_data()
    }

    /// Decay all channel levels by the given factor (0.0 to 1.0).
    /// Called from the UI update loop for visual smoothing.
    pub fn decay_channel_levels(&mut self, decay_factor: f32) {
        self.visualizer.decay_channel_levels(decay_factor);
    }
}
