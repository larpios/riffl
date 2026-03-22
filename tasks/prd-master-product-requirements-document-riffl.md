# Master Product Requirements Document: Riffl

**Project Name:** Riffl (formerly Ralph)  
**Version:** 1.5.0-MASTER  
**Status:** Finalized Implementation Roadmap  
**Date:** March 22, 2026  

---

##
 1. Executive Summary
Riffl is a high-performance, Rust-native music tracker that bridges the gap between traditional step-sequencing (MOD/XM/IT) and modern live-coding environments. It combines a precise, hex-friendly TUI with the generative power of the Rhai scripting engine
. This document establishes the "Strategic Vision" and the immediate "Phase 2" technical specifications for a Renoise-level instrument system and an ergonomic UI overhaul.

---

## 2. Strategic Milestones
*   **Phase 1 (Complete):** Perfection of `tracker-core`, stable `.rtm
` (MessagePack) format, and basic TUI stability.
*   **Phase 2 (Active):** **Renoise-Level Instruments** (Envelopes, Multi-samples) and **UI Visualizers** (Oscilloscopes, VU Meters).
*   **Phase 3:** Deep Live-Coding Integration
 (Rhai-based destructive and non-destructive grid manipulation).
*   **Phase 4:** GUI Transition & Plugin Hosting (CLAP/VST3 support for the "Pro" layer).

---

## 3. Functional Requirements

### 3.1 Renoise-Level Instrument System (Phase 2 Focus)

*   **Multi-Sample Mapping:** Map samples to specific MIDI note ranges (Keyzones) and velocity layers.
*   **Modulation Engine:** 
    *   **Envelopes:** ADSR and Graphical (multi-point) envelopes for Amplitude and Filter.
    *   **LFOs:** Per
-instrument LFOs for Pitch, Volume, and Panning.
*   **Integrated Slicing:** Procedural and manual sample slicing with trigger points mapped to notes.
*   **Scriptable Logic:** Rhai-driven envelopes allowing for real-time procedural modulation shapes.

### 3.2
 TUI Ergonomics & Visuals (`riffl-2qz`)
*   **Functional Density:** Real-time oscilloscopes (per-track), VU meters with peak hold, and a master FFT-based spectrum analyzer.
*   **Vim-Style Workflow:** 
    *   Modal editing (
Normal vs. Insert).
    *   Command Bar (`:`) for rapid operations (`:load`, `:bpm`, etc.).
    *   "Which-Key" discovery menu for keyboard shortcuts.
*   **Customization:** Configurable status bar (CPU/Memory/Pattern info) and a theme engine supporting
 YAML/TOML color schemes.

### 3.3 Hybrid Live-Coding Layer
*   **Macro Mode:** Destructive application of Rhai scripts to pattern selections (e.g., Euclidean rhythms).
*   **Parallel Agents:** Non-destructive Rhai scripts triggering notes alongside the tracker grid.
*   **
Custom Effect Commands:** Ability to trigger Rhai scripts via tracker effect columns (e.g., `Zxx`).

---

## 4. Technical Architecture
*   **Audio Engine (`tracker-core`):** Lock-free, sample-accurate audio thread in Rust; `no-std` compatible core where
 possible.
*   **UI Frontend (`tracker-tui`):** Built on `ratatui` with optimized custom widgets for high-refresh visualizers.
*   **Scripting:** Hardened `rhai` engine with a safe `pattern_api.rs` for real-time grid access.
*
   **File Format:** `.rtm` (Riffl Tracker Module) using MessagePack for high fidelity and Git-friendliness.

---

## 5. Business Model: Open-Core
*   **Community Edition (GPL/MIT):** Fully functional TUI, `.rtm` core
, and legacy format (MOD/XM/IT) import support.
*   **Pro Edition:** Gated features including CLAP/VST3 plugin hosting, advanced DSP effects, and the future graphical UI.

---

## 6. Success Metrics
1.  **UI Performance:** Stable 60FPS
 rendering even with multiple active oscilloscopes.
2.  **Instrument Parity:** Ability to replicate complex Renoise instruments within the TUI.
3.  **Engine Quality:** 100% playback accuracy for historical XM/IT modules in "Compatible" mode.
4.  **Community Growth:** A
 library of 10+ community-contributed Rhai scripts for generative sequencing.