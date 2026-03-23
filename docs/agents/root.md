# PROJECT KNOWLEDGE BASE

**Generated:** 2026-03-08
**Workspace:** Cargo workspace with 2 crates

## OVERVIEW

TUI music tracker (Renoise-inspired) with live coding DSL (Rhai scripting), built in Rust. Core stack: ratatui + crossterm (TUI), cpal (audio I/O), symphonia (decoding), hound (WAV export), rhai (scripting).

## STRUCTURE

```
crates/
├── tracker-core/            # Library crate — audio engine, pattern data, DSL, transport
│   └── src/
│       ├── lib.rs           # Crate root (exports: audio, dsl, pattern, transport, song, project, export)
│       ├── transport.rs     # BPM timing, row/pattern advancement, play/stop/pause state machine
│       ├── song.rs          # Song model: pattern pool (max 256), arrangement, instruments
│       ├── project.rs       # Save/load to .trs (JSON via serde_json)
│       ├── export.rs        # Offline WAV rendering through mixer
│       ├── audio/           # Low-latency audio engine (cpal) — see AGENTS.md inside
│       ├── dsl/             # Rhai scripting DSL — see AGENTS.md inside
│       └── pattern/         # Core musical data types — see AGENTS.md inside
├── tracker-tui/             # Binary crate — TUI frontend
│   └── src/
│       ├── main.rs          # Binary entry: terminal lifecycle, event loop, panic hook
│       ├── app.rs           # App state: orchestrates editor, audio, transport, UI, scripting
│       ├── editor/          # Vim-modal pattern editor — see AGENTS.md inside
│       ├── ui/              # Ratatui rendering — see AGENTS.md inside
│       └── input/           # Keybinding dispatch — see AGENTS.md inside
examples/
└── full_demo.rs             # AudioEngine API demo (in tracker-core)
benches/
└── mixer_bench.rs           # Criterion bench for Mixer::new (in tracker-core)
docs/
└── VISION.md                # Roadmap, monetization strategy, phase plan
```

## ARCHITECTURE

**Workspace split:** `tracker-core` is the frontend-agnostic library (audio, pattern, DSL, transport, song, project, export). `tracker-tui` is the TUI frontend that depends on `tracker-core`. This enables future GUI frontends to share the core engine.

**Data flow:**
```
Editor (cursor/input) → Pattern (data model) → Transport (timing) → Mixer (sample mixing) → AudioStream (cpal callback)
                                                                      ↑
DSL/ScriptEngine (Rhai) → generates Pattern data ─────────────────────┘
```

**Key types:** `App` (god object in tracker-tui), `Editor` (modal state machine), `Song` (pattern pool + arrangement), `Transport` (BPM driver returning `AdvanceResult`), `Mixer` (real-time sample mixing), `Pattern → Track → Row → Cell`.

**Cross-crate imports:** tracker-tui files use `use tracker_core::audio`, `use tracker_core::pattern`, etc. TUI-internal imports use `use crate::app`, `use crate::editor`, etc.

**Concurrency:** Audio callback on real-time thread. Shared state via `Arc<Mutex<>>`. Poisoned mutex recovery via `lock_unpoisoned()` helper in `dsl/engine.rs`.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new view/screen | `tracker-tui/src/app.rs` (`AppView` enum) + `ui/` | Add variant, match in `ui/mod.rs` render dispatch |
| Add keybinding | `tracker-tui/src/input/keybindings.rs` | Add `Action` variant, map in `map_key_to_action` |
| New pattern data type | `tracker-core/src/pattern/` | Add to `Cell`/`Row`, update serde derives |
| New audio effect | `tracker-core/src/audio/effect_processor.rs` | Implement in `EffectProcessor` |
| DSL function for scripts | `tracker-core/src/dsl/pattern_api.rs` | Register with Rhai engine |
| Export format | `tracker-core/src/export.rs` | Model after `export_wav` |
| Project persistence | `tracker-core/src/project.rs` | Uses `Song` serde, `.trs` extension |
| New instrument property | `tracker-core/src/song.rs` (`Instrument`) | Update struct + serde |

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
cargo build --workspace --all-features        # Build
cargo test --workspace --all-features         # Run all tests
cargo clippy --workspace --all-features -- -D warnings  # Lint (CI treats warnings as errors)
cargo fmt --all -- --check                    # Format check
cargo fmt --all                               # Auto-format
cargo run -p tracker-tui                      # Launch TUI app
cargo run -p tracker-core --example full_demo # Audio engine demo
cargo bench -p tracker-core                   # Mixer benchmark (criterion)
```

## CI

Single GitHub Actions job (`.github/workflows/ci.yml`): fmt → clippy → build → test. Runs on ubuntu-latest, installs `libasound2-dev` for ALSA. Uses `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2`. All commands use `--workspace` flag.

## NOTES

- No CLI argument parsing (no clap/structopt). App launches directly into TUI.
- No config file system. Project persistence only (`.trs` JSON files).
- Cargo workspace with 2 members: `tracker-core` (lib) and `tracker-tui` (bin). No feature flags defined.
- No rustfmt.toml, clippy.toml, or rust-toolchain.toml — uses defaults, enforced by CI.
- `.auto-claude/worktrees/` contains task branch snapshots — ignore for main development.
- `Auto Run Docs/` contains planning/ideation artifacts — not source code.

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
