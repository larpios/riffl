# Project Index: riffl

Generated: 2026-03-25

## Project Overview

**riffl** is a terminal-based music tracker written in Rust. It supports playback and editing of classic tracker module formats (XM, IT, S3M, MOD, NSF) with a live-coding DSL and a ratatui TUI.

## Directory Structure

```
riffl/
├── crates/
│   ├── tracker-core/          # Core library: audio engine, formats, DSL, pattern model
│   │   ├── src/
│   │   │   ├── lib.rs         # Public module exports
│   │   │   ├── audio/         # Audio engine, mixing, effects, DSP
│   │   │   ├── dsl/           # Rhai-based live-coding DSL
│   │   │   ├── format/        # Module format loaders (XM, IT, S3M, MOD, NSF)
│   │   │   ├── pattern/       # Pattern/track/row data model
│   │   │   ├── export.rs      # WAV/FLAC export
│   │   │   ├── project.rs     # Project save/load
│   │   │   ├── song.rs        # Song arrangement structure
│   │   │   └── transport.rs   # Playback transport (BPM, tick, position)
│   │   ├── tests/             # Integration tests + test module files
│   │   ├── benches/           # Criterion benchmarks
│   │   └── examples/          # Standalone runnable examples
│   └── tracker-tui/           # TUI application (depends on tracker-core)
│       ├── src/
│       │   ├── main.rs        # Binary entry point
│       │   ├── app.rs         # App state machine
│       │   ├── config.rs      # TOML config loading
│       │   ├── editor/        # Pattern editor logic
│       │   ├── input/         # Keyboard input + keybindings
│       │   ├── registry/      # Component/plugin registry
│       │   └── ui/            # Ratatui UI panels
│       └── Cargo.toml
├── test-opl/                  # Excluded workspace utility: OPL chip testing
├── research/                  # Submodule: Furnace tracker research
├── docs/agents/               # Agent docs: audio, dsl, editor, input, pattern, ui
├── flake.nix                  # Nix build
└── Cargo.toml                 # Workspace root
```

## Entry Points

- **TUI binary**: `crates/tracker-tui/src/main.rs`
- **Core library**: `crates/tracker-core/src/lib.rs`
- **Format loader API**: `crates/tracker-core/src/format/mod.rs` — `format::load(&[u8])`
- **Examples**: `crates/tracker-core/examples/` (full_demo, freq, debug_it)

## Core Modules (tracker-core)

### `audio`
| File | Purpose |
|------|---------|
| `engine.rs` | Main audio engine, CPAL stream management |
| `device.rs` | Audio device enumeration |
| `sample.rs` | Sample loading and playback |
| `loader.rs` | Sample file loading (via symphonia) |
| `mixer.rs` | Channel mixing, volume/pan |
| `dsp.rs` | DSP utilities |
| `chip.rs` | Chip/retro synthesis |
| `bus.rs` | Audio bus routing |
| `channel_strip.rs` | Per-channel strip processing |
| `glicol_mixer.rs` | Glicol graph-based mixer |
| `effect_processor.rs` | Tracker effect commands (Arpeggio, Portamento, Vibrato, etc.) |
| `effects/biquad.rs` | Biquad filter |
| `effects/delay.rs` | Delay effect |

### `format`
Supported module formats, all implementing `ModuleLoader`:
| Loader | Format |
|--------|--------|
| `xm.rs` | FastTracker II (.xm) |
| `it.rs` | Impulse Tracker (.it) |
| `s3m.rs` | Scream Tracker 3 (.s3m) |
| `protracker.rs` | ProTracker/Amiga (.mod) |
| `nsf.rs` | NES Sound Format (.nsf) via game-music-emu |

### `pattern`
- `pattern.rs` — `Pattern` struct (rows x channels grid)
- `track.rs` — `Track` struct
- `row.rs` — `Row` struct (note, instrument, volume, effect, effect param)

### `dsl`
- `engine.rs` — Rhai script engine integration
- `pattern_api.rs` — DSL API exposed to scripts
- `examples.rs` — Built-in DSL examples

### Other
- `song.rs` — `Song`: arrangement order, pattern list, instruments
- `transport.rs` — BPM, tick clock, position tracking
- `export.rs` — WAV/FLAC render-to-file
- `project.rs` — Project serialization (serde_json)
- `log.rs` — Logging

## TUI Panels (tracker-tui/src/ui/)

| File | Panel |
|------|-------|
| `layout.rs` | Root layout |
| `mod.rs` | UI module root |
| `theme.rs` | Catppuccin Mocha color theme |
| `arrangement.rs` | Song arrangement view |
| `pattern_list.rs` | Pattern browser |
| `instrument_editor.rs` | Instrument editor |
| `envelope_editor.rs` | Volume/panning envelope |
| `waveform_editor.rs` | Sample waveform view |
| `lfo_editor.rs` | LFO editor |
| `oscilloscope.rs` | Real-time oscilloscope |
| `fft_analyzer.rs` | FFT spectrum analyzer |
| `vu_meters.rs` | VU meters |
| `sample_browser.rs` | Sample file browser |
| `file_browser.rs` | File system browser |
| `code_editor.rs` | Live DSL code editor panel |
| `export_dialog.rs` | Export dialog |
| `modal.rs` | Modal dialog |
| `tutor.rs` | Interactive tutorial panel |

## Tests

Located in `crates/tracker-core/tests/`:
| File | Coverage |
|------|---------|
| `s3m_playback.rs` | S3M tick-accurate playback |
| `s3m_loading_test.rs` | S3M format loading |
| `s3m_debug_test.rs` | S3M debug/edge cases |
| `xm_it_loading_test.rs` | XM and IT format loading |
| `st15_loading_test.rs` | Scream Tracker 1.5 loading |

Test module files in `tests/test_modules/`: .xm, .it, .s3m, .mod, .mptm fixtures.

Benchmarks: `benches/mixer_bench.rs`, `benches/module_parsing_bench.rs`

## Key Dependencies

### tracker-core
| Crate | Version | Purpose |
|-------|---------|---------|
| `cpal` | 0.15 | Cross-platform audio I/O |
| `symphonia` | 0.5 | WAV/FLAC/OGG decoding |
| `rhai` | 1 | Embedded scripting DSL |
| `glicol_synth` | 0.13.5 | Graph-based synthesis |
| `hound` | 3.5 | WAV write/read |
| `serde` / `serde_json` | 1.0 | Project serialization |
| `game-music-emu` | 0.3 | NSF/chiptune playback |
| `opl-emu` | 0.4 | OPL2/3 chip emulation (optional `adlib` feature) |
| `petgraph` | 0.6 | Audio graph |
| `dasp_sample` | 0.11 | Sample type conversion |
| `thiserror` | 1.0 | Error types |

### tracker-tui
| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.30 | TUI framework |
| `crossterm` | 0.28 | Terminal backend |
| `toml` | 0.8 | Config file parsing |
| `sysinfo` | 0.32 | CPU/memory info |

## Build

```bash
# Build all
cargo build

# Run TUI
cargo run -p tracker-tui

# Run tests
cargo test

# Run benchmarks
cargo bench

# Nix build
nix build
```

## Feature Flags

| Flag | Crate | Effect |
|------|-------|--------|
| `adlib` | tracker-core | Enables OPL2/3 chip emulation via opl-emu |

## Recent Active Work (branch: HEAD)

- `audio/effect_processor.rs` — Effect command processing (modified)
- `audio/mixer.rs` — Mixer (modified)
- `tests/s3m_playback.rs` — S3M playback tests (modified)
- Added test module fixtures: test.mod, test.mptm, test.s3m, test.xm
