//! Voice state and modulation for audio synthesis.
//!
//! This module contains the `Voice` struct and its related modulation states
//! like LFO and ADSR envelopes.

use crate::song::{Adsr, Instrument, Lfo, LfoWaveform};

/// Evaluates an LFO waveform at the given phase (0.0 to 1.0).
/// Returns a value in the range [-1.0, 1.0].
pub fn evaluate_lfo_waveform(waveform: LfoWaveform, phase: f32) -> f32 {
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
pub struct VoiceLfoState {
    /// LFO position for volume modulation (0.0 to 1.0, wraps each cycle).
    pub volume: f32,
    /// LFO position for panning modulation.
    pub panning: f32,
    /// LFO position for pitch modulation.
    pub pitch: f32,
}

impl VoiceLfoState {
    pub fn new(instrument: Option<&Instrument>) -> Self {
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

    pub fn update(&mut self, instrument: &Instrument, sample_rate: u32, bpm: f64) {
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

    pub fn get_vol_value(&self, lfo: &Lfo) -> f32 {
        self.calculate_value(self.volume, lfo)
    }

    pub fn get_pan_value(&self, lfo: &Lfo) -> f32 {
        self.calculate_value(self.panning, lfo)
    }

    pub fn get_pitch_value(&self, lfo: &Lfo) -> f32 {
        self.calculate_value(self.pitch, lfo)
    }

    fn calculate_value(&self, phase: f32, lfo: &Lfo) -> f32 {
        if !lfo.enabled {
            return 0.0;
        }

        let raw_val = evaluate_lfo_waveform(lfo.waveform, phase);
        lfo.offset + raw_val * lfo.depth
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AdsrPhase {
    #[default]
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AdsrState {
    pub phase: AdsrPhase,
    pub value: f32,
    /// Current time in the phase (seconds).
    pub phase_time: f32,
    /// Value when entering the release phase.
    pub release_start_value: f32,
}

impl AdsrState {
    pub fn update(&mut self, adsr: &Adsr, key_on: bool, sample_rate: u32) -> f32 {
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
pub struct Voice {
    /// Index into the mixer's instrument list.
    pub instrument_index: usize,
    /// Index into the mixer's sample list.
    pub sample_index: usize,
    /// Current read position within the sample's audio data (in frames).
    pub position: f64,
    /// Playback rate relative to the sample's base rate (for pitch shifting).
    #[allow(dead_code)]
    pub playback_rate: f64,
    /// Volume multiplier derived from note velocity (0.0 - 1.0).
    pub velocity_gain: f32,
    /// Whether this voice is actively producing audio.
    pub active: bool,
    /// Current playback direction (1.0 for forward, -1.0 for reverse).
    /// Used for ping-pong loops.
    pub loop_direction: f64,
    /// Whether the key is currently held down.
    pub key_on: bool,
    /// Current volume envelope position in ticks.
    pub volume_envelope_tick: usize,
    /// Current panning envelope position in ticks.
    pub panning_envelope_tick: usize,
    /// Current pitch envelope position in ticks.
    pub pitch_envelope_tick: usize,
    /// ADSR state for volume.
    pub volume_adsr: AdsrState,
    /// ADSR state for panning.
    pub panning_adsr: AdsrState,
    /// ADSR state for pitch.
    pub pitch_adsr: AdsrState,
    /// Ratio to convert an absolute frequency (Hz) into relative playback_rate.
    pub hz_to_rate: f64,
    /// The absolute frequency of the note that triggered this voice.
    pub triggered_note_freq: f64,
    /// Fadeout multiplier for IT/XM instruments (0.0 - 1.0).
    /// Decreased by instrument.fadeout every tick when key_on is false.
    pub fadeout_multiplier: f32,
    /// Per-voice LFO phase positions for volume, panning, and pitch.
    pub lfo: VoiceLfoState,
}

impl Voice {
    pub fn new(
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

    pub fn with_position(mut self, position: f64) -> Self {
        self.position = position;
        self
    }
}
