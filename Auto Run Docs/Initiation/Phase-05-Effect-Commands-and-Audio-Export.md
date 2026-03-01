# Phase 05: Effect Commands & Audio Export

This phase implements tracker effect commands (volume slides, pitch slides, arpeggio, etc.) and adds the ability to render/export the song to WAV or FLAC audio files. By the end, users have expressive per-row effects and can export their compositions as audio files.

## Tasks

- [x] Define the effect command system in `src/pattern/effect.rs`:
  - `Effect` struct: `command: u8` (effect type), `value: u8` (parameter)
  - Standard tracker effect types enum:
    - `0xy` Arpeggio — cycle between base note, +x semitones, +y semitones
    - `1xx` Pitch slide up by xx units per row
    - `2xx` Pitch slide down by xx units per row
    - `3xx` Portamento to note (slide to target note at speed xx)
    - `4xy` Vibrato (speed x, depth y)
    - `Axy` Volume slide (x = up speed, y = down speed)
    - `Bxx` Position jump (jump to arrangement position xx)
    - `Cxx` Set volume to xx
    - `Dxx` Pattern break (jump to row xx of next pattern)
    - `Fxx` Set speed/BPM
  - Display formatting: effect shows as 3 hex chars (e.g., "A04", "C40", "F78")
  - Add `effects: Vec<Effect>` (up to 2 per cell) to the `Cell` struct
  - Register in `src/pattern/mod.rs`

- [ ] Write tests for effect parsing and display:
  - Test hex display formatting
  - Test that effect values are correctly encoded/decoded
  - Test integration with Cell — effects stored and retrieved correctly

- [ ] Run `cargo test` for effects and fix any failures

- [ ] Implement effect processing in the mixer/playback engine:
  - Create `src/audio/effect_processor.rs` with per-channel effect state
  - Track running state per channel: current pitch offset, volume, vibrato phase, portamento target
  - Process effects each row tick:
    - Arpeggio: modify pitch lookup on sub-ticks
    - Volume slide: adjust channel volume incrementally
    - Pitch slides: adjust playback rate incrementally
    - Portamento: slide toward target note
    - Vibrato: oscillate pitch with LFO
    - Set volume: immediate volume change
    - Set BPM: update transport tempo
    - Position jump / pattern break: signal transport to change position
  - Register in `src/audio/mod.rs`

- [ ] Update the pattern editor UI to display and edit effects:
  - Each cell now shows: `C#4 01 64 A04` (note, instrument, volume, effect)
  - Effect column is navigable — cursor can move to the effect sub-column
  - In Insert mode on effect column: type hex digits (0-9, A-F) to enter effect commands
  - Show effect mnemonics in a help bar when cursor is on effect column

- [ ] Implement audio export in `src/export.rs`:
  - `export_wav(path: &Path, song: &Song, samples: &[Sample], sample_rate: u32) -> Result<()>`
  - Offline rendering: process the entire song row-by-row through the mixer without real-time constraints
  - Write output to WAV file using the `hound` crate (add to Cargo.toml)
  - Support configurable sample rate (44100, 48000) and bit depth (16-bit, 24-bit)
  - Progress callback for UI integration (percentage complete)
  - Register `mod export;` in `src/main.rs`

- [ ] Add an export UI flow:
  - Keybinding: `Ctrl+E` opens export dialog modal
  - Modal shows: output path (default: `<project_name>.wav`), sample rate selection, bit depth selection
  - Confirm starts export, showing a progress indicator
  - On completion, display success message with file path and duration

- [ ] Write tests for audio export:
  - Test that exporting a simple pattern produces a valid WAV file
  - Test that the WAV file has correct metadata (sample rate, channels, duration)
  - Test that silence exports as near-zero samples
  - Test that a pattern with notes produces non-zero audio data

- [ ] Run `cargo test` and `cargo build` to verify everything compiles and passes
