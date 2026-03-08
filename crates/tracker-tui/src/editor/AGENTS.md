# Pattern Editor

Vim-inspired modal editor for the tracker pattern grid. Single-file module (1855 lines).

## KEY TYPES

- **`Editor`** — Main state machine. Owns pattern, cursor position, mode, undo/redo history, clipboard, selection.
- **`EditorMode`** — `Normal` (navigation), `Insert` (note/value entry), `Visual` (block selection).
- **`SubColumn`** — Cursor sub-position within a channel: `Note`, `Instrument`, `Volume`, `Effect`.
- **`Clipboard`** — Rectangular cell grid for copy/paste (single cell or block).

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add editor command | `mod.rs` | Add method on `Editor`, wire in `input/keybindings.rs` |
| Change cursor behavior | `mod.rs` | `move_*` methods handle navigation |
| Undo/redo logic | `mod.rs` | `push_undo()`, `undo()`, `redo()` with pattern snapshots |
| Selection/clipboard | `mod.rs` | Visual mode methods, `Clipboard` struct |
| Note entry logic | `mod.rs` | `enter_note()`, `enter_instrument()`, etc. |

## CONVENTIONS

- Editor methods return nothing — they mutate internal state. `app.rs` calls editor methods then re-renders.
- Undo stores full pattern snapshots (clone). Not diff-based.
- Cursor wraps at pattern boundaries (row 0 ↔ last row, channel 0 ↔ last channel).
- `step` field controls how many rows cursor advances after note entry in Insert mode.

## NOTES

- This is intentionally a single large file. The modal state machine is cohesive — splitting would fragment the state transitions.
- Visual mode supports rectangular block selection across rows and channels.
