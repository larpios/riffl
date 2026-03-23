# Implementation Plan: Effect Command Semantics, Compatibility Modes, and Effect Help

This plan addresses the inconsistent handling of effect commands by introducing a central effect registry, project-level compatibility modes, and enhanced editor feedback (status bar and detailed help).

## Proposed Changes

### 1. `tracker-core`: Central Effect Registry and Model

#### `crates/tracker-core/src/pattern/effect.rs`
- Define `EffectMode` enum: `RifflNative`, `Compatible`.
- Define `EffectRegistry` to store metadata for effects:
    - Identity (command code)
    - Parameters (name, range, parsing logic)
    - Continuation behavior (e.g., does `00` mean "continue"?)
    - Metadata: Name, short description, long description.
- Update `EffectType` to be more descriptive and support compatibility-only effects.
- Implement a lookup system to get metadata for a given `(command, param, mode)`.

#### `crates/tracker-core/src/song.rs`
- Add `effect_mode: EffectMode` to the `Song` struct.
- Default to `RifflNative` for new songs.

### 2. `tracker-core`: Fixed Playback Interpretation

#### `crates/tracker-core/src/audio/effect_processor.rs`
- Update `TrackerEffectProcessor` to aware of the `EffectMode`.
- Refactor `process_row` to use the `EffectRegistry` for command interpretation.
- Specifically handle continuation cases (like `300`) based on the `EffectMode` and previous channel state.
- Ensure that "speed 0" (if it's a continuation) does not stop playback but continues the previous effect.

### 3. `tracker-core`: Import Semantics

#### `crates/tracker-core/src/format/*.rs` (xm, it, s3m, protracker)
- Update importers to set the `EffectMode` to `Compatible` by default for foreign formats.
- Refactor effect conversion logic to map to the new typed effect model.
- Preserve source-specific continuation semantics when in `Compatible` mode.

### 4. `tracker-tui`: Editor Feedback

#### `crates/tracker-tui/src/editor/mod.rs`
- Add a method to `Editor` to get the current effect's help text based on the cursor position and `EffectMode`.

#### `crates/tracker-tui/src/ui/mod.rs` (Status Bar)
- Update the status bar rendering to show the effect summary when the cursor is on an effect sub-column.

#### `crates/tracker-tui/src/input/keybindings.rs`
- Add `Shift+K` binding for a new `ShowEffectHelp` action.

#### `crates/tracker-tui/src/app.rs` & `crates/tracker-tui/src/ui/help.rs` (or new file)
- Implement the `ShowEffectHelp` action to open a modal or a dedicated view showing detailed effect information.

## Phased Implementation Plan

### Phase 1: Core Registry & Project Mode (US-001, US-003)
1. Define `EffectMode` and add it to `Song`.
2. Create the initial `EffectRegistry` in `tracker-core`.
3. Update `EffectType` and `Effect` to support the new model.
4. Verify project saving/loading with the new mode.

### Phase 2: Playback & Import (US-002, US-004, US-005, US-006)
1. Refactor `TrackerEffectProcessor` to use the registry.
2. Fix `300` and other continuation cases in playback.
3. Update importers (starting with XM and ProTracker) to use `Compatible` mode and the new effect mappings.
4. Add regression tests for playback and import.

### Phase 3: TUI Integration (US-007, US-008)
1. Implement status bar effect summary.
2. Add `Shift+K` help view.
3. Verify TUI feedback matches playback behavior.

## Verification Plan

### Automated Tests
- **Unit Tests (`tracker-core`)**:
    - `EffectRegistry` lookup for various modes and commands.
    - `Song` serialization with `EffectMode`.
    - `TrackerEffectProcessor` processing of `300` in both modes.
    - Importers correctly setting mode and mapping effects.
- **Integration Tests**:
    - Load an XM file with `3xx` effects and verify `TrackerEffectProcessor` state after multiple rows.

### Manual Verification
1. Open an existing project (should default to Riffl Native).
2. Import an XM file (should default to Compatible).
3. Navigate to effect cells and verify status bar updates.
4. Press `Shift+K` on various effects (native, compatibility, unknown) and verify help content.
5. Listen to imported modules to ensure portamento continuation works as expected.
