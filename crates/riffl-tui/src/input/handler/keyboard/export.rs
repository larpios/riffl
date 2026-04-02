use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_export_dialog_key(app: &mut App, key: KeyEvent) {
    use crate::ui::export_dialog::ExportPhase;

    // In Done/Failed phases, any dismiss key closes the dialog
    match app.export_dialog.phase {
        ExportPhase::Done | ExportPhase::Failed => {
            if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                        app.export_dialog.close();
                    }
                    _ => {}
                }
            }
            return;
        }
        ExportPhase::Exporting => {
            // No input during export
            return;
        }
        ExportPhase::Configure => {}
    }

    if app.export_dialog.editing_path {
        if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT {
            match key.code {
                KeyCode::Esc => {
                    app.export_dialog.cancel_path_edit();
                }
                KeyCode::Enter => {
                    app.export_dialog.commit_path_edit();
                }
                KeyCode::Backspace => {
                    app.export_dialog.path_backspace();
                }
                KeyCode::Char(c) => {
                    app.export_dialog.path_push_char(c);
                }
                _ => {}
            }
        }
        return;
    }

    if key.modifiers != KeyModifiers::NONE {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.export_dialog.close();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.export_dialog.next_field();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.export_dialog.prev_field();
        }
        KeyCode::Char('l')
        | KeyCode::Right
        | KeyCode::Char('h')
        | KeyCode::Left
        | KeyCode::Char(' ') => {
            app.export_dialog.toggle_value();
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            use crate::ui::export_dialog::ExportField;
            match app.export_dialog.focused_field {
                ExportField::Confirm => {
                    if key.code == KeyCode::Enter {
                        app.execute_export();
                    }
                }
                ExportField::OutputPath => {
                    app.export_dialog.start_editing_path();
                }
                _ => {
                    if key.code == KeyCode::Enter {
                        // Enter on a field toggles it, then moves to next
                        app.export_dialog.toggle_value();
                        app.export_dialog.next_field();
                    }
                }
            }
        }
        KeyCode::Tab => {
            app.export_dialog.next_field();
        }
        _ => {}
    }
}

pub fn hex_char_to_digit(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}
