# Plan: Implement Sample Finetune (Milestone 1)

## Objective
Implement sample finetune support to allow for precise pitch adjustments at the instrument level and via the `E5x` effect command.

## Key Files
- `crates/tracker-core/src/song.rs`: Add `finetune` to `Instrument`.
- `crates/tracker-core/src/audio/effect_processor.rs`: Update `ChannelEffectState` to track finetune override.
- `crates/tracker-core/src/audio/mixer.rs`: Integrate finetune into playback rate calculation.

## Implementation Steps

### Phase 1: Data Model (crates/tracker-core/src/song.rs)
1.  **Update `Instrument` Struct**:
    - Add `finetune: i8` (range -8 to +7, per ProTracker convention).
2.  **Update `new()` and `Default`**:
    - Initialize `finetune` to `0`.
3.  **Add Builder Method**:
    - `with_finetune(mut self, finetune: i8) -> Self`.

### Phase 2: Effect State (crates/tracker-core/src/audio/effect_processor.rs)
1.  **Update `ChannelEffectState`**:
    - Add `finetune_override: Option<i8>`.
2.  **Update `new_row()`**:
    - Reset `finetune_override` to `None`.
3.  **Implement `E5x` logic in `process_row()`**:
    - Map `param & 0x0F` to -8..+7 and store in `finetune_override`.

### Phase 3: Mixer Integration (crates/tracker-core/src/audio/mixer.rs)
1.  **Update `tick()`**:
    - Retrieve `Instrument` finetune.
    - Retrieve `EffectProcessor` finetune override.
    - Calculate effective finetune (override takes priority).
    - Apply to `playback_rate` using formula: `playback_rate *= 2.0_f64.powf(finetune as f64 / (12.0 * 8.0))`.

## Verification
1.  **Unit Tests**:
    - Test `Instrument` finetune property.
    - Test `E5x` effect updates state.
    - Test `Mixer` correctly adjusts pitch based on finetune.
