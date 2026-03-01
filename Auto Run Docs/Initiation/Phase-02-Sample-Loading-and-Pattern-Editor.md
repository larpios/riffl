# Phase 02: Sample Loading & Pattern Editor

This phase adds real sample loading from disk (WAV, FLAC, OGG via symphonia) and builds the interactive pattern editor so users can enter and edit notes with their keyboard. By the end, users can load a sample file, type notes into the tracker grid, and hear their pattern play back with actual audio samples.

## Tasks

- [x] Implement sample loading from audio files in `src/audio/loader.rs`:
  - Create a `load_sample(path: &Path) -> Result<Sample>` function using symphonia
  - Support WAV, FLAC, and OGG/Vorbis formats (these features are already in Cargo.toml)
  - Decode audio to `Vec<f32>` normalized to -1.0..1.0 range
  - Handle mono and stereo files (convert mono to stereo by duplicating channels)
  - Resample to the engine's output sample rate if the file's rate differs (basic linear interpolation is fine for now)
  - Store the file name in `Sample.name`
  - Register in `src/audio/mod.rs`
  - ✅ Completed: `load_sample(path, target_sample_rate)` decodes via symphonia, mono→stereo conversion, linear interpolation resampling, `LoadError` variant added to `AudioError`

- [x] Write tests for the sample loader:
  - Test loading a programmatically-created WAV file (use `hound` crate or write raw WAV bytes to a temp file)
  - Test that mono files are correctly converted to stereo
  - Test error handling for invalid/missing files
  - Test that the loaded sample has correct metadata (sample rate, channels, duration)
  - ✅ 9 tests total in `loader.rs`: stereo WAV load, mono→stereo conversion (structure + values), missing file, invalid file, metadata, duration calculation, resampling frame count, resample_linear basic

- [x] Run `cargo test` for the sample loader and fix any failures
  - ✅ All 161 tests pass (116 lib + 161 bin, including 9 loader-specific tests). No failures to fix. Warnings are non-blocking (unused imports).

- [x] Build the pattern editor state machine in `src/editor/mod.rs`:
  - `EditorMode` enum: `Normal` (navigation), `Insert` (note entry), `Visual` (selection)
  - `Editor` struct wrapping a `Pattern` with cursor position (row, channel, sub-column), current mode, and edit history
  - Navigation methods: `move_up`, `move_down`, `move_left`, `move_right`, `page_up`, `page_down`, `home`, `end`
  - Note entry: in Insert mode, typing a letter A-G enters a note, number keys set octave, shift+number or separate column for velocity
  - Delete: backspace/delete clears current cell
  - Row operations: insert row (pushes rows down), delete row (pulls rows up)
  - Mode transitions: `i` enters Insert mode, `Escape` returns to Normal mode, `v` enters Visual mode
  - Register the `editor` module in `src/main.rs`
  - ✅ Completed: `EditorMode` enum (Normal/Insert/Visual), `Editor` struct with cursor (row, channel, sub_column), `SubColumn` enum, undo history (max 100), all navigation methods, note entry (A-G → pitch, 0-9 → octave), delete/insert/delete_row, mode transitions, visual selection, `char_to_pitch()`, `clamp_cursor()`. 49 tests covering modes, navigation, sub-column movement in Insert mode, note entry, undo, visual selection, row ops, edge cases. Registered in `src/main.rs` and `src/lib.rs`. All 211 tests pass.

- [x] Integrate the editor into App and update keybindings:
  - Replace the bare cursor/pattern fields in `App` with an `Editor` instance
  - Update `src/input/keybindings.rs` to handle editor modes:
    - Normal mode: hjkl navigation, `i` for insert, `v` for visual, `x` or `Delete` to clear cell, space for play/pause
    - Insert mode: A-G for note entry, 0-9 for octave, Escape to return to normal
  - Update `Action` enum with new actions: `EnterInsertMode`, `EnterNormalMode`, `EnterNote(char)`, `SetOctave(u8)`, `DeleteCell`, `InsertRow`, `DeleteRow`, `PageUp`, `PageDown`
  - ✅ Completed: Replaced `cursor_x`/`cursor_y`/`pattern` in App with `Editor` instance. `map_key_to_action()` now takes `EditorMode` parameter for mode-aware dispatch (Normal/Insert/Visual). Action enum expanded with all new variants. `handle_key_event` in main.rs routes through editor methods. UI footer shows mode indicator (NORMAL/INSERT/VISUAL) with mode-specific keybinding hints. All 224 tests pass (166 lib + 224 bin, 0 failures).

- [x] Update the pattern grid UI rendering to reflect editor state:
  - Show the current editor mode in the footer (NORMAL / INSERT / VISUAL)
  - In Insert mode, highlight the current cell differently (e.g., blinking or different color)
  - Display note columns with proper tracker formatting: `C#4 01 64 ...` (note, instrument, volume, effects)
  - Empty cells show `--- .. .. ...`
  - Channel headers with channel numbers
  - ✅ Completed: Insert mode uses LightMagenta highlight on the active sub-column (Note/Instrument/Volume/Effect) with dark background on inactive sub-columns, clearly distinguishing from Normal mode's yellow highlight. Visual mode selection highlighted in blue. New `format_cell_parts()` splits cell into sub-column strings for per-column styling. Added `insert_cursor_style()`, `insert_inactive_style()`, and `visual_selection_style()` to Theme. 5 new tests for `format_cell_parts`, 3 new theme tests. All 232 tests pass.

- [x] Add a basic sample browser / file picker using a modal:
  - When user presses a designated key (e.g., `F5` or `o`), open a file browser modal
  - The modal lists `.wav`, `.flac`, `.ogg` files in the current directory (or a configurable samples directory)
  - Navigate the file list with j/k, select with Enter
  - Loading a sample adds it to the instrument list and assigns it to the current instrument slot
  - Display loaded instruments in a sidebar or status area
  - ✅ Completed: `FileBrowser` struct in `src/ui/file_browser.rs` with directory scanning, sorted file listing, j/k navigation, Enter to load, Esc to cancel. `o` and `F5` keybindings in Normal mode open the browser. `Mixer::add_sample()` enables runtime sample loading. File browser renders as centered overlay (70×60%) with scrollable file list and selection highlight. On load, sample is decoded via `load_sample()` and added to mixer; success/error shown via modal. Footer shows `o:load` hint and `Inst:N` count. 14 new tests (9 file_browser, 3 mixer, 2 keybinding). All 246 tests pass (169 lib + 246 bin, 0 failures).

- [x] Wire sample playback into the pattern engine:
  - Update the mixer to look up loaded samples (not just the demo sine wave) by instrument index
  - When a note triggers during playback, calculate the playback rate based on the note's pitch relative to the sample's base pitch (C-4 = original rate)
  - Higher notes play faster, lower notes play slower (standard tracker pitch mapping)
  - Support note-off to stop a playing sample
  - ✅ Completed: Added `base_note` field to `Sample` (default C-4 = MIDI 48) with `with_base_note()` builder and `base_frequency()` method. Mixer now uses `sample.base_frequency()` instead of hardcoded 440Hz for pitch calculation. Demo sine sample tagged with base_note A-4 (57). 6 new tests: C-4 original rate, higher-note-plays-faster, custom base_note, instrument lookup by index, note-off stops sample, sample base_frequency. All 252 tests pass (0 failures).

- [ ] Run `cargo test` and `cargo build` to verify all code compiles and tests pass

- [ ] Manual verification: run `cargo run` and confirm:
  - Can navigate the pattern grid in Normal mode
  - Press `i` to enter Insert mode, type notes (e.g., "c", "4" for C-4)
  - Press Escape to return to Normal mode
  - Press space to play/pause — hear the entered notes with the demo sine sample
  - If a real .wav file is available, load it via the file picker and hear it play at entered pitches
