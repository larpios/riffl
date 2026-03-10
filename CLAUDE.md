# CLAUDE.md — AI Assistant Guide for tracker-rs

## Project Overview

**tracker-rs** is a TUI (Terminal User Interface) music tracker application with live coding capabilities, written in Rust. It combines the workflow of classic trackers (like Renoise) with algorithmic composition inspired by Strudel/TidalCycles. The codebase is a Cargo workspace with two crates:

- **`tracker-core`** — frontend-agnostic library (audio engine, pattern data, DSL, transport, project I/O)
- **`tracker-tui`** — TUI binary frontend using `ratatui` and `crossterm`

---

## Repository Structure

```
tracker-rs/
├── .github/
│   ├── workflows/ci.yml            # CI: fmt, clippy, build, test
│   └── copilot-instructions.md
├── crates/
│   ├── tracker-core/               # Library crate
│   │   ├── src/
│   │   │   ├── lib.rs              # Public module re-exports
│   │   │   ├── song.rs             # Song model (patterns, arrangement, instruments)
│   │   │   ├── transport.rs        # BPM-driven playback state machine
│   │   │   ├── project.rs          # Save/load .trs project files (JSON)
│   │   │   ├── export.rs           # Offline WAV rendering
│   │   │   ├── audio/
│   │   │   │   ├── engine.rs       # High-level AudioEngine API
│   │   │   │   ├── device.rs       # Audio device enumeration (cpal)
│   │   │   │   ├── stream.rs       # cpal audio stream (48kHz, 256-frame buffer)
│   │   │   │   ├── mixer.rs        # Sample mixing & pattern sequencing
│   │   │   │   ├── sample.rs       # Sample data structure
│   │   │   │   ├── loader.rs       # Sample decoding (symphonia: WAV/FLAC/OGG)
│   │   │   │   ├── dsp.rs          # DspProcessor trait, RampedParam
│   │   │   │   ├── channel_strip.rs# Per-track volume/pan processing
│   │   │   │   ├── bus.rs          # Send/return bus routing
│   │   │   │   ├── effect_processor.rs  # Tracker effect execution
│   │   │   │   ├── glicol_mixer.rs # Prototype Glicol engine (experimental)
│   │   │   │   ├── error.rs        # AudioError type
│   │   │   │   └── effects/
│   │   │   │       ├── biquad.rs   # Biquad filter
│   │   │   │       └── delay.rs    # Delay line
│   │   │   ├── pattern/
│   │   │   │   ├── mod.rs          # Module root & re-exports
│   │   │   │   ├── pattern.rs      # Pattern grid (rows × channels)
│   │   │   │   ├── note.rs         # Note (Pitch, octave, velocity, instrument)
│   │   │   │   ├── row.rs          # Row & Cell (note event, volume, effects)
│   │   │   │   ├── track.rs        # Track metadata (name, vol, pan, mute, solo)
│   │   │   │   └── effect.rs       # Tracker effect command type
│   │   │   └── dsl/
│   │   │       ├── engine.rs       # Rhai scripting engine wrapper
│   │   │       ├── pattern_api.rs  # DSL functions (note, scale, chord, euclidean)
│   │   │       ├── examples.rs     # Example DSL scripts
│   │   │       └── mod.rs
│   │   ├── benches/
│   │   │   └── mixer_bench.rs      # Criterion benchmarks
│   │   └── examples/
│   │       └── full_demo.rs        # Audio engine demonstration
│   │
│   └── tracker-tui/                # Binary crate
│       └── src/
│           ├── main.rs             # Entry point, event loop (~60 FPS), panic hook
│           ├── app.rs              # App state (orchestrates all subsystems, ~1400 LOC)
│           ├── editor/
│           │   └── mod.rs          # Vim-modal pattern editor (Normal/Insert/Visual)
│           ├── input/
│           │   ├── mod.rs
│           │   └── keybindings.rs  # Key→Action mapping (~50+ Action variants)
│           └── ui/
│               ├── mod.rs          # render(frame, app) dispatch
│               ├── layout.rs       # Terminal layout (left/center/right panes)
│               ├── theme.rs        # Color theme definitions
│               ├── pattern_list.rs # Pattern pool view
│               ├── instrument_list.rs  # Instrument editor
│               ├── arrangement.rs  # Song arrangement timeline
│               ├── code_editor.rs  # Rhai script editor
│               ├── file_browser.rs # Sample loader filesystem browser
│               ├── export_dialog.rs# WAV export options dialog
│               ├── modal.rs        # Modal dialog stack system
│               └── help.rs         # Keybinding help overlay
│
├── docs/
│   └── VISION.md                   # Architecture vision & 7-phase roadmap
├── AGENTS.md                       # Agent knowledge base (architecture summary)
├── ACCEPTANCE_TEST.md              # Audio engine acceptance test spec
├── README.md                       # Audio engine documentation
├── Cargo.toml                      # Workspace root (resolver = "2")
├── flake.nix                       # Nix development environment
└── run_tests.sh                    # Test runner convenience script
```

---

## Key Dependencies

| Dependency | Version | Purpose |
|---|---|---|
| `cpal` | 0.15 | Cross-platform audio I/O (ALSA/CoreAudio/WASAPI) |
| `symphonia` | 0.5 | Audio decoding (WAV, FLAC, OGG Vorbis) |
| `hound` | 3.5 | WAV file writing |
| `rhai` | 1 | Embedded scripting DSL for live coding |
| `serde` / `serde_json` | 1.0 | Project file serialization |
| `anyhow` | 1.0 | Error handling (application layer) |
| `glicol_synth` | 0.13.5 | Prototype Glicol audio engine |
| `petgraph` | 0.6 | Graph algorithms (DSP graph) |
| `ratatui` | 0.28 | TUI rendering |
| `crossterm` | 0.28 | Terminal control |
| `criterion` | 0.5 | Benchmarking (dev) |

---

## Development Commands

```bash
# Build everything
cargo build --workspace --all-features

# Run the TUI application
cargo run -p tracker-tui

# Run the audio engine demo
cargo run -p tracker-core --example full_demo

# Run all tests
cargo test --workspace --all-features

# Check formatting (CI gate)
cargo fmt --all -- --check

# Auto-fix formatting
cargo fmt --all

# Lint (CI gate — warnings are errors)
cargo clippy --workspace --all-features -- -D warnings

# Run benchmarks
cargo bench -p tracker-core

# Run convenience test script
./run_tests.sh
```

**System dependency for Linux:** `libasound2-dev` (ALSA headers required by cpal).

---

## CI Pipeline

GitHub Actions runs on `ubuntu-latest` for every push/PR:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-features -- -D warnings`
3. `cargo build --workspace --all-features`
4. `cargo test --workspace --all-features`

**All four must pass.** Fix fmt/clippy issues before committing.

---

## Core Data Model

### Pattern System
- **`Pitch`** — 12-note enum (C through B, sharp notation: `CSharp`, `DSharp`, etc.)
- **`Note`** — `{ pitch: Pitch, octave: u8 (0–9), velocity: u8 (0–127), instrument: u8 }`
- **`NoteEvent`** — `On(Note)` or `Off`
- **`Effect`** — `{ effect_type: u8, value: u8 }` — hex-style tracker commands
- **`Cell`** — `{ note: Option<NoteEvent>, instrument: Option<u8>, volume: Option<u8>, effects: Vec<Effect> }`
- **`Row`** — Vec of Cells, one per channel
- **`Pattern`** — 2D grid; default 64 rows × 8 channels (max configurable)
- **`Track`** — metadata: name, volume (0–255), pan (i8), mute, solo, instrument_index

### Song Model (`song.rs`)
- **`Instrument`** — `{ name, sample_index, sample_path, base_note, volume }`
- **`Song`** — `{ name, artist, bpm, patterns: Vec<Pattern> (max 256), arrangement: Vec<usize>, tracks: Vec<Track>, instruments: Vec<Instrument> }`

### Transport (`transport.rs`)
- **States:** `Stopped`, `Playing`, `Paused`
- **Modes:** `Pattern` (loop single pattern) or `Song` (follow arrangement)
- **Timing:** `ROWS_PER_BEAT = 4.0`; at 120 BPM → 0.125s per row
- **BPM range:** 20–999 (clamped at construction/mutation)
- **`advance(elapsed_seconds)`** → `AdvanceResult` (None | Row | PatternChange | Stopped)

### Audio Engine (`audio/`)
- Default stream: 48 kHz, 256-frame buffer (~5.3ms latency), stereo
- **`Mixer`** — reads pattern rows, triggers sample voices, mixes to stereo
  - Per-channel voice state: sample_index, playback position, rate, velocity gain
  - Applies channel strips (volume, pan, mute, solo)
  - Applies tracker effects via `EffectProcessor`
  - Routes through send/return bus system
- **Audio callback constraint:** Never allocate in the audio callback thread

### Project Files
- Extension: `.trs`
- Format: pretty-printed JSON via `serde_json`
- Audio samples are **not embedded** — only file paths are stored

---

## TUI Architecture

### App State (`app.rs`)
`App` is the central orchestrator (~1400 LOC). Key fields:
- `editor: Editor` — vim-modal pattern editor
- `song: Song` — the song model
- `transport: Transport` — BPM playback control
- `audio_engine: Option<AudioEngine>`
- `mixer: Arc<Mutex<Mixer>>` — shared with the audio thread
- `script_engine: ScriptEngine` — Rhai evaluator
- `modal_stack: Vec<Modal>` — layered dialog system
- `current_view: AppView` — active screen

**Views** (switched via function keys):
| Key | View |
|---|---|
| F1 | Pattern Editor |
| F2 | Arrangement |
| F3 | Instrument List |
| F4 | Code Editor (Rhai) |
| F5 | Pattern List |

### Editor (`editor/mod.rs`)
Vim-modal pattern editor:
- **Modes:** `Normal` (navigate), `Insert` (enter notes), `Visual` (select region)
- **Sub-columns per cell:** Note, Instrument, Volume, Effect
- **Undo/redo:** Pattern snapshots (Vec<Pattern>)
- **Clipboard:** Rectangular cell region copy/paste

### Input (`input/keybindings.rs`)
`map_key_to_action(key_event, editor_mode) -> Option<Action>` dispatches all keyboard input to ~50+ `Action` variants covering navigation, note entry, transport, track operations, views, DSL, and project management.

---

## Code Conventions

### Error Handling
- **Application layer:** `anyhow::Result<T>` with `?` propagation
- **Library modules:** Domain-specific error enums (e.g., `AudioError`)
- **Never use `.unwrap()`** in library code or audio callback
- **Mutex poisoning:** Use a safe unlock pattern — don't `.unwrap()` on `.lock()`

### Documentation
- **Module-level:** `//!` doc comments at top of each file
- **Public items:** `///` doc comments on all `pub` functions/types/fields
- **Never mix** `//!` and `///` in the same doc block header

### Testing
- Tests live inline in `#[cfg(test)] mod tests { ... }` at the bottom of each source file
- **Naming convention:** `test_<what>_<condition>` (e.g., `test_bpm_range_clamping`, `test_row_wrapping_with_loop`)
- High coverage expected: transport.rs has 40+ tests, song.rs has 20+, project.rs has 10+

### Known Allowed Lint Suppressions
- `#![allow(dead_code, unused_imports)]` in `tracker-tui/src/main.rs` — intentional for binary
- `#[allow(clippy::module_inception)]` in `pattern/mod.rs` — `pattern::pattern` naming is intentional

### Real-Time Audio Constraints
- **Never allocate heap memory** (Vec, Box, String, etc.) inside the cpal audio callback
- Pre-allocate all buffers before the callback
- Don't lock mutexes that could be contended for long periods in the audio thread

---

## DSL (Live Coding)

The `dsl/` module embeds **Rhai** as an expression language for algorithmic pattern generation.

Key DSL functions exposed to scripts:
- `note(pitch, octave, velocity)` — create a note
- `scale(root, scale_type)` — generate scale pitches
- `chord(root, chord_type)` — generate chord notes
- `euclidean(steps, pulses)` — Euclidean rhythm generator

Scripts are evaluated via `ScriptEngine::eval_with_pattern(code, pattern)` and return a modified `Pattern` plus a list of commands.

---

## Architecture Decisions & Rationale

| Decision | Rationale |
|---|---|
| Workspace split (`tracker-core` + `tracker-tui`) | Frontend-agnostic core allows future GUI frontend without rewriting audio logic |
| `Arc<Mutex<Mixer>>` shared with audio thread | Allows UI thread to update pattern/instrument data safely |
| Rhai for DSL | Safe, sandboxed, easy to integrate, no GC pressure |
| JSON for project files | Human-readable, debuggable, no binary format complexity |
| Sample paths not embedded | Avoids binary bloat; users manage their sample libraries |
| cpal for audio | Cross-platform: ALSA (Linux), CoreAudio (macOS), WASAPI (Windows) |

---

## Common Tasks & Where to Look

| Task | Files |
|---|---|
| Add a new tracker effect command | `crates/tracker-core/src/audio/effect_processor.rs`, `crates/tracker-core/src/pattern/effect.rs` |
| Add a new DSL function | `crates/tracker-core/src/dsl/pattern_api.rs` |
| Add a new keybinding/action | `crates/tracker-tui/src/input/keybindings.rs`, then handle in `app.rs` |
| Add a new UI view | `crates/tracker-tui/src/ui/` (new file), register in `ui/mod.rs`, add `AppView` variant |
| Change audio stream config | `crates/tracker-core/src/audio/stream.rs` |
| Modify the Song/Pattern data model | `crates/tracker-core/src/song.rs`, `crates/tracker-core/src/pattern/` |
| Update project serialization | `crates/tracker-core/src/project.rs` (mind backwards compatibility) |
| Add a DSP effect | `crates/tracker-core/src/audio/effects/` (new file), wire up in `mixer.rs` |

---

## Things to Avoid

- **Don't allocate in the audio callback** — causes audio glitches (xruns)
- **Don't unwrap() mutex locks** — use safe recovery or `expect()` with a clear message
- **Don't add features not requested** — the codebase avoids over-engineering
- **Don't skip clippy warnings** — CI treats warnings as errors
- **Don't embed binary audio data** in project files — keep .trs files as path references only
- **Don't assume an interactive TTY** in tracker-core — it must remain headless/library-safe

---

## Project Vision & Roadmap

See `docs/VISION.md` for the full 7-phase roadmap. Current state (as of 2026-03):

- **Phase 1 (Done):** Workspace split (tracker-core + tracker-tui)
- **Phase 2 (In Progress):** Audio engine upgrades — channel strips, send/return buses, built-in effects, Glicol prototype
- **Phase 3:** MIDI + automation
- **Phase 4:** Arrangement & song structure
- **Phase 5:** Plugin hosting (CLAP/VST3)
- **Phase 6:** GUI frontend
- **Phase 7:** Advanced DSL (mini-notation, pattern combinators)

---

## MIDI Note Reference

- C-0 = MIDI note 0, B-9 = MIDI note 119
- Range: 0–119 (validated at `Note` construction)
- Pitches use sharp notation (`CSharp` not `Db`)
