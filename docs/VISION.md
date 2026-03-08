# Tracker-rs Vision & Roadmap

**Last updated:** 2026-03-08

## What We're Building

A full-featured DAW with tracker-style interface and live coding capabilities (inspired by Strudel/TidalCycles), built in Rust. Not just a tracker — a complete music production environment where tracker workflow and algorithmic composition coexist.

### Core Identity

- **Tracker-first:** Pattern grid is the primary editing interface. Effect commands, step sequencing, hex values — the tracker DNA stays.
- **Live coding built-in:** Rhai scripting engine for algorithmic pattern generation, generative music, real-time composition. This is the differentiator no commercial DAW has.
- **Two frontends:** TUI (current, great for tracker workflow) and GUI (future, needed for waveforms, plugin UIs, automation curves).

### What Sets Us Apart

| Us | Traditional DAWs | Traditional Trackers |
|----|-------------------|----------------------|
| Tracker grid + live coding DSL | Piano roll + mouse workflow | Tracker grid only |
| Rust (fast, safe, no GC) | C++ (legacy, unsafe) | C/C++ |
| TUI + GUI dual frontend | GUI only | GUI only |
| Open-source core | Closed source | Mostly closed |
| Modern plugin hosting (CLAP-first) | VST2/3 + AU | Limited/none |

---

## Architecture Plan

### Target Structure (Cargo Workspace)

```
tracker-rs/
├── crates/
│   ├── tracker-core/        # Engine: audio, mixer, transport, pattern, song, DSL, project I/O
│   │   └── src/
│   │       ├── audio/       # cpal engine, mixer, effects, sample loading
│   │       ├── pattern/     # Note, Cell, Row, Track, Pattern
│   │       ├── dsl/         # Rhai scripting engine
│   │       ├── transport.rs # BPM, playback, row advancement
│   │       ├── song.rs      # Song model, instruments
│   │       ├── project.rs   # Save/load (.trs)
│   │       ├── export.rs    # WAV/FLAC rendering
│   │       ├── midi/        # MIDI I/O (future)
│   │       ├── automation/  # Automation lanes (future)
│   │       └── plugin/      # CLAP/VST3 hosting (future)
│   ├── tracker-tui/         # TUI frontend (ratatui + crossterm)
│   │   └── src/
│   │       ├── main.rs
│   │       ├── app.rs       # TUI-specific app state
│   │       ├── ui/          # Rendering
│   │       ├── input/       # Keybindings
│   │       └── editor/      # Editor state (cursor, modes)
│   └── tracker-gui/         # GUI frontend (future)
│       └── src/
├── docs/
├── Cargo.toml               # Workspace root
└── ...
```

### Key Architectural Principle

**The core is the product. Frontends are skins.**

`tracker-core` must be 100% frontend-agnostic. No ratatui types, no crossterm types, no UI concepts. It exposes:
- A `Song` model (patterns, instruments, arrangement)
- A `Transport` (play/stop/pause, BPM, position)
- A `Mixer` (audio rendering)
- A `ScriptEngine` (Rhai DSL evaluation)
- An `AudioEngine` (device management, stream output)

Both TUI and GUI import `tracker-core` and build their own UI/input layer on top.

---

## Development Phases

### Phase 1: Architecture Split
**Goal:** Refactor monolith into workspace (tracker-core + tracker-tui). No new features.
**Effort:** ~1-2 weeks
**Outcome:** Clean separation. Same functionality. Both `cargo run -p tracker-tui` and using `tracker-core` as a library work.
**Key work:**
- Extract core types from App god object
- Move audio/, pattern/, dsl/, transport, song, project, export into tracker-core
- Move ui/, input/, editor/, main.rs, app.rs into tracker-tui
- tracker-tui depends on tracker-core
- All existing tests pass

### Phase 2: Audio Engine Upgrades
**Goal:** Professional-grade mixing and effects.
**Effort:** ~2-4 weeks
**Outcome:** Channel strip architecture, built-in effects, improved sample engine.
**Key work:**
- Mixer rewrite: channel strips with volume/pan/mute/solo + insert effect chain
- Send/return buses (e.g., send track to reverb bus)
- Built-in effects: EQ (parametric), compressor, delay, reverb, filter
- Sample engine: multi-sample instruments, loop points, ADSR envelope
- Sample browser/manager

### Phase 3: MIDI + Automation
**Goal:** External controller support and parameter automation.
**Effort:** ~2-3 weeks
**Outcome:** MIDI in/out, automation lanes alongside pattern data.
**Key work:**
- MIDI I/O via `midir` crate
- MIDI learn (map controller knobs to parameters)
- MIDI file import/export
- Automation lane data model: per-track, per-parameter breakpoint curves
- Automation playback: interpolated per-tick in transport
- Tracker effect commands and automation curves coexist (commands override curves)

### Phase 4: Arrangement + Song Structure
**Goal:** Full song arrangement beyond pattern sequencing.
**Effort:** ~2-3 weeks
**Outcome:** Timeline view, pattern clips, markers, song sections.
**Key work:**
- Arrangement timeline: each track has pattern clips at arbitrary positions
- Pattern clips can overlap, loop, be muted
- Marker system: named positions (intro, verse, chorus)
- Loop regions for playback
- Song-level vs pattern-level editing modes

### Phase 5: Plugin Hosting (the hard one)
**Goal:** Host third-party audio plugins.
**Effort:** ~4-8 weeks
**Outcome:** CLAP and VST3 plugin scanning, loading, processing, UI hosting.
**Key work:**
- CLAP hosting via `clack-host` crate (preferred — modern, Rust-friendly)
- VST3 hosting via `vst3-sys` (ecosystem compatibility)
- Plugin scanning and caching
- Plugin parameter exposure to automation system
- Plugin UI hosting (requires GUI — this phase may overlap with Phase 6)
- Audio plugin processing integrated into mixer channel strips

### Phase 6: GUI Frontend
**Goal:** Graphical frontend sharing tracker-core.
**Effort:** ~6-10 weeks
**Outcome:** Full GUI with tracker grid, piano roll, waveforms, automation lanes, plugin windows.
**Key work:**
- Framework selection (egui, Iced, or Tauri — decide based on Phase 1-5 experience)
- Tracker grid rendering (pixel-perfect, themed)
- Waveform display (sample editor, arrangement)
- Automation lane editor (draggable breakpoints)
- Piano roll (optional — some users prefer tracker-only)
- Plugin GUI window hosting
- Theme system

### Phase 7: Live Coding Power
**Goal:** Strudel-level expressiveness in the scripting DSL.
**Effort:** ~3-5 weeks (ongoing)
**Outcome:** Pattern combinators, mini-notation, generative composition.
**Key work:**
- Mini-notation parser (Strudel/TidalCycles-style: `"bd sd [hh hh] cp"`)
- Pattern combinators: stack, cat, fast, slow, rev, every, sometimes
- Scale/chord awareness in pattern transformations
- Live-eval: modify playing patterns in real-time without stopping playback
- Script-controlled automation
- REPL mode alongside tracker editing

---

## Monetization Strategy

### Phase A: Community Building (now - Phase 3)
- **Open source** the full project (MIT or dual MIT/Apache-2.0)
- **GitHub Sponsors / Patreon** for early supporters
- Build community around the live-coding-meets-tracker niche
- Share progress on music production forums, TidalCycles community, Renoise forums
- YouTube demos of live coding with the tracker

### Phase B: Open-Core Transition (Phase 4-5)
- Core tracker + DSL stays open source forever
- **Paid features** (one-time license, ~$50-80 range like Renoise):
  - VST3/CLAP plugin hosting
  - Advanced effects (convolution reverb, multiband compressor)
  - Commercial export formats (MP3, AAC)
  - Multi-output routing
- Trial version: full features, export watermark or time-limited sessions

### Phase C: Ecosystem Revenue (Phase 6+)
- **Paid sample packs / instrument packs** curated for the tracker workflow
- **Premium DSL script library** — community marketplace for generative scripts
- **Commercial license** for embedding tracker-core in other products

### Pricing Reference Points
| Product | Price | Model |
|---------|-------|-------|
| Renoise | $75 (one-time) | Paid with free demo |
| Bitwig | $99-399 | Tiered one-time |
| Reaper | $60-225 | One-time, honor system trial |
| Strudel | Free | Donations/grants |
| VCV Rack | Free + paid plugins | Open-core |

**Target:** Renoise model. ~$75 one-time license for pro features. Free open-source core.

---

## Current State (as of 2026-03-08)

### What Exists
- TUI tracker with vim-modal editing (Normal/Insert/Visual modes)
- Pattern model: Note, Cell, Row, Track, Pattern (up to 256 patterns, 4 channels default)
- Audio engine: cpal-based, real-time mixer, ~5ms latency
- Effects: volume, panning, pitch slide, arpeggio, vibrato, tremolo, and more
- Rhai scripting DSL: note generation, chords, scales, euclidean rhythms
- Transport: BPM-driven playback with row/pattern advancement
- Project save/load (.trs JSON)
- WAV export
- Code editor with syntax display for Rhai scripts
- Arrangement view (pattern sequencing)
- ~30k lines of Rust, 79 source files

### What's Missing (vs the vision)
- [ ] Workspace architecture (monolith currently)
- [ ] Professional mixer (channel strips, sends/returns)
- [ ] Built-in effects (EQ, compressor, reverb, delay)
- [ ] MIDI I/O
- [ ] Automation lanes
- [ ] Full arrangement timeline
- [ ] Plugin hosting (CLAP/VST3)
- [ ] GUI frontend
- [ ] Advanced DSL (mini-notation, pattern combinators)
- [ ] Sample browser / multi-sample instruments
- [ ] Undo/redo beyond pattern snapshots

### Technical Debt
- `App` is a god object (~1400 lines) mixing core state and UI concerns
- Dual crate roots (main.rs + lib.rs both declare modules) — intentional but will be resolved by workspace split
- `#![allow(dead_code, unused_imports)]` in main.rs
- Many `unwrap()` calls that should use `?` or `lock_unpoisoned()`

---

## Notes for Future Sessions

- This file is the single source of truth for project direction.
- Update the "Current State" section after each significant milestone.
- Phase estimates are rough — adjust based on actual velocity.
- GUI framework decision deferred until Phase 5 experience informs the choice.
- The live coding angle is the competitive moat — never deprioritize it.
