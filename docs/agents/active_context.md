# ACTIVE PROJECT CONTEXT (Handoff)

**Current Date:** 2026-03-24
**Current Phase:** Phase 2 (Renoise-Level Instruments & UI Visualizers)
**Goal:** Implementation of multi-sample mapping, envelopes, and a TUI overhaul.

## 🎯 CURRENT PRIORITIES (P0)

1.  **riffl-3gv: Fix effect command implementations**
    - Focus: Core audio engine interpretation of effect commands (`3xx`, `5xy`, etc.).
    - Reference: `openspec/specs/effect-semantics/spec.md`.
2.  **riffl-agw: Unified Loader Architecture**
    - Focus: Improving how different module formats (XM, IT, S3M) are loaded into the core `Song` model.

## 🏗️ IN PROGRESS (P1)

- **US5.1: Accuracy Audit (XM/IT)**: Verifying playback accuracy against historical trackers.
- **US4.1: Manual Waveform Pencil**: Implementation of sample drawing/editing in TUI.
- **US2.1: Hybrid Layout (Instrument Mode)**: New UI mode for instrument and modulation editing.
- **US3.1: Independent Horizontal Axis (Follow Mode)**: Decoupling track scrolling from current playback position.

## 🧪 NEXT STEPS

- **Implement Envelopes (ADSR)**: The core struct `Instrument` in `tracker-core/src/song.rs` needs support for graphical envelopes.
- **TUI Overhaul**: Custom widgets for oscilloscopes and VU meters in `tracker-tui/src/ui/`.

## ⚠️ BLOCKERS / NOTES

- **None** currently identified.
- **Memory**: Use `bd remember` for session persistence.
- **Reference**: See `docs/agents/reference.md` for CLI and framework reference.

---
*Updated at end of session 2026-03-24.*
