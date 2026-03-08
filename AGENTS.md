# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-08
**Commit:** 67cc56d
**Branch:** main (detached HEAD)

## OVERVIEW

TUI music tracker (Renoise-inspired) with live coding DSL (Rhai scripting), built in Rust. Core stack: ratatui + crossterm (TUI), cpal (audio I/O), symphonia (decoding), hound (WAV export), rhai (scripting).

## STRUCTURE

```
src/
├── main.rs          # Binary entry: terminal lifecycle, event loop, panic hook
├── app.rs           # App state: orchestrates editor, audio, transport, UI, scripting
├── lib.rs           # Library crate root (exports: audio, dsl, editor, pattern)
├── transport.rs     # BPM timing, row/pattern advancement, play/stop/pause state machine
├── song.rs          # Song model: pattern pool (max 256), arrangement, instruments
├── project.rs       # Save/load to .trs (JSON via serde_json)
├── export.rs        # Offline WAV rendering through mixer
├── audio/           # Low-latency audio engine (cpal) — see src/audio/AGENTS.md
├── dsl/             # Rhai scripting DSL — see src/dsl/AGENTS.md
├── editor/          # Vim-modal pattern editor — see src/editor/AGENTS.md
├── pattern/         # Core musical data types — see src/pattern/AGENTS.md
├── ui/              # Ratatui rendering — see src/ui/AGENTS.md
└── input/           # Keybinding dispatch — see src/input/AGENTS.md
examples/
└── full_demo.rs     # AudioEngine API demo (device enum, sine playback)
benches/
└── mixer_bench.rs   # Criterion bench for Mixer::new
```

## ARCHITECTURE

**Dual crate roots:** Both `main.rs` (binary) and `lib.rs` (library) declare modules from `src/`. The binary uses `mod audio;` not `use tracker_rs::audio;` — modules compile twice. This is intentional for now.

**Data flow:**
```
Editor (cursor/input) → Pattern (data model) → Transport (timing) → Mixer (sample mixing) → AudioStream (cpal callback)
                                                                      ↑
DSL/ScriptEngine (Rhai) → generates Pattern data ─────────────────────┘
```

**Key types:** `App` (god object), `Editor` (modal state machine), `Song` (pattern pool + arrangement), `Transport` (BPM driver returning `AdvanceResult`), `Mixer` (real-time sample mixing), `Pattern → Track → Row → Cell`.

**Concurrency:** Audio callback on real-time thread. Shared state via `Arc<Mutex<>>`. Poisoned mutex recovery via `lock_unpoisoned()` helper in `dsl/engine.rs`.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new view/screen | `app.rs` (`AppView` enum) + `ui/` | Add variant, match in `ui/mod.rs` render dispatch |
| Add keybinding | `input/keybindings.rs` | Add `Action` variant, map in `map_key_to_action` |
| New pattern data type | `pattern/` | Add to `Cell`/`Row`, update serde derives |
| New audio effect | `audio/effect_processor.rs` | Implement in `EffectProcessor` |
| DSL function for scripts | `dsl/pattern_api.rs` | Register with Rhai engine |
| Export format | `export.rs` | Model after `export_wav` |
| Project persistence | `project.rs` | Uses `Song` serde, `.trs` extension |
| New instrument property | `song.rs` (`Instrument`) | Update struct + serde |

## CONVENTIONS

- **Doc comments:** `//!` for module-level (file top), `///` for items. Never mix in same header.
- **Error handling:** `anyhow::Result` for app layer; domain-specific error enums (e.g., `AudioError`) for library modules. Prefer `?` over `unwrap()` in non-test code.
- **Tests:** Inline `#[cfg(test)] mod tests` in every file. Names: `test_<what>_<condition>`.
- **Public API:** All public items must have doc comments. Prefer `impl Default` delegation to `new()`.
- **MIDI notes:** C-0 = 0, range 0–119 (C-0 to B-9). Validated at construction (`Note::new`).

## ANTI-PATTERNS (THIS PROJECT)

- `#![allow(dead_code, unused_imports)]` in `main.rs` — binary is lenient; library is not.
- `#[allow(clippy::module_inception)]` in `pattern/mod.rs` — `pattern::pattern` module naming is intentional.
- **Never allocate in audio callback** — real-time thread constraint. No `Vec::push`, `String::new`, `Box::new` inside mixer/stream callbacks.
- **Recover from poisoned mutexes** — don't `unwrap()` on `.lock()` in audio/DSL paths. Use `lock_unpoisoned()` pattern.

## COMMANDS

```bash
cargo build --all-features        # Build (matches CI)
cargo test --all-features         # Run all tests
cargo clippy --all-features -- -D warnings  # Lint (CI treats warnings as errors)
cargo fmt --all -- --check        # Format check
cargo fmt --all                   # Auto-format
cargo run                         # Launch TUI app
cargo run --example full_demo     # Audio engine demo
cargo bench                       # Mixer benchmark (criterion)
```

## CI

Single GitHub Actions job (`.github/workflows/ci.yml`): fmt → clippy → build → test. Runs on ubuntu-latest, installs `libasound2-dev` for ALSA. Uses `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2`.

## NOTES

- No CLI argument parsing (no clap/structopt). App launches directly into TUI.
- No config file system. Project persistence only (`.trs` JSON files).
- No workspace — single crate package. No feature flags defined.
- No rustfmt.toml, clippy.toml, or rust-toolchain.toml — uses defaults, enforced by CI.
- `.auto-claude/worktrees/` contains task branch snapshots — ignore for main development.
- `Auto Run Docs/` contains planning/ideation artifacts — not source code.
