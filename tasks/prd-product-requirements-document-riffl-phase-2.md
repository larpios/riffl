# Product Requirements Document: Riffl (Phase 2
 - Renoise-Level Instruments & TUI Overhaul)

**Project Name:** Riffl
 (formerly Ralph)  
**Version:** 1.5.0-PRD  
**Status:** Implementation Roadmap (Technical Specification)  
**
Date:** March 22, 2026  

---

## 1. Executive Summary
Riffl is a high-performance, Rust
-native music tracker designed to bridge the gap between traditional step-sequencing (MOD/XM/IT) and modern live-coding environments. This document focuses on **Phase 2**, which evolves the "Strategic Vision" into a concrete
 technical specification for a **Renoise-level Instrument System** and a complete **TUI Ergonomic Overhaul** to resolve the "Bland UI" issue (`riffl-2qz`).

---

## 2. Phase
 2 Focus: Renoise-Level Instrument System (Goal 1A)
The priority for V1.5 is a robust, sample-focused instrument architecture that moves beyond simple one-shot playback.

### 2.1 Multi-Sample & Keyzone Mapping
*   **Keyzones:** Ability to map
 multiple samples to different MIDI note ranges and velocity layers.
*   **Root Note & Tuning:** Per-sample root note definition and fine-tuning controls.
*   **Slicing:** Integrated sample slicing (procedural or manual) with trigger points mapped to notes.

### 2.2 En
velopes & Modulation
*   **ADSR Envelopes:** Standard Attack-Decay-Sustain-Release envelopes for Amplitude and Filter.
*   **Graphical Envelopes:** Support for multi-point linear/curved envelopes (simulating Renoise’s flexibility).
*   **LFOs
:** Built-in Low-Frequency Oscillators per instrument for Pitch, Volume, and Panning.

### 2.3 Scriptable Logic (Agent Integration)
*   **Rhai-Envelopes:** Scripts can define the "shape" of an envelope in real-time, allowing for procedural modulation that standard
 ADSR cannot achieve.

---

## 3. TUI User Experience & Aesthetic (Goal 2ABC)
To resolve the "Bland UI" critique, the interface will be overhauled with a focus on functional density, ergonomics, and customization.

### 3.1 Functional Density (Visualizers)
*   
**Real-time Oscilloscopes:** Per-track or Master mini-oscilloscopes showing waveform output.
*   **VU Meters:** High-refresh-rate volume meters with peak hold.
*   **Spectrum Analyzers:** A master spectrum analyzer (FFT-based) integrated into the top header or a dedicated overlay
.

### 3.2 Ergonomics & Navigation (Vim-Style)
*   **Command Bar:** A `:` command bar (e.g., `:load`, `:save`, `:bpm 140`) for rapid workflow.
*   **Which-Key Menu:** A discovery overlay that appears
 when a modifier (e.g., `Space`) is held, showing available keybindings.
*   **Modal Editing:** Clear "Insert" vs "Normal" modes for pattern editing, preventing accidental note entry.

### 3.3 Customization & Status Bar (`riffl-2qz`)
*
   **Configurable Status Bar:** The bottom bar will no longer contain hardcoded keybindings. It will show current status (CPU, Memory, Pattern/Row, Selection) and be user-configurable via `config.toml`.
*   **Theme System:** Support for custom color schemes (e.g., Gru
vbox, Nord, Solarized) via a YAML/TOML theme engine.

---

## 4. Live Coding vs. Tracker Workflow (Goal 3B/C)
Riffl will adopt a **Hybrid Procedural Layer** to blend the DSL (Rhai/Strudel-like) with the
 Tracker Grid.

*   **Macro Mode (Destructive):** Scripts can be invoked to "apply" transformations to the current pattern selection (e.g., `euclidean(5, 8)` to write notes into the grid).
*   **Parallel Layers (Non-Destructive):** The DSL can run "
Agents" that trigger notes in parallel with the pattern grid. These agents share the same mixer channels but are not written to the grid, allowing for live improvisation over a static beat.
*   **Effect Commands:** The DSL can be used to define custom "Effect Commands" (e.g., `Zxx`)
 that trigger Rhai scripts for complex DSP modulation.

---

## 5. File Format & Interop (Goal 4B)
*   **Primary Focus: `.rtm` (Riffl Tracker Module):** A MessagePack-based format optimized for speed and version control (Git-friendly).
*   **Legacy Compatibility:** MOD/XM/IT support is planned as an "Import Only" layer for Phase 3, ensuring the native format is perfected first to support the advanced Phase 2 features.

---

## 6. Technical Requirements
*   **Engine (`tracker-core`):** Lock-
free audio thread with sample-accurate scheduling.
*   **TUI (`tracker-tui`):** Built on `ratatui` with custom widgets for oscilloscopes and meters.
*   **Scripting:** `rhai` engine with a hardened `pattern_api.rs` for safe real
-time execution.

---

## 7. Success Metrics
1.  **UI Feedback:** Resolution of the "Bland UI" issue with positive user sentiment regarding the new visualizers.
2.  **Instrument Power:** Ability to reproduce complex Renoise-style multi-sample instruments within the TUI.
3
.  **Performance:** Stable 60FPS UI rendering even with multiple real-time oscilloscopes active.