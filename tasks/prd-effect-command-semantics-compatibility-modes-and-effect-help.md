# PRD: Effect Command Semantics, Compatibility Modes, and Effect Help

## Overview

Riffl currently handles effect commands inconsistently, especially for imported tracker data. A concrete example is `300`, which in XM-style semantics represents tone portamento continue, but is currently treated as speed `0`. This causes incorrect playback behavior and makes future UI work around effect meaning harder to build.

This feature defines a central Riffl effect model and registry, fixes import/playback interpretation, introduces explicit project-level effect modes (`Riffl native` and `Compatible mode`), and adds first-pass effect help in the TUI: a status-bar summary plus a `Shift+K` help view for the selected effect cell.

## Goals

- Correct effect-command interpretation during import and playback.
- Eliminate inconsistent effect semantics across import, storage, playback, and UI help.
- Establish a central effect registry as the source of truth for Riffl effect definitions.
- Support both `Riffl native` and `Compatible mode` at the project level.
- Preserve source-tracker continuation semantics in `Compatible mode`.
- Provide immediate in-editor effect meaning feedback for the selected cell.
- Provide a deeper help surface accessible with `Shift+K`.

## Quality Gates

These commands must pass for every user story:

- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-features -- -D warnings`
- `cargo fmt --all -- --check`

## User Stories

### US-001: Define a central typed effect model and registry
**Description:** As a Riffl developer, I want one central typed effect model and registry so that importers, playback, editor help, and future docs all use the same effect definitions.

**Acceptance Criteria:**
- [ ] A central effect registry exists in `tracker-core` for Riffl-native effect definitions.
- [ ] The registry defines effect identity, parameters, continuation behavior, metadata, and help text.
- [ ] The internal runtime/editor model is typed rather than relying only on raw numeric interpretation.
- [ ] The registry is usable by import, playback, status-bar help, and detailed help view code.
- [ ] Public APIs added for this model are documented.

### US-002: Fix playback interpretation of effect commands
**Description:** As a user, I want playback to interpret effect commands consistently so that imported and edited patterns sound correct.

**Acceptance Criteria:**
- [ ] Playback no longer misinterprets continuation-style effects such as `300` as unrelated commands like speed `0`.
- [ ] Continuation behavior is handled according to effect definition rather than ad hoc numeric checks.
- [ ] Existing supported effect commands execute through the typed interpretation layer.
- [ ] Unsupported effects do not crash playback.
- [ ] Tests cover at least one continuation case and one non-continuation case.

### US-003: Add project-level effect modes
**Description:** As a user, I want each project to choose between `Riffl native` and `Compatible mode` so that I can prioritize either Riffl semantics or source-tracker fidelity.

**Acceptance Criteria:**
- [ ] Projects can store and load an effect mode setting.
- [ ] Supported modes are `Riffl native` and `Compatible mode`.
- [ ] Mode selection is project-level, not per pattern.
- [ ] Existing projects load with a documented default mode.
- [ ] Mode choice affects import and interpretation behavior in a deterministic way.

### US-004: Support compatible import semantics
**Description:** As a user importing tracker files, I want `Compatible mode` to preserve source effect meaning so that playback matches the imported module as closely as possible.

**Acceptance Criteria:**
- [ ] Importers translate source effects into Riffl’s typed model while preserving source semantics where supported.
- [ ] If a source format defines a code as continuation, imported data preserves that continuation meaning in `Compatible mode`.
- [ ] Compatibility-only effects can exist when no exact Riffl-native effect exists.
- [ ] Compatibility effects execute according to source-format semantics when supported.
- [ ] Tests cover at least one imported effect whose source semantics differ from naive numeric interpretation.

### US-005: Define native import conversion behavior
**Description:** As a user importing into `Riffl native` mode, I want source effects converted into Riffl-native effects so that the project behaves as a native Riffl composition even if round-trip fidelity is lost.

**Acceptance Criteria:**
- [ ] In `Riffl native` mode, import converts supported external effects to native Riffl effects.
- [ ] When conversion changes behavior from the source format, that change is documented in the effect metadata/help surface.
- [ ] Exact source-faithful behavior is not required in native mode.
- [ ] Import conversion rules are deterministic and test-covered.

### US-006: Handle non-exact native mappings safely
**Description:** As a user, I want effects without an exact native equivalent to degrade predictably so that I understand when behavior is approximate.

**Acceptance Criteria:**
- [ ] In `Compatible mode`, effects without an exact Riffl-native equivalent may remain as compatibility-only effects.
- [ ] In `Riffl native` mode, import converts to the nearest native effect only when an explicit mapping exists.
- [ ] Approximate native conversions are labeled clearly as approximate in UI/help.
- [ ] If no explicit native mapping exists, the system preserves enough information to show the user what was imported.
- [ ] Unknown or unsupported values are preserved as raw data rather than discarded.

### US-007: Show effect meaning in the status bar
**Description:** As a pattern editor user, I want to see the meaning of the selected effect cell in the status bar so that I can understand commands while navigating.

**Acceptance Criteria:**
- [ ] When the cursor is on an effect cell, the status bar shows the effect label, parsed parameters, and a short explanation of playback behavior.
- [ ] The summary is generated from the same central effect registry/interpretation layer used by playback/import.
- [ ] Compatibility-only and approximate-conversion cases are labeled clearly.
- [ ] Unknown effects show a clear fallback message instead of blank or misleading text.
- [ ] Moving the cursor updates the displayed effect meaning.

### US-008: Add detailed effect help via `Shift+K`
**Description:** As a user, I want to press `Shift+K` on an effect cell to open a dedicated help view so that I can inspect the command in more detail.

**Acceptance Criteria:**
- [ ] `Shift+K` from an effect cell opens a dedicated effect-help view/page.
- [ ] The help view shows effect name, parsed parameters, and a human-readable description of playback behavior.
- [ ] The help view indicates whether the effect is native, compatibility-only, unknown, or an approximate conversion.
- [ ] The help content is sourced from the same central effect metadata as the status bar.
- [ ] The view can be exited cleanly back to the editor.

## Functional Requirements

1. FR-1: The system must define a central Riffl effect registry in `tracker-core` as the source of truth for native effect definitions.
2. FR-2: The system must represent effect commands internally with a typed model that can express command identity, parameter bytes, continuation semantics, and metadata.
3. FR-3: Importers must translate external tracker effects through the typed model instead of relying on raw numeric interpretation alone.
4. FR-4: The system must support a project-level effect mode with values `Riffl native` and `Compatible mode`.
5. FR-5: In `Compatible mode`, imported effects must preserve source-tracker semantics as faithfully as supported by Riffl.
6. FR-6: In `Compatible mode`, if a source effect has no exact native equivalent, the system may keep it as a compatibility-only effect.
7. FR-7: In `Riffl native` mode, imported effects must convert to native Riffl effects wherever explicit conversion rules exist.
8. FR-8: Approximate native conversions must be labeled clearly to the user.
9. FR-9: Unknown or unsupported effect values must be preserved as raw/imported data and surfaced clearly in UI/help.
10. FR-10: Playback must execute effects according to the typed interpretation layer and effect mode, not ad hoc numeric special cases.
11. FR-11: The editor must show effect meaning in the status bar for the currently selected effect cell.
12. FR-12: The editor must provide a `Shift+K` action that opens a dedicated effect-help view for the selected effect cell.
13. FR-13: Status-bar summaries and help view content must be derived from the same effect registry and interpretation logic used by playback/import.
14. FR-14: The system must provide enough metadata for future expansion to richer effect documentation.

## Non-Goals

- Full parity with every tracker’s entire effect set in this release.
- Perfect round-trip preservation across all foreign formats in `Riffl native` mode.
- A permanently visible inspector pane in v1.
- Full long-form effect documentation with examples for every command in the first release.
- Pattern-level mode switching.
- Solving unrelated editor display work outside effect help/status support.

## Technical Considerations

- Likely touch points include `tracker-core/src/pattern/`, `tracker-core/src/song.rs`, `tracker-core/src/project.rs`, importer logic, playback/effect execution paths, and `tracker-tui` editor/UI/input modules.
- The effect registry should live in `tracker-core`, with `tracker-tui` consuming interpretation/help APIs rather than duplicating logic.
- Persistence changes must maintain backward compatibility for older `.trs` projects where possible.
- Tests should emphasize import semantics, continuation behavior, mode-specific behavior, and UI-facing interpretation APIs.
- The design should avoid subsystem-specific effect definitions drifting apart.

## Success Metrics

- Imported continuation effects such as `300` no longer produce incorrect playback behavior.
- Playback, status-bar help, and detailed help agree on the same interpretation for the same effect cell.
- Projects can switch between `Riffl native` and `Compatible mode` with predictable results.
- Approximate conversions and compatibility-only effects are visible to users rather than hidden.
- Test coverage exists for representative import and interpretation edge cases.

## Open Questions

- What exact set of initial Riffl-native effects should be formalized in the first registry version?
- Which external tracker/module formats are in scope for the first compatibility pass?
- What is the default mode for newly created projects versus newly imported projects?
- How much detail should the `Shift+K` help page include in v1 beyond label, parameters, and behavior summary?
- Should there be a later user-facing command to convert an existing compatible project into native effects explicitly?