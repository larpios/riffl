# Plan: Implement Sample Loop Points (Milestone 1)

## Objective
Implement loop points support for audio samples, allowing for sustained notes without using large sample data.

## Key Files
- `crates/tracker-core/src/audio/sample.rs`: Add loop fields to `Sample`.
- `crates/tracker-core/src/audio/mixer.rs`: Update `Voice` logic to handle looping during rendering.

## Implementation Steps

### Phase 1: Data Model (crates/tracker-core/src/audio/sample.rs)
1.  **Add `LoopMode` Enum**:
    - `NoLoop`
    - `Forward` (loops from `end` back to `start`)
    - `PingPong` (alternates direction at `start` and `end`)
2.  **Update `Sample` Struct**:
    - `loop_mode: LoopMode`
    - `loop_start: Option<usize>`
    - `loop_end: Option<usize>`
3.  **Update Constructor and Default**:
    - Initialize with `NoLoop`, `None`, `None`.
4.  **Add Builder Methods**:
    - `with_loop(mut self, mode: LoopMode, start: usize, end: usize) -> Self`

### Phase 2: Mixer Logic (crates/tracker-core/src/audio/mixer.rs)
1.  **Update `Voice` Struct**:
    - Add `loop_direction: f64` (1.0 or -1.0) for Ping-Pong support.
2.  **Update `render()` loop**:
    - When `voice.position >= sample_frames`:
        - If `NoLoop`: Deactivate voice (existing behavior).
        - If `Forward`: Wrap `position` back to `loop_start` if `position >= loop_end`.
        - If `PingPong`: Reverse `loop_direction` and clamp to `loop_start` or `loop_end`.

## Verification
1.  **Unit Tests**:
    - Test `Sample` builder methods.
    - Test `Mixer::render` with a short sample and forward/ping-pong loops to ensure it keeps playing beyond the original length.
