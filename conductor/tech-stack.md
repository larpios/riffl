# Technology Stack: Tracker-rs

## Core Programming Language
- **Rust:** The primary language for the entire project, chosen for its performance, safety, and modern toolchain.

## Audio Engine & Processing
- **CPAL (Cross-Platform Audio Library):** Used for low-latency audio I/O on macOS, Linux, and Windows.
- **Glicol Synth:** The core synthesis and processing library.
- **Symphonia & Hound:** For robust audio file decoding and encoding (WAV, FLAC, OGG).
- **Modularity Note:** The audio architecture is designed to be modular, allowing for future changes to the synthesis engine or I/O backends.

## User Interface (TUI)
- **Ratatui:** The primary framework for building the terminal-based interface.
- **Crossterm:** The cross-platform terminal backend for input and rendering.
- **Modularity Note:** The UI logic is separated from the core engine, facilitating a future transition to a GUI if needed.

## Scripting & Automation
- **Rhai:** Currently used for coding and scripting capabilities.
- **Modularity Note:** The scripting engine is treated as a swappable component. Future exploration may involve other DSLs or languages (e.g., Lua or a custom-built solution).

## Data Persistence & Serialization
- **Serde (JSON & TOML):** Used for file-based serialization of patterns, instruments, and project configurations.

## Architecture & Tooling
- **Cargo Workspace:** To manage the `tracker-core` and `tracker-tui` crates.
- **Strict Modularity:** Every major component (Audio, UI, Scripting) should be accessible through well-defined interfaces to allow for easy replacement or extension.
