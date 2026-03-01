# Phase 06: Live Coding DSL Integration

This phase delivers the core differentiator of tracker-rs: a live coding scripting engine that can generate and manipulate patterns programmatically. Using Rhai as the embedded scripting language, users get a code editor panel alongside the tracker, where they can write scripts that generate notes, create algorithmic patterns, and transform existing pattern data in real time. This is where tracker-rs becomes more than a tracker — it becomes a hybrid instrument.

## Tasks

- [x] Add Rhai scripting engine dependency and create the DSL module:
  - Add `rhai = "1"` to Cargo.toml
  - Create `src/dsl/mod.rs` with module declarations
  - Create `src/dsl/engine.rs`:
    - `ScriptEngine` struct wrapping a Rhai `Engine` instance
    - Register custom types with Rhai: `Note`, `Pattern`, `Pitch`
    - Register built-in music functions:
      - `note(pitch_str, octave)` — create a Note (e.g., `note("C", 4)`)
      - `scale(root, mode, octave)` — return array of notes in a scale (major, minor, pentatonic, blues, dorian, mixolydian)
      - `chord(root, quality, octave)` — return array of notes forming a chord (major, minor, 7th, maj7, dim, aug)
      - `random_note(scale_notes)` — pick a random note from an array
      - `euclidean(pulses, steps)` — generate a Euclidean rhythm as array of booleans
      - `random(min, max)` — random integer in range
      - `random_float()` — random float 0.0-1.0
    - `eval(code: &str) -> Result<ScriptResult>` — execute code and return result
  - Register `mod dsl;` in `src/main.rs`

- [x] Create pattern manipulation functions accessible from scripts in `src/dsl/pattern_api.rs`:
  - `set_note(pattern, row, channel, note)` — place a note in the pattern
  - `clear_pattern(pattern)` — clear all cells
  - `fill_column(pattern, channel, notes_array)` — fill a channel with a repeating note sequence
  - `generate_beat(pattern, channel, euclidean_array, note)` — place notes where euclidean rhythm is true
  - `transpose(pattern, semitones)` — shift all notes by N semitones
  - `reverse(pattern)` — reverse row order
  - `rotate(pattern, offset)` — circular shift rows
  - `humanize(pattern, timing_variance, velocity_variance)` — add subtle randomness to velocity
  - All functions return a modified pattern (functional style) or modify in-place

- [ ] Write tests for the DSL engine:
  - Test note creation: `note("C", 4)` produces correct Note
  - Test scale generation: major scale from C4 returns correct 7 notes
  - Test chord generation: C major chord returns C, E, G
  - Test euclidean rhythm: `euclidean(3, 8)` returns expected pattern
  - Test pattern manipulation: set_note, transpose, reverse
  - Test error handling: invalid code produces helpful error messages, not panics

- [ ] Run `cargo test` for the DSL module and fix any failures

- [ ] Build the code editor panel UI in `src/ui/code_editor.rs`:
  - A text editor widget for writing Rhai scripts
  - Line numbers on the left
  - Basic syntax highlighting: keywords (let, if, for, fn, return) in one color, strings in another, numbers in another, comments (//) dimmed
  - Cursor navigation with arrow keys, Home/End, Page Up/Down
  - Text editing: typing inserts characters, Backspace/Delete removes, Enter creates new line
  - Multi-line support with vertical scrolling
  - Display area for script output/errors below the editor
  - Register in `src/ui/mod.rs`

- [ ] Integrate the code editor as a split view:
  - Add `CodeEditor` view to `AppView` enum (accessible via `F4`)
  - Add a split-screen mode: `Ctrl+\` toggles between full pattern view and 50/50 split (pattern left, code right)
  - When in code editor, the pattern view still shows and updates live
  - `Ctrl+Enter` executes the current script
  - Script output (generated pattern data) is applied to the current pattern or a preview pattern
  - Error messages from script execution displayed in the output area with line numbers

- [ ] Create a set of example scripts that ship as built-in templates:
  - Store as string constants in `src/dsl/examples.rs`
  - Accessible via a menu in the code editor (e.g., `Ctrl+T` for templates)
  - Templates include:
    - "Simple Beat" — 4/4 kick-snare pattern using euclidean rhythms
    - "Random Melody" — random notes from a pentatonic scale
    - "Arpeggiator" — cycle through chord tones across rows
    - "Probability Beat" — notes placed with random probability
  - Each template includes comments explaining what it does

- [ ] Wire live script execution to the audio engine:
  - When a script generates or modifies a pattern, the changes should be immediately audible if playback is active
  - Scripts run on a separate evaluation context (not blocking the audio thread)
  - If a script modifies the current playing pattern, changes take effect on the next loop iteration
  - Add a "live mode" toggle where scripts auto-re-evaluate on every pattern loop

- [ ] Run `cargo test` and `cargo build` to verify everything compiles and passes

- [ ] Manual verification with `cargo run`:
  - Open the code editor panel with F4 or Ctrl+\
  - Type a simple script: `let n = note("C", 4); set_note(pattern, 0, 0, n);`
  - Execute with Ctrl+Enter — see the note appear in the pattern grid
  - Try a template script — hear it play back
  - Verify error messages appear for invalid scripts
  - Test live mode: modify a playing pattern's script and hear changes on next loop
