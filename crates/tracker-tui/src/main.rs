#![allow(dead_code, unused_imports)]
mod app;
mod config;
mod editor;
mod input;
mod registry;
mod ui;

use crate::app::AppView;
use crate::ui::code_editor::ModeKind;
use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind},
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
    // Initialize logging as early as possible
    let _ = tracker_core::log::init();

    // Check for --dump-config before doing any terminal setup
    if std::env::args().any(|arg| arg == "--dump-config") {
        let config = crate::config::Config::load();
        if let Ok(toml_str) = toml::to_string_pretty(&config) {
            println!("{}", toml_str);
        } else {
            println!("Failed to serialize config to TOML");
        }
        return Ok(());
    }

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
            // No terminal yet, can use println/eprintln safely here
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

    // Resolve module directories and ensure the default one exists
    let _module_dirs = config.resolve_module_dirs();
    let default_modules = crate::config::Config::default_modules_dir();
    let _ = std::fs::create_dir_all(&default_modules);

    // Create and initialize app
    let mut app = App::new();
    app.set_sample_dirs(sample_dirs);

    // Apply config: set theme from config file (or default "mocha")
    let theme_kind = config.theme_kind();
    app.theme = crate::ui::theme::Theme::from_kind(theme_kind.clone());
    app.theme_kind = theme_kind;
    app.config = config;

    // Re-apply roots so persisted bookmarks from config appear at startup.
    // set_sample_dirs (above) ran before app.config was assigned, so bookmarks
    // were not applied on that first call.
    app.refresh_browser_roots();

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
    execute!(stdout, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    execute!(io::stdout(), event::DisableMouseCapture)?;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.should_run() {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key);
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(app, mouse);
                }
                Event::Resize(_width, _height) => {}
                _ => {}
            }
        }

        app.refresh_system_stats();
        app.update()?;
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    // ':' always opens command mode — intercept before any panel/view handler.
    // This ensures command mode is reachable from every view, even when a panel is focused.
    if key.code == KeyCode::Char(':')
        && key.modifiers == KeyModifiers::NONE
        && !app.command_mode
        && !app.has_modal()
        && !app.has_file_browser()
        && !app.has_export_dialog()
        && !app.show_help
        && !app.show_effect_help
        && !app.show_tutor
    {
        app.command_mode = true;
        app.command_input.clear();
        return;
    }

    // BPM prompt mode: handle inline BPM input
    if app.bpm_prompt_mode {
        match key.code {
            KeyCode::Enter => app.execute_bpm_prompt(),
            KeyCode::Esc => {
                app.bpm_prompt_mode = false;
                app.bpm_prompt_input.clear();
            }
            KeyCode::Backspace => {
                app.bpm_prompt_input.pop();
            }
            KeyCode::Char(c @ '0'..='9') if key.modifiers == KeyModifiers::NONE => {
                app.bpm_prompt_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // Pattern length prompt mode: handle inline length input
    if app.len_prompt_mode {
        match key.code {
            KeyCode::Enter => app.execute_len_prompt(),
            KeyCode::Esc => {
                app.len_prompt_mode = false;
                app.len_prompt_input.clear();
            }
            KeyCode::Backspace => {
                app.len_prompt_input.pop();
            }
            KeyCode::Char(c @ '0'..='9') if key.modifiers == KeyModifiers::NONE => {
                app.len_prompt_input.push(c);
            }
            _ => {}
        }
        return;
    }

    // Command mode: handle line input
    if app.command_mode {
        match key.code {
            KeyCode::Enter => app.execute_command(),
            KeyCode::Esc => {
                app.command_mode = false;
                app.command_input.clear();
                app.command_history_index = None;
            }
            KeyCode::Backspace => {
                app.command_input.pop();
            }
            KeyCode::Up => {
                if let Some(idx) = app.command_history_index {
                    if idx + 1 < app.command_history.len() {
                        app.command_history_index = Some(idx + 1);
                        app.command_input =
                            app.command_history[app.command_history.len() - 1 - (idx + 1)].clone();
                    }
                } else if !app.command_history.is_empty() {
                    app.command_history_index = Some(0);
                    app.command_input = app.command_history.last().unwrap().clone();
                }
            }
            KeyCode::Down => {
                if let Some(idx) = app.command_history_index {
                    if idx == 0 {
                        app.command_history_index = None;
                        app.command_input.clear();
                    } else {
                        app.command_history_index = Some(idx - 1);
                        app.command_input =
                            app.command_history[app.command_history.len() - 1 - (idx - 1)].clone();
                    }
                }
            }
            KeyCode::Tab => {
                let input = app.command_input.trim();
                let candidates = [
                    "bpm", "t", "tempo", "step", "load", "save", "volume", "quit", "q", "q!", "w",
                    "wq", "e", "tutor",
                ];
                if let Some(match_idx) = candidates.iter().position(|c| c.starts_with(input)) {
                    app.command_input = candidates[match_idx].to_string();
                }
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.command_input.push(c);
                app.command_history_index = None;
            }
            _ => {}
        }
        return;
    }

    // If a modal is open, handle modal-specific input first
    if app.has_modal() {
        match key.code {
            // Quit confirmation: Enter = quit, Esc = cancel
            KeyCode::Enter if app.pending_quit => {
                app.close_modal();
                app.force_quit();
            }
            KeyCode::Esc if app.pending_quit => {
                app.pending_quit = false;
                app.close_modal();
            }

            // Sample action menu: load as new instrument
            KeyCode::Char('l') if app.pending_sample_path.is_some() => {
                let path = app.pending_sample_path.take().unwrap();
                app.close_modal();
                match app.load_sample_from_path(&path) {
                    Ok(idx) => {
                        let name = app
                            .instrument_names()
                            .get(idx)
                            .cloned()
                            .unwrap_or_else(|| "sample".to_string());
                        app.open_modal(ui::modal::Modal::info(
                            "Sample Loaded".to_string(),
                            format!("Loaded '{}' as instrument {:02X}", name, idx),
                        ));
                    }
                    Err(e) => {
                        app.open_modal(ui::modal::Modal::error("Load Failed".to_string(), e));
                    }
                }
            }

            // Sample action menu: assign to currently selected instrument
            KeyCode::Char('a') if app.pending_sample_path.is_some() => {
                if let Some(inst_idx) = app.instrument_selection() {
                    let path = app.pending_sample_path.take().unwrap();
                    app.close_modal();
                    match app.assign_sample_to_instrument(&path, inst_idx) {
                        Ok(()) => {
                            app.open_modal(ui::modal::Modal::info(
                                "Sample Assigned".to_string(),
                                format!("Assigned to instrument {:02X}", inst_idx),
                            ));
                        }
                        Err(e) => {
                            app.open_modal(ui::modal::Modal::error("Assign Failed".to_string(), e));
                        }
                    }
                }
                // If no instrument selected, do nothing (stay in menu)
            }

            // Sample action menu: cancel
            KeyCode::Esc if app.pending_sample_path.is_some() => {
                app.pending_sample_path = None;
                app.close_modal();
            }

            // Dismiss any other modal
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                app.close_modal();
            }
            _ => {}
        }
        return;
    }

    // If tutor view is open, handle navigation and close
    if app.show_tutor {
        let max_scroll = {
            let content = ui::tutor::content_line_count();
            let term_rows = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
            let visible = ((term_rows as u32 * 92 / 100) as u16).saturating_sub(2);
            content.saturating_sub(visible)
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.show_tutor = false;
                app.tutor_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.tutor_scroll = app.tutor_scroll.saturating_add(1).min(max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.tutor_scroll = app.tutor_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.tutor_scroll = app.tutor_scroll.saturating_add(10).min(max_scroll);
            }
            KeyCode::PageUp => {
                app.tutor_scroll = app.tutor_scroll.saturating_sub(10);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.tutor_scroll = 0;
            }
            _ => {}
        }
        return;
    }

    // If effect help overlay is open, handle navigation and close
    if app.show_effect_help {
        // Compute max scroll: estimate 25 effects * 4 lines each = 100 lines
        let max_scroll = 100u16.saturating_sub(20);
        match key.code {
            KeyCode::Esc | KeyCode::Char('K') | KeyCode::Char('q') => {
                app.show_effect_help = false;
                app.effect_help_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.effect_help_scroll = app.effect_help_scroll.saturating_add(1).min(max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.effect_help_scroll = app.effect_help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.effect_help_scroll = app.effect_help_scroll.saturating_add(10).min(max_scroll);
            }
            KeyCode::PageUp => {
                app.effect_help_scroll = app.effect_help_scroll.saturating_sub(10);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.effect_help_scroll = 0;
            }
            _ => {}
        }
        return;
    }

    // If help overlay is open, handle navigation and close
    if app.show_help {
        // Compute max scroll: content lines minus visible inner height (85% of terminal - 2 borders)
        let max_scroll = {
            let content = ui::help::content_line_count();
            let term_rows = crossterm::terminal::size().map(|(_, h)| h).unwrap_or(24);
            let visible = ((term_rows as u32 * 85 / 100) as u16).saturating_sub(2);
            content.saturating_sub(visible)
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                app.show_help = false;
                app.help_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.help_scroll = app.help_scroll.saturating_add(1).min(max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(10).min(max_scroll);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.help_scroll = 0;
            }
            _ => {}
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
    if app.current_view == AppView::SampleBrowser && handle_sample_browser_key(app, key) {
        return;
    }

    // If instrument editor panel is focused, handle its input first
    if app.current_view == AppView::InstrumentList
        && app.inst_editor.focused
        && handle_instrument_editor_key(app, key)
    {
        return;
    }

    // If envelope editor panel is focused, handle its input first
    if app.current_view == AppView::InstrumentList
        && app.env_editor.focused
        && handle_envelope_editor_key(app, key)
    {
        return;
    }

    // If waveform editor panel is focused, handle its input first
    if app.current_view == AppView::InstrumentList
        && app.waveform_editor.focused
        && handle_waveform_editor_key(app, key)
    {
        return;
    }

    // In InstrumentList, Tab/Shift-Tab cycle focus between the right panels:
    //   none → env_editor → waveform_editor → none  (Tab)
    //   none → waveform_editor → env_editor → inst_editor → none  (Shift-Tab)
    let is_tab = key.code == crossterm::event::KeyCode::Tab;
    let is_backtab = key.code == crossterm::event::KeyCode::BackTab;
    if app.current_view == AppView::InstrumentList
        && app.instrument_selection().map_or(false, |i| i < app.song.instruments.len())
        && (is_tab || is_backtab)
    {
        if app.inst_editor.focused {
            app.inst_editor.unfocus();
            if is_tab {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            }
            // Shift-Tab from inst_editor → back to list (no focus)
        } else if app.env_editor.focused {
            app.env_editor.unfocus();
            if is_tab {
                app.waveform_editor.focus();
            } else if is_backtab {
                app.inst_editor.focus();
            }
        } else if app.waveform_editor.focused {
            app.waveform_editor.unfocus();
            if is_backtab {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            }
            // Tab from waveform → back to list (no focus)
        } else {
            // Nothing focused: Tab → envelope, Shift-Tab → waveform
            if is_tab {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            } else {
                app.waveform_editor.focus();
            }
        }
        return;
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

    // In Insert mode on Effect/Instrument/Volume sub-columns, intercept hex digit
    // keys (0-9, A-F) for data entry instead of their normal note/octave mappings.
    if matches!(app.editor.mode(), EditorMode::Insert | EditorMode::Replace)
        && key.modifiers == crossterm::event::KeyModifiers::NONE
    {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if let Some(digit) = hex_char_to_digit(c) {
                match app.editor.sub_column() {
                    SubColumn::Effect => app.editor.enter_effect_digit(digit),
                    SubColumn::Instrument => app.editor.enter_instrument_digit(digit),
                    SubColumn::Volume => app.editor.enter_volume_digit(digit),
                    SubColumn::Note => {} // fall through to note entry
                }
                if app.editor.sub_column() != SubColumn::Note {
                    app.mark_dirty();
                    return;
                }
            }
        }
    }

    // Replace-once state: r was pressed, intercept the next key
    if app.pending_replace {
        app.pending_replace = false;
        if app.editor.sub_column() == SubColumn::Note {
            match key.code {
                crossterm::event::KeyCode::Esc => {} // cancel silently
                crossterm::event::KeyCode::Char('~') => {
                    app.editor.replace_cell_note_off();
                    app.mark_dirty();
                }
                crossterm::event::KeyCode::Char(c)
                    if matches!(
                        c,
                        'a' | 'w' | 's' | 'e' | 'd' | 'f' | 't' | 'g' | 'y' | 'h' | 'u' | 'j' | 'k'
                    ) =>
                {
                    if let Some((pitch, _oct_offset)) = Editor::piano_key_to_pitch(c) {
                        let octave = app.editor.current_octave();
                        app.editor.replace_once(pitch);
                        app.mark_dirty();
                        if app.current_view == AppView::PatternEditor {
                            app.preview_note_pitch(pitch, octave);
                        }
                    }
                }
                _ => {} // any other key: cancel silently
            }
        }
        return;
    }

    // Chord handling for Normal mode (e.g. dd = delete row, gg = go to top)
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
                    ('g', 'g') => {
                        app.editor.go_to_row(0);
                        return;
                    }
                    _ => {
                        // Not a recognized chord — fall through with the new key
                    }
                }
            }
            // 'd' and 'g' start chords; consume and wait for next key
            if c == 'd' {
                app.pending_key = Some('d');
                return;
            }
            if c == 'g' {
                app.pending_key = Some('g');
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

    // Intercept note entry while in InstrumentList to preview samples without editing the pattern
    if app.current_view == AppView::InstrumentList {
        match action {
            Action::EnterNote(c) => {
                if let Some((pitch, oct_offset)) = Editor::piano_key_to_pitch(c) {
                    let base_octave = app.editor.current_octave();
                    let octave = (base_octave as i8 + oct_offset).clamp(0, 9) as u8;
                    if let Some(inst_idx) = app.instrument_selection() {
                        app.preview_instrument_note_pitch(inst_idx, pitch, octave);
                    }
                }
                return;
            }
            Action::EnterNoteOff | Action::EnterNoteCut => {
                return; // ignore silently
            }
            _ => {}
        }
    }

    // Grab terminal width once for horizontal tracking
    let term_width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);

    match action {
        // Navigation — delegate to editor (or instrument/pattern list)
        Action::MoveLeft => {
            if app.follow_mode && app.transport.is_playing() && app.current_view == AppView::PatternEditor {
                // Follow mode: h pans the view left, doesn't move cursor channel
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
            } else if app.follow_mode && app.transport.is_playing() && app.current_view == AppView::PatternEditor {
                // Follow mode: j/k are blocked — the playhead owns vertical position
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
            } else if app.follow_mode && app.transport.is_playing() && app.current_view == AppView::PatternEditor {
                // Follow mode: j/k are blocked — the playhead owns vertical position
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_selection_up();
            } else {
                app.editor.move_up();
            }
        }
        Action::MoveRight => {
            if app.follow_mode && app.transport.is_playing() && app.current_view == AppView::PatternEditor {
                // Follow mode: l pans the view right, doesn't move cursor channel
                app.scroll_view_right(term_width);
            } else {
                app.editor.move_right();
                app.ensure_cursor_visible_horizontally(term_width);
            }
        }
        Action::PageUp => app.editor.page_up(),
        Action::PageDown => app.editor.page_down(),

        // Mode transitions
        Action::EnterInsertMode => app.editor.enter_insert_mode(),
        Action::EnterNormalMode => app.editor.enter_normal_mode(),
        Action::EnterVisualMode => app.editor.enter_visual_mode(),
        Action::EnterReplaceMode => app.editor.enter_replace_mode(),

        // Note entry (Insert mode) — piano keyboard layout
        Action::EnterNote(c) => {
            if app.current_view == AppView::Arrangement {
                if let Some(digit) = hex_char_to_digit(c) {
                    app.arrangement_set_pattern_digit(digit);
                }
                return;
            }

            if let Some((pitch, oct_offset)) = Editor::piano_key_to_pitch(c) {
                let base_octave = app.editor.current_octave();
                let octave = (base_octave as i8 + oct_offset).clamp(0, 9) as u8;
                app.editor.enter_note_with_octave(pitch, octave);
                // Capture as draw_note for draw mode repeat
                {
                    use tracker_core::pattern::note::{Note, NoteEvent};
                    let inst = app.editor.current_instrument();
                    app.draw_note = Some(NoteEvent::On(Note::new(pitch, octave, 100, inst)));
                }
                app.mark_dirty();
                // Preview the note through the current instrument's sample
                if app.current_view == AppView::PatternEditor {
                    app.preview_note_pitch(pitch, octave);
                }
            }
        }
        Action::EnterNoteOff => {
            app.editor.enter_note_off();
            app.mark_dirty();
        }
        Action::EnterNoteCut => {
            app.editor.enter_note_cut();
            app.mark_dirty();
        }
        Action::SetOctave(oct) => app.editor.set_octave(oct),
        Action::StepUp => app.editor.step_up(),
        Action::StepDown => app.editor.step_down(),

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

        // Go to last row (G)
        Action::GoToBottom => app.editor.go_to_row(usize::MAX),
        // Go to first row (gg)
        Action::GoToTop => app.editor.go_to_row(0),
        // Go to start of song (Ctrl+Home)
        Action::GoToStart => app.jump_to_start(),
        // Go to end of song (Ctrl+End)
        Action::GoToEnd => app.jump_to_end(),
        // Go to specific row (prompt)
        Action::GoToRow => app.editor.go_to_row(usize::MAX),

        // Reset horizontal view to leftmost channel (Ctrl+Left)
        Action::ResetHorizontalView => app.reset_horizontal_view(),

        // Quantize
        Action::Quantize => {
            app.editor.quantize();
            app.mark_dirty();
        }

        // Track management
        Action::AddTrack => {
            app.editor.add_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }
        Action::DeleteTrack => {
            app.editor.delete_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }
        Action::CloneTrack => {
            app.editor.clone_track();
            app.sync_mixer_channels();
            app.mark_dirty();
        }

        // Interpolation
        Action::Interpolate => {
            app.editor.interpolate();
            app.mark_dirty();
        }

        // Editing
        Action::DeleteCell => {
            if app.current_view == AppView::Arrangement {
                app.arrangement_delete_at_cursor();
            } else {
                app.editor.delete_cell();
            }
            app.mark_dirty();
        }
        Action::InsertRow => {
            if app.current_view == AppView::Arrangement {
                app.arrangement_add_at_cursor();
            } else {
                app.editor.insert_row();
            }
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
        Action::ToggleFollowMode => app.follow_mode = !app.follow_mode,

        // BPM
        Action::OpenBpmPrompt => app.open_bpm_prompt(),
        Action::TapTempo => app.tap_tempo(),

        // Pattern length
        Action::OpenLenPrompt => app.open_len_prompt(),

        // Loop region
        Action::SetLoopStart => app.set_loop_start(),
        Action::SetLoopEnd => app.set_loop_end(),
        Action::ToggleLoopRegion => app.toggle_loop_region_active(),

        // Draw mode
        Action::ToggleDrawMode => app.toggle_draw_mode(),

        // View switching
        Action::SwitchView(view) => app.set_view(view),

        // Project save/load
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

        // Export
        Action::OpenExportDialog => app.open_export_dialog(),

        // Application
        Action::Quit => app.quit(),
        Action::ToggleHelp => {
            app.show_help = !app.show_help;
            app.help_scroll = 0; // always start from the top
        }
        Action::ToggleEffectHelp => {
            app.show_effect_help = !app.show_effect_help;
            app.effect_help_scroll = 0;
        }
        Action::ShowWhichKey => {
            app.which_key_mode = !app.which_key_mode;
        }
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
            } else if app.current_view == AppView::PatternEditor && app.transport.is_stopped() {
                // Play From Cursor: Enter while stopped starts playback at the cursor row
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
            if app.current_view == AppView::PatternEditor
                && app.editor.mode() == EditorMode::Normal
                && app.editor.sub_column() == SubColumn::Note
            {
                app.pending_replace = true;
            } else if app.current_view == AppView::InstrumentList {
                app.open_modal(ui::modal::Modal::info(
                    "Rename Instrument".to_string(),
                    "Enter new name in the terminal.".to_string(),
                ));
            }
        }
        Action::EditInstrument => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.inst_editor.focus();
            }
        }
        Action::SelectInstrument => {
            if app.current_view == AppView::InstrumentList {
                app.select_instrument();
            }
        }
        Action::ToggleInstrumentMiniPanel => {
            app.toggle_instrument_mini_panel();
        }
        Action::ToggleInstrumentExpanded => {
            app.toggle_instrument_expanded();
        }

        // Pattern management (only when in pattern list view)
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
            }
        }
        Action::SelectPattern => {
            if app.current_view == AppView::PatternList {
                app.select_pattern();
            }
        }

        // Envelope editor actions
        Action::EnvCycle => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                app.env_editor.cycle_envelope_type();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            }
        }
        Action::EnvPrev => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                app.env_editor.prev_envelope_type();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            }
        }
        Action::EnvMoveUp => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let env_type = app.env_editor.envelope_type;
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    app.env_editor.move_point_up(envelope, env_type);
                }
            }
        }
        Action::EnvMoveDown => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let env_type = app.env_editor.envelope_type;
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    app.env_editor.move_point_down(envelope, env_type);
                }
            }
        }
        Action::EnvMoveLeft => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    app.env_editor.move_point_left(envelope);
                }
            }
        }
        Action::EnvMoveRight => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    app.env_editor.move_point_right(envelope);
                }
            }
        }
        Action::EnvAddPoint => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    let frame = app
                        .env_editor
                        .selected_point
                        .and_then(|i| envelope.points.get(i).map(|p| p.frame))
                        .unwrap_or(0);
                    app.env_editor
                        .add_point_at(envelope, frame.saturating_add(32), 0.5);
                }
            }
        }
        Action::EnvDeletePoint => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    app.env_editor.delete_selected_point(envelope);
                }
            }
        }
        Action::EnvSelectFirst => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_first_point(envelope);
                }
            }
        }
        Action::EnvSelectLast => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                    app.env_editor.select_last_point(envelope);
                }
            }
        }
        Action::EnvChangeValue => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    let envelope = app
                        .env_editor
                        .get_envelope_mut(&mut app.song.instruments[idx]);
                    let delta = if key.code == crossterm::event::KeyCode::Char('+')
                        || key.code == crossterm::event::KeyCode::Char('=')
                    {
                        0.05
                    } else {
                        -0.05
                    };
                    app.env_editor.change_value(envelope, delta);
                }
            }
        }
        Action::EnvToggleEnabled => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                if let Some(idx) = app.instrument_selection() {
                    app.env_editor
                        .toggle_envelope_enabled(&mut app.song.instruments[idx]);
                }
            }
        }

        Action::WfTogglePencil => {
            if let Some(_idx) = app.instrument_selection() {
                app.waveform_editor.focus();
                app.waveform_editor.toggle_pencil_mode();
            }
        }
        Action::WfToggleLoop => {
            if let Some(idx) = app.instrument_selection() {
                app.waveform_editor.focus();
                app.waveform_editor.toggle_loop_mode();
                let sample_idx = app.song.instruments[idx].sample_index;
                if let Some(si) = sample_idx {
                    if let Some(sample) = app.loaded_samples().get(si) {
                        let mut s = sample.as_ref().clone();
                        if app.waveform_editor.is_loop_mode_enabled() {
                            let frame_count = s.frame_count();
                            s.loop_mode = tracker_core::audio::sample::LoopMode::Forward;
                            s.loop_start = 0;
                            s.loop_end = frame_count.saturating_sub(1);
                        } else {
                            s.loop_mode = tracker_core::audio::sample::LoopMode::NoLoop;
                        }
                        app.set_sample_loop_settings(
                            idx,
                            si,
                            s.loop_mode,
                            s.loop_start,
                            s.loop_end,
                        );
                    }
                }
            }
        }
        Action::WfSetLoopStart => {
            if let Some(idx) = app.instrument_selection() {
                app.waveform_editor.focus();
                if let Some(si) = app.song.instruments[idx].sample_index {
                    if let Some(sample) = app.loaded_samples().get(si) {
                        let s = sample.as_ref();
                        app.set_sample_loop_settings(
                            idx,
                            si,
                            tracker_core::audio::sample::LoopMode::Forward,
                            app.waveform_editor.cursor_sample,
                            s.loop_end,
                        );
                    }
                }
            }
        }
        Action::WfSetLoopEnd => {
            if let Some(idx) = app.instrument_selection() {
                app.waveform_editor.focus();
                if let Some(si) = app.song.instruments[idx].sample_index {
                    if let Some(sample) = app.loaded_samples().get(si) {
                        let s = sample.as_ref();
                        app.set_sample_loop_settings(
                            idx,
                            si,
                            tracker_core::audio::sample::LoopMode::Forward,
                            s.loop_start,
                            app.waveform_editor.cursor_sample,
                        );
                    }
                }
            }
        }
        Action::WfMoveCursorLeft => {
            if let Some(idx) = app.instrument_selection() {
                if let Some(sample) = app
                    .loaded_samples()
                    .get(app.song.instruments[idx].sample_index.unwrap_or(0))
                {
                    app.waveform_editor.focus();
                    app.waveform_editor.move_cursor_left(sample.frame_count());
                }
            }
        }
        Action::WfMoveCursorRight => {
            if let Some(idx) = app.instrument_selection() {
                if let Some(sample) = app
                    .loaded_samples()
                    .get(app.song.instruments[idx].sample_index.unwrap_or(0))
                {
                    app.waveform_editor.focus();
                    app.waveform_editor.move_cursor_right(sample.frame_count());
                }
            }
        }
        Action::WfValueUp => {
            app.waveform_editor.focus();
            app.waveform_editor.pencil_value_up();
        }
        Action::WfValueDown => {
            app.waveform_editor.focus();
            app.waveform_editor.pencil_value_down();
        }
        Action::WfDrawSample => {
            if app.draw_waveform_sample().is_ok() {
                app.waveform_editor.focus();
            }
        }
        Action::WfFocus => {
            app.waveform_editor.focus();
        }
        Action::WfUnfocus => {
            app.waveform_editor.unfocus();
        }

        Action::None => {}
    }
}

/// Handle keys when the instrument editor panel is focused.
/// Returns true if the key was consumed.
fn handle_instrument_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    use crate::ui::instrument_editor::InstrumentField;
    use crossterm::event::{KeyCode, KeyModifiers};

    if !app.inst_editor.focused {
        return false;
    }

    // Text-edit mode for Name field
    if app.inst_editor.text_editing {
        match key.code {
            KeyCode::Enter => {
                if let Some(new_name) = app.inst_editor.finish_text_edit() {
                    app.set_instrument_name(new_name);
                }
                return true;
            }
            KeyCode::Esc => {
                app.inst_editor.cancel_text_edit();
                return true;
            }
            KeyCode::Backspace => {
                app.inst_editor.input_buffer.pop();
                return true;
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.inst_editor.input_buffer.push(c);
                return true;
            }
            _ => return true, // Swallow all other keys while text-editing
        }
    }

    // Normal editor navigation
    match key.code {
        KeyCode::Char('j') => {
            app.inst_editor.next_field();
            return true;
        }
        KeyCode::Char('k') => {
            app.inst_editor.prev_field();
            return true;
        }
        KeyCode::Esc => {
            app.inst_editor.unfocus();
            return true;
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            if app.inst_editor.field == InstrumentField::Name {
                if let Some(idx) = app.instrument_selection() {
                    if idx < app.song.instruments.len() {
                        let name = app.song.instruments[idx].name.clone();
                        app.inst_editor.start_text_edit(&name);
                    }
                }
            }
            return true;
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            match app.inst_editor.field {
                InstrumentField::Volume => app.adjust_instrument_volume(5),
                InstrumentField::BaseNote => app.adjust_instrument_base_note(1),
                InstrumentField::Finetune => app.adjust_instrument_finetune(1),
                InstrumentField::LoopStart => app.adjust_instrument_loop_start(100),
                InstrumentField::LoopEnd => app.adjust_instrument_loop_end(100),
                InstrumentField::KeyzoneNoteMin => app.adjust_keyzone_note_min(1),
                InstrumentField::KeyzoneNoteMax => app.adjust_keyzone_note_max(1),
                InstrumentField::KeyzoneVelMin => app.adjust_keyzone_velocity_min(1),
                InstrumentField::KeyzoneVelMax => app.adjust_keyzone_velocity_max(1),
                InstrumentField::Name => {}
                InstrumentField::LoopMode
                | InstrumentField::KeyzoneList
                | InstrumentField::KeyzoneSample
                | InstrumentField::KeyzoneBaseNote => {}
            }
            return true;
        }
        KeyCode::Char('-') => {
            match app.inst_editor.field {
                InstrumentField::Volume => app.adjust_instrument_volume(-5),
                InstrumentField::BaseNote => app.adjust_instrument_base_note(-1),
                InstrumentField::Finetune => app.adjust_instrument_finetune(-1),
                InstrumentField::LoopStart => app.adjust_instrument_loop_start(-100),
                InstrumentField::LoopEnd => app.adjust_instrument_loop_end(-100),
                InstrumentField::KeyzoneNoteMin => app.adjust_keyzone_note_min(-1),
                InstrumentField::KeyzoneNoteMax => app.adjust_keyzone_note_max(-1),
                InstrumentField::KeyzoneVelMin => app.adjust_keyzone_velocity_min(-1),
                InstrumentField::KeyzoneVelMax => app.adjust_keyzone_velocity_max(-1),
                InstrumentField::Name => {}
                InstrumentField::LoopMode
                | InstrumentField::KeyzoneList
                | InstrumentField::KeyzoneSample
                | InstrumentField::KeyzoneBaseNote => {}
            }
            return true;
        }
        KeyCode::Char(' ') => {
            if app.inst_editor.field == InstrumentField::LoopMode {
                app.cycle_instrument_loop_mode();
            }
            return true;
        }
        _ => {}
    }
    false
}

/// Handle keys when the envelope editor panel is focused.
/// Returns true if the key was consumed.
fn handle_envelope_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::KeyCode;

    if !app.env_editor.focused {
        return false;
    }

    let Some(idx) = app.instrument_selection() else {
        return false;
    };

    match key.code {
        KeyCode::Esc => {
            app.env_editor.unfocus();
            true
        }
        KeyCode::Tab => {
            app.env_editor.cycle_envelope_type();
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            app.env_editor.select_first_point(envelope);
            true
        }
        KeyCode::BackTab => {
            app.env_editor.prev_envelope_type();
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            app.env_editor.select_first_point(envelope);
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let env_type = app.env_editor.envelope_type;
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.move_point_up(envelope, env_type);
            app.mark_dirty();
            true
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let env_type = app.env_editor.envelope_type;
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.move_point_down(envelope, env_type);
            app.mark_dirty();
            true
        }
        KeyCode::Char('h') | KeyCode::Left => {
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.move_point_left(envelope);
            app.mark_dirty();
            true
        }
        KeyCode::Char('l') | KeyCode::Right => {
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.move_point_right(envelope);
            app.mark_dirty();
            true
        }
        KeyCode::Char('0') => {
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            app.env_editor.select_first_point(envelope);
            true
        }
        KeyCode::Char('$') => {
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            app.env_editor.select_last_point(envelope);
            true
        }
        KeyCode::Char('a') => {
            let frame = {
                let sel = app.env_editor.selected_point;
                let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
                sel.and_then(|i| envelope.points.get(i).map(|p| p.frame.saturating_add(4)))
                    .unwrap_or(0)
            };
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.add_point_at(envelope, frame, 0.5);
            app.mark_dirty();
            true
        }
        KeyCode::Char('x') => {
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.delete_selected_point(envelope);
            app.mark_dirty();
            true
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.change_value(envelope, 0.05);
            app.mark_dirty();
            true
        }
        KeyCode::Char('-') => {
            let envelope = app
                .env_editor
                .get_envelope_mut(&mut app.song.instruments[idx]);
            app.env_editor.change_value(envelope, -0.05);
            app.mark_dirty();
            true
        }
        KeyCode::Char('e') => {
            app.env_editor
                .toggle_envelope_enabled(&mut app.song.instruments[idx]);
            app.mark_dirty();
            true
        }
        _ => false,
    }
}

fn handle_waveform_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    use tracker_core::audio::sample::LoopMode;

    if !app.waveform_editor.focused {
        return false;
    }

    if key.modifiers != KeyModifiers::NONE {
        return false;
    }

    let idx = match app.instrument_selection() {
        Some(i) if i < app.song.instruments.len() => i,
        _ => return false,
    };

    // Pencil-mode-specific keys
    if app.waveform_editor.edit_mode == crate::ui::waveform_editor::WaveformEditMode::Pencil {
        match key.code {
            KeyCode::Up => {
                app.waveform_editor.pencil_value_up();
                return true;
            }
            KeyCode::Down => {
                app.waveform_editor.pencil_value_down();
                return true;
            }
            KeyCode::Enter => {
                let _ = app.draw_waveform_sample();
                return true;
            }
            KeyCode::Char('p') => {
                app.waveform_editor.exit_pencil_mode();
                return true;
            }
            _ => {}
        }
    }

    // Keys common to both modes
    match key.code {
        KeyCode::Char('h') | KeyCode::Left => {
            if let Some(si) = app.song.instruments[idx].sample_index {
                if let Some(s) = app.loaded_samples().get(si) {
                    app.waveform_editor.move_cursor_left(s.frame_count());
                }
            }
            true
        }
        KeyCode::Right => {
            if let Some(si) = app.song.instruments[idx].sample_index {
                if let Some(s) = app.loaded_samples().get(si) {
                    app.waveform_editor.move_cursor_right(s.frame_count());
                }
            }
            true
        }
        KeyCode::Char('l') => {
            app.cycle_instrument_loop_mode();
            true
        }
        KeyCode::Char('[') => {
            if let Some(si) = app.song.instruments[idx].sample_index {
                if let Some(sample) = app.loaded_samples().get(si) {
                    let cursor = app.waveform_editor.cursor_sample;
                    let loop_end = sample.loop_end.max(cursor);
                    app.set_sample_loop_settings(idx, si, LoopMode::Forward, cursor, loop_end);
                }
            }
            true
        }
        KeyCode::Char(']') => {
            if let Some(si) = app.song.instruments[idx].sample_index {
                if let Some(sample) = app.loaded_samples().get(si) {
                    let cursor = app.waveform_editor.cursor_sample;
                    let loop_start = sample.loop_start.min(cursor);
                    app.set_sample_loop_settings(idx, si, LoopMode::Forward, loop_start, cursor);
                }
            }
            true
        }
        KeyCode::Char('p') => {
            app.waveform_editor.toggle_pencil_mode();
            true
        }
        KeyCode::Esc => {
            app.waveform_editor.unfocus();
            true
        }
        _ => false,
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
                        app.command_input.clear();
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

/// Lazily update the waveform panel cache to match the current browser selection.
///
/// Loads peaks when a WAV file is selected and the cache is stale; clears the
/// cache when the selection is a directory or an unsupported file type.
fn maybe_update_waveform(app: &mut App) {
    use crate::ui::sample_browser::compute_waveform_peaks;

    let path = match app
        .sample_browser
        .selected_path()
        .filter(|_| app.sample_browser.selected_is_file())
        .map(|p| p.to_path_buf())
    {
        Some(p) => p,
        None => {
            app.sample_browser.clear_waveform();
            return;
        }
    };

    let is_wav = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    if !is_wav {
        app.sample_browser.clear_waveform();
        return;
    }

    // Only reload when the selection has changed.
    if app.sample_browser.waveform_path() == Some(path.as_path()) {
        return;
    }

    let peaks = compute_waveform_peaks(&path, 128);
    app.sample_browser.set_waveform_peaks(path, peaks);
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

    let consumed = match key.code {
        // Navigation — also clears any active preview so offset resets on item change
        KeyCode::Char('j') | KeyCode::Down => {
            app.reset_browser_preview();
            app.sample_browser.move_down();
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.reset_browser_preview();
            app.sample_browser.move_up();
            true
        }

        // Enter directory (l always navigates; Right scrubs when previewing)
        KeyCode::Char('l') => {
            app.sample_browser.enter_dir();
            true
        }
        KeyCode::Right => {
            if app.browser_preview_active {
                app.scrub_browser_preview(true);
            } else {
                app.sample_browser.enter_dir();
            }
            true
        }

        // Go up a directory (h / Backspace always navigate; Left scrubs when previewing)
        KeyCode::Char('h') | KeyCode::Backspace => {
            app.sample_browser.go_up();
            true
        }
        KeyCode::Left => {
            if app.browser_preview_active {
                app.scrub_browser_preview(false);
            } else {
                app.sample_browser.go_up();
            }
            true
        }

        // Jump back to the roots list from anywhere in the filesystem
        KeyCode::Char('~') => {
            app.sample_browser.reset_to_roots();
            true
        }

        // Load file or enter directory
        KeyCode::Enter => {
            if app.sample_browser.selected_is_file() {
                let path = app.sample_browser.selected_path().map(|p| p.to_path_buf());
                if let Some(path) = path {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    if matches!(ext.as_str(), "mod" | "xm" | "it" | "s3m") {
                        // Module files always import the whole song — no choice needed.
                        match app.import_file(&path) {
                            Ok(()) => {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("file")
                                    .to_string();
                                app.open_modal(ui::modal::Modal::info(
                                    "Module Imported".to_string(),
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
                    } else {
                        // Show an action menu so the user can choose what to do.
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("sample")
                            .to_string();

                        let assign_line = app
                            .instrument_selection()
                            .and_then(|i| {
                                app.song.instruments.get(i).map(|inst| {
                                    format!("\n  [a]  Assign to '{}' (slot {:02X})", inst.name, i)
                                })
                            })
                            .unwrap_or_default();

                        let message =
                            format!("'{filename}'\n\n  [l]  Load as new instrument{assign_line}");

                        app.pending_sample_path = Some(path);
                        app.open_modal(ui::modal::Modal::menu("Load Sample".to_string(), message));
                    }
                }
            } else {
                app.sample_browser.enter_dir();
            }
            true
        }

        // Preview selected file — Space toggles play/stop; does not restart if already playing
        KeyCode::Char(' ') => {
            if app.sample_browser.selected_is_file() {
                app.toggle_browser_preview();
            }
            true
        }

        // Bookmark selected directory — b toggles bookmark, persists to config
        KeyCode::Char('b') => {
            app.toggle_browser_bookmark();
            true
        }

        _ => false,
    };

    if consumed {
        maybe_update_waveform(app);
    }
    consumed
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
        KeyCode::Char('h') | KeyCode::Left => {
            app.file_browser.go_up();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.file_browser.selected_is_dir() {
                app.file_browser.enter_selected_dir();
            }
        }
        KeyCode::Enter => {
            if app.file_browser.selected_is_dir() {
                app.file_browser.enter_selected_dir();
                return;
            }

            let is_module = app
                .file_browser
                .selected_path()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .map(|e| {
                    let e = e.to_ascii_lowercase();
                    e == "mod" || e == "xm" || e == "it" || e == "s3m"
                })
                .unwrap_or(false);

            if is_module {
                let path = app.file_browser.selected_path().map(|p| p.to_path_buf());
                if let Some(path) = path {
                    match app.import_file(&path) {
                        Ok(()) => {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            app.close_file_browser();
                            app.open_modal(ui::modal::Modal::info(
                                "Module Imported".to_string(),
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

fn point_in_rect(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

fn instrument_view_chunks(content_area: ratatui::layout::Rect) -> [ratatui::layout::Rect; 4] {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(20),
        ])
        .split(content_area);

    [chunks[0], chunks[1], chunks[2], chunks[3]]
}

fn apply_instrument_field_drag(
    app: &mut App,
    field: crate::ui::instrument_editor::InstrumentField,
    delta: i16,
) {
    if delta == 0 {
        return;
    }

    match field {
        crate::ui::instrument_editor::InstrumentField::BaseNote => {
            app.adjust_instrument_base_note(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::Volume => {
            app.adjust_instrument_volume(delta as i32 * 5);
        }
        crate::ui::instrument_editor::InstrumentField::Finetune => {
            app.adjust_instrument_finetune(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::LoopStart => {
            app.adjust_instrument_loop_start(delta as i32 * 128);
        }
        crate::ui::instrument_editor::InstrumentField::LoopEnd => {
            app.adjust_instrument_loop_end(delta as i32 * 128);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneNoteMin => {
            app.adjust_keyzone_note_min(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneNoteMax => {
            app.adjust_keyzone_note_max(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneVelMin => {
            app.adjust_keyzone_velocity_min(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneVelMax => {
            app.adjust_keyzone_velocity_max(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::Name
        | crate::ui::instrument_editor::InstrumentField::LoopMode
        | crate::ui::instrument_editor::InstrumentField::KeyzoneList
        | crate::ui::instrument_editor::InstrumentField::KeyzoneSample
        | crate::ui::instrument_editor::InstrumentField::KeyzoneBaseNote => {}
    }
}

fn handle_instrument_editor_mouse(app: &mut App, mouse: MouseEvent, area: ratatui::layout::Rect) {
    use crate::ui::instrument_editor::{field_at_row, InstrumentField};

    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !point_in_rect(area, mouse.column, mouse.row) {
                return;
            }

            app.inst_editor.focus();
            if mouse.row < inner.y || mouse.row >= inner.y + inner.height {
                return;
            }

            let row_offset = mouse.row.saturating_sub(inner.y);
            let Some(field) = field_at_row(row_offset) else {
                return;
            };
            app.inst_editor.field = field;

            match field {
                InstrumentField::LoopMode => app.cycle_instrument_loop_mode(),
                InstrumentField::Name => {}
                _ if field.is_draggable() => {
                    app.inst_editor.start_drag(field, mouse.column, mouse.row);
                }
                _ => {}
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let Some(field) = app.inst_editor.dragging() else {
                return;
            };
            let Some((dx, dy)) = app
                .inst_editor
                .update_drag_position(mouse.column, mouse.row)
            else {
                return;
            };
            apply_instrument_field_drag(app, field, dx - dy);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.inst_editor.end_drag();
        }
        _ => {}
    }
}

fn handle_envelope_mouse(app: &mut App, mouse: MouseEvent, area: ratatui::layout::Rect) {
    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);
    if inner.width == 0 || inner.height < 4 {
        return;
    }

    let graph_height = inner.height.saturating_sub(3);
    if graph_height == 0 {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !point_in_rect(area, mouse.column, mouse.row) {
                return;
            }

            app.env_editor.focus();
            if mouse.column < inner.x
                || mouse.column >= inner.x + inner.width
                || mouse.row < inner.y
                || mouse.row >= inner.y + graph_height
            {
                return;
            }

            let Some(idx) = app.instrument_selection() else {
                return;
            };
            let env_type = app.env_editor.envelope_type;
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            let local_x = mouse.column.saturating_sub(inner.x);
            let local_y = mouse.row.saturating_sub(inner.y);

            let selected = crate::ui::envelope_editor::point_at_position(
                envelope,
                env_type,
                inner.width as usize,
                graph_height as usize,
                local_x,
                local_y,
            );
            app.env_editor.select_point(selected);
            if selected.is_some() {
                app.env_editor.start_drag(0.0);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if !app.env_editor.is_dragging() {
                return;
            }
            let Some(idx) = app.instrument_selection() else {
                return;
            };
            if mouse.column < inner.x || mouse.row < inner.y {
                return;
            }

            let local_x = mouse.column.saturating_sub(inner.x);
            let local_y = mouse.row.saturating_sub(inner.y);
            let env_type = app.env_editor.envelope_type;
            if let Some(selected_point) = app.env_editor.selected_point {
                let envelope = app
                    .env_editor
                    .get_envelope_mut(&mut app.song.instruments[idx]);
                crate::ui::envelope_editor::update_point_from_position(
                    envelope,
                    env_type,
                    selected_point,
                    inner.width as usize,
                    graph_height as usize,
                    local_x,
                    local_y.min(graph_height.saturating_sub(1)),
                );
                app.mark_dirty();
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.env_editor.end_drag();
        }
        _ => {}
    }
}

fn handle_instrument_view_mouse(
    app: &mut App,
    mouse: MouseEvent,
    content_area: ratatui::layout::Rect,
) {
    let [list_area, editor_area, envelope_area, waveform_area] =
        instrument_view_chunks(content_area);

    if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
        app.inst_editor.end_drag();
        app.env_editor.end_drag();
        app.waveform_editor.end_loop_marker_drag();
    }

    if matches!(mouse.kind, MouseEventKind::ScrollDown) {
        app.instrument_selection_down();
        return;
    }
    if matches!(mouse.kind, MouseEventKind::ScrollUp) {
        app.instrument_selection_up();
        return;
    }

    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        && point_in_rect(list_area, mouse.column, mouse.row)
    {
        let inner = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .inner(list_area);
        let row = mouse.row.saturating_sub(inner.y);
        let instrument_idx = row.saturating_sub(1) as usize;
        if row >= 1 && instrument_idx < app.song.instruments.len() {
            app.set_instrument_selection(Some(instrument_idx));
        }
    }

    if app.instrument_selection().is_none() {
        return;
    }

    let inst_dragging = app.inst_editor.dragging().is_some();
    let env_dragging = app.env_editor.is_dragging();
    let wf_dragging = app.waveform_editor.is_loop_marker_dragging();

    if point_in_rect(editor_area, mouse.column, mouse.row) || inst_dragging {
        handle_instrument_editor_mouse(app, mouse, editor_area);
    }
    if point_in_rect(envelope_area, mouse.column, mouse.row) || env_dragging {
        handle_envelope_mouse(app, mouse, envelope_area);
    }
    if point_in_rect(waveform_area, mouse.column, mouse.row) || wf_dragging {
        handle_waveform_mouse(app, mouse, waveform_area);
    }
}

/// Handle mouse events for pattern editor navigation and selection.
fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    use crate::ui::layout;

    if app.has_modal() || app.has_export_dialog() || app.has_file_browser() {
        return;
    }

    if app.show_help || app.show_tutor {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if app.show_help {
                    app.help_scroll = app.help_scroll.saturating_add(3);
                } else if app.show_tutor {
                    app.tutor_scroll = app.tutor_scroll.saturating_add(3);
                }
            }
            MouseEventKind::ScrollUp => {
                if app.show_help {
                    app.help_scroll = app.help_scroll.saturating_sub(3);
                } else if app.show_tutor {
                    app.tutor_scroll = app.tutor_scroll.saturating_sub(3);
                }
            }
            _ => {}
        }
        return;
    }

    let full_area = ratatui::layout::Rect::new(0, 0, 80, 24);
    let (_header_area, content_area, _footer_area) = layout::create_main_layout(full_area, 3, 1);

    if app.current_view == AppView::InstrumentList {
        handle_instrument_view_mouse(app, mouse, content_area);
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            if app.current_view == AppView::PatternEditor {
                app.editor.page_down();
            } else if app.current_view == AppView::Arrangement {
                let len = app.song.arrangement.len();
                app.arrangement_view.move_down(len);
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_down();
            }
        }
        MouseEventKind::ScrollUp => {
            if app.current_view == AppView::PatternEditor {
                app.editor.page_up();
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_view.move_up();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_up();
            }
        }
        MouseEventKind::Down(btn) | MouseEventKind::Drag(btn) | MouseEventKind::Up(btn) => {
            if btn != MouseButton::Left && btn != MouseButton::Right {
                return;
            }

            // Check if click is in header area - reset horizontal view
            if mouse.row < content_area.y && app.channel_scroll > 0 {
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    app.reset_horizontal_view();
                }
                return;
            }

            if mouse.column < content_area.x
                || mouse.column >= content_area.x + content_area.width
                || mouse.row < content_area.y
                || mouse.row >= content_area.y + content_area.height
            {
                return;
            }

            let mouse_x = mouse.column;
            let mouse_y = mouse.row;

            let pattern_width = content_area.width.saturating_sub(2);
            let pattern_height = content_area.height.saturating_sub(2);
            let pattern_x = content_area.x + 1;
            let pattern_y = content_area.y + 1;

            let _pattern_area =
                ratatui::layout::Rect::new(pattern_x, pattern_y, pattern_width, pattern_height);

            let ch_scroll = calculate_channel_scroll_for_mouse(
                app.editor.cursor_channel(),
                pattern_width,
                app.editor.pattern().num_channels(),
            );

            let header_height = 1u16;
            let visible_rows = pattern_height.saturating_sub(header_height) as usize;
            let scroll_offset = calculate_scroll_offset_for_mouse(
                app.editor.cursor_row(),
                visible_rows,
                app.editor.pattern().num_rows(),
            );

            match btn {
                MouseButton::Left => match mouse.kind {
                    MouseEventKind::Down(_) => {
                        if app.current_view == AppView::PatternEditor {
                            let local_x = mouse_x.saturating_sub(pattern_x);
                            let local_y = mouse_y.saturating_sub(pattern_y);

                            if local_y < header_height {
                                return;
                            }

                            let (row, ch) = app.editor.set_cursor_from_mouse(
                                local_y.saturating_sub(header_height),
                                local_x,
                                scroll_offset,
                                ch_scroll,
                            );

                            if app.editor.mode() != EditorMode::Visual {
                                if app.editor.mode() == EditorMode::Insert {
                                    app.editor.enter_normal_mode();
                                }
                                app.editor.enter_visual_mode();
                                app.editor.set_visual_anchor(row, ch);
                            } else {
                                app.editor.set_visual_anchor(row, ch);
                            }
                        }
                    }
                    MouseEventKind::Drag(_) => {
                        if app.current_view == AppView::PatternEditor
                            && app.editor.mode() == EditorMode::Visual
                        {
                            let local_x = mouse_x.saturating_sub(pattern_x);
                            let local_y = mouse_y.saturating_sub(pattern_y);

                            if local_y < header_height {
                                return;
                            }

                            let (row, ch) = app.editor.set_cursor_from_mouse(
                                local_y.saturating_sub(header_height),
                                local_x,
                                scroll_offset,
                                ch_scroll,
                            );
                            app.editor.set_cursor(row, ch);
                        }
                    }
                    MouseEventKind::Up(_) => {}
                    _ => {}
                },
                MouseButton::Right => {
                    if app.current_view == AppView::PatternEditor {
                        let local_x = mouse_x.saturating_sub(pattern_x);
                        let local_y = mouse_y.saturating_sub(pattern_y);

                        if local_y < header_height {
                            return;
                        }

                        let _ = app.editor.set_cursor_from_mouse(
                            local_y.saturating_sub(header_height),
                            local_x,
                            scroll_offset,
                            ch_scroll,
                        );
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_waveform_mouse(app: &mut App, mouse: MouseEvent, waveform_area: ratatui::layout::Rect) {
    // Check if click is within waveform area
    if mouse.column < waveform_area.x
        || mouse.column >= waveform_area.x + waveform_area.width
        || mouse.row < waveform_area.y
        || mouse.row >= waveform_area.y + waveform_area.height
    {
        return;
    }

    let idx = match app.instrument_selection() {
        Some(i) if i < app.song.instruments.len() => i,
        _ => return,
    };

    let sample_idx = match app.song.instruments[idx].sample_index {
        Some(si) => si,
        None => return,
    };

    let samples = app.loaded_samples();
    let sample = match samples.get(sample_idx) {
        Some(s) => s.as_ref(),
        None => return,
    };

    let frame_count = sample.frame_count();
    if frame_count == 0 {
        return;
    }

    // Calculate which sample frame the mouse is pointing to
    let local_x = mouse.column.saturating_sub(waveform_area.x + 2); // Account for left padding
    let grid_width = (waveform_area.width.saturating_sub(4)).max(1) as usize;
    let frame_at_cursor = if local_x < grid_width as u16 {
        ((local_x as usize * frame_count) / grid_width).min(frame_count.saturating_sub(1))
    } else {
        frame_count.saturating_sub(1)
    };

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check for Shift key to set loop start
            if mouse
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
            {
                app.waveform_editor
                    .start_loop_marker_drag(crate::ui::waveform_editor::LoopMarkerDrag::Start);
                app.set_sample_loop_settings(
                    idx,
                    sample_idx,
                    tracker_core::audio::sample::LoopMode::Forward,
                    frame_at_cursor,
                    sample.loop_end,
                );
            }
            // Check for Ctrl key to set loop end
            else if mouse
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                app.set_sample_loop_settings(
                    idx,
                    sample_idx,
                    tracker_core::audio::sample::LoopMode::Forward,
                    sample.loop_start,
                    frame_at_cursor,
                );
            }
            // Normal click: move cursor to position
            else {
                app.waveform_editor.set_cursor(frame_at_cursor);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Drag while holding left button: update loop marker being dragged
            match app.waveform_editor.dragging_loop_marker() {
                crate::ui::waveform_editor::LoopMarkerDrag::Start => {
                    app.set_sample_loop_settings(
                        idx,
                        sample_idx,
                        tracker_core::audio::sample::LoopMode::Forward,
                        frame_at_cursor.min(sample.loop_end.saturating_sub(1)),
                        sample.loop_end,
                    );
                }
                crate::ui::waveform_editor::LoopMarkerDrag::End => {
                    app.set_sample_loop_settings(
                        idx,
                        sample_idx,
                        tracker_core::audio::sample::LoopMode::Forward,
                        sample.loop_start,
                        frame_at_cursor.max(sample.loop_start + 1),
                    );
                }
                crate::ui::waveform_editor::LoopMarkerDrag::None => {}
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.waveform_editor.end_loop_marker_drag();
        }
        _ => {}
    }
}

fn calculate_channel_scroll_for_mouse(
    cursor_channel: usize,
    available_width: u16,
    num_channels: usize,
) -> usize {
    const CHANNEL_COL_WIDTH: u16 = 17;
    const ROW_NUM_WIDTH: u16 = 6;

    let channel_space = available_width.saturating_sub(ROW_NUM_WIDTH);
    let visible_channels = (channel_space / CHANNEL_COL_WIDTH) as usize;
    if visible_channels == 0 {
        return 0;
    }
    if visible_channels >= num_channels {
        return 0;
    }
    if cursor_channel < visible_channels / 2 {
        0
    } else if cursor_channel + visible_channels / 2 >= num_channels {
        num_channels.saturating_sub(visible_channels)
    } else {
        cursor_channel.saturating_sub(visible_channels / 2)
    }
}

fn calculate_scroll_offset_for_mouse(
    cursor_row: usize,
    visible_rows: usize,
    total_rows: usize,
) -> usize {
    if visible_rows >= total_rows {
        return 0;
    }
    if cursor_row < visible_rows / 2 {
        0
    } else if cursor_row + visible_rows / 2 >= total_rows {
        total_rows.saturating_sub(visible_rows)
    } else {
        cursor_row.saturating_sub(visible_rows / 2)
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
