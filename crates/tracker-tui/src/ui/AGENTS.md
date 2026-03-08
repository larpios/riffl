# UI Rendering

Ratatui-based TUI rendering layer. All visual output — pattern grid, arrangement view, code editor, dialogs.

## STRUCTURE

```
ui/
├── mod.rs              # render() dispatch: routes AppView to correct renderer
├── layout.rs           # Layout helpers: create_main_layout, create_split_layout
├── theme.rs            # Theme struct: color palette for all UI elements
├── code_editor.rs      # Code editor widget (Rhai script editing, syntax display)
├── arrangement.rs      # ArrangementView: song sequence grid
├── instrument_list.rs  # Instrument list panel
├── export_dialog.rs    # WAV export dialog (config + progress)
├── file_browser.rs     # File browser for sample loading
└── modal.rs            # Modal dialog system (stacked modals)
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new view | `mod.rs` | Add match arm in `render()` for new `AppView` variant |
| Change colors/theme | `theme.rs` | Modify `Theme` struct fields |
| Layout changes | `layout.rs` | Adjust `create_main_layout` ratios |
| New dialog | `modal.rs` + `mod.rs` | Create dialog widget, push to `modal_stack` |
| Pattern grid rendering | `mod.rs` | `render_content()` draws the tracker grid |
| Split view (editor+code) | `mod.rs` | Controlled by `app.split_view` flag |

## CONVENTIONS

- Renderers take `&App` (read-only). UI never mutates state — that's `app.rs`'s job.
- `render()` in `mod.rs` is the single entry point called from `main.rs` event loop.
- Views dispatch on `AppView` enum: `PatternEditor`, `Arrangement`, `InstrumentList`, `CodeEditor`.
- Header (transport info, BPM) and footer (mode, help) rendered in every view.

## NOTES

- `code_editor.rs` is the largest file (1255 lines) — handles syntax highlighting, cursor, scroll for Rhai scripts.
- Split view: when `app.split_view && AppView::PatternEditor`, pattern renders left, code editor renders right.
- Modal stack: modals render on top of content via `Clear` widget + overlay pattern.
