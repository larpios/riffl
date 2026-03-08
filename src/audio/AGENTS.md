# Audio Engine

Low-latency audio playback engine built on cpal. Real-time sample mixing, effect processing, device management.

## STRUCTURE

```
audio/
├── mod.rs              # Public API re-exports
├── engine.rs           # AudioEngine: high-level API (init, start, stop, pause)
├── stream.rs           # AudioStream: cpal stream wrapper, real-time callback
├── mixer.rs            # Mixer: multi-channel sample mixing, per-row triggering
├── effect_processor.rs # EffectProcessor: volume, panning, pitch slide, arpeggio
├── sample.rs           # Sample: audio buffer (Arc<Vec<f32>>), metadata, resampling
├── loader.rs           # load_sample(): symphonia decoder (WAV/FLAC/OGG)
├── device.rs           # AudioDevice/DeviceInfo: cpal device enumeration
└── error.rs            # AudioError enum, AudioResult type alias
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| New effect type | `effect_processor.rs` | Add to `EffectProcessor` match arms |
| Change mixing logic | `mixer.rs` | `Mixer::fill_buffer()` is the hot path |
| Support new audio format | `loader.rs` | Add symphonia feature + decoder branch |
| Change latency/buffer | `stream.rs` | `StreamConfig` defaults (256 frames) |
| Device selection UI | `device.rs` | `AudioDevice::enumerate()` returns `DeviceInfo` |
| New error variant | `error.rs` | Add to `AudioError` enum |

## CONVENTIONS

- `AudioEngine` owns stream lifecycle. `Mixer` is passed into stream callback via `Arc<Mutex<>>`.
- `Sample` is `Arc`-wrapped for zero-copy sharing between mixer channels.
- `effect_processor.rs` processes effects per-cell per-tick; returns `TransportCommand` for tempo/position changes.

## ANTI-PATTERNS

- **NEVER allocate in audio callback.** `stream.rs` callback and `mixer.rs` `fill_buffer()` run on the real-time thread. No `Vec::push`, `String`, `Box::new`, or any heap allocation.
- **NEVER `unwrap()` on mutex locks** in audio paths — use `lock_unpoisoned()` pattern or graceful fallback.
- `Sample` buffers are pre-allocated at load time. Resampling uses pre-computed ratios.

## NOTES

- Mixer renders row-by-row: advances transport, triggers notes, mixes active voices into output buffer.
- Default config: 256 frames @ 48kHz ≈ 5.33ms latency.
- `C4_MIDI` constant (60) defined in `sample.rs` for pitch reference.
