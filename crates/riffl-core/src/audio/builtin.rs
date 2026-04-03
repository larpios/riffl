//! Built-in instrument bank — synthesised samples that are always available
//! without loading any files from disk.
//!
//! Waveforms (sine, saw, square, triangle, noise) are stored as looping
//! wavetables so they sustain indefinitely.  Drum sounds (kick, snare,
//! hi-hat) are one-shot samples with amplitude / pitch decay baked in.

use std::f32::consts::PI;
use std::sync::Arc;

use crate::song::Instrument;

use super::sample::{LoopMode, Sample};

// ---------------------------------------------------------------------------
// Wavetable constants
//
// One period of each waveform is stored as WAVETABLE_SIZE samples.
// The stored sample_rate is chosen so that playing back at base_note A-4 (69)
// yields exactly 440 Hz at any output sample rate:
//
//   playback_rate = (target_freq / base_freq) * (WAVE_SR / output_sr)
//   For A-4: playback_rate = WAVE_SR / output_sr = WAVETABLE_SIZE * 440 / output_sr
//   → the wavetable loops 440 × per output-second  ✓
// ---------------------------------------------------------------------------
const WAVETABLE_SIZE: usize = 2048;
const WAVE_BASE_NOTE: u8 = 69; // A-4 = 440 Hz
const WAVE_SR: u32 = (WAVETABLE_SIZE as f64 * 440.0) as u32; // 901_120 Hz

// ---------------------------------------------------------------------------
// Looping waveforms
// ---------------------------------------------------------------------------

fn gen_sine() -> Sample {
    let data: Vec<f32> = (0..WAVETABLE_SIZE)
        .map(|i| (2.0 * PI * i as f32 / WAVETABLE_SIZE as f32).sin())
        .collect();
    Sample::new(data, WAVE_SR, 1, Some("Sine".into()))
        .with_base_note(WAVE_BASE_NOTE)
        .with_loop(LoopMode::Forward, 0, WAVETABLE_SIZE - 1)
}

fn gen_saw() -> Sample {
    let data: Vec<f32> = (0..WAVETABLE_SIZE)
        .map(|i| 2.0 * i as f32 / WAVETABLE_SIZE as f32 - 1.0)
        .collect();
    Sample::new(data, WAVE_SR, 1, Some("Saw".into()))
        .with_base_note(WAVE_BASE_NOTE)
        .with_loop(LoopMode::Forward, 0, WAVETABLE_SIZE - 1)
}

fn gen_square() -> Sample {
    let data: Vec<f32> = (0..WAVETABLE_SIZE)
        .map(|i| if i < WAVETABLE_SIZE / 2 { 0.8 } else { -0.8 })
        .collect();
    Sample::new(data, WAVE_SR, 1, Some("Square".into()))
        .with_base_note(WAVE_BASE_NOTE)
        .with_loop(LoopMode::Forward, 0, WAVETABLE_SIZE - 1)
}

fn gen_triangle() -> Sample {
    let data: Vec<f32> = (0..WAVETABLE_SIZE)
        .map(|i| {
            let phase = i as f32 / WAVETABLE_SIZE as f32;
            if phase < 0.25 {
                4.0 * phase
            } else if phase < 0.75 {
                2.0 - 4.0 * phase
            } else {
                4.0 * phase - 4.0
            }
        })
        .collect();
    Sample::new(data, WAVE_SR, 1, Some("Triangle".into()))
        .with_base_note(WAVE_BASE_NOTE)
        .with_loop(LoopMode::Forward, 0, WAVETABLE_SIZE - 1)
}

/// White-noise loop long enough that the repetition period is inaudible.
fn gen_noise() -> Sample {
    const NOISE_LEN: usize = 65536;
    let mut rng: u32 = 0x9E3779B9;
    let data: Vec<f32> = (0..NOISE_LEN)
        .map(|_| {
            rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (rng as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect();
    Sample::new(data, 44100, 1, Some("Noise".into()))
        .with_base_note(69)
        .with_loop(LoopMode::Forward, 0, NOISE_LEN - 1)
}

// ---------------------------------------------------------------------------
// Drum sounds  (one-shot, amplitude / pitch decay baked into PCM data)
// ---------------------------------------------------------------------------

/// 808-style kick: sine wave with exponential pitch and amplitude decay.
fn gen_kick(sr: u32) -> Sample {
    let duration = 0.35_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut phase = 0.0_f32;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let freq = 50.0 + 110.0 * (-t * 22.0_f32).exp();
        let amp = (-t * 12.0_f32).exp();
        data.push(amp * (2.0 * PI * phase).sin());
        phase = (phase + freq / sr as f32).fract();
    }
    // C-4 base note — pattern C notes play the kick at its recorded pitch
    Sample::new(data, sr, 1, Some("Kick".into())).with_base_note(48)
}

/// Snare: filtered-noise body with a short sine transient.
fn gen_snare(sr: u32) -> Sample {
    let duration = 0.18_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut rng: u32 = 0xDEADBEEF;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let noise_amp = (-t * 28.0_f32).exp();
        let body_amp = (-t * 50.0_f32).exp();
        let body = (2.0 * PI * 180.0 * t).sin();
        data.push(noise_amp * noise * 0.75 + body_amp * body * 0.5);
    }
    Sample::new(data, sr, 1, Some("Snare".into())).with_base_note(48)
}

/// Closed hi-hat: bright noise burst with very fast decay.
fn gen_hihat(sr: u32) -> Sample {
    let duration = 0.055_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut rng: u32 = 0xCAFEBABE;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let amp = (-t * 90.0_f32).exp();
        data.push(amp * noise);
    }
    Sample::new(data, sr, 1, Some("Hi-hat".into())).with_base_note(48)
}

/// Open hi-hat: same bright noise but slower (~0.4 s) decay.
fn gen_open_hihat(sr: u32) -> Sample {
    let duration = 0.40_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut rng: u32 = 0xBADC0FFE;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let amp = (-t * 8.0_f32).exp();
        data.push(amp * noise);
    }
    Sample::new(data, sr, 1, Some("Open HH".into())).with_base_note(48)
}

/// Clap: three staggered noise bursts mimicking a hand clap.
fn gen_clap(sr: u32) -> Sample {
    let duration = 0.18_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut rng: u32 = 0x13131313;
    // Three micro-bursts at 0, 6, 12 ms
    let offsets = [0.0_f32, 0.006, 0.012];
    for i in 0..n {
        let t = i as f32 / sr as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let amp: f32 = offsets
            .iter()
            .map(|&o| {
                let dt = t - o;
                if dt >= 0.0 {
                    (-dt * 120.0_f32).exp()
                } else {
                    0.0
                }
            })
            .sum();
        data.push((amp * noise).clamp(-1.0, 1.0));
    }
    Sample::new(data, sr, 1, Some("Clap".into())).with_base_note(48)
}

/// Crash cymbal: long bright-noise decay (~1.5 s).
fn gen_crash(sr: u32) -> Sample {
    let duration = 1.5_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut rng: u32 = 0xFEEDFACE;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let amp = (-t * 3.5_f32).exp();
        data.push(amp * noise * 0.9);
    }
    Sample::new(data, sr, 1, Some("Crash".into())).with_base_note(48)
}

/// Tom: mid-pitch drum — like the kick but lighter decay and higher start freq.
fn gen_tom(sr: u32) -> Sample {
    let duration = 0.25_f32;
    let n = (sr as f32 * duration) as usize;
    let mut data = Vec::with_capacity(n);
    let mut phase = 0.0_f32;
    let mut rng: u32 = 0xABCDEF01;
    for i in 0..n {
        let t = i as f32 / sr as f32;
        let freq = 90.0 + 120.0 * (-t * 18.0_f32).exp();
        let tone_amp = (-t * 14.0_f32).exp();
        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = (rng as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let noise_amp = (-t * 35.0_f32).exp() * 0.25;
        data.push(tone_amp * (2.0 * PI * phase).sin() + noise_amp * noise);
        phase = (phase + freq / sr as f32).fract();
    }
    Sample::new(data, sr, 1, Some("Tom".into())).with_base_note(48)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// The built-in instrument bank returned by [`builtin_bank`].
pub struct BuiltinBank {
    /// Ready-to-use samples, in the same order as `instruments`.
    pub samples: Vec<Arc<Sample>>,
    /// Instrument definitions, each pointing to the corresponding sample index.
    pub instruments: Vec<Instrument>,
}

/// Build the default instrument bank for a new empty song.
///
/// Sample / instrument indices are stable — existing scripts that reference
/// `KICK`, `SNARE`, etc. will continue to work after new entries are appended.
///
/// | Index | Constant  | Name     | Type              |
/// |-------|-----------|----------|-------------------|
/// | 0     | SINE      | Sine     | looping wavetable |
/// | 1     | SAW       | Saw      | looping wavetable |
/// | 2     | SQUARE    | Square   | looping wavetable |
/// | 3     | TRIANGLE  | Triangle | looping wavetable |
/// | 4     | NOISE     | Noise    | looping noise     |
/// | 5     | KICK      | Kick     | one-shot drum     |
/// | 6     | SNARE     | Snare    | one-shot drum     |
/// | 7     | HIHAT     | Hi-hat   | one-shot drum     |
/// | 8     | OHIHAT    | Open HH  | one-shot drum     |
/// | 9     | CLAP      | Clap     | one-shot drum     |
/// | 10    | CRASH     | Crash    | one-shot drum     |
/// | 11    | TOM       | Tom      | one-shot drum     |
pub fn builtin_bank(output_sample_rate: u32) -> BuiltinBank {
    let raw: Vec<Sample> = vec![
        gen_sine(),
        gen_saw(),
        gen_square(),
        gen_triangle(),
        gen_noise(),
        gen_kick(output_sample_rate),
        gen_snare(output_sample_rate),
        gen_hihat(output_sample_rate),
        gen_open_hihat(output_sample_rate),
        gen_clap(output_sample_rate),
        gen_crash(output_sample_rate),
        gen_tom(output_sample_rate),
    ];

    let samples: Vec<Arc<Sample>> = raw.into_iter().map(Arc::new).collect();

    let names = [
        "Sine", "Saw", "Square", "Triangle", "Noise",
        "Kick", "Snare", "Hi-hat", "Open HH", "Clap", "Crash", "Tom",
    ];
    let instruments: Vec<Instrument> = names
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            let mut inst = Instrument::new(name);
            inst.sample_index = Some(i);
            inst
        })
        .collect();

    BuiltinBank {
        samples,
        instruments,
    }
}
