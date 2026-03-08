# Amiga Period Clock Fix - Findings

## The Problem

`PitchCalculator` uses `AMIGA_NTSC_CLOCK / musical_freq` to compute a period, but
`note.frequency()` returns **musical Hz** (440 Hz = A4), not sample playback rate.

Amiga period math operates on the **sample advance rate**, not musical Hz.

For A4 on a standard MOD sample (C5=8363 Hz, base_note=C4):
- Correct Amiga period: ~252
- Current (wrong): `3_546_895 / 440 = 8061`  (32x too large)
- Period deltas therefore 32x too small → barely audible slides

## The Fix

### Effective Clock Formula

When working in musical Hz space, the correct "period clock" per channel is:

```
period_clock = AMIGA_PAL_CLOCK * base_freq / sample_rate
```

Where:
- `AMIGA_PAL_CLOCK = 3_546_895.0`  (rename from AMIGA_NTSC_CLOCK — value is PAL, not NTSC)
- `base_freq = sample.base_frequency()`  (261.63 Hz for C4)
- `sample_rate = sample.sample_rate()`   (e.g., 8363 for standard MOD)

Verification: `3_546_895 * 261.63 / 8363 ≈ 110_966`
A4 period = `110_966 / 440 ≈ 252`  ✓

## Required Code Changes

### 1. `crates/tracker-core/src/audio/pitch.rs`

- Rename `AMIGA_NTSC_CLOCK` → `AMIGA_PAL_CLOCK` (value stays 3_546_894.6)
- Add `period_clock: f64` parameter to `apply_slide` and `apply_portamento`
  replacing the hardcoded constant. Callers pass their channel's computed clock.

```rust
pub const AMIGA_PAL_CLOCK: f64 = 3_546_894.6;

pub fn apply_slide(current_freq: f64, param_up: u8, param_down: u8,
                   mode: SlideMode, period_clock: f64) -> f64 { ... }

pub fn apply_portamento(current_freq: f64, target_freq: f64,
                        speed: f64, mode: SlideMode, period_clock: f64) -> f64 { ... }
```

### 2. `crates/tracker-core/src/audio/effect_processor.rs`

Add field to `ChannelEffectState`:
```rust
/// Effective Amiga period clock for this channel (AmigaPeriod mode only).
/// Set by the mixer when a note triggers: AMIGA_PAL_CLOCK * base_freq / sample_rate
pub period_clock: f64,
```
Default: `AMIGA_PAL_CLOCK` (will be overridden on note trigger).

Update `advance_portamento_tick` and `advance_pitch_slide_tick` to pass `self.period_clock`.

Add to `TrackerEffectProcessor`:
```rust
pub fn set_period_clock(&mut self, channel: usize, clock: f64) {
    if let Some(ch) = self.channels.get_mut(channel) {
        ch.period_clock = clock;
    }
}
```

### 3. `crates/tracker-core/src/audio/mixer.rs`

At the note trigger site (~line 636), after computing `base_freq` and before
creating the voice, set the period clock on the effect processor:

```rust
// Inside AmigaPeriod mode note trigger, after computing base_freq:
if self.effect_processor.channel_state(ch)
    .map(|s| s.slide_mode == SlideMode::AmigaPeriod)
    .unwrap_or(false)
{
    let period_clock = crate::audio::pitch::AMIGA_PAL_CLOCK
        * base_freq
        / sample.sample_rate() as f64;
    self.effect_processor.set_period_clock(ch, period_clock);
}
```

This needs to be done at both note trigger sites:
- Line ~636: standard note-on trigger path
- Line ~716: pending note (EDx note delay) path — already has base_freq available

The tone portamento instrument-update path (~line 813) does NOT need it since it
doesn't change which sample is playing.

## Note on S3M Portamento Speed Scaling

S3M portamento params are already stored as raw period deltas (`effect.param as f64`)
per the existing code in `process_row`. No additional ×4 scaling is needed — that
factor is OpenMPT's internal precision artifact, not part of the S3M spec behavior.

## Summary of All Files to Touch

| File | Change |
|------|--------|
| `audio/pitch.rs` | Rename const; add `period_clock` param to both functions; update tests |
| `audio/effect_processor.rs` | Add `period_clock` field; pass it in tick advances; add setter |
| `audio/mixer.rs` | Set period_clock on effect processor when note triggers in AmigaPeriod mode |
