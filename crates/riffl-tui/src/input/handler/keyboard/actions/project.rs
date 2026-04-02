use crate::app::{App, AppView};
use crate::input::keybindings::Action;
use crate::ui;

pub(super) fn handle(app: &mut App, action: &Action) {
    match action {
        Action::SaveProject => app.save_project(),
        Action::LoadProject => {
            if let Some(path) = app.project_path.clone() {
                app.load_project(&path);
            } else {
                let path = std::path::PathBuf::from("untitled.rtm");
                if path.exists() {
                    app.load_project(&path);
                } else {
                    app.open_modal(ui::modal::Modal::info(
                        "No Project".to_string(),
                        "No project file found. Save first with Ctrl+S.".to_string(),
                    ));
                }
            }
        }
        Action::OpenExportDialog => app.open_export_dialog(),
        Action::AddPattern => {
            if app.current_view == AppView::PatternList {
                app.add_pattern();
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_create_pattern();
            }
        }
        Action::DeletePattern => {
            if app.current_view == AppView::PatternList {
                app.delete_pattern();
            }
        }
        Action::ClonePattern => {
            if app.current_view == AppView::PatternList {
                app.duplicate_pattern();
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_clone_pattern();
            }
        }
        Action::SelectPattern => {
            if app.current_view == AppView::PatternList {
                app.select_pattern();
            }
        }
        Action::ArrangementMoveEntryUp => {
            if app.current_view == AppView::Arrangement {
                app.arrangement_move_entry_up();
            }
        }
        Action::ArrangementMoveEntryDown => {
            if app.current_view == AppView::Arrangement {
                app.arrangement_move_entry_down();
            }
        }
        Action::EnterCommandMode => {
            app.command_mode = true;
            app.command_input.clear();
        }
        Action::OpenModal => {
            app.command_mode = true;
            app.command_input = "goto ".to_string();
        }
        Action::Quit => app.quit(),
        Action::Cancel => {
            app.pending_quit = false;
            app.close_modal();
        }
        Action::Confirm => {
            if app.pending_quit {
                app.close_modal();
                app.force_quit();
            } else if app.has_modal() {
                app.close_modal();
            } else if app.current_view == AppView::PatternEditor && !app.transport.is_playing() {
                app.play_from_cursor();
            } else if app.current_view == AppView::InstrumentList {
                if app.instrument_selection().is_some() {
                    app.inst_editor.focus();
                } else {
                    app.select_instrument();
                }
            } else if app.current_view == AppView::PatternList {
                app.select_pattern();
            } else if app.current_view == AppView::Arrangement {
                let pos = app.arrangement_view.cursor();
                app.transport.jump_to_arrangement_position(pos);
                app.load_arrangement_pattern(pos);
                app.set_view(AppView::PatternEditor);
            }
        }
        Action::None => {}
        _ => {}
    }
}
