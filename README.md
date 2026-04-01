# Riffl

![Screenshot](assets/screenshot.png)

[![Ko-fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/larpios)

> [!CAUTION]
> This project is still in development.

Riffl is a work-in-progress music tracker written in Rust.

## 🛠️ Current Status & Features

The project is in early development. It currently includes:

- **Module Playback:** Support for loading and playing `.mod`, `.s3m`, `.xm`, and `.it` files.
- **Audio Engine:** A custom mixer using `cpal` for output and `glicol` for internal routing.
- **FM Synthesis:** An integrated OPL2/3 emulator (via `opl-emu`) for AdLib-style sounds.
- **Interface:** A terminal-based UI built with `ratatui`.
- **Scripting:** Experimental `rhai` integration for generating patterns via code.
- **Nix Support:** Flake-based development environment and build pipeline.

> [!CAUTION]
> These features can still be buggy as the project is still in development.

## 🎯 Project Identity



- **Tracker Workflow:** Precise, hex-friendly, and highly ergonomic TUI interface.
- **Rust Powered:** Built for performance, safety, and low-latency audio.

## ⚡ Quick Start

### Prerequisites
- **Rust:** Install via [rustup.rs](https://rustup.rs/)
- **Audio Libraries:**
  - **macOS/Windows:** No additional dependencies.
  - **Linux (Debian/Ubuntu):** `sudo apt-get install libasound2-dev`
  - **Linux (Fedora):** `sudo dnf install alsa-lib-devel`

### Install

The project is still in development, but if you want to try it out, you can install it like so:

```bash
cargo install --git https://github.com/larpios/riffl riffl-tui
```

### Build & Run

You can also just clone the repository and build and run the project like so:

```bash
cargo run -p riffl-tui
```

## ☕ Support the Project

If you find Riffl useful and would like to support its development, you can buy me a coffee!

[![Ko-fi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/larpios)

