# Phase 03: Transport Controls & Multi-Track Support

This phase builds proper transport controls (play, stop, pause, BPM adjustment, pattern looping) and extends the single-channel pattern into multi-track support with per-track volume, mute, and solo. By the end, users can compose multi-channel patterns with independent instruments per track and control playback precisely.

## Tasks

- [x] Implement a proper transport system in `src/transport.rs`:
  - `Transport` struct with state machine: `Stopped`, `Playing`, `Paused`
  - Fields: `bpm: f64` (range 20-999), `current_row: usize`, `current_pattern: usize`, `loop_enabled: bool`, `tick_accumulator: f64`
  - `advance(delta_time: f64) -> Option<usize>` — returns the next row index when it's time to advance based on BPM, or None if not yet time
  - BPM-to-row-timing: at 120 BPM with 4 rows per beat (speed 6 in tracker terms), each row lasts ~125ms
  - `play()`, `stop()`, `pause()`, `toggle_play_pause()` methods
  - `set_bpm(bpm)`, `adjust_bpm(delta)` for tempo control
  - Register `mod transport;` in `src/main.rs`
  - *Completed: Created `src/transport.rs` with `TransportState` enum (Stopped/Playing/Paused), `Transport` struct with all specified fields and methods, BPM clamping (20-999), accumulator-based row timing at 4 rows/beat, loop/no-loop end-of-pattern behavior, and registered module in `src/main.rs`. 15 unit tests included inline and all pass.*

- [x] Write tests for the transport system:
  - Test state transitions (Stopped → Playing → Paused → Playing → Stopped)
  - Test BPM timing accuracy: at 120 BPM, rows advance at correct intervals
  - Test row wrapping at pattern boundary when looping
  - Test BPM range clamping
  - *Completed: All 15 inline tests in `src/transport.rs` already cover these requirements comprehensively — state transitions (4 tests), BPM timing accuracy (2 tests), row wrapping with/without loop (2 tests), BPM range clamping (2 tests), plus additional edge cases (advance when not playing, toggle loop, set_num_rows clamping, stop resets position, initial state). All 15 tests pass.*

- [x] Run `cargo test` for transport and fix any failures
  - *Completed: All 267 tests pass (0 failures), including all 15 transport-specific tests. No fixes needed. Warnings present (unused imports, dead code) but no errors.*

- [ ] Integrate the transport into App, replacing the ad-hoc playback state:
  - Replace `is_playing`, `current_row`, `bpm` fields in `App` with a `Transport` instance
  - Update the main event loop to call `transport.advance(delta_time)` each tick
  - When transport advances to a new row, trigger the mixer to process that row
  - Update keybindings:
    - Space: toggle play/pause
    - Escape (during playback): stop and return cursor to row 0
    - `+`/`-` or `F1`/`F2`: adjust BPM up/down by 1 (hold shift for ±10)
    - `L`: toggle loop mode

- [ ] Update the UI to display transport information:
  - Header bar shows: BPM value, play/pause/stop state icon, current row/total rows, loop indicator
  - During playback, the pattern view auto-scrolls to keep the current playback row visible
  - Playback row highlighted with a distinct color (e.g., bright green bar) separate from cursor highlight
  - When stopped, cursor and playback position are independent

- [ ] Extend the pattern data model for multi-track support:
  - Each `Pattern` already has channels — ensure it supports at least 8 channels
  - Add `Track` metadata struct in `src/pattern/track.rs`: `name: String`, `volume: f32` (0.0-1.0), `pan: f32` (-1.0 to 1.0), `muted: bool`, `solo: bool`, `instrument_index: Option<usize>`
  - Add `tracks: Vec<Track>` to `Pattern` or a new `Song` struct that holds patterns + track metadata
  - Solo logic: if any track is soloed, only soloed tracks produce audio

- [ ] Update the mixer to handle multi-track audio:
  - Process each channel/track independently
  - Apply per-track volume and pan
  - Implement mute/solo filtering
  - Mix all track outputs into the final stereo buffer
  - Pan law: equal-power panning (-3dB center)

- [ ] Update the pattern editor UI for multi-track display:
  - Show track headers above each channel column with track number and instrument name
  - Tab key moves cursor between tracks (channels)
  - Add keybindings for track operations:
    - `M` (in normal mode): toggle mute on current track
    - `S` (in normal mode): toggle solo on current track
  - Visual indicators for muted (dimmed text) and soloed (highlighted header) tracks
  - Horizontal scrolling if pattern has more tracks than fit on screen

- [ ] Run `cargo test` and `cargo build` to verify everything compiles and passes

- [ ] Manual verification with `cargo run`:
  - Create a pattern with notes in multiple tracks
  - Adjust BPM with keyboard shortcuts
  - Play/pause/stop works correctly
  - Mute/solo individual tracks during playback
  - Pattern loops when loop mode is enabled
  - Auto-scroll follows playback position
