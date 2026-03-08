# Pattern Data Model

Core musical data types for the tracker grid. All types are `Serialize`/`Deserialize` for project persistence.

## STRUCTURE

```
pattern/
├── mod.rs      # Re-exports: Note, NoteEvent, Pitch, Effect, Cell, Row, Track, Pattern
├── note.rs     # Pitch enum (C–B), Note (pitch+octave), NoteOff, NoteEvent
├── effect.rs   # Effect struct, EffectType enum (volume, panning, slide, arpeggio, etc.)
├── row.rs      # Cell (note + instrument + volume + effects), Row (Vec<Cell>)
├── track.rs    # Track metadata (number, name, volume, pan, mute, solo, instrument)
└── pattern.rs  # Pattern (rows × channels grid), resize, cell access
```

## TYPE HIERARCHY

```
Pattern
├── rows: Vec<Row>          # Fixed-length row sequence (default 64)
│   └── Row
│       └── cells: Vec<Cell>    # One Cell per channel
│           └── Cell
│               ├── note: Option<NoteEvent>   # NoteOn(Note) | NoteOff
│               ├── instrument: Option<u8>     # 0–255
│               ├── volume: Option<u8>         # 0–128
│               └── effects: Vec<Effect>       # Up to MAX_EFFECTS_PER_CELL (2)
└── num_channels: usize     # Track count (default 4)
```

## CONVENTIONS

- **MIDI mapping:** C-0 = 0, B-9 = 119. Validated in `Note::new()` with `assert!`.
- `#[allow(clippy::module_inception)]` on `pattern::pattern` — intentional naming.
- `NoteEvent` is an enum: `NoteOn(Note)` or `NoteOff`. Not `Option<Note>` — off is distinct from empty.
- `Effect` has `EffectType` enum + `value: u8`. Max 2 effects per cell (`MAX_EFFECTS_PER_CELL`).
- All types derive `Debug, Clone, PartialEq, Serialize, Deserialize`.

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| New note property | `note.rs` | Extend `Note` struct, update `new()` validation |
| New effect type | `effect.rs` | Add variant to `EffectType` |
| Change grid dimensions | `pattern.rs` | `Pattern::new(rows, channels)` |
| Cell data change | `row.rs` | Modify `Cell` struct fields |
| Track metadata | `track.rs` | `Track` struct (volume, pan, mute, solo) |
