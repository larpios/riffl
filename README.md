# Riffl

Riffl is an ambitious Rust-based music tracker that blends the structured pattern workflow of a traditional tracker with the expressive live-coding power of [Strudel](https://strudel.cc/).

## 🎯 Project Identity

- **Tracker Workflow:** Precise, hex-friendly, and highly ergonomic TUI interface.
- **Live Coding:** Integrated Rhai scripting engine for algorithmic pattern generation.
- **Rust Powered:** Built for performance, safety, and low-latency audio.

## ⚡ Quick Start

### Prerequisites

- **Rust:** Install via [rustup.rs](https://rustup.rs/)
- **Audio Libraries:**
  - **macOS/Windows:** No additional dependencies.
  - **Linux (Debian/Ubuntu):** `sudo apt-get install libasound2-dev`
  - **Linux (Fedora):** `sudo dnf install alsa-lib-devel`

### Build & Run

```bash
cargo run -p tracker-tui
```

## 🏗️ Architecture

Riffl is organized as a Cargo workspace:
- `crates/tracker-core`: The headless engine (audio, mixer, transport, DSL).
- `crates/tracker-tui`: The terminal-based user interface.
- `crates/tracker-gui`: Future graphical frontend (planned).

## 🗺️ Roadmap & Vision

For the detailed development plan, technical architecture, and monetization strategy, see [docs/roadmap.org](docs/roadmap.org).

## 🧪 Testing & Quality

We maintain a high bar for audio quality and performance.
- Run tests: `cargo test`
- Run acceptance suite: `./run_acceptance_tests.sh`
- Check [ACCEPTANCE_TEST.md](ACCEPTANCE_TEST.md) for details.

## 🤝 Contributing

See [AGENTS.md](AGENTS.md) for development guidelines and our issue tracking protocol using `bd`.
