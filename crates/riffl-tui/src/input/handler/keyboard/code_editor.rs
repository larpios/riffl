use crate::app::{App, AppView};
use crate::ui::code_editor::ModeKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_code_editor_key(app: &mut App, key: KeyEvent) {
    // If template menu is open, handle template navigation
    if app.code_editor.show_templates {
        if key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => app.code_editor.template_up(),
                KeyCode::Down | KeyCode::Char('j') => app.code_editor.template_down(),
                KeyCode::Enter => app.code_editor.load_selected_template(),
                KeyCode::Esc => app.code_editor.close_templates(),
                _ => {}
            }
        }
        return;
    }

    // F5: run script (works on all terminals, no kitty protocol required)
    if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::F(5) {
        app.execute_script(&[]);
        return;
    }

    // Ctrl-modified keys: handle special code editor shortcuts
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
            KeyCode::Enter => {
                app.execute_script(&[]);
                return;
            }
            KeyCode::Char('\\') => {
                app.toggle_split_view();
                return;
            }
            KeyCode::Char('t') => {
                app.code_editor.toggle_templates();
                return;
            }
            KeyCode::Char('l') => {
                app.toggle_live_mode();
                return;
            }
            _ => {}
        }
    }

    // No modifiers: text editing and navigation
    if key.modifiers == KeyModifiers::NONE {
        // In Normal mode: navigation and view-switching only, no text input
        match app.code_editor.mode {
            ModeKind::Normal => {
                match key.code {
                    // Enter insert mode
                    KeyCode::Char('i') => {
                        app.code_editor.mode = ModeKind::Insert;
                        return;
                    }
                    // Enter visual mode
                    KeyCode::Char('v') => {
                        app.code_editor.mode = ModeKind::Visual;
                        return;
                    }
                    // View switching with F-keys (same as pattern editor)
                    KeyCode::F(1) => {
                        app.set_view(AppView::PatternEditor);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(2) => {
                        app.set_view(AppView::Arrangement);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(3) => {
                        app.set_view(AppView::InstrumentList);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(4) => {
                        app.set_view(AppView::CodeEditor);
                        return;
                    }
                    KeyCode::F(5) => {
                        app.set_view(AppView::PatternList);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(6) => {
                        app.set_view(AppView::SampleBrowser);
                        app.split_view = false;
                        return;
                    }
                    // Command mode
                    KeyCode::Char(':') => {
                        app.command_mode = true;
                        app.command_input.clear();
                        return;
                    }
                    // Escape in normal mode: leave code editor (same as before)
                    KeyCode::Esc => {
                        if app.split_view {
                            app.code_editor.active = false;
                        } else {
                            app.set_view(AppView::PatternEditor);
                        }
                        return;
                    }
                    // Navigation still works in Normal mode
                    KeyCode::Left | KeyCode::Char('h') => {
                        app.code_editor.move_left();
                        return;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        app.code_editor.move_right();
                        return;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.code_editor.move_up();
                        return;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.code_editor.move_down();
                        return;
                    }
                    KeyCode::Home => {
                        app.code_editor.move_home();
                        return;
                    }
                    KeyCode::End => {
                        app.code_editor.move_end();
                        return;
                    }
                    KeyCode::PageUp => {
                        app.code_editor.page_up(20);
                        return;
                    }
                    KeyCode::PageDown => {
                        app.code_editor.page_down(20);
                        return;
                    }
                    _ => return,
                }
            }
            ModeKind::Insert => {
                match key.code {
                    // Escape: exit insert mode (back to normal, don't leave the view)
                    KeyCode::Esc => {
                        app.code_editor.mode = ModeKind::Normal;
                        return;
                    }
                    // Text editing
                    KeyCode::Char(c) => {
                        app.code_editor.insert_char(c);
                        return;
                    }
                    KeyCode::Enter => {
                        app.code_editor.insert_newline();
                        return;
                    }
                    KeyCode::Backspace => {
                        app.code_editor.backspace();
                        return;
                    }
                    KeyCode::Delete => {
                        app.code_editor.delete();
                        return;
                    }
                    // Cursor navigation
                    KeyCode::Left => {
                        app.code_editor.move_left();
                        return;
                    }
                    KeyCode::Right => {
                        app.code_editor.move_right();
                        return;
                    }
                    KeyCode::Up => {
                        app.code_editor.move_up();
                        return;
                    }
                    KeyCode::Down => {
                        app.code_editor.move_down();
                        return;
                    }
                    KeyCode::Home => {
                        app.code_editor.move_home();
                        return;
                    }
                    KeyCode::End => {
                        app.code_editor.move_end();
                        return;
                    }
                    KeyCode::PageUp => {
                        app.code_editor.page_up(20);
                        return;
                    }
                    KeyCode::PageDown => {
                        app.code_editor.page_down(20);
                        return;
                    }
                    // View switching with F-keys still works in Insert mode
                    KeyCode::F(1) => {
                        app.set_view(AppView::PatternEditor);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(2) => {
                        app.set_view(AppView::Arrangement);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(3) => {
                        app.set_view(AppView::InstrumentList);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(4) => {
                        app.set_view(AppView::CodeEditor);
                        return;
                    }
                    KeyCode::F(5) => {
                        app.set_view(AppView::PatternList);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::F(6) => {
                        app.set_view(AppView::SampleBrowser);
                        app.split_view = false;
                        return;
                    }
                    KeyCode::Tab => {
                        // Insert 2 spaces for indentation
                        app.code_editor.insert_char(' ');
                        app.code_editor.insert_char(' ');
                        return;
                    }
                    _ => {}
                }
            }
            ModeKind::Visual => {
                // Escape: exit visual mode (back to normal, don't leave the view)
                if key.code == KeyCode::Esc {
                    app.code_editor.mode = ModeKind::Normal;
                    return;
                }
            }
        }
    }

    // Shift+characters for uppercase in the editor
    if key.modifiers == KeyModifiers::SHIFT {
        if let KeyCode::Char(c) = key.code {
            app.code_editor.insert_char(c);
        }
    }
}
