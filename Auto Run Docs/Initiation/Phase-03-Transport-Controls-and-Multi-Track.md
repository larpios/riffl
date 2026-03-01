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

- [x] Integrate the transport into App, replacing the ad-hoc playback state:
  - Replace `is_playing`, `current_row`, `bpm` fields in `App` with a `Transport` instance
  - Update the main event loop to call `transport.advance(delta_time)` each tick
  - When transport advances to a new row, trigger the mixer to process that row
  - Update keybindings:
    - Space: toggle play/pause
    - Escape (during playback): stop and return cursor to row 0
    - `+`/`-` or `F1`/`F2`: adjust BPM up/down by 1 (hold shift for ±10)
    - `L`: toggle loop mode
  - *Completed: Replaced `is_playing`, `current_row`, `bpm`, `last_row_time` fields with `Transport` instance and `last_update: Instant` for delta-time calculation. `update()` now calls `transport.advance(delta)` and handles auto-stop when loop is disabled. Added `toggle_play()` with proper Stopped→Playing, Playing→Paused, Paused→Playing transitions. Added `stop()`, `adjust_bpm()`, `toggle_loop()` methods. New Action variants: `Stop`, `BpmUp`, `BpmDown`, `BpmUpLarge`, `BpmDownLarge`, `ToggleLoop`. Keybindings: `=`/`+`/F2 for BPM +1, `-`/F1 for BPM -1, Shift+F1/F2 for ±10, Shift+L for loop toggle. Escape during playback stops transport. Updated UI header to show PLAYING/PAUSED/STOPPED state with distinct colors and [LOOP] indicator. All 273 tests pass.*

- [x] Update the UI to display transport information:
  - Header bar shows: BPM value, play/pause/stop state icon, current row/total rows, loop indicator
  - During playback, the pattern view auto-scrolls to keep the current playback row visible
  - Playback row highlighted with a distinct color (e.g., bright green bar) separate from cursor highlight
  - When stopped, cursor and playback position are independent
  - *Completed: Added Unicode transport icons (▶ ⏸ ⏹) to header status line. Pattern view auto-scrolls to follow playback row during Playing state, and follows editor cursor when Stopped/Paused. Playback row gets full-width green background bar (Black text on Green bg) distinct from cursor highlight (LightGreen bg when overlapping). Channel separators and trailing spaces also get green bg for seamless playback bar. When paused, playback position remains highlighted so user can see where playback will resume. 3 new scroll tests added. All 276 tests pass, build succeeds.*

- [x] Extend the pattern data model for multi-track support:
  - Each `Pattern` already has channels — ensure it supports at least 8 channels
  - Add `Track` metadata struct in `src/pattern/track.rs`: `name: String`, `volume: f32` (0.0-1.0), `pan: f32` (-1.0 to 1.0), `muted: bool`, `solo: bool`, `instrument_index: Option<usize>`
  - Add `tracks: Vec<Track>` to `Pattern` or a new `Song` struct that holds patterns + track metadata
  - Solo logic: if any track is soloed, only soloed tracks produce audio
  - *Completed: Created `src/pattern/track.rs` with `Track` struct containing all specified fields (name, volume 0.0-1.0, pan -1.0 to 1.0, muted, solo, instrument_index), value clamping on set_volume/set_pan, toggle_mute/toggle_solo methods, and `is_audible(any_soloed)` implementing solo logic (muted always silent; if any track soloed, only soloed tracks audible). Added `any_track_soloed()` helper. Updated `DEFAULT_CHANNELS` from 4 to 8. Added `tracks: Vec<Track>` to `Pattern` struct, auto-created in `Pattern::new()` with numbered names. Added pattern-level accessors: `tracks()`, `tracks_mut()`, `get_track()`, `get_track_mut()`, `any_track_soloed()`, `is_channel_audible()`. 17 unit tests in track.rs + 7 integration tests in pattern.rs. All 300 tests pass.*

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
