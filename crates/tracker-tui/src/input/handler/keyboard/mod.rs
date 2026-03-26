use crate::app::{App, AppView};
use crate::editor::{Editor, EditorMode, SubColumn};
use crate::input::keybindings::{map_key_to_action, Action};
use crate::ui;
use crate::ui::code_editor::ModeKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod actions;
mod browsers;
mod code_editor;
mod export;
mod panels;

use browsers::{handle_file_browser_key, handle_sample_browser_key};
use code_editor::handle_code_editor_key;
use export::{handle_export_dialog_key, hex_char_to_digit};
use panels::{handle_envelope_editor_key, handle_instrument_editor_key, handle_waveform_editor_key};

pub fn handle_key_event(app: &mut App, key: KeyEvent) {
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

    actions::handle_action(app, action, key);
}
