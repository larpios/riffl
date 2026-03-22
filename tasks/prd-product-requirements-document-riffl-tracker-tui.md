# Product Requirements Document: Riffl (Tracker-TUI)


**Project Name:** Riffl (formerly Ralph)
**Version:** 1.0.0-PRD
**Status:** Strategic Vision & Core Implementation
**Date:** March 22, 2026

---

## 1. Executive Summary
Riffl is a high-performance, Rust-native music tracker designed to bridge the gap between traditional step-sequencing (MOD/XM/IT) and modern live-coding environments. It targets
 power users who require the precision of a tracker grid with the generative flexibility of a scripting engine (Rhai). Riffl operates as a split-architecture system: `tracker-core` (audio engine/DSL) and `tracker-tui` (terminal interface).

## 2. Strategic Vision (Goal 1A)

The long-term objective is to evolve Riffl from a robust TUI-based tracker into a cross-platform GUI and plugin suite (CLAP/VST3), establishing a new industry standard for "Programmable Trackers."

### 2.1 Project Milestones
- **Phase 1 (Current):** Perfection
 of `tracker-core`, `.rtm` format, and TUI stability.
- **Phase 2:** Advanced Instrument System (Renoise-level envelopes/multi-samples).
- **Phase 3:** Live-coding integration (Rhai-based grid manipulation).
- **Phase 4:**
 GUI Transition & Plugin Hosting (The "Pro" Layer).

---

## 3. Functional Requirements

### 3.1 Core Tracker Features
- **Grid-Based Sequencing:** Standard Pattern/Order-list workflow.
- **Advanced Effect Commands:** Full support for legacy effect parity (Arpeggio, Vibr
ato, Portamento) and Riffl-specific extended commands.
- **Instrument System:** 
    - Support for multi-samples.
    - **Hybrid Logic (Goal 2C):** Rhai scripting integration for complex instrument behaviors, dynamic envelopes, and procedural modulation.

### 3.2
 File Formats & Compatibility (Goal 4C)
- **Native Format (`.rtm`):** A high-fidelity, JSON or MessagePack-based format designed for version control and Riffl-specific features.
- **Import Modes:**
    - **Riffl Mode:** Native processing, full features enabled
.
    - **Compatible Mode:** Strict adherence to legacy MOD/XM/IT behavior to ensure 1:1 playback of historical modules.

### 3.3 Audio Engine (`tracker-core`)
- **Sample-Accurate Timing:** Jitter-free playback regardless of UI load.
- **Plugin Hosting (
Pro):** Capability to host CLAP and VST3 instruments/effects.
- **Export:** High-quality WAV/FLAC rendering and per-track stem export.

---

## 4. User Interface (TUI)
- **Visual Aesthetic:** Move beyond "bland UI" using sophisticated
 TUI primitives (Ratatui-based).
- **Interactive Status Bar:** Real-time feedback on CPU usage, BPM, current pattern, and engine state.
- **Modal Editing:** Vim-inspired keybindings for rapid grid navigation and pattern manipulation.

---

## 5. Technical Requirements & Architecture
- **Language:** Rust (Stable).
- **Architecture:** 
    - `tracker-core`: No-std compatible core logic where possible; lock-free audio thread.
    - `tracker-tui`: Terminal-based frontend using `ratatui`.
- **Scripting Engine:** Rhai for
 live-coding and instrument logic.
- **State Management:** Real-time synchronization between the UI thread and the Audio thread using crossbeam channels or atomic rings.

---

## 6. Monetization & Licensing (Goal 3A)
Riffl follows an **Open-Core** model:
- **Community
 Edition:** Fully functional TUI, `.rtm` core, and legacy format support. Open-source (GPL/MIT).
- **Pro Edition (Gated Features):** 
    - CLAP/VST3 plugin hosting.
    - Advanced DSP effects.
    - Commercial support/GUI enhancements
.
- **License Enforcement:** Minimalist license-key validation for "Pro" features without DRM-bloat.

---

## 7. Success Metrics
1. **Engine Parity:** 100% playback accuracy for XM/IT test modules in "Compatible" mode.
2. **Performance
:** Sub-5% CPU usage for a 64-channel module on modern hardware.
3. **Community:** Successful integration of at least 10 community-contributed Rhai scripts for generative music.