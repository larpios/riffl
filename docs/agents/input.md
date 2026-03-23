# Input Handling

Vim-style keybinding dispatch. Maps crossterm key events to `Action` enum variants.

## STRUCTURE

```
input/
├── mod.rs           # Module declaration
└── keybindings.rs   # Action enum, map_key_to_action(), all keybinding logic
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new keybinding | `keybindings.rs` | Add `Action` variant, add match arm in `map_key_to_action()` |
| Change mode-specific keys | `keybindings.rs` | Bindings are mode-aware (Normal/Insert/Visual) |
| Handle new action | `app.rs` + `main.rs` | Match on `Action` in event loop, call appropriate `App`/`Editor` method |

## CONVENTIONS

- `Action` enum is the exhaustive list of user-triggerable operations.
- `map_key_to_action()` takes `KeyEvent` + current `EditorMode` + context → returns `Option<Action>`.
- Mode-aware: same key can map to different actions in Normal vs Insert vs Visual mode.
- Function keys (F1–F4) switch views regardless of mode.
- Global actions (quit, save, transport) work in all modes.

## NOTES

- `keybindings.rs` is 963 lines — large due to exhaustive key mappings across all modes and contexts.
- Actions are consumed in `main.rs` `run_app()` → dispatched to `App` methods or `Editor` methods.
