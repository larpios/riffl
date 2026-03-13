# Plan: Implement Missing ProTracker Effects (Milestone 1)

## Objective
Implement the missing ProTracker-compatible effect commands to achieve Milestone 1 parity.

### Missing Effects
- `5xy` — Tone Portamento + Volume Slide
- `6xy` — Vibrato + Volume Slide
- `7xy` — Tremolo
- `9xx` — Sample Offset
- `Exy` — Extended Effects (E1x–EEx)

## Key Files & Context
- `crates/tracker-core/src/pattern/effect.rs`: `EffectType` enum and mappings.
- `crates/tracker-core/src/audio/effect_processor.rs`: `ChannelEffectState` and `TrackerEffectProcessor` logic.
- `crates/tracker-core/src/audio/mixer.rs`: `Voice` and `Mixer` logic for `SampleOffset`.

## Implementation Steps

### Phase 1: Data Model (crates/tracker-core/src/pattern/effect.rs)
1.  **Add Enum Variants**:
    - `TonePortamentoVolumeSlide` (0x5)
    - `VibratoVolumeSlide` (0x6)
    - `Tremolo` (0x7)
    - `SampleOffset` (0x9)
    - `Extended` (0xE)
2.  **Update Mappings**:
    - `from_command(u8) -> Option<Self>`
    - `to_command(self) -> u8`
    - `mnemonic(&self) -> &'static str`
3.  **Tests**: Update `tests/test_effect_type_from_command` and others to include new variants.

### Phase 2: State Tracking (crates/tracker-core/src/audio/effect_processor.rs)
1.  **Update `ChannelEffectState`**:
    - Add `tremolo_phase: f64`, `tremolo_speed: u8`, `tremolo_depth: u8`, `tremolo_active: bool`, `tremolo_waveform: u8`.
    - Add `sample_offset: Option<usize>`.
    - Add `vibrato_waveform: u8`.
    - Add `glissando: bool`.
    - Add `pattern_loop_start_row: Option<usize>`, `pattern_loop_count: u8`.
2.  **Update `new_row()`**: Reset transient per-row flags.

### Phase 3: Effect Logic (crates/tracker-core/src/audio/effect_processor.rs)
1.  **Implement `5xy` & `6xy`**:
    - Use existing `portamento_speed` and `vibrato_speed/depth`.
    - Apply `VolumeSlide` logic concurrently.
2.  **Implement `7xy`**:
    - Add `tremolo_pitch_ratio()` and `advance_tremolo()` similar to vibrato.
3.  **Implement `9xx`**:
    - Set `sample_offset` state.
4.  **Implement `Exy` Sub-commands**:
    - Handle `param >> 4` (sub-command) and `param & 0x0F` (sub-param).
    - Handle `FinePortamento`, `Glissando`, `Waveform`, `Finetune`, `PatternLoop`, `Retrigger`, `NoteCut`, `NoteDelay`, `PatternDelay`.

### Phase 4: Mixer Integration (crates/tracker-core/src/audio/mixer.rs)
1.  **Handle `SampleOffset`**:
    - Update `Voice::new()` or `Mixer::trigger_note()` to apply the offset from `ChannelEffectState`.

## Verification & Testing
1.  **Unit Tests**:
    - Test each new `EffectType` in `effect.rs`.
    - Test `TrackerEffectProcessor::process_row` for each command.
    - Test `ChannelEffectState` modulation functions (vibrato/tremolo).
2.  **Manual Verification**:
    - Use the TUI to input these effects and verify the audio output (if possible via automated tests or listening).
