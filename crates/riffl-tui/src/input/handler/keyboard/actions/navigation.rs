use crate::app::{App, AppView};
use crate::editor::EditorMode;
use crate::input::keybindings::Action;

pub(super) fn handle(app: &mut App, action: &Action, term_width: u16) -> bool {
    match action {
        Action::MoveLeft => {
            if app.follow_mode
                && app.transport.is_playing()
                && app.current_view == AppView::PatternEditor
            {
                app.scroll_view_left();
            } else {
                app.editor.move_left();
                app.ensure_cursor_visible_horizontally(term_width);
            }
        }
        Action::MoveDown => {
            if app.current_view == AppView::InstrumentList {
                app.inst_editor.unfocus();
                app.instrument_selection_down();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_down();
            } else if app.follow_mode
                && app.transport.is_playing()
                && app.current_view == AppView::PatternEditor
            {
                // Follow mode: j/k blocked — the playhead owns vertical position
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_selection_down();
            } else if matches!(app.editor.mode(), EditorMode::Insert | EditorMode::Replace) {
                app.editor.extend_down();
                app.apply_draw_note();
            } else {
                app.editor.move_down();
            }
        }
        Action::MoveUp => {
            if app.current_view == AppView::InstrumentList {
                app.inst_editor.unfocus();
                app.instrument_selection_up();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_up();
            } else if app.follow_mode
                && app.transport.is_playing()
                && app.current_view == AppView::PatternEditor
            {
                // Follow mode: j/k blocked — the playhead owns vertical position
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_selection_up();
            } else {
                app.editor.move_up();
            }
        }
        Action::MoveRight => {
            if app.follow_mode
                && app.transport.is_playing()
                && app.current_view == AppView::PatternEditor
            {
                app.scroll_view_right(term_width);
            } else {
                app.editor.move_right();
                app.ensure_cursor_visible_horizontally(term_width);
            }
        }
        Action::PageUp => app.editor.page_up(),
        Action::PageDown => app.editor.page_down(),
        Action::GoToBottom => app.editor.go_to_row(usize::MAX),
        Action::GoToTop => app.editor.go_to_row(0),
        Action::GoToStart => app.jump_to_start(),
        Action::GoToEnd => app.jump_to_end(),
        Action::GoToRow => {
            app.command_mode = true;
            app.command_input = "goto ".to_string();
        }
        Action::ResetHorizontalView => app.reset_horizontal_view(),
        _ => return false,
    }
    true
}
