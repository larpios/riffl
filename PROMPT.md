# Riffl: Rust Music Tracker ⚡

Riffl is an ambitious music tracker blending the structured pattern workflow of a traditional tracker with the expressive live-coding power of [Strudel](https://strudel.cc/).

## 🎯 Project Identity
- **Goal:** Really capable, visually appealing, and ergonomic TUI/GUI app.
- **Core:** Low-latency Rust audio engine, Rhai DSL for live coding.
- **Inspiration:** Strudel, Renoise, ProTracker, Furnace, OpenMPT.
- **Genres:** Capable of anything from Dariacore to Breakcore or YTPMVs.

## 🏗️ Architecture
- **Single Source of Truth:** [docs/ROADMAP.org](docs/ROADMAP.org)
- **Engine-Agnostic Core:** `crates/tracker-core`
- **Frontends:** `crates/tracker-tui` (current), `crates/tracker-gui` (planned).

## 🛡️ AI Mandates & Workflows
- **Issue Tracking:** ALWAYS use **bd (beads)** for ALL task/issue tracking. Do NOT use markdown TODOs.
- **Atomic Commits:** Follow conventional commits after each task.
- **Token Efficiency:** Use `rtk` to delegate commands (cargo, git, etc).
- **Documentation:** ALWAYS update [docs/ROADMAP.org](docs/ROADMAP.org) when milestones or tasks change.

## 🔍 Navigation
- **Architecture & Roadmap:** [docs/ROADMAP.org](docs/ROADMAP.org)
- **Dev Guidelines:** [AGENTS.md](AGENTS.md)
- **Testing:** [ACCEPTANCE_TEST.md](ACCEPTANCE_TEST.md)

## Requirements
- Read the [docs/ROADMAP.org](docs/ROADMAP.org) to figure out things to do
- Make sure to update the [docs/ROADMAP.org](docs/ROADMAP.org)
- Make atomic commits as you go
