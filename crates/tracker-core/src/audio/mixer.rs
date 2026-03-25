//! Audio mixer/sequencer that connects patterns to the audio engine.
//!
//! The mixer reads pattern data row by row, triggers sample playback for
//! note events, and mixes all active voices into a stereo output buffer.

use crate::audio::bus::{self, BusSystem};
use crate::audio::channel_strip::ChannelStrip;
use crate::audio::dsp::ProcessSpec;
use crate::audio::effect_processor::{TrackerEffectProcessor, TransportCommand};
use crate::audio::sample::{LoopMode, Sample};
use crate::pattern::note::NoteEvent;
use crate::pattern::pattern::Pattern;
use crate::pattern::track::Track;
use crate::pattern::EffectType;
use crate::song::{Adsr, Instrument, LfoWaveform};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Number of samples in each per-channel oscilloscope ring buffer.
pub const OSCILLOSCOPE_BUF_SIZE: usize = 512;

/// Number of samples in the master bus FFT capture buffer.
pub const FFT_BUF_SIZE: usize = 1024;

fn f32_to_u32_bits(f: f32) -> u32 {
    f.to_bits()
}

fn u32_bits_to_f32(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn atomic_max_f32(atomic: &AtomicU32, new_val: f32) {
    let new_bits = f32_to_u32_bits(new_val);
    let old_bits = atomic.load(Ordering::Relaxed);
    if new_bits > old_bits {
        atomic.store(new_bits, Ordering::Relaxed);
    }
}

/// Evaluates an LFO waveform at the given phase (0.0 to 1.0).
/// Returns a value in the range [-1.0, 1.0].
fn evaluate_lfo_waveform(waveform: LfoWaveform, phase: f32) -> f32 {
    match waveform {
        LfoWaveform::Sine => (phase * 2.0 * std::f32::consts::PI).sin(),
        LfoWaveform::Triangle => {
            if phase < 0.25 {
                phase * 4.0
            } else if phase < 0.75 {
                2.0 - phase * 4.0
            } else {
                phase * 4.0 - 4.0
            }
        }
        LfoWaveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        LfoWaveform::Sawtooth => phase * 2.0 - 1.0,
        LfoWaveform::ReverseSaw => 1.0 - phase * 2.0,
        LfoWaveform::Random => {
            let bits = phase.to_bits();
            let mut x = bits ^ (bits >> 16);
            x = x.wrapping_mul(0x85ebca6b);
            x = x ^ (x >> 13);
            x = x.wrapping_mul(0xc2b2ae35);
            x = x ^ (x >> 16);
            (x as f32 / u32::MAX as f32) * 2.0 - 1.0
        }
    }
}

/// Per-voice LFO position state for parameter modulation.
#[derive(Debug, Clone, Copy, Default)]
struct VoiceLfoState {
    /// LFO position for volume modulation (0.0 to 1.0, wraps each cycle).
    volume: f32,
    /// LFO position for panning modulation.
    panning: f32,
    /// LFO position for pitch modulation.
    pitch: f32,
}

impl VoiceLfoState {
    fn new(instrument: Option<&Instrument>) -> Self {
        Self {
            volume: instrument
                .and_then(|i| i.volume_lfo.as_ref())
                .map(|l| l.phase)
                .unwrap_or(0.0),
            panning: instrument
                .and_then(|i| i.panning_lfo.as_ref())
                .map(|l| l.phase)
                .unwrap_or(0.0),
            pitch: instrument
                .and_then(|i| i.pitch_lfo.as_ref())
                .map(|l| l.phase)
                .unwrap_or(0.0),
        }
    }

    fn update(&mut self, instrument: &Instrument, sample_rate: u32, bpm: f64) {
        if let Some(lfo) = &instrument.volume_lfo {
            if lfo.enabled && lfo.rate > 0.0 {
                let rate_hz = if lfo.sync_to_bpm {
                    bpm / 60.0 * lfo.rate as f64
                } else {
                    lfo.rate as f64
                };
                self.volume = (self.volume + rate_hz as f32 / sample_rate as f32) % 1.0;
            }
        }
        if let Some(lfo) = &instrument.panning_lfo {
            if lfo.enabled && lfo.rate > 0.0 {
                let rate_hz = if lfo.sync_to_bpm {
                    bpm / 60.0 * lfo.rate as f64
                } else {
                    lfo.rate as f64
                };
                self.panning = (self.panning + rate_hz as f32 / sample_rate as f32) % 1.0;
            }
        }
        if let Some(lfo) = &instrument.pitch_lfo {
            if lfo.enabled && lfo.rate > 0.0 {
                let rate_hz = if lfo.sync_to_bpm {
                    bpm / 60.0 * lfo.rate as f64
                } else {
                    lfo.rate as f64
                };
                self.pitch = (self.pitch + rate_hz as f32 / sample_rate as f32) % 1.0;
            }
        }
    }

    fn get_vol_value(&self, lfo: &crate::song::Lfo) -> f32 {
        self.calculate_value(self.volume, lfo)
    }

    fn get_pan_value(&self, lfo: &crate::song::Lfo) -> f32 {
        self.calculate_value(self.panning, lfo)
    }

    fn get_pitch_value(&self, lfo: &crate::song::Lfo) -> f32 {
        self.calculate_value(self.pitch, lfo)
    }

    fn calculate_value(&self, phase: f32, lfo: &crate::song::Lfo) -> f32 {
        if !lfo.enabled {
            return 0.0;
        }

        let raw_val = evaluate_lfo_waveform(lfo.waveform, phase);
        lfo.offset + raw_val * lfo.depth
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum AdsrPhase {
    #[default]
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

#[derive(Debug, Clone, Copy, Default)]
struct AdsrState {
    phase: AdsrPhase,
    value: f32,
    /// Current time in the phase (seconds).
    phase_time: f32,
    /// Value when entering the release phase.
    release_start_value: f32,
}

impl AdsrState {
    fn update(&mut self, adsr: &Adsr, key_on: bool, sample_rate: u32) -> f32 {
        let dt = 1.0 / sample_rate as f32;

        if !key_on && self.phase != AdsrPhase::Release && self.phase != AdsrPhase::Done {
            self.phase = AdsrPhase::Release;
            self.phase_time = 0.0;
            self.release_start_value = self.value;
        }

        match self.phase {
            AdsrPhase::Attack => {
                let attack_secs = adsr.attack / 1000.0;
                if attack_secs > 0.0 {
                    self.value = (self.phase_time / attack_secs).min(1.0);
                    self.phase_time += dt;
                    if self.phase_time >= attack_secs {
                        self.phase = AdsrPhase::Decay;
                        self.phase_time = 0.0;
                    }
                } else {
                    self.value = 1.0;
                    self.phase = AdsrPhase::Decay;
                    self.phase_time = 0.0;
                }
            }
            AdsrPhase::Decay => {
                let decay_secs = adsr.decay / 1000.0;
                if decay_secs > 0.0 {
                    let range = 1.0 - adsr.sustain;
                    self.value = 1.0 - (self.phase_time / decay_secs).min(1.0) * range;
                    self.phase_time += dt;
                    if self.phase_time >= decay_secs {
                        self.phase = AdsrPhase::Sustain;
                        self.phase_time = 0.0;
                        self.value = adsr.sustain;
                    }
                } else {
                    self.value = adsr.sustain;
                    self.phase = AdsrPhase::Sustain;
                    self.phase_time = 0.0;
                }
            }
            AdsrPhase::Sustain => {
                self.value = adsr.sustain;
                // stays here until key_on is false
            }
            AdsrPhase::Release => {
                let release_secs = adsr.release / 1000.0;
                if release_secs > 0.0 {
                    // start from capture value (which might be sustain level or somewhere in A/D)
                    let remaining = (self.phase_time / release_secs).min(1.0);
                    self.value = self.release_start_value * (1.0 - remaining);
                    self.phase_time += dt;
                    if self.phase_time >= release_secs {
                        self.phase = AdsrPhase::Done;
                        self.value = 0.0;
                    }
                } else {
                    self.value = 0.0;
                    self.phase = AdsrPhase::Done;
                }
            }
            AdsrPhase::Done => {
                self.value = 0.0;
            }
        }

        self.value
    }
}

/// State for a single voice playing a sample.
#[derive(Debug, Clone)]
struct Voice {
    /// Index into the mixer's instrument list.
    instrument_index: usize,
    /// Index into the mixer's sample list.
    sample_index: usize,
    /// Current read position within the sample's audio data (in frames).
    position: f64,
    /// Playback rate relative to the sample's base rate (for pitch shifting).
    #[allow(dead_code)]
    playback_rate: f64,
    /// Volume multiplier derived from note velocity (0.0 - 1.0).
    velocity_gain: f32,
    /// Whether this voice is actively producing audio.
    active: bool,
    /// Current playback direction (1.0 for forward, -1.0 for reverse).
    /// Used for ping-pong loops.
    loop_direction: f64,
    /// Whether the key is currently held down.
    key_on: bool,
    /// Current volume envelope position in ticks.
    volume_envelope_tick: usize,
    /// Current panning envelope position in ticks.
    panning_envelope_tick: usize,
    /// Current pitch envelope position in ticks.
    pitch_envelope_tick: usize,
    /// ADSR state for volume.
    volume_adsr: AdsrState,
    /// ADSR state for panning.
    panning_adsr: AdsrState,
    /// ADSR state for pitch.
    pitch_adsr: AdsrState,
    /// Ratio to convert an absolute frequency (Hz) into relative playback_rate.
    hz_to_rate: f64,
    /// The absolute frequency of the note that triggered this voice.
    triggered_note_freq: f64,
    /// Fadeout multiplier for IT/XM instruments (0.0 - 1.0).
    /// Decreased by instrument.fadeout every tick when key_on is false.
    fadeout_multiplier: f32,
    /// Per-voice LFO phase positions for volume, panning, and pitch.
    lfo: VoiceLfoState,
}

impl Voice {
    fn new(
        instrument: Option<&Instrument>,
        instrument_index: usize,
        sample_index: usize,
        playback_rate: f64,
        velocity_gain: f32,
        hz_to_rate: f64,
        triggered_note_freq: f64,
    ) -> Self {
        Self {
            instrument_index,
            sample_index,
            position: 0.0,
            playback_rate,
            velocity_gain,
            active: true,
            loop_direction: 1.0,
            key_on: true,
            volume_envelope_tick: 0,
            panning_envelope_tick: 0,
            pitch_envelope_tick: 0,
            volume_adsr: AdsrState::default(),
            panning_adsr: AdsrState::default(),
            pitch_adsr: AdsrState::default(),
            hz_to_rate,
            triggered_note_freq,
            fadeout_multiplier: 1.0,
            lfo: VoiceLfoState::new(instrument),
        }
    }

    fn with_position(mut self, position: f64) -> Self {
        self.position = position;
        self
    }
}

/// Pending note trigger for Note Delay (EDx).
#[derive(Debug, Clone)]
struct PendingNote {
    channel: usize,
    instrument_index: usize,
    sample_index: usize,
    playback_rate: f64,
    velocity_gain: f32,
    hz_to_rate: f64,
    triggered_note_freq: f64,
    /// Effective Amiga period clock for this note trigger.
    period_clock: f64,
    offset: Option<usize>,
    trigger_frame: u32,
}

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
    /// Per-channel peak levels for VU meters (left, right) as atomic u32 bit patterns.
    /// Updated during render(), read by UI thread.
    channel_levels: Vec<(AtomicU32, AtomicU32)>,
    /// Per-channel oscilloscope ring buffers (mono mix of L+R).
    /// Written by the audio thread in render(), read by the UI thread for display.
    /// Each buffer is OSCILLOSCOPE_BUF_SIZE samples, written circularly.
    oscilloscope_bufs: Vec<Vec<f32>>,
    /// Per-channel write position into the oscilloscope ring buffer.
    oscilloscope_write_pos: Vec<AtomicU32>,
    /// Master bus FFT capture ring buffer (mono).
    fft_buf: Vec<f32>,
    /// Write position into the FFT ring buffer.
    fft_write_pos: AtomicU32,
    /// Current BPM for BPM-synced LFO calculations.
    bpm: f64,
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

        let channel_levels: Vec<(AtomicU32, AtomicU32)> = (0..num_channels)
            .map(|_| (AtomicU32::new(0), AtomicU32::new(0)))
            .collect();

        let oscilloscope_bufs: Vec<Vec<f32>> = (0..num_channels)
            .map(|_| vec![0.0f32; OSCILLOSCOPE_BUF_SIZE])
            .collect();
        let oscilloscope_write_pos: Vec<AtomicU32> =
            (0..num_channels).map(|_| AtomicU32::new(0)).collect();

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
            channel_levels,
            oscilloscope_bufs,
            oscilloscope_write_pos,
            fft_buf: vec![0.0f32; FFT_BUF_SIZE],
            fft_write_pos: AtomicU32::new(0),
            bpm: 120.0,
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

                self.channel_levels.push((
                    std::sync::atomic::AtomicU32::new(0),
                    std::sync::atomic::AtomicU32::new(0),
                ));
                self.oscilloscope_bufs
                    .push(vec![0.0f32; OSCILLOSCOPE_BUF_SIZE]);
                self.oscilloscope_write_pos.push(AtomicU32::new(0));
            }
        } else {
            // trim tracks if shrinking
            self.voices.truncate(num_channels);
            self.channel_strips.truncate(num_channels);
            self.channel_levels.truncate(num_channels);
            self.oscilloscope_bufs.truncate(num_channels);
            self.oscilloscope_write_pos.truncate(num_channels);
        }

        // Push the changes down to the effect processor as well
        self.effect_processor.resize_channels(num_channels);
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

    /// Process a pattern row, triggering or stopping samples based on note events.
    ///
    /// For each channel in the row:
    /// - `NoteEvent::On(note)`: Start playing the sample at the instrument index,
    ///   pitched to match the note's frequency, with velocity-based volume.
    /// - `NoteEvent::Off`: Stop the voice on that channel.
    /// - `NoteEvent::Cut`: Immediately hard-silence the voice (no envelope release).
    /// - No event: The existing voice continues playing.
    ///
    /// Returns any transport commands generated by effects (BPM changes,
    /// position jumps, pattern breaks).
    pub fn tick(&mut self, row_index: usize, pattern: &Pattern) -> Vec<TransportCommand> {
        // Sync track mixing state (volume, pan, mute/solo)
        self.update_tracks(pattern.tracks());

        let row = match pattern.get_row(row_index) {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Clear pending notes from previous row
        self.pending_notes.clear();
        self.effect_processor.reset_row_slides();

        let mut transport_commands = Vec::new();

        for (ch, cell) in row.iter().enumerate() {
            if ch >= self.voices.len() {
                break;
            }

            // Skip muted/non-soloed channels: don't trigger new notes
            let audible = !self
                .channel_strips
                .get(ch)
                .is_some_and(ChannelStrip::is_silent);

            // In classical trackers, specifying an instrument number OR triggering a new
            // note resets the channel volume, clearing any previous volume slides or overrides.
            if cell.instrument.is_some() || matches!(&cell.note, Some(NoteEvent::On(_))) {
                if let Some(state) = self.effect_processor.channel_state_mut(ch) {
                    state.volume_override = None;
                }
            }

            // Determine the note frequency for effect processing
            let note_frequency = match &cell.note {
                Some(NoteEvent::On(note)) => Some(note.frequency()),
                _ => None,
            };

            // Process effects for this channel
            let cmds = self
                .effect_processor
                .process_row(ch, &cell.effects, note_frequency);

            if let Some(vol) = cell.volume {
                if let Some(state) = self.effect_processor.channel_state_mut(ch) {
                    state.volume_override = Some((vol as f32 / 64.0).clamp(0.0, 2.0));
                }
            }

            for cmd in &cmds {
                if let TransportCommand::SetTpl(tpl) = cmd {
                    self.set_tpl(*tpl);
                }
            }

            transport_commands.extend(cmds);

            // Apply effect-based panning override (8xx) to the channel strip.
            // Panning is stored as 0.0 (left) → 1.0 (right); strip uses -1.0 → 1.0.
            if let Some(pan_01) = self.effect_processor.channel_panning(ch) {
                let pan = pan_01 * 2.0 - 1.0;
                if let Some(strip) = self.channel_strips.get_mut(ch) {
                    strip.set_effect_pan_immediate(pan);
                }
            }

            match &cell.note {
                Some(NoteEvent::On(note)) => {
                    if !audible {
                        // Muted channel: stop any playing voice, don't start new one
                        self.voices[ch] = None;
                        continue;
                    }

                    let has_tone_porta = cell.effects.iter().any(|e| {
                        matches!(
                            e.effect_type(),
                            Some(EffectType::PortamentoToNote)
                                | Some(EffectType::TonePortamentoVolumeSlide)
                                | Some(EffectType::PortamentoFine)
                                | Some(EffectType::PortamentoExtraFine)
                        )
                    });

                    let note_frequency = note.frequency();
                    let instrument_idx = cell.instrument.unwrap_or(note.instrument) as usize;

                    // Resolve sample index: use keyzone matching if instrument has keyzones,
                    // otherwise fall back to instrument's sample_index.
                    let resolved_sample_idx =
                        if let Some(inst) = self.instruments.get(instrument_idx) {
                            inst.resolve_sample_index(note.midi_note(), note.velocity)
                        } else {
                            Some(instrument_idx)
                        };

                    if let Some(resolved_sample_idx) = resolved_sample_idx {
                        if resolved_sample_idx < self.samples.len() {
                            let sample = &self.samples[resolved_sample_idx];
                            // Calculate playback rate to pitch the sample to the desired note.
                            // The sample's base_note (default C-4) plays at original speed.
                            // Higher notes play faster, lower notes play slower.
                            let base_freq = sample.base_frequency();
                            let target_freq = note_frequency;
                            let sample_rate_ratio =
                                sample.sample_rate() as f64 / self.output_sample_rate as f64;

                            let mut hz_to_rate = (1.0 / base_freq) * sample_rate_ratio;

                            // Apply sample-level finetune (in cents)
                            if sample.finetune != 0 {
                                hz_to_rate *= 2.0_f64.powf(sample.finetune as f64 / 1200.0);
                            }

                            // Apply finetune from instrument or effect override
                            let finetune = if let Some(ft_override) =
                                self.effect_processor.finetune_override(ch)
                            {
                                ft_override
                            } else {
                                self.instruments
                                    .get(instrument_idx)
                                    .map(|inst| inst.finetune)
                                    .unwrap_or(0)
                            };

                            if finetune != 0 {
                                // ProTracker finetune formula: 1 unit = 1/8th of a semitone
                                hz_to_rate *= 2.0_f64.powf(finetune as f64 / (12.0 * 8.0));
                            }

                            let playback_rate = target_freq * hz_to_rate;

                            let is_amiga = self
                                .effect_processor
                                .channel_state(ch)
                                .map(|s| {
                                    s.slide_mode == crate::audio::pitch::SlideMode::AmigaPeriod
                                })
                                .unwrap_or(false);

                            let clock = if self.format_is_s3m {
                                crate::audio::pitch::AMIGA_S3M_CLOCK
                            } else {
                                crate::audio::pitch::AMIGA_PAL_CLOCK
                            };

                            let period_clock = if is_amiga {
                                clock * base_freq / sample.sample_rate() as f64
                            } else {
                                clock
                            };

                            if is_amiga {
                                self.effect_processor.set_period_clock(ch, period_clock);
                            }

                            let inst_vol = self
                                .instruments
                                .get(instrument_idx)
                                .map(|inst| inst.volume)
                                .unwrap_or(1.0);
                            let velocity = note.velocity as f32;
                            let velocity_gain = (velocity / 127.0) * inst_vol * sample.volume;

                            // Apply instrument panning override to the effect processor
                            if let Some(inst_pan) =
                                self.instruments.get(instrument_idx).and_then(|i| i.panning)
                            {
                                if let Some(state) = self.effect_processor.channel_state_mut(ch) {
                                    state.panning_override = Some((inst_pan + 1.0) / 2.0);
                                }
                            }

                            if has_tone_porta && self.voices[ch].is_some() {
                                // A tone portamento effect exists and a voice is already playing.
                                // Do not trigger a new sample. The TrackerEffectProcessor handles target pitch updates.
                                // However, we update the voice's instrument settings (volume, panning, etc) if an instrument is specified.
                                if let Some(voice) = &mut self.voices[ch] {
                                    // Reset key_on and fadeout state for the new note
                                    voice.key_on = true;
                                    voice.fadeout_multiplier = 1.0;

                                    if cell.instrument.is_some() {
                                        voice.instrument_index = instrument_idx;
                                        voice.velocity_gain = velocity_gain;
                                        voice.hz_to_rate = hz_to_rate;
                                        // Update LFO state (reset envelopes) if instrument specified
                                        voice.lfo = VoiceLfoState::new(
                                            self.instruments.get(instrument_idx),
                                        );
                                        // Reset envelopes for new instrument
                                        voice.volume_envelope_tick = 0;
                                        voice.panning_envelope_tick = 0;
                                        voice.pitch_envelope_tick = 0;
                                        voice.volume_adsr = AdsrState::default();
                                        voice.panning_adsr = AdsrState::default();
                                        voice.pitch_adsr = AdsrState::default();
                                    }
                                }
                            } else {
                                // Check for Note Delay (EDx)
                                if let Some(delay_tick) = self
                                    .effect_processor
                                    .channel_state(ch)
                                    .and_then(|s| s.note_delay_tick)
                                {
                                    let frames_per_row = self
                                        .effect_processor
                                        .channel_state(ch)
                                        .map(|s| s.frames_per_row)
                                        .unwrap_or(6000);
                                    let frames_per_tick = frames_per_row / self.tpl;
                                    let trigger_frame = delay_tick as u32 * frames_per_tick;

                                    let offset = self.effect_processor.sample_offset(ch);

                                    self.pending_notes.push(PendingNote {
                                        channel: ch,
                                        instrument_index: instrument_idx,
                                        sample_index: resolved_sample_idx,
                                        playback_rate,
                                        velocity_gain,
                                        hz_to_rate,
                                        triggered_note_freq: target_freq,
                                        period_clock,
                                        offset,
                                        trigger_frame,
                                    });
                                } else {
                                    let mut voice = Voice::new(
                                        self.instruments.get(instrument_idx),
                                        instrument_idx,
                                        resolved_sample_idx,
                                        playback_rate,
                                        velocity_gain,
                                        hz_to_rate,
                                        target_freq,
                                    );
                                    if let Some(offset) = self.effect_processor.sample_offset(ch) {
                                        voice = voice.with_position(offset as f64);
                                    }

                                    self.voices[ch] = Some(voice);
                                }
                            }
                        }
                    }
                }
                Some(NoteEvent::Off) => {
                    if let Some(voice) = &mut self.voices[ch] {
                        let has_envelope = self
                            .instruments
                            .get(voice.instrument_index)
                            .is_some_and(|inst| {
                                inst.volume_adsr.is_some()
                                    || inst.volume_envelope.as_ref().is_some_and(|env| env.enabled)
                            });

                        let has_sustain_loop =
                            self.samples.get(voice.sample_index).is_some_and(|s| {
                                s.sustain_loop_mode != crate::audio::sample::LoopMode::NoLoop
                                    && s.sustain_loop_end > s.sustain_loop_start
                            });

                        if has_envelope || has_sustain_loop {
                            // Release the key: envelope continues past sustain,
                            // and sustain loop releases so the sample plays through.
                            voice.key_on = false;
                        } else {
                            self.voices[ch] = None;
                        }
                    }
                    if let Some(s) = self.effect_processor.channel_state_mut(ch) {
                        s.reset();
                    }
                }
                Some(NoteEvent::Cut) => {
                    // Hard-silence: kill voice immediately with no envelope release.
                    self.voices[ch] = None;
                    if let Some(s) = self.effect_processor.channel_state_mut(ch) {
                        s.reset();
                    }
                }
                None => {
                    // No note event — check if an instrument was provided with a tone portamento.
                    // This handles cases like `305 .. .. 04` where instrument 4 is set without a new note.
                    if let Some(instrument_idx) = cell.instrument {
                        let instrument_idx = instrument_idx as usize;
                        let has_tone_porta = cell.effects.iter().any(|e| {
                            matches!(
                                e.effect_type(),
                                Some(EffectType::PortamentoToNote)
                                    | Some(EffectType::TonePortamentoVolumeSlide)
                                    | Some(EffectType::PortamentoFine)
                                    | Some(EffectType::PortamentoExtraFine)
                            )
                        });

                        if has_tone_porta && self.voices[ch].is_some() {
                            let last_freq = self.effect_processor.last_note_frequency(ch);

                            // Resolve sample for the current instrument at the last frequency.
                            let midi_note =
                                (12.0f64 * (last_freq / 440.0f64).log2() + 69.0f64).round() as u8;
                            let resolved_sample_idx = if let Some(inst) =
                                self.instruments.get(instrument_idx)
                            {
                                if inst.keyzones.is_empty() {
                                    Some(instrument_idx)
                                } else {
                                    inst.resolve_sample_index(midi_note, 100) // Default velocity
                                }
                            } else {
                                Some(instrument_idx)
                            };

                            if let Some(resolved_sample_idx) = resolved_sample_idx {
                                if resolved_sample_idx < self.samples.len() {
                                    let sample = &self.samples[resolved_sample_idx];
                                    let base_freq = sample.base_frequency();
                                    let sample_rate_ratio = sample.sample_rate() as f64
                                        / self.output_sample_rate as f64;

                                    let mut hz_to_rate = (1.0 / base_freq) * sample_rate_ratio;
                                    if sample.finetune != 0 {
                                        hz_to_rate *= 2.0_f64.powf(sample.finetune as f64 / 1200.0);
                                    }

                                    let finetune = if let Some(ft_override) =
                                        self.effect_processor.finetune_override(ch)
                                    {
                                        ft_override
                                    } else {
                                        self.instruments
                                            .get(instrument_idx)
                                            .map(|inst| inst.finetune)
                                            .unwrap_or(0)
                                    };

                                    if finetune != 0 {
                                        hz_to_rate *= 2.0_f64.powf(finetune as f64 / (12.0 * 8.0));
                                    }

                                    let inst_vol = self
                                        .instruments
                                        .get(instrument_idx)
                                        .map(|inst| inst.volume)
                                        .unwrap_or(1.0);
                                    let velocity_gain = (100.0 / 127.0) * inst_vol * sample.volume;

                                    // Apply instrument panning override to the effect processor
                                    if let Some(inst_pan) =
                                        self.instruments.get(instrument_idx).and_then(|i| i.panning)
                                    {
                                        if let Some(state) =
                                            self.effect_processor.channel_state_mut(ch)
                                        {
                                            state.panning_override = Some((inst_pan + 1.0) / 2.0);
                                        }
                                    }

                                    if let Some(voice) = &mut self.voices[ch] {
                                        voice.instrument_index = instrument_idx;
                                        voice.sample_index = resolved_sample_idx;
                                        voice.velocity_gain = velocity_gain;
                                        voice.hz_to_rate = hz_to_rate;

                                        // Reset key_on and fadeout state for the new instrument trigger
                                        voice.key_on = true;
                                        voice.fadeout_multiplier = 1.0;

                                        voice.lfo = VoiceLfoState::new(
                                            self.instruments.get(instrument_idx),
                                        );
                                        // Reset envelopes for new instrument
                                        voice.volume_envelope_tick = 0;
                                        voice.panning_envelope_tick = 0;
                                        voice.pitch_envelope_tick = 0;
                                        voice.volume_adsr = AdsrState::default();
                                        voice.panning_adsr = AdsrState::default();
                                        voice.pitch_adsr = AdsrState::default();
                                    }

                                    // Update period clock for the channel if in Amiga mode
                                    let is_amiga = self
                                        .effect_processor
                                        .channel_state(ch)
                                        .map(|s| {
                                            s.slide_mode
                                                == crate::audio::pitch::SlideMode::AmigaPeriod
                                        })
                                        .unwrap_or(false);
                                    if is_amiga {
                                        let clock = if self.format_is_s3m {
                                            crate::audio::pitch::AMIGA_S3M_CLOCK
                                        } else {
                                            crate::audio::pitch::AMIGA_PAL_CLOCK
                                        };
                                        let period_clock =
                                            clock * base_freq / sample.sample_rate() as f64;
                                        self.effect_processor.set_period_clock(ch, period_clock);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        transport_commands
    }

    /// Render audio into a stereo interleaved f32 buffer.
    ///
    /// Mixes all active voices into the output buffer. Each frame consists
    /// of two samples (left, right). Mono samples are duplicated to both channels.
    ///
    /// # Arguments
    /// * `output` - Mutable slice of f32 samples to fill (stereo interleaved: L, R, L, R, ...)
    pub fn render(&mut self, output: &mut [f32]) {
        let num_frames = output.len() / 2;

        let output_sample_rate = self.output_sample_rate;

        // Wrap main voice rendering in a block so borrows are released before preview.
        {
            let channel_strips = &mut self.channel_strips;
            let bus_system = &mut self.bus_system;
            let effect_processor = &mut self.effect_processor;
            let voices = &mut self.voices;
            let samples = &self.samples;

            // Clear the buffer first
            for sample in output.iter_mut() {
                *sample = 0.0;
            }

            bus_system.clear_all(num_frames);
            let num_buses = bus_system.num_buses();

            for (ch, voice_slot) in voices.iter_mut().enumerate() {
                let strip = &mut channel_strips[ch];

                for frame in 0..num_frames {
                    // Check for pending note trigger for this channel at this frame
                    let mut triggered_now = false;
                    let current_row_frame = effect_processor
                        .channel_state(ch)
                        .unwrap()
                        .row_frame_counter;

                    if let Some(pos) = self
                        .pending_notes
                        .iter()
                        .position(|pn| pn.channel == ch && pn.trigger_frame == current_row_frame)
                    {
                        let pn = self.pending_notes.remove(pos);
                        effect_processor.set_period_clock(ch, pn.period_clock);
                        let mut voice = Voice::new(
                            self.instruments.get(pn.instrument_index),
                            pn.instrument_index,
                            pn.sample_index,
                            pn.playback_rate,
                            pn.velocity_gain,
                            pn.hz_to_rate,
                            pn.triggered_note_freq,
                        );
                        if let Some(offset) = pn.offset {
                            voice = voice.with_position(offset as f64);
                        }
                        if let Some(env_override) = effect_processor
                            .channel_state(ch)
                            .unwrap()
                            .envelope_position_override
                        {
                            voice.volume_envelope_tick = env_override;
                        }
                        *voice_slot = Some(voice);
                        triggered_now = true;
                    }

                    let voice = match voice_slot {
                        Some(v) if v.active => v,
                        _ => {
                            effect_processor.advance_frame(ch);
                            continue;
                        }
                    };

                    let render_state = effect_processor.voice_render_state(ch);
                    let ch_state = effect_processor.channel_state(ch).unwrap();

                    // Apply any envelope position overrides (Lxx) from the effect processor at row start
                    if ch_state.row_frame_counter == 0 {
                        if let Some(pos) = ch_state.envelope_position_override {
                            voice.volume_envelope_tick = pos;
                        }
                    }

                    // Sub-row timing logic
                    let frames_per_tick = ch_state.frames_per_row / ch_state.ticks_per_row as u32;
                    let current_tick = ch_state.row_frame_counter / frames_per_tick;
                    let tick_frame = ch_state.row_frame_counter % frames_per_tick;

                    // Note Cut (ECx)
                    if let Some(cut_tick) = ch_state.note_cut_tick {
                        if current_tick >= cut_tick as u32 {
                            voice.active = false;
                        }
                    }

                    // Retrigger (E9x)
                    if !triggered_now {
                        // Don't retrigger a note that just started
                        if let Some(retrigger_interval) = ch_state.retrigger_interval {
                            if retrigger_interval > 0
                                && current_tick > 0
                                && tick_frame == 0
                                && current_tick.is_multiple_of(retrigger_interval as u32)
                            {
                                voice.position = 0.0;
                                // Apply retrigger volume action
                                match ch_state.retrigger_volume_action {
                                    0 | 8 => {} // No change
                                    1 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 1.0 / 64.0).max(0.0)
                                    }
                                    2 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 2.0 / 64.0).max(0.0)
                                    }
                                    3 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 4.0 / 64.0).max(0.0)
                                    }
                                    4 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 8.0 / 64.0).max(0.0)
                                    }
                                    5 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 16.0 / 64.0).max(0.0)
                                    }
                                    6 => voice.velocity_gain *= 2.0 / 3.0,
                                    7 => voice.velocity_gain *= 0.5,
                                    9 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 1.0 / 64.0).min(1.0)
                                    }
                                    10 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 2.0 / 64.0).min(1.0)
                                    }
                                    11 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 4.0 / 64.0).min(1.0)
                                    }
                                    12 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 8.0 / 64.0).min(1.0)
                                    }
                                    13 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 16.0 / 64.0).min(1.0)
                                    }
                                    14 => voice.velocity_gain *= 3.0 / 2.0,
                                    15 => voice.velocity_gain *= 2.0,
                                    _ => {}
                                }
                            }
                        }
                    }

                    if !voice.active {
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    // Envelope and Modulation processing
                    let mut env_vol = 1.0;
                    let mut env_pan = 0.0; // Panning offset (-1.0 to 1.0)
                    let mut env_pitch = 0.0; // Pitch offset in semitones

                    if let Some(inst) = self.instruments.get(voice.instrument_index) {
                        // Update LFO phases
                        voice.lfo.update(inst, output_sample_rate, self.bpm);

                        // Fadeout processing
                        if !voice.key_on
                            && inst.fadeout > 0
                            && tick_frame == frames_per_tick.saturating_sub(1)
                        {
                            let delta = inst.fadeout as f32 / 65536.0;
                            voice.fadeout_multiplier = (voice.fadeout_multiplier - delta).max(0.0);
                            if voice.fadeout_multiplier <= 0.0001 {
                                voice.active = false;
                            }
                        }

                        // Volume Modulations
                        if let Some(adsr) = &inst.volume_adsr {
                            env_vol *=
                                voice
                                    .volume_adsr
                                    .update(adsr, voice.key_on, output_sample_rate);
                            if voice.volume_adsr.phase == AdsrPhase::Done {
                                voice.active = false;
                            }
                        }
                        if let Some(vol_env) = &inst.volume_envelope {
                            if vol_env.enabled {
                                let (val, next_tick) =
                                    vol_env.evaluate(voice.volume_envelope_tick, voice.key_on);
                                env_vol *= val;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.volume_envelope_tick = next_tick;
                                }
                                if !voice.key_on
                                    && vol_env.points.last().is_some_and(|p| {
                                        voice.volume_envelope_tick >= p.frame as usize
                                    })
                                    && val <= 0.001
                                {
                                    voice.active = false;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.volume_lfo {
                            env_vol *= (1.0 + voice.lfo.get_vol_value(lfo)).max(0.0);
                        }

                        // Panning Modulations
                        if let Some(adsr) = &inst.panning_adsr {
                            env_pan +=
                                voice
                                    .panning_adsr
                                    .update(adsr, voice.key_on, output_sample_rate)
                                    * 2.0
                                    - 1.0;
                        }
                        if let Some(pan_env) = &inst.panning_envelope {
                            if pan_env.enabled {
                                let (val, next_tick) =
                                    pan_env.evaluate(voice.panning_envelope_tick, voice.key_on);
                                env_pan += val * 2.0 - 1.0;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.panning_envelope_tick = next_tick;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.panning_lfo {
                            env_pan += voice.lfo.get_pan_value(lfo);
                        }

                        // Pitch Modulations
                        if let Some(adsr) = &inst.pitch_adsr {
                            env_pitch +=
                                (voice
                                    .pitch_adsr
                                    .update(adsr, voice.key_on, output_sample_rate)
                                    * 2.0
                                    - 1.0)
                                    * 12.0;
                        }
                        if let Some(pitch_env) = &inst.pitch_envelope {
                            if pitch_env.enabled {
                                let (val, next_tick) =
                                    pitch_env.evaluate(voice.pitch_envelope_tick, voice.key_on);
                                env_pitch += val * 12.0;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.pitch_envelope_tick = next_tick;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.pitch_lfo {
                            env_pitch += voice.lfo.get_pitch_value(lfo) * 12.0;
                        }
                    }

                    if !voice.active {
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    let sample = match samples.get(voice.sample_index) {
                        Some(s) => s,
                        None => {
                            voice.active = false;
                            effect_processor.advance_frame(ch);
                            continue;
                        }
                    };

                    let sample_data = sample.data();
                    let sample_channels = sample.channels() as usize;
                    let sample_frames = sample.frame_count();

                    if sample_frames == 0 {
                        voice.active = false;
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    let src_frame = voice.position as usize;

                    use crate::audio::sample::LoopMode;

                    // Determine effective loop mode, start, and end.
                    let (eff_loop_mode, eff_loop_start, eff_loop_end) = if voice.key_on
                        && sample.sustain_loop_mode != LoopMode::NoLoop
                        && sample.sustain_loop_end > sample.sustain_loop_start
                    {
                        (
                            sample.sustain_loop_mode,
                            sample.sustain_loop_start,
                            sample.sustain_loop_end,
                        )
                    } else {
                        (sample.loop_mode, sample.loop_start, sample.loop_end)
                    };

                    match eff_loop_mode {
                        LoopMode::NoLoop => {
                            if src_frame >= sample_frames {
                                voice.active = false;
                                effect_processor.advance_frame(ch);
                                continue;
                            }
                        }
                        LoopMode::Forward => {
                            if voice.position > eff_loop_end as f64 {
                                let loop_len = (eff_loop_end - eff_loop_start + 1) as f64;
                                let offset =
                                    (voice.position - eff_loop_start as f64).rem_euclid(loop_len);
                                voice.position = eff_loop_start as f64 + offset;
                            }
                        }
                        LoopMode::PingPong => {
                            if voice.loop_direction > 0.0 && src_frame > eff_loop_end {
                                voice.loop_direction = -1.0;
                                voice.position = eff_loop_end as f64;
                            } else if voice.loop_direction < 0.0 && src_frame < eff_loop_start {
                                voice.loop_direction = 1.0;
                                voice.position = eff_loop_start as f64;
                            }
                        }
                    }

                    // Final safety check for buffer access
                    let src_frame = voice.position as usize;
                    if src_frame >= sample_frames {
                        voice.active = false;
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    // Compute effective playback rate using the combined pitch ratio from effects and modulation.
                    let pitch_mod_ratio = 2.0f64.powf(env_pitch as f64 / 12.0);
                    let effective_rate = voice.triggered_note_freq
                        * voice.hz_to_rate
                        * render_state.pitch_ratio
                        * pitch_mod_ratio
                        * voice.loop_direction;

                    // Read sample data with linear interpolation
                    let (left, right) = {
                        let pos_floor = voice.position.floor() as usize;
                        let frac = (voice.position - pos_floor as f64) as f32;
                        let next_frame = {
                            let next = pos_floor + 1;
                            match eff_loop_mode {
                                LoopMode::NoLoop => next,
                                LoopMode::Forward => {
                                    if next > eff_loop_end {
                                        eff_loop_start
                                    } else {
                                        next
                                    }
                                }
                                LoopMode::PingPong => {
                                    if voice.loop_direction > 0.0 {
                                        if next > eff_loop_end {
                                            eff_loop_end
                                        } else {
                                            next
                                        }
                                    } else if pos_floor > eff_loop_start {
                                        pos_floor - 1
                                    } else {
                                        eff_loop_start
                                    }
                                }
                            }
                        };

                        let get_stereo = |f: usize| {
                            if f >= sample_frames {
                                (0.0, 0.0)
                            } else if sample_channels >= 2 {
                                let idx = f * sample_channels;
                                (sample_data[idx], sample_data[idx + 1])
                            } else {
                                (sample_data[f], sample_data[f])
                            }
                        };

                        let (l1, r1) = get_stereo(pos_floor);
                        let (l2, r2) = get_stereo(next_frame);

                        (l1 + (l2 - l1) * frac, r1 + (r2 - r1) * frac)
                    };

                    let combined_channel_gain =
                        render_state.gain.unwrap_or(render_state.channel_volume);

                    let (left_gain, right_gain) = strip.next_gains_modulated(
                        env_vol * combined_channel_gain,
                        env_pan,
                        render_state.pan_override.map(|p| p * 2.0 - 1.0),
                    );

                    let out_idx = frame * 2;
                    let global_vol_mult = effect_processor.global_volume;
                    let post_l = left
                        * voice.velocity_gain
                        * left_gain
                        * global_vol_mult
                        * voice.fadeout_multiplier;
                    let post_r = right
                        * voice.velocity_gain
                        * right_gain
                        * global_vol_mult
                        * voice.fadeout_multiplier;

                    output[out_idx] += post_l;
                    output[out_idx + 1] += post_r;

                    let (peak_l, peak_r) = &self.channel_levels[ch];
                    atomic_max_f32(peak_l, post_l.abs());
                    atomic_max_f32(peak_r, post_r.abs());

                    // Write mono mix to oscilloscope ring buffer
                    if let (Some(buf), Some(pos_atomic)) = (
                        self.oscilloscope_bufs.get_mut(ch),
                        self.oscilloscope_write_pos.get(ch),
                    ) {
                        let pos =
                            pos_atomic.load(Ordering::Relaxed) as usize % OSCILLOSCOPE_BUF_SIZE;
                        buf[pos] = (post_l + post_r) * 0.5;
                        pos_atomic.store(
                            ((pos + 1) % OSCILLOSCOPE_BUF_SIZE) as u32,
                            Ordering::Relaxed,
                        );
                    }

                    for bus_idx in 0..num_buses {
                        let send_level = strip.next_send_level(bus_idx);
                        if send_level > 0.0001 {
                            bus_system.accumulate(bus_idx, frame, post_l, post_r, send_level);
                        }
                    }

                    voice.position += effective_rate;

                    // Advance frame-level effect modulations
                    effect_processor.advance_frame(ch);
                }
            }

            effect_processor.advance_global_frame();
            bus_system.process_and_mix(output, num_frames);
        } // end main voice block — field borrows released

        // Preview voice: renders a one-shot sample directly into output,
        // bypassing channel strips (no mute/solo/pan, preview volume = 0.7).
        let preview_done = if let Some(ref pv) = self.preview_sample {
            let pv_rate = self.preview_rate;
            let pv_frames = pv.frame_count();
            let pv_channels = pv.channels() as usize;
            let pv_data = pv.data();
            let mut done = false;
            for frame in 0..num_frames {
                let pos = self.preview_pos as usize;
                if pos >= pv_frames {
                    done = true;
                    break;
                }
                let l = pv_data[pos * pv_channels];
                let r = if pv_channels > 1 {
                    pv_data[pos * pv_channels + 1]
                } else {
                    l
                };
                output[frame * 2] += l * 0.7;
                output[frame * 2 + 1] += r * 0.7;
                self.preview_pos += pv_rate;
            }
            done
        } else {
            false
        };
        if preview_done {
            self.preview_sample = None;
        }

        // Write mono mix to FFT capture buffer
        for frame in 0..num_frames {
            let mono = (output[frame * 2] + output[frame * 2 + 1]) * 0.5;
            let pos = self.fft_write_pos.load(Ordering::Relaxed) as usize % FFT_BUF_SIZE;
            self.fft_buf[pos] = mono;
            self.fft_write_pos
                .store(((pos + 1) % FFT_BUF_SIZE) as u32, Ordering::Relaxed);
        }

        // Clamp output to [-1.0, 1.0] to prevent clipping distortion
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

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
        self.fft_buf.fill(0.0);
        self.fft_write_pos.store(0, Ordering::Relaxed);
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

    /// Reset all channel levels to zero.
    pub fn reset_channel_levels(&mut self) {
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
            // Use atomic store to reset position
            pos.store(0, Ordering::Relaxed);
        }
    }

    /// Read the oscilloscope waveform for a channel.
    /// Returns a slice of the ring buffer in chronological order (oldest first).
    /// The returned Vec has exactly `OSCILLOSCOPE_BUF_SIZE` samples.
    pub fn oscilloscope_data(&self, channel: usize) -> Vec<f32> {
        if let (Some(buf), Some(pos_atomic)) = (
            self.oscilloscope_bufs.get(channel),
            self.oscilloscope_write_pos.get(channel),
        ) {
            let write_pos = pos_atomic.load(Ordering::Relaxed) as usize % OSCILLOSCOPE_BUF_SIZE;
            let mut result = Vec::with_capacity(OSCILLOSCOPE_BUF_SIZE);
            // Read from write_pos (oldest) wrapping around to write_pos-1 (newest)
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
    /// Called from the UI update loop for visual smoothing.
    pub fn decay_channel_levels(&mut self, decay_factor: f32) {
        for (l, r) in &self.channel_levels {
            let current_l = u32_bits_to_f32(l.load(Ordering::Relaxed));
            let current_r = u32_bits_to_f32(r.load(Ordering::Relaxed));
            let decayed_l = current_l * decay_factor;
            let decayed_r = current_r * decay_factor;
            l.store(f32_to_u32_bits(decayed_l), Ordering::Relaxed);
            r.store(f32_to_u32_bits(decayed_r), Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::effect::{Effect, EffectType};
    use crate::pattern::note::{Note, NoteEvent, Pitch};
    use crate::pattern::row::Cell;
    use crate::song::{Lfo, LfoWaveform};

    /// Create a simple sine wave sample at 440Hz for testing.
    /// Base note is set to A-4 (MIDI 57) to match the 440Hz content.
    fn make_test_sample(sample_rate: u32, duration_secs: f32) -> Sample {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut data = Vec::with_capacity(num_samples);
        let freq = 440.0;
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            data.push((2.0 * std::f32::consts::PI * freq * t).sin());
        }
        Sample::new(data, sample_rate, 1, Some("sine440".to_string())).with_base_note(57)
        // A-4
    }

    #[test]
    fn test_mixer_tpl_change_affects_timing() {
        let data = vec![1.0f32; 100000];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);

        mixer.update_tempo(120.0);
        mixer.set_tpl(6);
        mixer.tick(0, &crate::pattern::pattern::Pattern::new(1, 1));

        let state_tpl6 = mixer
            .effect_processor()
            .channel_state(0)
            .unwrap()
            .ticks_per_row;

        mixer.set_tpl(12);
        mixer.tick(0, &crate::pattern::pattern::Pattern::new(1, 1));

        let state_tpl12 = mixer
            .effect_processor()
            .channel_state(0)
            .unwrap()
            .ticks_per_row;

        assert_eq!(state_tpl6, 6, "Initial TPL should be 6");
        assert_eq!(state_tpl12, 12, "TPL change to 12 should be reflected");
    }

    #[test]
    fn test_mixer_tick_triggers_voice() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        let note = Note::new(Pitch::A, 4, 100, 0);
        pattern.set_note(0, 0, note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_tick_note_off() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        pattern.set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_empty_row_continues() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Empty row — voice should continue
        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_tick_out_of_bounds_row() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let pattern = Pattern::new(16, 4);

        // Should not panic on out-of-bounds row
        mixer.tick(100, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_invalid_instrument() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Instrument index 5 doesn't exist (only one sample loaded)
        let note = Note::new(Pitch::A, 4, 100, 5);
        pattern.set_note(0, 0, note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_render_silence_when_no_voices() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut output = vec![0.0f32; 512];

        mixer.render(&mut output);

        // Output should be all zeros
        assert!(output.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_mixer_render_produces_audio() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // Output should contain non-zero samples
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Render should produce non-zero audio data");
    }

    #[test]
    fn test_mixer_render_velocity_scaling() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut mixer_loud = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut mixer_quiet = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);

        let mut pattern_loud = Pattern::new(16, 4);
        pattern_loud.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        let mut pattern_quiet = Pattern::new(16, 4);
        pattern_quiet.set_note(0, 0, Note::new(Pitch::A, 4, 32, 0));

        mixer_loud.tick(0, &pattern_loud);
        mixer_quiet.tick(0, &pattern_quiet);

        let mut output_loud = vec![0.0f32; 512];
        let mut output_quiet = vec![0.0f32; 512];

        mixer_loud.render(&mut output_loud);
        mixer_quiet.render(&mut output_quiet);

        // Loud output should have higher peak amplitude
        let peak_loud: f32 = output_loud.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_quiet: f32 = output_quiet.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_loud > peak_quiet,
            "Loud peak ({}) should exceed quiet peak ({})",
            peak_loud,
            peak_quiet
        );
    }

    #[test]
    fn test_mixer_render_multiple_voices() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_note(0, 1, Note::new(Pitch::E, 4, 100, 0));
        pattern.set_note(0, 2, Note::new(Pitch::G, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 3);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Multiple voices should produce audio");
    }

    #[test]
    fn test_mixer_render_clamping() {
        // Create a loud sample
        let num_samples = 4410;
        let data: Vec<f32> = vec![1.0; num_samples];
        let sample = Sample::new(data, 44100, 1, Some("loud".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Trigger on all 4 channels at max velocity
        for ch in 0..4 {
            pattern.set_note(0, ch, Note::new(Pitch::A, 4, 127, 0));
        }

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // All samples should be clamped to [-1.0, 1.0]
        assert!(
            output.iter().all(|&s| (-1.0..=1.0).contains(&s)),
            "Output should be clamped to [-1.0, 1.0]"
        );
    }

    #[test]
    fn test_mixer_stop_all() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        mixer.stop_all();
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_sample_and_instrument_volume() {
        let sample = Arc::new(make_test_sample(44100, 0.25).with_volume(0.5));
        let mut inst = Instrument::new("Test").with_volume(0.8);
        inst.sample_index = Some(0);
        let instruments = vec![inst];
        let mut mixer = Mixer::new(vec![sample], instruments, 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // velocity 127 (1.0), inst volume 0.8, sample volume 0.5
        // expected final gain = 1.0 * 0.8 * 0.5 = 0.4
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        if let Some(voice) = &mixer.voices[0] {
            assert!((voice.velocity_gain - 0.4).abs() < 0.001);
        } else {
            panic!("Voice should be triggered");
        }
    }

    #[test]
    fn test_mixer_voice_ends_at_sample_boundary() {
        // Very short sample (10 frames at 44100Hz)
        let data: Vec<f32> = vec![0.5; 10];
        let sample = Sample::new(data, 44100, 1, Some("short".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render more frames than the sample contains — voice should deactivate
        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        assert_eq!(
            mixer.active_voice_count(),
            0,
            "Voice should deactivate after sample ends"
        );
    }

    #[test]
    fn test_mixer_zero_velocity() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 0, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // Zero velocity should produce silence
        assert!(
            output.iter().all(|&s| s == 0.0),
            "Zero velocity should produce silence"
        );
    }

    #[test]
    fn test_mixer_stereo_sample() {
        // Stereo sample: L=0.5, R=-0.5 repeated
        let mut data = Vec::new();
        for _ in 0..100 {
            data.push(0.5); // Left
            data.push(-0.5); // Right
        }
        let sample = Sample::new(data, 44100, 2, Some("stereo".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 20]; // 10 stereo frames
        mixer.render(&mut output);

        // Left and right channels should have different signs
        // (at least for the first frame where position is 0)
        let left = output[0];
        let right = output[1];
        assert!(left > 0.0, "Left channel should be positive, got {}", left);
        assert!(
            right < 0.0,
            "Right channel should be negative, got {}",
            right
        );
    }

    #[test]
    fn test_mixer_c4_plays_at_original_rate() {
        // A sample with default base_note C-4: playing C-4 should give playback_rate ~1.0
        // (when sample rate matches output rate)
        let data: Vec<f32> = vec![0.5; 4410];
        let sample = Sample::new(data, 44100, 1, Some("test".to_string()));
        assert_eq!(sample.base_note(), 48); // C-4 default

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut pattern = Pattern::new(16, 4);
        // C-4 note should play at original rate
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render and verify audio is produced
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "C-4 on C-4-based sample should produce audio");
    }

    #[test]
    fn test_mixer_higher_note_plays_faster() {
        // Higher notes should consume the sample faster (higher playback rate)
        let data: Vec<f32> = (0..4410).map(|i| i as f32 / 4410.0).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut mixer_low = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut mixer_high = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);

        let mut pattern_low = Pattern::new(16, 4);
        pattern_low.set_note(0, 0, Note::new(Pitch::C, 3, 100, 0)); // C-3: one octave below base

        let mut pattern_high = Pattern::new(16, 4);
        pattern_high.set_note(0, 0, Note::new(Pitch::C, 5, 100, 0)); // C-5: one octave above base

        mixer_low.tick(0, &pattern_low);
        mixer_high.tick(0, &pattern_high);

        // Render same number of frames
        let mut output_low = vec![0.0f32; 512];
        let mut output_high = vec![0.0f32; 512];
        mixer_low.render(&mut output_low);
        mixer_high.render(&mut output_high);

        // The high-pitched version should have progressed further through the ramp sample,
        // producing higher average values in the output (since the ramp goes 0→1)
        let avg_low: f32 =
            output_low.iter().map(|s| s.abs()).sum::<f32>() / output_low.len() as f32;
        let avg_high: f32 =
            output_high.iter().map(|s| s.abs()).sum::<f32>() / output_high.len() as f32;
        assert!(
            avg_high > avg_low,
            "Higher note should progress faster through sample (avg_high={} > avg_low={})",
            avg_high,
            avg_low
        );
    }

    #[test]
    fn test_mixer_custom_base_note() {
        // Sample with base_note set to A-4 (57): playing A-4 should be original rate
        let data: Vec<f32> = vec![0.8; 4410];
        let sample = Sample::new(data, 44100, 1, Some("a4_sample".to_string())).with_base_note(57); // A-4

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);

        // At original rate (1.0), each frame reads consecutive samples
        // All samples are 0.8, so output should be ~0.8 * (100/127) * pan_gain
        // Default center pan with equal-power law: gain = 1/√2 ≈ 0.707
        let velocity_gain = 100.0 / 127.0;
        // Center pan gain is cos(45 deg)
        let center_pan_gain = std::f32::consts::FRAC_PI_4.cos();
        let expected_val = 0.8 * velocity_gain * center_pan_gain;
        assert!(
            (output[0] - expected_val).abs() < 0.01,
            "A-4 on A-4-based sample should play at original rate, got {} expected ~{}",
            output[0],
            expected_val
        );
    }

    #[test]
    fn test_mixer_instrument_lookup_by_index() {
        // Create two distinct samples and verify instrument index selects the right one
        let sample_a = Sample::new(vec![0.3; 4410], 44100, 1, Some("A".to_string()));
        let sample_b = Sample::new(vec![0.9; 4410], 44100, 1, Some("B".to_string()));

        let mut mixer = Mixer::new(
            vec![Arc::new(sample_a), Arc::new(sample_b)],
            Vec::new(),
            4,
            44100,
        );

        let mut pattern = Pattern::new(16, 4);
        // Channel 0: instrument 0 (quieter sample)
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        // Channel 1: instrument 1 (louder sample)
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 127, 1));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        let mut output_a = vec![0.0f32; 64];
        let mut output_b = vec![0.0f32; 64];

        // Render with only instrument 0
        let mut mixer_a = Mixer::new(
            vec![Arc::new(Sample::new(vec![0.3; 4410], 44100, 1, None))],
            Vec::new(),
            4,
            44100,
        );
        let mut pat_a = Pattern::new(16, 4);
        pat_a.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer_a.tick(0, &pat_a);
        mixer_a.render(&mut output_a);

        // Render with only instrument 1 (louder)
        let mut mixer_b = Mixer::new(
            vec![
                Arc::new(Sample::new(vec![0.0; 1], 44100, 1, None)),
                Arc::new(Sample::new(vec![0.9; 4410], 44100, 1, None)),
            ],
            Vec::new(),
            4,
            44100,
        );
        let mut pat_b = Pattern::new(16, 4);
        pat_b.set_note(0, 0, Note::new(Pitch::C, 4, 127, 1));
        mixer_b.tick(0, &pat_b);
        mixer_b.render(&mut output_b);

        let peak_a: f32 = output_a.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_b: f32 = output_b.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_b > peak_a,
            "Instrument 1 (0.9) should be louder than instrument 0 (0.3): {} vs {}",
            peak_b,
            peak_a
        );
    }

    #[test]
    fn test_mixer_note_off_stops_sample() {
        let sample = Sample::new(vec![0.5; 44100], 44100, 1, None); // 1 second sample
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        // Trigger note
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render some audio to confirm it's playing
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(
            output.iter().any(|&s| s != 0.0),
            "Voice should be producing audio"
        );

        // Note off
        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);

        // Render after note-off should be silent
        let mut output2 = vec![0.0f32; 64];
        mixer.render(&mut output2);
        assert!(
            output2.iter().all(|&s| s == 0.0),
            "After note-off, output should be silent"
        );
    }

    #[test]
    fn test_sample_base_frequency() {
        // C-4 default base note should give ~261.63 Hz
        let sample = Sample::new(vec![], 44100, 1, None);
        assert!((sample.base_frequency() - 261.63).abs() < 0.1);

        // A-4 base note should give 440 Hz
        let sample_a4 = Sample::new(vec![], 44100, 1, None).with_base_note(57);
        assert!((sample_a4.base_frequency() - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_mixer_empty_sample_deactivates() {
        let sample = Sample::new(vec![], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        // Voice was created but sample is empty

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        assert_eq!(
            mixer.active_voice_count(),
            0,
            "Empty sample should deactivate voice"
        );
    }

    #[test]
    fn test_mixer_add_sample() {
        let sample1 = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample1)], Vec::new(), 4, 44100);
        assert_eq!(mixer.sample_count(), 1);

        let sample2 = make_test_sample(44100, 0.5);
        let idx = mixer.add_sample(Arc::new(sample2));
        assert_eq!(idx, 1);
        assert_eq!(mixer.sample_count(), 2);
    }

    #[test]
    fn test_mixer_sample_name() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        assert_eq!(mixer.sample_name(0), Some("sine440"));
        assert_eq!(mixer.sample_name(1), None);
    }

    #[test]
    fn test_mixer_add_sample_playback() {
        let sample1 = make_test_sample(44100, 0.25);
        let sample2 = Sample::new(vec![0.8; 4410], 44100, 1, Some("loud".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample1)], Vec::new(), 4, 44100);
        let idx = mixer.add_sample(Arc::new(sample2));

        let mut pattern = Pattern::new(16, 4);
        // Use the newly added sample (instrument 1)
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, idx as u8));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Added sample should produce audio");
    }

    // --- Multi-track mixing tests ---

    #[test]
    fn test_mixer_muted_channel_produces_silence() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));
        // Mute channel 0
        pattern.get_track_mut(0).unwrap().toggle_mute();

        mixer.tick(0, &pattern);
        // Muted channels should not trigger voices
        assert_eq!(mixer.active_voice_count(), 0);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(
            output.iter().all(|&s| s == 0.0),
            "Muted channel should produce silence"
        );
    }

    #[test]
    fn test_mixer_solo_filters_non_soloed() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 127, 0));
        // Solo channel 1 only
        pattern.get_track_mut(1).unwrap().toggle_solo();

        mixer.tick(0, &pattern);
        // Channel 0 not soloed → no voice; channel 1 soloed → voice
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_track_volume_applied() {
        let sample = Arc::new(Sample::new(
            vec![1.0; 4410],
            44100,
            1,
            Some("test".to_string()),
        ));

        // Full volume
        let mut mixer_full = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut pattern_full = Pattern::new(16, 4);
        pattern_full.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer_full.tick(0, &pattern_full);

        // Half volume
        let mut mixer_half = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut pattern_half = Pattern::new(16, 4);
        pattern_half.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern_half.get_track_mut(0).unwrap().set_volume(0.5);
        mixer_half.tick(0, &pattern_half);

        let mut output_full = vec![0.0f32; 64];
        let mut output_half = vec![0.0f32; 64];
        for _ in 0..10 {
            mixer_full.render(&mut output_full);
            mixer_half.render(&mut output_half);
        }

        let peak_full: f32 = output_full.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_half: f32 = output_half.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_full > peak_half,
            "Full volume ({}) should be louder than half volume ({})",
            peak_full,
            peak_half
        );
        // Half volume should be roughly half the peak (within pan law)
        let ratio = peak_half / peak_full;
        assert!(
            (ratio - 0.5).abs() < 0.1,
            "Half volume ratio should be ~0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_mixer_pan_left_only_left_channel() {
        let sample = Sample::new(vec![1.0; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern.get_track_mut(0).unwrap().set_pan(-1.0); // Full left

        mixer.update_tracks(pattern.tracks());
        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        // Render enough to finish the 0.005s ramp (approx 220 samples at 44.1kHz)
        for _ in 0..100 {
            mixer.render(&mut output);
        }

        // Check that right channel is silent
        let right_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);
        let left_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);

        assert!(left_peak > 0.0, "Left channel should have audio");
        assert!(
            right_peak < 0.001,
            "Right channel should be silent with full-left pan, got {}",
            right_peak
        );
    }

    #[test]
    fn test_mixer_pan_right_only_right_channel() {
        let sample = Sample::new(vec![1.0; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern.get_track_mut(0).unwrap().set_pan(1.0); // Full right

        mixer.update_tracks(pattern.tracks());
        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        // Render enough to finish the 0.005s ramp
        for _ in 0..100 {
            mixer.render(&mut output);
        }

        let left_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);
        let right_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);

        assert!(right_peak > 0.0, "Right channel should have audio");
        assert!(
            left_peak < 0.001,
            "Left channel should be silent with full-right pan, got {}",
            left_peak
        );
    }

    #[test]
    fn test_mixer_update_tracks_syncs_state() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut tracks = vec![
            Track::new("Kick"),
            Track::new("Snare"),
            Track::new("Hat"),
            Track::new("Bass"),
        ];
        tracks[0].set_volume(0.5);
        tracks[0].set_pan(-0.5);
        tracks[1].muted = true;
        tracks[2].solo = true;

        mixer.update_tracks(&tracks);

        assert!(mixer.is_channel_silent(0));
        assert!(mixer.is_channel_silent(1));
        assert!(!mixer.is_channel_silent(2));
        assert!(mixer.is_channel_silent(3));
    }

    #[test]
    fn test_mixer_muted_voice_still_advances_position() {
        let sample = Sample::new(vec![0.5; 4410], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        // Start with unmuted to trigger voice
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        pattern.get_track_mut(0).unwrap().toggle_mute();
        mixer.update_tracks(pattern.tracks());

        let mut output = vec![0.0f32; 512];
        for _ in 0..4 {
            mixer.render(&mut output);
        }

        let peak: f32 = output.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(peak < 0.0001, "Muted render should be effectively silent");
    }

    #[test]
    fn test_mixer_multi_track_independent_mix() {
        let sample = Sample::new(vec![0.2; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Channel 0: full volume, full right
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern.get_track_mut(0).unwrap().set_pan(1.0);
        // Channel 1: full volume, full left
        pattern.set_note(0, 1, Note::new(Pitch::E, 4, 127, 0));
        pattern.get_track_mut(1).unwrap().set_pan(-1.0);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        let mut output = vec![0.0f32; 64];
        // Render enough to finish potential ramps
        for _ in 0..100 {
            mixer.render(&mut output);
        }

        let left: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);
        let right: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);

        // Since both have same volume and sample value, they should be roughly equal but present
        assert!(left > 0.0);
        assert!(right > 0.0);
        assert!((left - right).abs() < 0.01);
    }

    #[test]
    fn test_mixer_forward_loop() {
        use crate::audio::sample::LoopMode;
        // 10 frame sample, loop frames 5-9
        let data = vec![0.5; 10];
        let sample = Sample::new(data, 44100, 1, None).with_loop(LoopMode::Forward, 5, 9);

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render 20 frames (more than sample length)
        let mut output = vec![0.0f32; 40];
        mixer.render(&mut output);

        // Voice should still be active because it's looping
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_subframe_interpolation() {
        // Sample data: 0.0, 1.0 (at frames 0 and 1)
        let data: Vec<f32> = vec![0.0, 1.0];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);

        // Manually set position to 0.5 — without interpolation we'd get 0.0 or 1.0
        // with linear interpolation we should get 0.5
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 0.5;
        }

        let mut output = vec![0.0f32; 2];
        mixer.render(&mut output);

        // Center pan gain is ~0.707 (cos(pi/4))
        let expected = 0.5 * std::f32::consts::FRAC_PI_4.cos();
        assert!(
            (output[0] - expected).abs() < 0.01,
            "Expected ~{}, got {}",
            expected,
            output[0]
        );
    }

    #[test]
    fn test_mixer_forward_loop_boundary() {
        use crate::audio::sample::LoopMode;
        // 10 frame sample, loop frames 5-9 (end inclusive)
        // Values: 0.0, 0.1, 0.2, ..., 0.9
        let data: Vec<f32> = (0..10).map(|i| i as f32 / 10.0).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None).with_loop(LoopMode::Forward, 5, 9));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);

        // Position just at the end of loop
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 9.5;
        }

        let mut output = vec![0.0f32; 2];
        mixer.render(&mut output);

        // Interpolation between 0.9 (end) and 0.5 (start)
        // pos 9.5: l1=0.9, l2=0.5, frac=0.5 => 0.9 + (0.5-0.9)*0.5 = 0.7
        let expected = 0.7 * 0.70710677;
        assert!(
            (output[0] - expected).abs() < 0.01,
            "Expected ~{}, got {}",
            expected,
            output[0]
        );
    }

    #[test]
    fn test_mixer_pingpong_loop_boundary() {
        use crate::audio::sample::LoopMode;
        // Values: 0.0, 0.1, 0.2, ..., 0.9
        let data: Vec<f32> = (0..10).map(|i| i as f32 / 10.0).collect();
        let sample =
            Arc::new(Sample::new(data, 44100, 1, None).with_loop(LoopMode::PingPong, 5, 9));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);

        // Position just at the end of loop, moving forward
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 8.5;
            voice.loop_direction = 1.0;
        }

        // Render 3 frames (6 samples)
        let mut output = vec![0.0f32; 6];
        mixer.render(&mut output);

        // Frame 1: pos 8.5. No reversal. pos -> 9.5
        // Frame 2: pos 9.5. No reversal (9 > 9 is false). pos -> 10.5
        // Frame 3: pos 10.5. src_frame = 10. 10 > 9 is true. REVERSAL.
        //          loop_dir -> -1.0. pos -> 9.0.
        //          Then it renders pos 9.0. pos -> 9.0 + (-1.0) = 8.0.

        let voice = mixer.voices[0].as_ref().unwrap();
        assert_eq!(voice.loop_direction, -1.0);
        assert_eq!(voice.position, 8.0);
    }

    // --- Preview toggle & scrub ---

    #[test]
    fn test_is_preview_playing_false_initially() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        assert!(!mixer.is_preview_playing());
    }

    #[test]
    fn test_is_preview_playing_true_after_trigger() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        mixer.trigger_preview(Arc::clone(&sample), 1.0);
        assert!(mixer.is_preview_playing());
    }

    #[test]
    fn test_is_preview_playing_false_after_stop() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        mixer.trigger_preview(Arc::clone(&sample), 1.0);
        mixer.stop_preview();
        assert!(!mixer.is_preview_playing());
    }

    #[test]
    fn test_trigger_preview_at_sets_offset() {
        let sample = Arc::new(make_test_sample(44100, 1.0));
        let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        // Start 0.1s (4410 frames) into the sample
        mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 4410);
        assert!(mixer.is_preview_playing());
        // Render a small buffer — should not panic, preview starts mid-sample
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(
            mixer.is_preview_playing(),
            "still playing after small render"
        );
    }

    #[test]
    fn test_trigger_preview_at_zero_same_as_trigger_preview() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 0);
        assert!(mixer.is_preview_playing());
    }

    #[test]
    fn test_preview_pos_and_total_no_preview() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let (pos, total) = mixer.preview_pos_and_total();
        assert_eq!(pos, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_preview_pos_and_total_after_trigger_at() {
        let sample = Arc::new(make_test_sample(44100, 1.0)); // 44100 frames
        let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 4410);
        let (pos, total) = mixer.preview_pos_and_total();
        assert_eq!(pos, 4410);
        assert_eq!(total, sample.frame_count());
    }

    #[test]
    fn test_mixer_note_delay_edx() {
        let data = vec![1.0f32; 100];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        mixer.update_tempo(120.0); // Sync tempo for 44100Hz
        let mut pattern = Pattern::new(16, 1);

        // ED3: Delay by 3 ticks. Default 6 ticks per row.
        // 120 BPM => 125ms per row. 6 ticks => 20.83ms per tick.
        // 3 ticks => 62.5ms. At 44100Hz => 2756.25 frames.
        // trigger_frame = 3 * (5512 / 6) = 3 * 918 = 2754.
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::Extended, 0xD3));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);

        // Voice should NOT be active initially
        assert!(mixer.voices[0].is_none());
        assert_eq!(mixer.pending_notes.len(), 1);

        // Render some frames (less than 3 ticks)
        let mut output = vec![0.0f32; 2000 * 2];
        mixer.render(&mut output);
        assert!(output.iter().all(|&s| s == 0.0));
        assert!(mixer.voices[0].is_none());

        // Render more frames to pass the trigger point (2754)
        let mut output2 = vec![0.0f32; 1000 * 2];
        mixer.render(&mut output2);

        // Voice should now be active
        assert!(mixer.voices[0].is_some());
        // And we should have some audio in output2
        assert!(output2.iter().any(|&s| s > 0.0));
    }

    #[test]
    fn test_mixer_note_cut_ecx() {
        let data = vec![1.0f32; 10000];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        mixer.update_tempo(120.0);
        let mut pattern = Pattern::new(16, 1);

        // EC2: Cut after 2 ticks. 2 ticks = 2 * 918 = 1836 frames.
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::Extended, 0xC2));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);
        assert!(mixer.voices[0].is_some());

        // Render 1000 frames (less than 2 ticks)
        let mut output = vec![0.0f32; 1000 * 2];
        mixer.render(&mut output);
        assert!(output.iter().any(|&s| s > 0.0));
        assert!(mixer.voices[0].as_ref().unwrap().active);

        // Render more frames to pass the cut point (1836)
        let mut output2 = vec![0.0f32; 2000 * 2];
        mixer.render(&mut output2);

        // Voice should now be inactive
        assert!(!mixer.voices[0].as_ref().unwrap().active);
    }

    #[test]
    fn test_mixer_tremor_effect() {
        let data = vec![1.0f32; 100000];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        mixer.update_tempo(60.0);
        let mut pattern = Pattern::new(16, 1);

        // Txy: Tremor - ON for x ticks, OFF for y ticks
        // T31: 3 ticks on, 1 tick off
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::Tremor, 0x31));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);

        // Verify tremor state is set
        let state = mixer.effect_processor().channel_state(0).unwrap();
        assert!(state.tremor_active, "Tremor should be active");
        assert_eq!(state.tremor_on, 3, "Tremor ON should be 3 ticks");
        assert_eq!(state.tremor_off, 1, "Tremor OFF should be 1 tick");
    }

    // --- VU Meter Tests ---

    #[test]
    fn test_mixer_channel_levels_initialized_to_zero() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        for ch in 0..4 {
            let (l, r) = mixer.get_channel_level(ch);
            assert_eq!(l, 0.0, "Channel {} left should be 0 initially", ch);
            assert_eq!(r, 0.0, "Channel {} right should be 0 initially", ch);
        }
    }

    #[test]
    fn test_mixer_channel_levels_invalid_channel() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let (l, r) = mixer.get_channel_level(99);
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn test_mixer_render_tracks_peak_levels() {
        let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        let (l, r) = mixer.get_channel_level(0);
        assert!(l > 0.0, "Channel 0 left peak should be > 0, got {}", l);
        assert!(r > 0.0, "Channel 0 right peak should be > 0, got {}", r);
    }

    #[test]
    fn test_mixer_render_peak_levels_accumulate() {
        let sample = Sample::new(vec![0.5f32; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 256];
        mixer.render(&mut output);
        let (l1, _) = mixer.get_channel_level(0);

        mixer.render(&mut output);
        let (l2, _) = mixer.get_channel_level(0);

        assert_eq!(
            l1, l2,
            "Peak should remain the same across renders without new audio"
        );
    }

    #[test]
    fn test_mixer_reset_channel_levels() {
        let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        let (l, _r) = mixer.get_channel_level(0);
        assert!(l > 0.0);

        mixer.reset_channel_levels();

        let (l, r) = mixer.get_channel_level(0);
        assert_eq!(l, 0.0, "Left peak should be reset to 0");
        assert_eq!(r, 0.0, "Right peak should be reset to 0");
    }

    #[test]
    fn test_mixer_decay_channel_levels() {
        let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        let (l, _) = mixer.get_channel_level(0);
        assert!(l > 0.0);

        mixer.decay_channel_levels(0.5);

        let (l2, _) = mixer.get_channel_level(0);
        assert!(
            (l2 - l * 0.5).abs() < 0.001,
            "Peak should decay to 50%, got {} expected ~{}",
            l2,
            l * 0.5
        );
    }

    #[test]
    fn test_mixer_decay_to_zero() {
        let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // 20 iterations of 0.9 decay = 0.9^20 ≈ 0.12 of original
        for _ in 0..20 {
            mixer.decay_channel_levels(0.9);
        }

        let (l, _) = mixer.get_channel_level(0);
        assert!(l < 0.1, "Peak should decay to less than 10%, got {}", l);
    }

    // --- Effect Processing Edge Cases ---

    #[test]
    fn test_mixer_arpeggio_effect_changes_pitch() {
        let data: Vec<f32> = (0..44100).map(|i| i as f32 / 100.0).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None).with_base_note(48));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        mixer.update_tempo(60.0);
        let mut pattern = Pattern::new(16, 1);

        // 0xy: arpeggio with x=4 (major third), y=7 (perfect fifth)
        // C-4 → C-4 → E-4 → G-4 cycles
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::Arpeggio, 0x47));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);

        // First third: base pitch
        let mut output1 = vec![0.0f32; 500];
        mixer.render(&mut output1);
        let peak1: f32 = output1.iter().map(|s| s.abs()).fold(0.0, f32::max);

        // Second third: +4 semitones (C to E)
        let mut output2 = vec![0.0f32; 500];
        mixer.render(&mut output2);
        let peak2: f32 = output2.iter().map(|s| s.abs()).fold(0.0, f32::max);

        // Third third: +7 semitones (C to G)
        let mut output3 = vec![0.0f32; 500];
        mixer.render(&mut output3);
        let peak3: f32 = output3.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak1 > 0.0 && peak2 > 0.0 && peak3 > 0.0,
            "All arpeggio phases should produce audio"
        );
    }

    #[test]
    fn test_mixer_portamento_slide_changes_pitch() {
        let data: Vec<f32> = (0..44100).map(|i| (i as f32 / 100.0).sin() * 0.5).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None).with_base_note(48));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        mixer.update_tempo(60.0);
        let mut pattern = Pattern::new(16, 1);

        // Row 0: Start on C-4
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        // Row 1: Portamento to E-4 (3xx with speed parameter)
        let mut cell1 = Cell::default();
        cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 0)));
        cell1
            .effects
            .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
        pattern.set_cell(1, 0, cell1);

        mixer.tick(0, &pattern);
        let freq_before = mixer.voices[0].as_ref().map(|v| v.triggered_note_freq);
        let c4_freq = Note::new(Pitch::C, 4, 127, 0).frequency();
        assert_eq!(
            freq_before,
            Some(c4_freq),
            "Initial note should be C-4 frequency"
        );

        // Render some frames
        let mut output1 = vec![0.0f32; 5000];
        mixer.render(&mut output1);

        // Apply portamento on row 1
        mixer.tick(1, &pattern);

        // Portamento should be active, voice should continue
        assert!(
            mixer.voices[0].is_some(),
            "Voice should continue during portamento"
        );
    }

    #[test]
    fn test_mixer_tone_portamento_updates_instrument() {
        let data = vec![1.0f32; 1000];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut inst1 = Instrument::new("Inst1").with_volume(0.5);
        inst1.sample_index = Some(0);
        let mut inst2 = Instrument::new("Inst2").with_volume(1.0);
        inst2.sample_index = Some(0);

        let mut mixer = Mixer::new(vec![sample], vec![inst1, inst2], 1, 44100);
        let mut pattern = Pattern::new(16, 1);

        // Row 0: Start note with instrument 0 (vol 0.5)
        let mut cell0 = Cell::default();
        cell0.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell0.instrument = Some(0);
        pattern.set_cell(0, 0, cell0);

        mixer.tick(0, &pattern);
        // Render a bit to advance position from 0
        let mut output = vec![0.0f32; 100];
        mixer.render(&mut output);

        let vol_before = mixer.voices[0].as_ref().unwrap().velocity_gain;
        assert!((vol_before - 0.5).abs() < 0.001);

        // Row 1: Tone portamento to E-4 with instrument 1 (vol 1.0)
        let mut cell1 = Cell::default();
        cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 1)));
        cell1.instrument = Some(1);
        cell1
            .effects
            .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
        pattern.set_cell(1, 0, cell1);

        mixer.tick(1, &pattern);

        let voice = mixer.voices[0].as_ref().unwrap();
        let vol_after = voice.velocity_gain;

        // Volume should be updated to 1.0 (from inst2)
        assert!(
            (vol_after - 1.0).abs() < 0.001,
            "Volume should be updated to 1.0, got {}",
            vol_after
        );
        // Position should NOT be reset (no re-trigger)
        assert!(
            voice.position > 0.0,
            "Voice should not be re-triggered (position should be > 0)"
        );
    }

    #[test]
    fn test_mixer_tone_portamento_instrument_only() {
        let data = vec![1.0f32; 1000];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut inst1 = Instrument::new("Inst1").with_volume(0.5);
        inst1.sample_index = Some(0);
        let mut inst2 = Instrument::new("Inst2").with_volume(1.0);
        inst2.sample_index = Some(0);

        let mut mixer = Mixer::new(vec![sample], vec![inst1, inst2], 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        println!(
            "DEBUG: pattern rows = {}, channels = {}",
            pattern.row_count(),
            pattern.num_channels()
        );

        // Row 0: Start note with instrument 0 (vol 0.5)
        let mut cell0 = Cell::default();
        cell0.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell0.instrument = Some(0);
        pattern.set_cell(0, 0, cell0);

        mixer.tick(0, &pattern);
        // Render a bit to advance position from 0
        let mut output = vec![0.0f32; 100];
        mixer.render(&mut output);

        let vol_before = mixer.voices[0].as_ref().unwrap().velocity_gain;
        assert!((vol_before - 0.5).abs() < 0.001);

        // Row 1: Tone portamento with instrument 1 (vol 1.0) with NEW NOTE
        let mut cell1 = Cell::default();
        cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 0)));
        cell1.instrument = Some(1);
        cell1
            .effects
            .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
        pattern.set_cell(1, 0, cell1);

        mixer.tick(1, &pattern);

        let voice = mixer.voices[0].as_ref().unwrap();
        let vol_after = voice.velocity_gain;

        // Volume should be updated to 1.0 (from inst2)
        println!(
            "DEBUG: vol_after = {}, inst_idx = {}",
            vol_after, voice.instrument_index
        );
        assert!(
            (vol_after - 1.0).abs() < 0.001,
            "Volume should be updated to 1.0, got {}",
            vol_after
        );
        assert_eq!(voice.instrument_index, 1);
        assert!(voice.position > 0.0, "Voice should not be re-triggered");
    }

    #[test]
    fn test_mixer_volume_column_applied() {
        let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);

        // Volume column (v40 = half volume)
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.volume = Some(0x40); // 64 decimal
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        let mut output = vec![0.0f32; 256];
        mixer.render(&mut output);

        let peak: f32 = output.iter().map(|s| s.abs()).fold(0.0, f32::max);
        // Volume column 0x40 = 64/64 = 1.0, applied via volume_override
        // Should produce audio at normalized level
        assert!(peak > 0.0, "Volume column should produce audio");
    }

    #[test]
    fn test_mixer_voice_stealing_on_new_note() {
        let sample = Sample::new(vec![0.5; 44100], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut pattern = Pattern::new(16, 4);

        // Start note on channel 0
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // New note on same channel should steal the voice
        pattern.set_note(1, 0, Note::new(Pitch::E, 4, 100, 0));
        mixer.tick(1, &pattern);

        // Should still have 1 voice, just restarted
        assert_eq!(
            mixer.active_voice_count(),
            1,
            "New note on same channel should replace voice"
        );

        // Voice position should be reset (new note)
        let voice_pos = mixer.voices[0].as_ref().map(|v| v.position);
        assert_eq!(voice_pos, Some(0.0), "New note should reset voice position");
    }

    #[test]
    fn test_mixer_set_volume_effect_cxx() {
        let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);

        // C20: set volume to 32/64 = 0.5
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::SetVolume, 0x20));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        let mut output1 = vec![0.0f32; 256];
        mixer.render(&mut output1);
        let peak_half: f32 = output1.iter().map(|s| s.abs()).fold(0.0, f32::max);

        // Now set full volume C40
        let mut cell2 = Cell::default();
        cell2.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell2
            .effects
            .push(Effect::from_type(EffectType::SetVolume, 0x40));
        let mut pattern2 = Pattern::new(16, 1);
        pattern2.set_cell(0, 0, cell2);

        mixer.tick(0, &pattern2);
        let mut output2 = vec![0.0f32; 256];
        mixer.render(&mut output2);
        let peak_full: f32 = output2.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_full > peak_half,
            "Full volume ({}) should be louder than half ({}), actual: {}",
            peak_full,
            peak_half / 0.5 * 1.0,
            peak_full
        );
    }

    #[test]
    fn test_mixer_sample_offset_9xx() {
        let data: Vec<f32> = (0..10000).map(|i| i as f32).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None));
        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);

        // 9xx: sample offset 512 bytes (2 * 256)
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::SampleOffset, 0x02));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);

        // Voice position should be set to offset
        let pos = mixer.voices[0].as_ref().map(|v| v.position);
        assert_eq!(
            pos,
            Some(512.0),
            "Sample offset 9xx should set voice position"
        );
    }

    #[test]
    fn test_mixer_set_panning_8xx() {
        let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);

        // 8xx: set panning (0x00=full left, 0x80=center, 0xFF=full right)
        let mut cell = Cell::default();
        cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
        cell.effects
            .push(Effect::from_type(EffectType::SetPanning, 0x00));
        pattern.set_cell(0, 0, cell);

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 256];
        mixer.render(&mut output);

        let left: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);
        let right: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);

        assert!(left > 0.0, "Left channel should have audio");
        assert!(
            right < 0.001,
            "Right channel should be silent with full-left panning"
        );
    }

    // --- LFO modulation tests ---

    #[test]
    fn test_voice_lfo_state_default() {
        let state = VoiceLfoState::default();
        assert_eq!(state.volume, 0.0);
        assert_eq!(state.panning, 0.0);
        assert_eq!(state.pitch, 0.0);
    }

    #[test]
    fn test_voice_lfo_state_from_instrument() {
        let mut inst = Instrument::new("Test");
        inst.volume_lfo = Some(Lfo::sine(4.0, 0.5));
        let state = VoiceLfoState::new(Some(&inst));
        assert_eq!(state.volume, 0.0);
        assert_eq!(state.panning, 0.0);
        assert_eq!(state.pitch, 0.0);
    }

    #[test]
    fn test_evaluate_lfo_waveform_sine() {
        let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Sine, 0.0);
        assert!(
            (val_at_zero - 0.0).abs() < 0.001,
            "Sine at 0 should be ~0, got {}",
            val_at_zero
        );

        let val_at_quarter = evaluate_lfo_waveform(LfoWaveform::Sine, 0.25);
        assert!(
            (val_at_quarter - 1.0).abs() < 0.001,
            "Sine at 0.25 should be ~1, got {}",
            val_at_quarter
        );

        let val_at_half = evaluate_lfo_waveform(LfoWaveform::Sine, 0.5);
        assert!(
            (val_at_half - 0.0).abs() < 0.001,
            "Sine at 0.5 should be ~0, got {}",
            val_at_half
        );

        let val_at_3qtr = evaluate_lfo_waveform(LfoWaveform::Sine, 0.75);
        assert!(
            (val_at_3qtr + 1.0).abs() < 0.001,
            "Sine at 0.75 should be ~-1, got {}",
            val_at_3qtr
        );
    }

    #[test]
    fn test_evaluate_lfo_waveform_triangle() {
        let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.0);
        assert!(
            (val_at_zero - 0.0).abs() < 0.001,
            "Triangle at 0 should be 0, got {}",
            val_at_zero
        );

        let val_at_quarter = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.25);
        assert!(
            (val_at_quarter - 1.0).abs() < 0.001,
            "Triangle at 0.25 should be 1, got {}",
            val_at_quarter
        );

        let val_at_half = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.5);
        assert!(
            (val_at_half - 0.0).abs() < 0.001,
            "Triangle at 0.5 should be 0, got {}",
            val_at_half
        );

        let val_at_3quarter = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.75);
        assert!(
            (val_at_3quarter - (-1.0)).abs() < 0.001,
            "Triangle at 0.75 should be -1, got {}",
            val_at_3quarter
        );
    }

    #[test]
    fn test_evaluate_lfo_waveform_square() {
        let val_low = evaluate_lfo_waveform(LfoWaveform::Square, 0.0);
        assert!(
            (val_low - 1.0).abs() < 0.001,
            "Square at 0 should be 1, got {}",
            val_low
        );

        let val_high = evaluate_lfo_waveform(LfoWaveform::Square, 0.5);
        assert!(
            (val_high - (-1.0)).abs() < 0.001,
            "Square at 0.5 should be -1, got {}",
            val_high
        );

        let val_high2 = evaluate_lfo_waveform(LfoWaveform::Square, 0.9);
        assert!(
            (val_high2 - (-1.0)).abs() < 0.001,
            "Square at 0.9 should be -1, got {}",
            val_high2
        );
    }

    #[test]
    fn test_evaluate_lfo_waveform_sawtooth() {
        let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.0);
        assert!(
            (val_at_zero - (-1.0)).abs() < 0.001,
            "Sawtooth at 0 should be -1, got {}",
            val_at_zero
        );

        let val_at_half = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.5);
        assert!(
            (val_at_half - 0.0).abs() < 0.001,
            "Sawtooth at 0.5 should be 0, got {}",
            val_at_half
        );

        let val_at_end = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.999);
        assert!(
            (val_at_end - 0.998).abs() < 0.01,
            "Sawtooth at 0.999 should be ~0.998, got {}",
            val_at_end
        );
    }

    #[test]
    fn test_evaluate_lfo_waveform_reverse_saw() {
        let val_at_zero = evaluate_lfo_waveform(LfoWaveform::ReverseSaw, 0.0);
        assert!(
            (val_at_zero - 1.0).abs() < 0.001,
            "ReverseSaw at 0 should be 1, got {}",
            val_at_zero
        );

        let val_at_half = evaluate_lfo_waveform(LfoWaveform::ReverseSaw, 0.5);
        assert!(
            (val_at_half - 0.0).abs() < 0.001,
            "ReverseSaw at 0.5 should be 0, got {}",
            val_at_half
        );
    }

    #[test]
    fn test_evaluate_lfo_waveform_random() {
        let val = evaluate_lfo_waveform(LfoWaveform::Random, 0.1);
        assert!(
            val >= -1.0 && val <= 1.0,
            "Random LFO value should be in [-1, 1], got {}",
            val
        );
    }

    #[test]
    fn test_mixer_lfo_vol_modulation() {
        // Create a sample with constant amplitude
        let data: Vec<f32> = vec![0.5; 48000];
        let sample = Sample::new(data, 48000, 1, Some("test".to_string()));
        let mut inst = Instrument::new("Test");
        inst.sample_index = Some(0);
        inst.volume_lfo = Some(Lfo::sine(10.0, 0.5));
        let instruments = vec![inst];

        let mut mixer = Mixer::new(vec![Arc::new(sample)], instruments, 4, 48000);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output1 = vec![0.0f32; 480];
        let mut output2 = vec![0.0f32; 480];
        mixer.render(&mut output1);
        mixer.render(&mut output2);

        let avg1: f32 = output1.iter().map(|s| s.abs()).sum::<f32>() / output1.len() as f32;
        let avg2: f32 = output2.iter().map(|s| s.abs()).sum::<f32>() / output2.len() as f32;
        assert!(
            avg1 > 0.0,
            "Voice with volume LFO should produce non-zero audio, got {}",
            avg1
        );
        assert!(
            (avg1 - avg2).abs() < 0.01 || avg1 != avg2,
            "LFO modulation should vary output between renders"
        );
    }

    #[test]
    fn test_mixer_lfo_pitch_modulation() {
        // Create a sample that's a linear ramp (easy to measure pitch changes)
        let data: Vec<f32> = (0..96000).map(|i| i as f32 / 96000.0).collect();
        let sample = Sample::new(data, 48000, 1, Some("ramp".to_string()));

        let mut inst_no_lfo = Instrument::new("NoLFO");
        inst_no_lfo.sample_index = Some(0);
        let mut inst_with_lfo = Instrument::new("WithLFO");
        inst_with_lfo.sample_index = Some(0);
        inst_with_lfo.pitch_lfo = Some(Lfo::sine(10.0, 0.5));

        let mut mixer_no_lfo =
            Mixer::new(vec![Arc::new(sample.clone())], vec![inst_no_lfo], 4, 48000);
        let mut mixer_with_lfo = Mixer::new(
            vec![Arc::new(sample.clone())],
            vec![inst_with_lfo],
            4,
            48000,
        );

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer_no_lfo.tick(0, &pattern);
        mixer_with_lfo.tick(0, &pattern);

        let mut output_no_lfo = vec![0.0f32; 480];
        let mut output_with_lfo = vec![0.0f32; 480];
        mixer_no_lfo.render(&mut output_no_lfo);
        mixer_with_lfo.render(&mut output_with_lfo);

        let _peak_no_lfo: f32 = output_no_lfo.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_with_lfo: f32 = output_with_lfo.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_with_lfo > 0.0,
            "Voice with pitch LFO should produce audio, got {}",
            peak_with_lfo
        );
    }

    #[test]
    fn test_mixer_lfo_zero_rate_no_modulation() {
        // LFO with 0 rate should not modulate
        let data: Vec<f32> = vec![0.5; 48000];
        let sample = Sample::new(data, 48000, 1, Some("test".to_string()));

        let mut inst = Instrument::new("Test");
        inst.sample_index = Some(0);
        inst.volume_lfo = Some(Lfo {
            waveform: LfoWaveform::Sine,
            rate: 0.0,
            depth: 1.0,
            offset: 0.0,
            enabled: true,
            phase: 0.0,
            sync_to_bpm: false,
        });
        inst.pitch_lfo = Some(Lfo {
            waveform: LfoWaveform::Sine,
            rate: 0.0,
            depth: 1.0,
            offset: 0.0,
            enabled: true,
            phase: 0.0,
            sync_to_bpm: false,
        });
        inst.panning_lfo = Some(Lfo {
            waveform: LfoWaveform::Sine,
            rate: 0.0,
            depth: 1.0,
            offset: 0.0,
            enabled: true,
            phase: 0.0,
            sync_to_bpm: false,
        });

        let mut mixer = Mixer::new(vec![Arc::new(sample)], vec![inst], 4, 48000);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 480];
        mixer.render(&mut output);

        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(
            has_audio,
            "Voice should still produce audio with zero-rate LFO"
        );
    }

    #[test]
    fn test_mixer_keyzone_sample_selection() {
        use crate::song::Keyzone;

        // Two samples: low-pitched sine and high-pitched sine
        let low_sample = Arc::new(make_test_sample(44100, 0.25));
        let high_sample = Arc::new(make_test_sample(44100, 0.25));

        let mut inst = Instrument::new("Piano");
        inst.keyzones = vec![
            Keyzone::new(0).with_note_range(0, 59),   // low sample
            Keyzone::new(1).with_note_range(60, 119), // high sample
        ];

        let mut mixer = Mixer::new(vec![low_sample, high_sample], vec![inst], 4, 44100);

        // Trigger a low note (C-3 = MIDI 36) on instrument 0
        let mut pattern = Pattern::new(16, 4);
        let low_note = Note::new(Pitch::C, 3, 100, 0);
        pattern.set_note(0, 0, low_note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Verify it picked sample index 0 (low keyzone)
        let voice = mixer.voices[0].as_ref().unwrap();
        assert_eq!(voice.sample_index, 0);

        // Now trigger a high note (C-6 = MIDI 72) on instrument 0
        let mut pattern2 = Pattern::new(16, 4);
        let high_note = Note::new(Pitch::C, 6, 100, 0);
        pattern2.set_note(0, 1, high_note);

        mixer.tick(0, &pattern2);
        mixer.tick(1, &pattern2);
        let voice = mixer.voices[1].as_ref().unwrap();
        assert_eq!(voice.sample_index, 1);
    }

    #[test]
    fn test_mixer_keyzone_no_match_silent() {
        use crate::song::Keyzone;

        let sample = Arc::new(make_test_sample(44100, 0.25));

        let mut inst = Instrument::new("Sparse");
        // Only covers notes 60-72
        inst.keyzones = vec![Keyzone::new(0).with_note_range(60, 72)];

        let mut mixer = Mixer::new(vec![sample], vec![inst], 4, 44100);

        // Trigger note outside keyzone range (C-3 = MIDI 36)
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 3, 100, 0));

        mixer.tick(0, &pattern);
        // No keyzone matches, so no voice should be triggered
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_no_keyzones_backward_compat() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut inst = Instrument::new("Simple");
        inst.sample_index = Some(0);
        let instruments = vec![inst];
        // No keyzones -- should use instrument_idx as sample_index directly
        // actually now it uses inst.sample_index fallback.
        let mut mixer = Mixer::new(vec![sample], instruments, 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
        assert_eq!(mixer.voices[0].as_ref().unwrap().sample_index, 0);
    }
}
