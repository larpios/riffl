# Plan: Implement Sample Default Volume (Milestone 1)

## Objective
Ensure that the `Instrument`'s default volume setting is correctly applied as a multiplier to the note velocity during playback.

## Key Files
- `crates/tracker-core/src/audio/mixer.rs`: Update `tick()` to apply instrument volume.

## Implementation Steps

### Phase 1: Mixer Logic (crates/tracker-core/src/audio/mixer.rs)
1.  **Update `tick()`**:
    - When triggering a note (`NoteEvent::On`), retrieve the corresponding `Instrument`.
    - Retrieve the `instrument.volume` (0.0 - 1.0).
    - Multiply the existing `velocity_gain` (derived from note velocity) by `instrument.volume`.
    - Formula: `final_gain = (note.velocity / 127.0) * instrument.volume`.

## Verification
1.  **Unit Tests**:
    - Add a test case to `audio::mixer` where a note is triggered using an instrument with `volume < 1.0`.
    - Verify the resulting voice's `velocity_gain` is correctly scaled.
