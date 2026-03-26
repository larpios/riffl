//! Audio mixer/sequencer that connects patterns to the audio engine.
//!
//! The mixer reads pattern data row by row, triggers sample playback for
//! note events, and mixes all active voices into a stereo output buffer.

use crate::audio::bus::{self, BusSystem};
use crate::audio::channel_strip::ChannelStrip;
use crate::audio::dsp::ProcessSpec;
use crate::audio::effect_processor::TrackerEffectProcessor;
use crate::audio::pending_note::PendingNote;
use crate::audio::sample::Sample;
use crate::audio::visualizer::Visualizer;
use crate::audio::voice::Voice;
use crate::pattern::track::Track;
use crate::song::Instrument;

use std::sync::Arc;

mod preview;
mod render;
#[cfg(test)]
mod tests;
mod tick;

/// Audio mixer that reads pattern data and produces mixed audio output.
///
/// The mixer holds references to loaded samples and maintains per-channel
/// voice state. When `tick()` is called with a row index and pattern, it
/// processes note events and updates voice states. The `render()` method
/// fills an audio buffer by mixing all active voices.
///
/// Multi-track support: the mixer stores per-channel mixing state (volume,
/// pan, mute/solo) synced from track metadata. Equal-power panning is used
/// with -3dB center.
pub struct Mixer {
    /// Set to true if current file is S3M (uses 14.3MHz clock).
    format_is_s3m: bool,
    /// Loaded audio samples available for playback.
    samples: Vec<Arc<Sample>>,
    /// Instrument definitions (mapping name to sample index).
    instruments: Vec<Instrument>,
    /// Per-channel voice state (one voice per channel).
    voices: Vec<Option<Voice>>,
    /// Output sample rate in Hz (used for pitch calculation).
    output_sample_rate: u32,
    /// Per-channel mixing state (volume, pan, mute, solo).
    channel_strips: Vec<ChannelStrip>,
    /// Per-channel effect processing state.
    effect_processor: TrackerEffectProcessor,
    /// Number of ticks per line (TPL).
    tpl: u32,
    /// Pending note triggers (EDx effect).
    pending_notes: Vec<PendingNote>,
    /// Send/return bus system for effects routing.
    bus_system: BusSystem,
    /// One-shot preview sample (e.g. audition from browser). Independent of pattern voices.
    preview_sample: Option<Arc<Sample>>,
    /// Current read position (in frames) within the preview sample.
    preview_pos: f64,
    /// Playback rate for the preview voice (accounts for pitch + sample/output rate ratio).
    preview_rate: f64,
    /// Visualization and monitoring state.
    pub visualizer: Visualizer,
    /// Current BPM for BPM-synced LFO calculations.
    bpm: f64,
    /// Global panning separation (0-128, 128 is full stereo).
    pan_separation: u8,
    /// Panning law to use for rendering.
    panning_law: crate::song::PanningLaw,
}

impl Mixer {
    /// Create a new mixer with the given samples, instruments, and channel count.
    ///
    /// # Arguments
    /// * `samples` - The loaded audio samples indexed by instrument number
    /// * `instruments` - Instrument definitions
    /// * `num_channels` - Number of pattern channels (one voice per channel)
    /// * `output_sample_rate` - The output sample rate in Hz
    pub fn new(
        samples: Vec<Arc<Sample>>,
        instruments: Vec<Instrument>,
        num_channels: usize,
        output_sample_rate: u32,
    ) -> Self {
        let mut bus_system = BusSystem::new(bus::DEFAULT_NUM_BUSES);
        bus_system.prepare(ProcessSpec {
            sample_rate: output_sample_rate as f32,
            max_block_frames: 2048,
            channels: 2,
        });

        let channel_strips: Vec<ChannelStrip> = (0..num_channels)
            .map(|_| {
                let mut strip = ChannelStrip::new();
                strip.set_sample_rate(output_sample_rate as f32);
                strip.ensure_send_levels(bus_system.num_buses());
                strip
            })
            .collect();

        Self {
            format_is_s3m: false,
            samples,
            instruments,
            voices: vec![None; num_channels],
            output_sample_rate,
            channel_strips,
            effect_processor: TrackerEffectProcessor::new(num_channels, output_sample_rate),
            tpl: 6,
            pending_notes: Vec::new(),
            bus_system,
            preview_sample: None,
            preview_pos: 0.0,
            preview_rate: 1.0,
            visualizer: Visualizer::new(num_channels),
            bpm: 120.0,
            pan_separation: 128,
            panning_law: crate::song::PanningLaw::EqualPower,
        }
    }

    /// Dynamically adjust the operational channel count.
    /// Resizes voices, mixer channel strips, VU meters, and internal effect processors.
    pub fn set_num_channels(&mut self, num_channels: usize) {
        if num_channels > self.voices.len() {
            // pad out more tracks to match requested configuration
            let sample_rate = self.output_sample_rate as f32;
            let num_buses = self.bus_system.num_buses();
            for _ in self.voices.len()..num_channels {
                self.voices.push(None);

                let mut strip = ChannelStrip::new();
                strip.set_sample_rate(sample_rate);
                strip.ensure_send_levels(num_buses);
                self.channel_strips.push(strip);
            }
        } else {
            // trim tracks if shrinking
            self.voices.truncate(num_channels);
            self.channel_strips.truncate(num_channels);
        }

        self.visualizer.set_num_channels(num_channels);

        // Push the changes down to the effect processor as well
        self.effect_processor.resize_channels(num_channels);
    }

    /// Snap all channel strip pans to match the provided tracks immediately (no ramp).
    ///
    /// Call this once when loading a file so that even the very first rendered
    /// sample plays at the correct stereo position. Regular `update_tracks` uses
    /// a smoothing ramp which causes a 5ms delay before reaching the target,
    /// making note attacks sound centered.
    pub fn snap_channel_pans(&mut self, tracks: &[Track]) {
        for (ch, strip) in self.channel_strips.iter_mut().enumerate() {
            let pan = tracks.get(ch).map_or(0.0, |t| t.pan);
            strip.set_effect_pan_immediate(pan);
        }
    }

    /// Update per-channel mixing state from track metadata.
    ///
    /// This syncs the mixer's internal mixing state with the track
    /// volume, pan, mute, and solo settings from the pattern.
    pub fn update_tracks(&mut self, tracks: &[Track]) {
        let any_soloed = tracks.iter().any(|t| t.solo);

        for (ch, strip) in self.channel_strips.iter_mut().enumerate() {
            if let Some(track) = tracks.get(ch) {
                strip.ensure_send_levels(self.bus_system.num_buses());
                strip.update_from_track(
                    track.volume,
                    track.pan,
                    track.muted,
                    any_soloed,
                    track.solo,
                    &track.send_levels,
                );
            } else {
                strip.ensure_send_levels(self.bus_system.num_buses());
                strip.update_from_track(1.0, 0.0, false, false, false, &[]);
            }
        }
    }

    /// Set the global volume multiplier for the song.
    pub fn set_global_volume(&mut self, volume: f32) {
        self.effect_processor.global_volume = volume;
    }
}
