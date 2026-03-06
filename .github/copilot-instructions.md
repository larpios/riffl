# Copilot Instructions for tracker-rs

## Project Overview

`tracker-rs` is a TUI (terminal user interface) music tracker application written in Rust, inspired by Renoise and Strudel. It combines tracker-style pattern editing with live coding capabilities via an embedded Rhai scripting engine.

## Repository Structure

```
src/
├── main.rs             # Application entry point
├── app.rs              # Top-level App state and event loop
├── lib.rs              # Public library exports
├── transport.rs        # Playback state, BPM timing, row/pattern advancement
├── song.rs             # Song arrangement and project structure
├── project.rs          # Project save/load (serde_json)
├── export.rs           # Audio export (WAV via hound)
├── audio/              # Low-latency audio engine (cpal)
│   ├── mod.rs
│   ├── engine.rs       # High-level AudioEngine API
│   ├── stream.rs       # Audio stream management
│   ├── device.rs       # Device enumeration and selection
│   └── error.rs        # AudioError and AudioResult types
├── dsl/                # Rhai scripting DSL for pattern generation
│   ├── mod.rs
│   ├── engine.rs       # ScriptEngine wrapping Rhai
│   ├── pattern_api.rs  # Music functions registered to the Rhai engine
│   └── examples.rs     # Bundled DSL example scripts
├── editor/             # TUI editor state and cursor logic
├── pattern/            # Core musical data types
│   ├── mod.rs
│   ├── note.rs         # Pitch, Note, NoteOff, NoteEvent
│   ├── effect.rs       # Effect column types
│   ├── row.rs          # Pattern row (notes + effects)
│   ├── track.rs        # Track (sequence of rows)
│   └── pattern.rs      # Pattern (collection of tracks)
├── ui/                 # Ratatui TUI rendering
└── input/              # Input handling (crossterm)
examples/               # Runnable Cargo examples
```

## Build, Lint, and Test Commands

```bash
# Install Linux audio dependency first (CI and Linux development)
sudo apt-get install -y libasound2-dev

# Build
cargo build --all-features

# Run all tests
cargo test --all-features

# Lint (treat warnings as errors, as CI does)
cargo clippy --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Auto-format
cargo fmt --all

# Run acceptance tests
./run_acceptance_tests.sh

# Run a specific example
cargo run --example full_demo
```

CI runs `fmt --check`, `clippy -D warnings`, `build`, and `test` on every push/PR (see `.github/workflows/ci.yml`).

## Key Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | TUI rendering |
| `crossterm` | Terminal input/output |
| `cpal` | Cross-platform audio I/O |
| `symphonia` | Audio file decoding (WAV, FLAC, OGG) |
| `hound` | WAV export |
| `rhai` | Embedded scripting engine for the DSL |
| `serde` / `serde_json` | Project serialization |
| `anyhow` | Error handling in application code |
| `rand` | Random number generation in DSL functions |

## Coding Conventions

### Documentation Comments
- Use `//!` (inner doc comments) for **module-level** documentation at the top of files (e.g., `transport.rs`, `dsl/examples.rs`).
- Use `///` (outer doc comments) for **items** (structs, enums, functions, fields).
- Do not mix styles within a single file header — `rustfmt` may reorder imports and detach `///` comments from their intended position.

### Error Handling
- Use `anyhow::Result` / `anyhow::Error` for fallible application-layer functions.
- Define domain-specific error enums (e.g., `AudioError`) for library-layer modules.
- Prefer `?` for error propagation; avoid `unwrap()` in non-test code except where a panic is explicitly documented.

### Testing
- Unit tests live in `#[cfg(test)] mod tests { ... }` blocks inside the same file as the code under test.
- Test function names follow `test_<what>_<condition>` snake_case.
- Use `#[should_panic(expected = "...")]` to assert on panic messages when testing invalid inputs.

### General Style
- Follow standard Rust formatting (`cargo fmt`).
- All public items must have doc comments.
- Prefer `impl Default` delegation to `new()` where sensible.
- Numeric ranges are validated at construction time with `assert!` in `new()` constructors (see `Note::new`).
- MIDI note numbers: C-0 = 0, range 0–119 (C-0 to B-9).

### Audio Callback Constraints
- The audio callback runs on a real-time thread; **never allocate memory** inside the callback.
- Use `Arc<Mutex<>>` to share state between the audio thread and the main thread.
- Recover from poisoned mutexes instead of panicking — use the `lock_unpoisoned()` helper pattern in `src/dsl/engine.rs`.

### DSL / Rhai Integration
- Music functions exposed to Rhai scripts are registered in `dsl/pattern_api.rs`.
- Scripts return a `Pattern`, a primitive value, or unit; see `ScriptResult` in `dsl/engine.rs`.
- Keep Rhai-facing functions simple and strongly typed; use `rhai::{Array, Dynamic, INT}` for interop.

## Architecture Notes

- **Audio Pipeline:** `AudioEngine` → `AudioDevice` → `AudioStream` → real-time callback.
- **Transport:** `Transport` in `transport.rs` drives row/pattern advancement based on BPM and elapsed time. It returns an `AdvanceResult` on each tick.
- **Pattern Model:** `Pattern` → `Track` → `Row` → `(Option<NoteEvent>, Vec<Effect>)`. All types implement `Serialize`/`Deserialize`.
- **TUI:** Rendered with `ratatui`; input events from `crossterm` are processed in the main `app.rs` event loop.
