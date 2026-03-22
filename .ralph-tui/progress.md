# Ralph Progress Log

This file tracks progress across iterations. Agents update this file
after each iteration and it's included in prompts for context.

## Codebase Patterns (Study These First)

- When adding new `Action` enum variants, ensure they're added to ALL three `ActionMetadata` trait implementations: `name()`, `description()`, and `category()`. Missing any one causes compile errors.
- Pre-existing bugs unrelated to your fix may block CI. Check if build failures existed before your changes.
- Pattern tests use `Effect::from_type(EffectType, param)` for creating effect cells.

*Add reusable patterns discovered during development here.*

---

## 2026-03-23 - riffl-j60.4
- Added 9 comprehensive tests for mixer tick/render cycle
- Files changed:
  - `crates/tracker-core/src/audio/mixer.rs`: Added 9 new tests covering:
    - `test_mixer_arpeggio_effect_changes_pitch`: Arpeggio 0xy effect
    - `test_mixer_portamento_slide_changes_pitch`: Portamento 3xx effect
    - `test_mixer_volume_column_applied`: Volume column (vxx) via Cell
    - `test_mixer_tpl_change_affects_timing`: TPL change verification
    - `test_mixer_voice_stealing_on_new_note`: Voice replacement on same channel
    - `test_mixer_set_volume_effect_cxx`: Set volume Cxx effect
    - `test_mixer_sample_offset_9xx`: Sample offset 9xx effect
    - `test_mixer_tremor_effect`: Tremor effect Txy
    - `test_mixer_set_panning_8xx`: Panning 8xx effect
- **Learnings:**
  - TPL changes update `ticks_per_row` but not `frames_per_row` (BPM-driven)
  - Tremor effect sets state (`tremor_active`, `tremor_on`, `tremor_off`) not audio output
  - Voice stealing is per-channel - new note on same channel replaces existing voice
---

## 2026-03-23 - riffl-j60.2
- Fixed non-exhaustive match errors for `GoToStart` and `GoToEnd` actions in `keybindings.rs`
- Added `GoToStart`/`GoToEnd` to `name()`, `description()`, and `category()` match arms
- Also fixed pre-existing bug: added `set_cursor_channel()` method to `Editor` struct (was being called in `app.rs` but didn't exist)
- Files changed:
  - `crates/tracker-tui/src/input/keybindings.rs`: Added match arms for `GoToStart` and `GoToEnd` in 3 locations
  - `crates/tracker-tui/src/editor/mod.rs`: Added `set_cursor_channel()` setter method
- **Learnings:**
  - When adding Action variants, must update ALL three trait methods (`name`, `description`, `category`)
  - Always verify if build errors are pre-existing vs introduced by your changes
---

## 2026-03-22 - riffl-j60.1
- Fixed 3 clippy warnings in tracker-core audio engine
- Files changed:
  - `crates/tracker-core/src/audio/effect_processor.rs`: Changed `.get(0)` to `.first()` (2 occurrences)
  - `crates/tracker-core/src/audio/mixer.rs`: Used `.is_multiple_of()` instead of modulo, collapsed nested if, added `#[allow(dead_code)]` for `playback_rate`
- **Learnings:**
  - `.is_multiple_of()` is available in Rust 1.79+ for u32 types - prefer this over manual modulo checks
  - When collapsing nested if statements, carefully track closing braces to avoid mismatch
  - `#[allow(dead_code)]` on struct fields allows dead field without full struct annotation
---

