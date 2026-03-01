# Phase 01: Audio Engine & Core Data Model

This phase integrates the existing audio engine code from the unmerged `auto-claude/002-audio-engine-with-cpal` branch into the main TUI application, builds the pattern data model, and wires them together so that by the end, the tracker can play a hardcoded demo pattern through real speakers. This is the critical foundation — after this phase, the app launches, shows a tracker grid, and produces sound.

## Tasks

- [x] Integrate the audio engine modules from the `auto-claude/002-audio-engine-with-cpal` branch into the existing `src/audio/` directory on `main`. Do NOT merge the branch — cherry-pick the code manually to avoid conflicts with `main.rs` and other files:
  - Copy the contents of these files from the branch (use `git show auto-claude/002-audio-engine-with-cpal:src/audio/<file>`) into the corresponding paths on main:
    - `src/audio/device.rs` — audio device enumeration and selection via cpal
    - `src/audio/engine.rs` — high-level AudioEngine API (new, play, stop, pause, resume, with_callback)
    - `src/audio/error.rs` — AudioError and AudioResult types
    - `src/audio/stream.rs` — AudioStream, StreamBuilder, StreamConfig, AudioCallback
  - Update `src/audio/mod.rs` to declare all submodules (`device`, `engine`, `error`, `stream`, `sample`) and re-export the public API
  - Keep the existing `src/audio/sample.rs` as-is (it's already on main)
  - Verify the code compiles with `cargo check`
  > ✅ Completed: The audio engine was already merged via PR #2. Restored TUI main.rs (PR merge had replaced it with a test tone demo), added sample.rs module, updated mod.rs with all 5 submodules (device, engine, error, sample, stream) and re-exports, added lib.rs for example compatibility, fixed layout.rs Rc<[Rect]> iteration issue. All 80 tests pass.

- [ ] Create the pattern data model in `src/pattern/` with proper types for a tracker grid. Reference the data structures from the `auto-claude/005-basic-pattern-editor` branch's `src/pattern.rs` for inspiration, but restructure into a clean module:
  - `src/pattern/mod.rs` — module declarations and re-exports
  - `src/pattern/note.rs` — `Pitch` enum (C through B with sharps/flats), `Note` struct (pitch, octave 0-9, velocity 0-127, instrument index), `NoteOff` sentinel, display formatting as "C#4" style strings
  - `src/pattern/row.rs` — `Cell` struct (Option<Note>, Option<u8> instrument, Option<u8> volume, effect commands), `Row` as a Vec<Cell> across channels
  - `src/pattern/pattern.rs` — `Pattern` struct with configurable rows (default 64) and channels (default 4), methods: `get_cell(row, channel)`, `set_cell(row, channel, cell)`, `set_note(row, channel, note)`, `clear_cell(row, channel)`, `insert_row(at)`, `delete_row(at)`, `num_rows()`, `num_channels()`
  - Register the `pattern` module in `src/main.rs` with `mod pattern;`

- [ ] Write unit tests for the pattern data model:
  - Test `Note` creation, display formatting, and parsing from strings like "C#4", "A-5"
  - Test `Pattern` construction with default and custom dimensions
  - Test cell get/set/clear operations
  - Test row insert/delete and boundary conditions
  - Test that pattern dimensions are enforced (no out-of-bounds panics)

- [ ] Run `cargo test` and fix any compilation errors or test failures in the pattern module

- [ ] Create a simple audio mixer/sequencer in `src/audio/mixer.rs` that connects patterns to the audio engine:
  - `Mixer` struct holding a reference to loaded samples (Vec<Sample>) and current playback state
  - `tick(row_index, pattern) -> Vec<f32>` method that reads the current row from a pattern, looks up samples by instrument index, and mixes their audio data into a stereo output buffer
  - Basic sample playback: when a note triggers, start reading from the sample's audio data at the appropriate position
  - Simple volume scaling based on note velocity (0-127 mapped to 0.0-1.0)
  - Register in `src/audio/mod.rs`

- [ ] Integrate audio engine into the App struct and create a demo playback path:
  - Add `AudioEngine` (wrapped in an Option for graceful fallback if no audio device) to `App` in `src/app.rs`
  - Add transport state fields to `App`: `is_playing: bool`, `current_row: usize`, `bpm: f64` (default 120.0), `tick_counter` for timing
  - Add a demo `Pattern` with a few hardcoded notes (e.g., C4, E4, G4, C5 in a simple 16-row sequence) to `App::new()`
  - Add a demo `Sample` — generate a simple sine wave programmatically (440Hz, 0.25s duration, 44100Hz sample rate) as the demo instrument so no external files are needed
  - Wire spacebar in keybindings to toggle `is_playing`
  - In the main event loop (`src/main.rs`), when `is_playing` is true, advance `current_row` based on BPM timing and feed the pattern row to the mixer to produce audio through the engine
  - The goal: press space → hear a simple melodic pattern loop

- [ ] Update the TUI to show tracker-relevant information instead of the demo 10x10 grid:
  - Replace the placeholder grid in `src/ui/mod.rs` with a pattern grid renderer that displays the demo pattern's notes (rows × channels)
  - Each cell should display note name (e.g., "C-4", "---" for empty) in fixed-width columns
  - Show row numbers on the left (00-63 in hex, tracker convention)
  - Highlight the current playback row when playing
  - Keep the cursor navigation working (hjkl moves through the pattern grid)
  - Update the header to show "tracker-rs" title, BPM, and play/stop status
  - Update the footer to show relevant keybindings (space=play, q=quit, hjkl=navigate)

- [ ] Run `cargo build` and `cargo test` to verify everything compiles and all tests pass. Fix any issues.

- [ ] Run the application with `cargo run` and verify the end-to-end experience:
  - App launches with the tracker grid visible
  - Arrow keys / hjkl navigate the pattern
  - Spacebar starts/stops playback
  - Audio plays through speakers (the sine wave demo pattern)
  - `q` quits cleanly
  - If audio fails to initialize (e.g., no audio device in CI), the app should still launch and display the UI without crashing — just skip audio playback gracefully
