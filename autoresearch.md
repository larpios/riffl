# Autoresearch: Riffl Music Tracker Feature Development

## Objective

Advance Riffl toward Renoise-level capability by systematically implementing
features from the roadmap. Focus on Milestone 2.5 (Pattern Editor & Transport
Essentials) first — these are blocking for real composition use.

## Metrics

- **Primary**: Roadmap TODO items completed (count, higher is better)
- **Current Best**: 7 (baseline)
- **Secondary**: test pass rate, build success, subjective UX quality

## Benchmark Command

```bash
cargo test 2>&1
```

Parse: test result: ok. N tests; 0 failed → success.
Count TODO items remaining in docs/roadmap.org for Milestone 2.5 as primary metric.

## Files in Scope

- `crates/tracker-tui/src/input/keybindings.rs` — Key-to-action mapping (normal/insert/visual modes)
- `crates/tracker-tui/src/editor/mod.rs` — Pattern editor state machine (note entry, undo, modes)
- `crates/tracker-tui/src/app.rs` — App state, action dispatch, transport wiring
- `crates/tracker-tui/src/ui/help.rs` — Help screen / cheatsheet overlay
- `crates/tracker-tui/src/ui/pattern_list.rs` — Pattern list view
- `crates/tracker-core/src/song.rs` — Song/instrument/pattern model
- `crates/tracker-core/src/transport.rs` — Transport (BPM, playback, row advance)
- `crates/tracker-core/src/audio/mixer.rs` — Audio mixer / voice render
- `crates/tracker-core/src/audio/effect_processor.rs` — Effect command processing
- `docs/roadmap.org` — Feature roadmap (must be updated as items complete)

## Off Limits

- `crates/tracker-core/src/audio/glicol_mixer.rs` — Parked Glicol integration
- `.ralph/`, `.claude/`, `.serena/` — Agent state, not source
- `Cargo.lock` — Do not manually edit

## Constraints

- All `cargo test` tests must pass after each change
- `cargo build` must succeed
- No breaking changes to song serialization format (`.trs` files)
- Follow existing code style (no unsafe, document public APIs)
- Atomic commits per feature with conventional commit messages

## What's Been Tried

### Run 1 — Baseline (KEEP, metric=7)

Items already implemented, roadmap updated to mark them DONE. cargo test: pass.
- `r — Replace-Once Mode` (commit e813a24c)
- `Play From Cursor` (commit ea27f43c)
- `Note Interpolation`, Follow Mode, BPM Inline Edit + Tap Tempo, Loop Region, Transpose Selection

Baseline established at 7 completed Milestone 2.5 items.

### Planned Feature Queue (priority order)

1. **Piano-keyboard note entry** — map QWERTY rows to piano layout (Z=C, S=C#, X=D, ..., Q=C+1 octave). This is the #1 usability issue — every serious tracker uses this layout. Replaces the current a-g=note-name mapping.
2. **Note Off discoverability** — rebind `` ` `` → `~` or `1`, add to help screen.
3. **Note Cut (^^^) distinct from Note Off (===)** — add NoteEvent::Cut variant, ECx effect wiring.
4. **Better Help Screen / :tutor** — in-app reference for all keybindings.
5. **Per-Pattern Row Count** — each Pattern stores its own row count.
6. **Draw Mode** — hold-repeat or toggle mode for rapid note entry.
7. **ProTracker effect commands** — Exy sub-effects, 5xy, 6xy, 7xy tremolo.
