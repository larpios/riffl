#![allow(dead_code, unused_imports)]
mod app;
mod config;
mod editor;
mod input;
mod ui;

use crate::app::AppView;
use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use editor::{Editor, EditorMode, SubColumn};
use input::keybindings::{map_key_to_action, Action};

/// Tick rate for the event loop (16ms ≈ 60 FPS for smooth BPM timing)
const TICK_RATE: Duration = Duration::from_millis(16);

fn main() -> Result<()> {
    // Set up panic hook to restore terminal before panicking
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Initialize terminal — requires a TTY (won't work in CI/headless environments)
    let mut terminal = match init_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("riffl: Failed to initialize terminal: {}", e);
            eprintln!("This application requires an interactive terminal (TTY) to run.");
            return Err(e);
        }
    };

    // Resolve sample directories and ensure the default one exists
    let cli_sample_dir = parse_sample_dir_flag();
    let config = crate::config::Config::load();
    let sample_dirs = config.resolve_sample_dirs(cli_sample_dir.as_deref());
    let default_samples = crate::config::Config::default_samples_dir();
    let _ = std::fs::create_dir_all(&default_samples);

    // Create and initialize app
    let mut app = App::new();
    app.set_sample_dirs(sample_dirs);
    app.init()?;

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal()?;

    // Propagate any errors from the app
    result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    while app.should_run() {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key);
                    }
                }
                Event::Resize(_width, _height) => {}
                _ => {}
            }
        }

        app.update()?;
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Command mode: handle line input
    if app.command_mode {
        match key.code {
            KeyCode::Enter => app.execute_command(),
            KeyCode::Esc => {
                app.command_mode = false;
                app.command_input.clear();
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.command_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // If a modal is open, handle modal-specific input first
    if app.has_modal() {
        let action = map_key_to_action(key, app.editor_mode());
        match action {
            Action::Cancel | Action::Confirm | Action::EnterNormalMode => {
                app.close_modal();
            }
            _ => {}
        }
        return;
    }

    // If help overlay is open, Esc or ? closes it
    if app.show_help {
        if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
            app.show_help = false;
        }
        return;
    }

    // If export dialog is open, handle export dialog input
    if app.has_export_dialog() {
        handle_export_dialog_key(app, key);
        return;
    }

    // If file browser is open, handle file browser input
    if app.has_file_browser() {
        handle_file_browser_key(app, key);
        return;
    }

    // If the sample browser view is active, handle browser-specific keys.
    // Unhandled keys fall through to normal processing so view switching,
    // command mode, help, transport, etc. all continue to work.
    if app.current_view == AppView::SampleBrowser {
        if handle_sample_browser_key(app, key) {
            return;
        }
    }

    // If code editor is active, handle code editor input first
    if app.is_code_editor_active() {
        handle_code_editor_key(app, key);
        return;
    }

    // Escape during playback: stop transport (in addition to normal Escape behavior)
    if key.code == crossterm::event::KeyCode::Esc && !app.transport.is_stopped() {
        app.stop();
    }

    // In Insert mode on the Effect sub-column, intercept hex digit keys (0-9, A-F)
    // for effect entry instead of their normal note/octave mappings.
    if app.editor.mode() == EditorMode::Insert
        && app.editor.sub_column() == SubColumn::Effect
        && key.modifiers == crossterm::event::KeyModifiers::NONE
    {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if let Some(digit) = hex_char_to_digit(c) {
                app.editor.enter_effect_digit(digit);
                app.mark_dirty();
                return;
            }
        }
    }

    // Chord handling for Normal mode (e.g. dd = delete row)
    if app.editor.mode() == EditorMode::Normal
        && key.modifiers == crossterm::event::KeyModifiers::NONE
    {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if let Some(pending) = app.pending_key.take() {
                match (pending, c) {
                    ('d', 'd') => {
                        app.editor.delete_row();
                        app.mark_dirty();
                        return;
                    }
                    _ => {
                        // Not a recognized chord — fall through with the new key
                    }
                }
            }
            // 'd' starts a chord; consume and wait for next key
            if c == 'd' {
                app.pending_key = Some('d');
                return;
            }
        } else {
            // Non-char key clears any pending chord
            app.pending_key = None;
        }
    } else {
        app.pending_key = None;
    }

    let action = map_key_to_action(key, app.editor_mode());

    match action {
        // Navigation — delegate to editor (or instrument/pattern list)
        Action::MoveLeft => app.editor.move_left(),
        Action::MoveDown => {
            if app.current_view == AppView::InstrumentList {
                app.instrument_selection_down();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_down();
            } else if app.editor.mode() == EditorMode::Insert {
                app.editor.extend_down();
            } else {
                app.editor.move_down();
            }
        }
        Action::MoveUp => {
            if app.current_view == AppView::InstrumentList {
                app.instrument_selection_up();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_up();
            } else {
                app.editor.move_up();
            }
        }
        Action::MoveRight => app.editor.move_right(),
        Action::PageUp => app.editor.page_up(),
        Action::PageDown => app.editor.page_down(),

        // Mode transitions
        Action::EnterInsertMode => app.editor.enter_insert_mode(),
        Action::EnterNormalMode => app.editor.enter_normal_mode(),
        Action::EnterVisualMode => app.editor.enter_visual_mode(),

        // Note entry (Insert mode)
        Action::EnterNote(c) => {
            if let Some(pitch) = Editor::char_to_pitch(c) {
                app.editor.enter_note(pitch);
                app.mark_dirty();
            }
        }
        Action::SetOctave(oct) => app.editor.set_octave(oct),

        // Clipboard
        Action::Copy => app.editor.copy(),
        Action::Paste => {
            app.editor.paste();
            app.mark_dirty();
        }
        Action::Cut => {
            app.editor.cut();
            app.mark_dirty();
        }
        Action::Redo => {
            app.editor.redo();
            app.mark_dirty();
        }

        // Transpose
        Action::TransposeUp => {
            app.editor.transpose_selection(1);
            app.mark_dirty();
        }
        Action::TransposeDown => {
            app.editor.transpose_selection(-1);
            app.mark_dirty();
        }
        Action::TransposeOctaveUp => {
            app.editor.transpose_selection(12);
            app.mark_dirty();
        }
        Action::TransposeOctaveDown => {
            app.editor.transpose_selection(-12);
            app.mark_dirty();
        }

        // Octave navigation
        Action::OctaveUp => app.editor.octave_up(),
        Action::OctaveDown => app.editor.octave_down(),

        // Go to row (basic - jumps to row 0 for now, could be enhanced with input)
        Action::GoToRow => app.editor.go_to_row(0),

        // Quantize
        Action::Quantize => {
            app.editor.quantize();
            app.mark_dirty();
        }

        // Track management
        Action::AddTrack => {
            app.editor.add_track();
            app.mark_dirty();
        }
        Action::DeleteTrack => {
            app.editor.delete_track();
            app.mark_dirty();
        }
        Action::CloneTrack => {
            app.editor.clone_track();
            app.mark_dirty();
        }

        // Interpolation
        Action::Interpolate => {
            app.editor.interpolate();
            app.mark_dirty();
        }

        // Editing
        Action::DeleteCell => {
            app.editor.delete_cell();
            app.mark_dirty();
        }
        Action::InsertRow => {
            app.editor.insert_row();
            app.mark_dirty();
        }
        Action::InsertRowBelow => {
            app.editor.insert_row_below();
            app.mark_dirty();
        }
        Action::DeleteRow => {
            app.editor.delete_row();
            app.mark_dirty();
        }
        Action::EnterCommandMode => {
            app.command_mode = true;
            app.command_input.clear();
        }
        Action::Undo => {
            app.editor.undo();
        }

        // Transport
        Action::TogglePlay => app.toggle_play(),
        Action::Stop => app.stop(),
        Action::BpmUp => app.adjust_bpm(1.0),
        Action::BpmDown => app.adjust_bpm(-1.0),
        Action::BpmUpLarge => app.adjust_bpm(10.0),
        Action::BpmDownLarge => app.adjust_bpm(-10.0),
        Action::ToggleLoop => app.toggle_loop(),
        Action::TogglePlaybackMode => app.toggle_playback_mode(),
        Action::JumpNextPattern => app.jump_next_pattern(),
        Action::JumpPrevPattern => app.jump_prev_pattern(),

        // Track operations
        Action::ToggleMute => app.toggle_mute_current_track(),
        Action::ToggleSolo => app.toggle_solo_current_track(),
        Action::NextTrack => app.editor.next_track(),

        // Code editor
        Action::ToggleSplitView => app.toggle_split_view(),
        Action::ExecuteScript => app.execute_script(),
        Action::OpenTemplates => app.code_editor.toggle_templates(),
        Action::ToggleLiveMode => app.toggle_live_mode(),

        // View switching
        Action::SwitchView(view) => app.set_view(view),

        // Project save/load
        Action::SaveProject => app.save_project(),
        Action::LoadProject => {
            if let Some(path) = app.project_path.clone() {
                app.load_project(&path);
            } else {
                let path = std::path::PathBuf::from("untitled.trs");
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

        // Export
        Action::OpenExportDialog => app.open_export_dialog(),

        // Application
        Action::Quit => app.quit(),
        Action::ToggleHelp => app.show_help = !app.show_help,
        Action::OpenModal => app.open_test_modal(),
        Action::OpenFileBrowser => app.open_file_browser(),
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
            } else if app.current_view == AppView::InstrumentList {
                app.select_instrument();
            } else if app.current_view == AppView::PatternList {
                app.select_pattern();
            }
        }

        // Instrument management (only when in instrument list view)
        Action::AddInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.add_instrument();
            }
        }
        Action::DeleteInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.delete_instrument();
            }
        }
        Action::RenameInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.open_modal(ui::modal::Modal::info(
                    "Rename Instrument".to_string(),
                    "Enter new name in the terminal.".to_string(),
                ));
            }
        }
        Action::EditInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.open_modal(ui::modal::Modal::info(
                    "Edit Instrument".to_string(),
                    "Volume/Base Note editing coming soon.".to_string(),
                ));
            }
        }
        Action::SelectInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.select_instrument();
            }
        }

        // Pattern management (only when in pattern list view)
        Action::AddPattern => {
            if app.current_view == AppView::PatternList {
                app.add_pattern();
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
            }
        }
        Action::SelectPattern => {
            if app.current_view == AppView::PatternList {
                app.select_pattern();
            }
        }

        Action::None => {}
    }
}

fn handle_code_editor_key(app: &mut App, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

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

    // Ctrl-modified keys: handle special code editor shortcuts
    if key.modifiers == KeyModifiers::CONTROL {
        match key.code {
            KeyCode::Enter => {
                app.execute_script();
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
        if !app.code_editor.insert_mode {
            match key.code {
                // Enter insert mode
                KeyCode::Char('i') => {
                    app.code_editor.insert_mode = true;
                    return;
                }
                // View switching (same as pattern editor)
                KeyCode::Char('1') => {
                    app.set_view(app::AppView::PatternEditor);
                    app.split_view = false;
                    return;
                }
                KeyCode::Char('2') => {
                    app.set_view(app::AppView::Arrangement);
                    app.split_view = false;
                    return;
                }
                KeyCode::Char('3') => {
                    app.set_view(app::AppView::InstrumentList);
                    app.split_view = false;
                    return;
                }
                KeyCode::Char('4') => {
                    app.set_view(app::AppView::CodeEditor);
                    return;
                }
                KeyCode::Char('5') => {
                    app.set_view(app::AppView::PatternList);
                    app.split_view = false;
                    return;
                }
                // Command mode
                KeyCode::Char(':') => {
                    app.command_mode = true;
                    return;
                }
                // Escape in normal mode: leave code editor (same as before)
                KeyCode::Esc => {
                    if app.split_view {
                        app.code_editor.active = false;
                    } else {
                        app.set_view(app::AppView::PatternEditor);
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

        match key.code {
            // Escape: exit insert mode (back to normal, don't leave the view)
            KeyCode::Esc => {
                app.code_editor.insert_mode = false;
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
            // View switching with F-keys still works
            KeyCode::F(1) => {
                app.set_view(app::AppView::PatternEditor);
                app.split_view = false;
                return;
            }
            KeyCode::F(2) => {
                app.set_view(app::AppView::Arrangement);
                app.split_view = false;
                return;
            }
            KeyCode::F(3) => {
                app.set_view(app::AppView::InstrumentList);
                app.split_view = false;
                return;
            }
            KeyCode::F(4) => {
                app.set_view(app::AppView::CodeEditor);
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

    // Shift+characters for uppercase in the editor
    if key.modifiers == KeyModifiers::SHIFT {
        if let KeyCode::Char(c) = key.code {
            app.code_editor.insert_char(c);
        }
    }
}

/// Returns `true` if the key was consumed by the sample browser, `false` to fall through.
fn handle_sample_browser_key(app: &mut App, key: KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Only consume plain (no-modifier) navigation keys.
    // Everything else falls through so view switching, command mode, help,
    // transport shortcuts, etc. all keep working.
    if key.modifiers != KeyModifiers::NONE {
        return false;
    }

    match key.code {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => { app.sample_browser.move_down(); true }
        KeyCode::Char('k') | KeyCode::Up => { app.sample_browser.move_up(); true }

        // Enter directory
        KeyCode::Char('l') | KeyCode::Right => {
            app.sample_browser.enter_dir();
            true
        }

        // Go up a directory
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Backspace => {
            app.sample_browser.go_up();
            true
        }

        // Load file or enter directory
        KeyCode::Enter => {
            if app.sample_browser.selected_is_file() {
                let is_mod = app
                    .sample_browser
                    .selected_path()
                    .and_then(|p| p.extension())
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("mod"))
                    .unwrap_or(false);

                if is_mod {
                    let path = app.sample_browser.selected_path().map(|p| p.to_path_buf());
                    if let Some(path) = path {
                        match app.import_mod_file(&path) {
                            Ok(()) => {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("file")
                                    .to_string();
                                app.open_modal(ui::modal::Modal::info(
                                    "MOD Imported".to_string(),
                                    format!("Loaded '{name}'"),
                                ));
                            }
                            Err(e) => {
                                app.open_modal(ui::modal::Modal::error(
                                    "Import Failed".to_string(),
                                    e,
                                ));
                            }
                        }
                    }
                } else {
                    match app.load_sample_from_browser() {
                        Ok(idx) => {
                            let name = app
                                .instrument_names()
                                .get(idx)
                                .cloned()
                                .unwrap_or_else(|| "sample".to_string());
                            app.open_modal(ui::modal::Modal::info(
                                "Sample Loaded".to_string(),
                                format!("Loaded '{name}' as instrument {:02X}", idx),
                            ));
                        }
                        Err(e) => {
                            app.open_modal(ui::modal::Modal::error("Load Failed".to_string(), e));
                        }
                    }
                }
            } else {
                app.sample_browser.enter_dir();
            }
            true
        }

        _ => false,
    }
}

fn handle_file_browser_key(app: &mut App, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    if key.modifiers != KeyModifiers::NONE {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.close_file_browser();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.file_browser.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.file_browser.move_up();
        }
        KeyCode::Enter => {
            let is_mod = app
                .file_browser
                .selected_path()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("mod"))
                .unwrap_or(false);

            if is_mod {
                let path = app.file_browser.selected_path().map(|p| p.to_path_buf());
                if let Some(path) = path {
                    match app.import_mod_file(&path) {
                        Ok(()) => {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            app.close_file_browser();
                            app.open_modal(ui::modal::Modal::info(
                                "MOD Imported".to_string(),
                                format!("Imported '{}'", name),
                            ));
                        }
                        Err(msg) => {
                            app.close_file_browser();
                            app.open_modal(ui::modal::Modal::error(
                                "Import Failed".to_string(),
                                msg,
                            ));
                        }
                    }
                }
            } else {
                match app.load_selected_sample() {
                    Ok(idx) => {
                        let name = app
                            .instrument_names()
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());
                        app.close_file_browser();
                        app.open_modal(ui::modal::Modal::info(
                            "Sample Loaded".to_string(),
                            format!("Loaded '{}' as instrument {:02X}", name, idx),
                        ));
                    }
                    Err(msg) => {
                        app.close_file_browser();
                        app.open_modal(ui::modal::Modal::error("Load Failed".to_string(), msg));
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_export_dialog_key(app: &mut App, key: KeyEvent) {
    use crate::ui::export_dialog::ExportPhase;
    use crossterm::event::{KeyCode, KeyModifiers};

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
        KeyCode::Enter => {
            use crate::ui::export_dialog::ExportField;
            match app.export_dialog.focused_field {
                ExportField::Confirm => {
                    app.execute_export();
                }
                _ => {
                    // Enter on a field toggles it, then moves to next
                    app.export_dialog.toggle_value();
                    app.export_dialog.next_field();
                }
            }
        }
        KeyCode::Tab => {
            app.export_dialog.next_field();
        }
        _ => {}
    }
}

/// Convert a character to a hex digit value (0-15), or None if not a hex digit.
/// Parse `--sample-dir <path>` from the process arguments, returning the value if present.
fn parse_sample_dir_flag() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--sample-dir" {
            return iter.next().cloned();
        }
        if let Some(val) = arg.strip_prefix("--sample-dir=") {
            return Some(val.to_string());
        }
    }
    None
}

fn hex_char_to_digit(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_char_to_digit_numerics() {
        assert_eq!(hex_char_to_digit('0'), Some(0));
        assert_eq!(hex_char_to_digit('5'), Some(5));
        assert_eq!(hex_char_to_digit('9'), Some(9));
    }

    #[test]
    fn test_hex_char_to_digit_lowercase() {
        assert_eq!(hex_char_to_digit('a'), Some(10));
        assert_eq!(hex_char_to_digit('c'), Some(12));
        assert_eq!(hex_char_to_digit('f'), Some(15));
    }

    #[test]
    fn test_hex_char_to_digit_uppercase() {
        assert_eq!(hex_char_to_digit('A'), Some(10));
        assert_eq!(hex_char_to_digit('C'), Some(12));
        assert_eq!(hex_char_to_digit('F'), Some(15));
    }

    #[test]
    fn test_hex_char_to_digit_invalid() {
        assert_eq!(hex_char_to_digit('g'), None);
        assert_eq!(hex_char_to_digit('G'), None);
        assert_eq!(hex_char_to_digit('z'), None);
        assert_eq!(hex_char_to_digit(' '), None);
        assert_eq!(hex_char_to_digit('#'), None);
    }
}
