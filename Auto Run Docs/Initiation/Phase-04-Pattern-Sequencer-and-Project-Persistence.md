# Phase 04: Pattern Sequencer & Project Persistence

This phase adds the ability to create multiple patterns, arrange them into a song sequence, and save/load entire projects to disk. It also adds copy/paste operations for efficient pattern editing. By the end, users can compose multi-pattern songs, save their work, and reload it later.

## Tasks

- [x] Create the song/arrangement data model in `src/song.rs`:
  - `Song` struct containing:
    - `name: String`
    - `artist: String`
    - `bpm: f64`
    - `patterns: Vec<Pattern>` — the pattern pool (up to 256 patterns)
    - `arrangement: Vec<usize>` — ordered list of pattern indices forming the song sequence
    - `tracks: Vec<Track>` — global track metadata (volume, pan, mute, solo, instrument)
    - `instruments: Vec<Instrument>` — instrument definitions linking to samples
  - `Instrument` struct: `name: String`, `sample_index: Option<usize>`, `base_note: Note` (default C-4), `volume: f32`
  - Methods: `add_pattern()`, `remove_pattern(index)`, `duplicate_pattern(index)`, `reorder_arrangement(from, to)`, `insert_in_arrangement(position, pattern_index)`
  - Register `mod song;` in `src/main.rs`

- [x] Write tests for the Song data model:
  - Test pattern pool management (add, remove, duplicate)
  - Test arrangement manipulation (insert, remove, reorder)
  - Test that removing a pattern updates arrangement indices correctly
  - Test instrument assignment
  > ✅ 21 tests already exist in `src/song.rs` covering all subtasks. All pass.

- [x] Run `cargo test` for the song module and fix any failures
  > ✅ All 21 song tests pass. No failures to fix.

- [x] Implement clipboard and pattern operations in the editor:
  - Add a `Clipboard` struct to hold copied cell data (single cell, row, column, or rectangular selection)
  - Copy operation (`y` or `Ctrl+C`): copy current cell, or selection if in Visual mode
  - Paste operation (`p` or `Ctrl+V`): paste clipboard contents at cursor position
  - Cut operation (`d` in visual mode or `Ctrl+X`): copy + clear
  - Transpose selection: `Shift+Up/Down` transposes selected notes by 1 semitone, `Ctrl+Shift+Up/Down` by 1 octave
  - Interpolate: fill selected column with linear interpolation between first and last values (useful for volume/effect ramps)
  > ✅ Implemented: Clipboard struct in `src/editor/mod.rs`, 9 new Action variants in keybindings, transpose via `Note::transpose()` and `Pitch::from_semitone()`. All 392 tests pass (70 new tests added across editor, keybindings, and note modules).

- [x] Build an arrangement view UI in `src/ui/arrangement.rs`:
  - A separate view (toggled with `F2` or a tab system) showing the song's pattern sequence vertically
  - Each row shows: position number, pattern index, first few notes as preview
  - Navigation: j/k moves between arrangement positions
  - Operations: Enter to jump to/edit that pattern, `a` to append a pattern, `d` to remove, `n` to create new empty pattern
  - Visual indicator of current playback position during song playback
  - Register in `src/ui/mod.rs`
  > ✅ Implemented: `ArrangementView` struct with cursor navigation, pattern append/remove/create operations, `render_arrangement()` function with scroll, playback position highlighting, and pattern note preview. Song and ArrangementView added to App. 23 new tests (415 total pass).

- [ ] Add a view/tab switching system to the App:
  - `AppView` enum: `PatternEditor`, `Arrangement`, `InstrumentList`
  - F1 = Pattern Editor, F2 = Arrangement, F3 = Instrument List
  - Each view has its own render function and keybinding context
  - Status bar shows which view is active
  - Update `src/ui/mod.rs` to dispatch to the correct renderer based on active view

- [ ] Implement project save/load using serde and JSON (add `serde`, `serde_json` to Cargo.toml):
  - Derive `Serialize`/`Deserialize` on all data model structs: `Song`, `Pattern`, `Note`, `Pitch`, `Cell`, `Row`, `Track`, `Instrument`
  - `save_project(path: &Path, song: &Song) -> Result<()>` — serialize song to JSON and write to `.trs` file
  - `load_project(path: &Path) -> Result<Song>` — read and deserialize from `.trs` file
  - Sample data is NOT embedded — store file paths as references. Samples are loaded from their original paths on project load.
  - Put save/load functions in `src/project.rs`
  - Keybindings: `Ctrl+S` saves, `Ctrl+O` opens a file picker modal to load

- [ ] Write tests for project save/load:
  - Test round-trip: create a Song with patterns and notes, save to temp file, load back, assert equality
  - Test that all note/pattern data survives serialization
  - Test error handling for corrupt/missing files

- [ ] Run `cargo test` and `cargo build` to verify everything compiles

- [ ] Update the transport to support song-level playback:
  - When playing a song, advance through the arrangement sequence (pattern after pattern)
  - At end of arrangement: stop (or loop back to beginning if loop mode is on)
  - Display current arrangement position + pattern row in the header
  - Allow jumping to a specific arrangement position
