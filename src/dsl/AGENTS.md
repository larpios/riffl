# DSL / Scripting Engine

Rhai-based live coding DSL for algorithmic pattern generation. Scripts generate patterns, notes, chords, scales, and rhythms programmatically.

## STRUCTURE

```
dsl/
├── mod.rs          # Module exports
├── engine.rs       # ScriptEngine: Rhai wrapper, eval, ScriptResult, lock_unpoisoned()
├── pattern_api.rs  # Music functions registered to Rhai (note, chord, scale, euclidean, etc.)
└── examples.rs     # Bundled example scripts (shown in code editor)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add DSL function | `pattern_api.rs` | Register with `engine.register_fn()` |
| Change script evaluation | `engine.rs` | `ScriptEngine::eval()` and `ScriptResult` enum |
| Add example script | `examples.rs` | Add to examples list (shown in UI) |
| Fix mutex issues | `engine.rs` | `lock_unpoisoned()` helper recovers poisoned mutexes |

## CONVENTIONS

- Rhai functions in `pattern_api.rs` must be simple, strongly typed. Use `rhai::{Array, Dynamic, INT}` for interop.
- `ScriptResult` variants: `Pattern(Pattern)`, primitive values, or unit.
- Scripts can return a `Pattern` that gets applied to the editor grid, or a value displayed in the output.
- `lock_unpoisoned()` in `engine.rs` — recovers from `PoisonError` by extracting the inner value. Used wherever `Arc<Mutex<>>` is locked in DSL/audio code paths.

## ANTI-PATTERNS

- **Never panic on mutex lock failure** — audio thread may have poisoned the mutex. Always use `lock_unpoisoned()`.
- **Keep Rhai-facing functions pure when possible** — avoid side effects that touch audio state directly.

## NOTES

- `engine.rs` is the largest file (1649 lines) — contains the full Rhai engine setup, all registered functions, and script execution logic.
- The DSL generates `Pattern` data that flows into the editor grid, which then feeds into the mixer for playback.
