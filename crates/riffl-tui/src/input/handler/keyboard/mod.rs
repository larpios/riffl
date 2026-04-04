use crate::app::{App, AppView};
use crate::editor::{Editor, EditorMode, SubColumn};
use crate::input::keybindings::{map_key_to_action, Action};
use crate::registry::{CommandMetadata, CommandRegistry};
use crate::ui;
use crate::ui::code_editor::ModeKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod actions;
mod browsers;
mod code_editor;
mod export;
mod panels;
pub mod prompts;

use browsers::{handle_file_browser_key, handle_sample_browser_key};
use code_editor::handle_code_editor_key;
use export::{handle_export_dialog_key, hex_char_to_digit};
use panels::{
    handle_envelope_editor_key, handle_instrument_editor_key, handle_waveform_editor_key,
};
use prompts::PromptAction;

/// The active input context, used to route key events to the right handler.
/// Variants are ordered by priority — higher-priority contexts are checked first
/// in `current_input_context()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputContext {
    BpmPrompt,
    LenPrompt,
    CommandMode,
    Modal,
    TutorOverlay,
    EffectHelpOverlay,
    HelpOverlay,
    ExportDialog,
    FileBrowser,
    SampleBrowser,
    InstrumentEditorPanel,
    EnvelopeEditorPanel,
    WaveformEditorPanel,
    CodeEditor,
    /// Default — pattern editor, arrangement, instrument list (non-panel), etc.
    Normal,
}

/// Determine which input context is active based on the current app state.
/// This is the single authoritative place that encodes dispatch priority.
fn current_input_context(app: &App) -> InputContext {
    if app.bpm_prompt.active {
        return InputContext::BpmPrompt;
    }
    if app.len_prompt.active {
        return InputContext::LenPrompt;
    }
    if app.command_mode {
        return InputContext::CommandMode;
    }
    if app.has_modal() {
        return InputContext::Modal;
    }
    if app.show_tutor {
        return InputContext::TutorOverlay;
    }
    if app.show_effect_help {
        return InputContext::EffectHelpOverlay;
    }
    if app.show_help {
        return InputContext::HelpOverlay;
    }
    if app.has_export_dialog() {
        return InputContext::ExportDialog;
    }
    if app.has_file_browser() {
        return InputContext::FileBrowser;
    }
    if app.current_view == AppView::SampleBrowser {
        return InputContext::SampleBrowser;
    }
    if app.current_view == AppView::InstrumentList {
        if app.inst_editor.focused {
            return InputContext::InstrumentEditorPanel;
        }
        if app.env_editor.focused {
            return InputContext::EnvelopeEditorPanel;
        }
        if app.waveform_editor.focused {
            return InputContext::WaveformEditorPanel;
        }
    }
    if app.is_code_editor_active() {
        return InputContext::CodeEditor;
    }
    InputContext::Normal
}

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

    match current_input_context(app) {
        InputContext::BpmPrompt => {
            if let Some(PromptAction::Confirm) = app.bpm_prompt.handle_key(key) {
                app.execute_bpm_prompt();
            }
        }

        InputContext::LenPrompt => {
            if let Some(PromptAction::Confirm) = app.len_prompt.handle_key(key) {
                app.execute_len_prompt();
            }
        }

        InputContext::CommandMode => match key.code {
            KeyCode::Enter => {
                app.command_tab_prefix = None;
                app.command_completion_idx = None;
                app.execute_command();
            }
            KeyCode::Esc => {
                app.command_mode = false;
                app.command_input.clear();
                app.command_history_index = None;
                app.command_tab_prefix = None;
                app.command_completion_idx = None;
            }
            KeyCode::Backspace => {
                app.command_input.pop();
                app.command_tab_prefix = None;
                app.command_completion_idx = None;
            }
            KeyCode::Up => {
                if app.command_completion_idx.is_some() {
                    let prefix = app.command_tab_prefix.clone().unwrap_or_default();
                    let matching: Vec<_> = CommandRegistry::all_commands()
                        .into_iter()
                        .filter(|c| {
                            std::iter::once(c.name())
                                .chain(c.aliases())
                                .any(|n| n.starts_with(prefix.as_str()))
                        })
                        .collect();
                    if !matching.is_empty() {
                        let idx = app.command_completion_idx.unwrap_or(0);
                        let prev = if idx == 0 {
                            matching.len() - 1
                        } else {
                            idx - 1
                        };
                        app.command_completion_idx = Some(prev);
                        app.command_input = matching[prev].name().to_string();
                    }
                } else if let Some(idx) = app.command_history_index {
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
                if app.command_completion_idx.is_some() {
                    let prefix = app.command_tab_prefix.clone().unwrap_or_default();
                    let matching: Vec<_> = CommandRegistry::all_commands()
                        .into_iter()
                        .filter(|c| {
                            std::iter::once(c.name())
                                .chain(c.aliases())
                                .any(|n| n.starts_with(prefix.as_str()))
                        })
                        .collect();
                    if !matching.is_empty() {
                        let idx = app.command_completion_idx.unwrap_or(0);
                        let next = (idx + 1) % matching.len();
                        app.command_completion_idx = Some(next);
                        app.command_input = matching[next].name().to_string();
                    }
                } else if let Some(idx) = app.command_history_index {
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
                let prefix = app.command_tab_prefix.clone().unwrap_or_else(|| {
                    app.command_input
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string()
                });
                app.command_tab_prefix = Some(prefix.clone());

                let matching: Vec<_> = CommandRegistry::all_commands()
                    .into_iter()
                    .filter(|c| {
                        std::iter::once(c.name())
                            .chain(c.aliases())
                            .any(|n| n.starts_with(prefix.as_str()))
                    })
                    .collect();

                if matching.is_empty() {
                    return;
                }

                let next = match app.command_completion_idx {
                    None => 0,
                    Some(i) => (i + 1) % matching.len(),
                };
                app.command_completion_idx = Some(next);
                app.command_input = matching[next].name().to_string();
            }
            KeyCode::Char(c)
                if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.command_input.push(c);
                app.command_history_index = None;
                app.command_tab_prefix = None;
                app.command_completion_idx = None;
            }
            _ => {}
        },

        InputContext::Modal => match key.code {
            KeyCode::Enter if app.pending_quit => {
                app.close_modal();
                app.force_quit();
            }
            KeyCode::Esc if app.pending_quit => {
                app.pending_quit = false;
                app.close_modal();
            }
            KeyCode::Char('l') if app.pending_sample_path.is_some() => {
                let path = app.pending_sample_path.take().unwrap();
                app.close_modal();
                match app.load_sample_from_path(&path) {
                    Ok(idx) => {
                        let name = app
                            .song
                            .instruments
                            .get(idx)
                            .map(|i| i.name.clone())
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
            }
            KeyCode::Esc if app.pending_sample_path.is_some() => {
                app.pending_sample_path = None;
                app.close_modal();
            }
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                app.close_modal();
            }
            _ => {}
        },

        InputContext::TutorOverlay => {
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
        }

        InputContext::EffectHelpOverlay => {
            let max_scroll = 100u16.saturating_sub(20);
            match key.code {
                KeyCode::Esc | KeyCode::Char('K') | KeyCode::Char('q') => {
                    app.show_effect_help = false;
                    app.effect_help_scroll = 0;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    app.effect_help_scroll =
                        app.effect_help_scroll.saturating_add(1).min(max_scroll);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.effect_help_scroll = app.effect_help_scroll.saturating_sub(1);
                }
                KeyCode::PageDown => {
                    app.effect_help_scroll =
                        app.effect_help_scroll.saturating_add(10).min(max_scroll);
                }
                KeyCode::PageUp => {
                    app.effect_help_scroll = app.effect_help_scroll.saturating_sub(10);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    app.effect_help_scroll = 0;
                }
                _ => {}
            }
        }

        InputContext::HelpOverlay => {
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
        }

        InputContext::ExportDialog => handle_export_dialog_key(app, key),

        InputContext::FileBrowser => handle_file_browser_key(app, key),

        // SampleBrowser: if the browser doesn't consume the key, fall through
        // to normal processing so global actions (view switching, transport, etc.) work.
        InputContext::SampleBrowser => {
            if !handle_sample_browser_key(app, key) {
                handle_normal_key(app, key);
            }
        }

        // Panel contexts: if the panel doesn't consume the key, fall through
        // to normal processing (which includes Tab-cycle and action dispatch).
        InputContext::InstrumentEditorPanel => {
            if !handle_instrument_editor_key(app, key) {
                handle_normal_key(app, key);
            }
        }
        InputContext::EnvelopeEditorPanel => {
            if !handle_envelope_editor_key(app, key) {
                handle_normal_key(app, key);
            }
        }
        InputContext::WaveformEditorPanel => {
            if !handle_waveform_editor_key(app, key) {
                handle_normal_key(app, key);
            }
        }

        InputContext::CodeEditor => handle_code_editor_key(app, key),

        InputContext::Normal => handle_normal_key(app, key),
    }
}

/// Handle a key event in the "normal" context: InstrumentList Tab cycling,
/// pattern editor hex entry, replace-once, chord handling, and action dispatch.
/// Also called as a fallback when SampleBrowser or panel handlers don't consume a key.
fn handle_normal_key(app: &mut App, key: KeyEvent) {
    // In InstrumentList, Tab/Shift-Tab cycle focus between the right panels:
    //   none → env_editor → waveform_editor → none  (Tab)
    //   none → waveform_editor → env_editor → inst_editor → none  (Shift-Tab)
    let is_tab = key.code == crossterm::event::KeyCode::Tab;
    let is_backtab = key.code == crossterm::event::KeyCode::BackTab;
    if app.current_view == AppView::InstrumentList
        && app
            .instrument_selection()
            .is_some_and(|i| i < app.song.instruments.len())
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
        } else {
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

    // Escape during playback: stop transport
    if key.code == crossterm::event::KeyCode::Esc && !app.transport.is_stopped() {
        app.stop();
    }

    // In Insert/Replace mode on non-Note sub-columns, intercept hex digit keys.
    if matches!(app.editor.mode(), EditorMode::Insert | EditorMode::Replace)
        && key.modifiers == crossterm::event::KeyModifiers::NONE
    {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if let Some(digit) = hex_char_to_digit(c) {
                match app.editor.sub_column() {
                    SubColumn::Effect | SubColumn::Effect2 => app.editor.enter_effect_digit(digit),
                    SubColumn::Instrument => app.editor.enter_instrument_digit(digit),
                    SubColumn::Volume => app.editor.enter_volume_digit(digit),
                    SubColumn::Note => {}
                }
                if app.editor.sub_column() != SubColumn::Note {
                    app.mark_dirty();
                    return;
                }
            }
        }
    }

    // Replace-once: r was pressed, intercept the next key.
    if app.pending_replace {
        app.pending_replace = false;
        if app.editor.sub_column() == SubColumn::Note {
            match key.code {
                crossterm::event::KeyCode::Esc => {}
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
                _ => {}
            }
        }
        return;
    }

    // Count prefix accumulation (Normal and Visual modes, no pending chord).
    // Digits 1-9 grow the repetition count; '0' extends it only when prefix is
    // non-empty (empty prefix → '0' is the GoToStart motion, not a count).
    let in_modal_mode = matches!(
        app.editor.mode(),
        EditorMode::Normal | EditorMode::Visual | EditorMode::VisualLine
    );
    if in_modal_mode
        && key.modifiers == crossterm::event::KeyModifiers::NONE
        && app.pending_key.is_none()
    {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && (c != '0' || !app.editor.count_prefix().is_empty()) {
                app.editor.push_count_digit(c);
                return;
            }
        }
    }

    // Chord / prefix handling.
    // 'gg' (go to top) works in Normal and Visual modes.
    // All other chords (dd, m{x}, '{x}, "{x}, q{x}, @{x}) are Normal-mode only.
    let in_visual = app.editor.mode().is_visual();
    if in_modal_mode && key.modifiers == crossterm::event::KeyModifiers::NONE {
        if let crossterm::event::KeyCode::Char(c) = key.code {
            if let Some(pending) = app.pending_key.take() {
                // 'gg' works in all three modes; clears any pending count.
                if pending == 'g' && c == 'g' {
                    app.editor.clear_count();
                    app.editor.go_to_row(0);
                    return;
                }

                // The remaining chords are Normal-mode only
                if !in_visual {
                    match (pending, c) {
                        ('d', 'd') => {
                            app.editor.clear_count();
                            app.editor.delete_row();
                            app.mark_dirty();
                            return;
                        }
                        // m{x} — set mark
                        ('m', x) if x.is_ascii_alphabetic() => {
                            app.editor.clear_count();
                            app.editor.set_mark(x);
                            return;
                        }
                        // '{x} — goto mark
                        ('\'', x) if x.is_ascii_alphabetic() => {
                            app.editor.clear_count();
                            app.editor.goto_mark(x);
                            return;
                        }
                        // "{x} — set active register for next yank/paste/cut
                        ('"', x) if x.is_ascii_alphanumeric() => {
                            app.editor.clear_count();
                            app.editor.set_active_register(x);
                            return;
                        }
                        // q{x} — start recording macro into register x
                        ('q', x) if x.is_ascii_alphabetic() => {
                            app.editor.clear_count();
                            if !app.replaying_macro {
                                app.macros.entry(x).or_default().clear();
                                app.macro_recording = Some(x);
                            }
                            return;
                        }
                        // @@ — replay last used macro
                        ('@', '@') => {
                            app.editor.clear_count();
                            if let Some(slot) = app.last_macro {
                                replay_macro(app, slot, key);
                            }
                            return;
                        }
                        // @{x} — replay macro in register x
                        ('@', x) if x.is_ascii_alphabetic() => {
                            app.editor.clear_count();
                            replay_macro(app, x, key);
                            return;
                        }
                        _ => {
                            // Unknown chord — re-dispatch the second key normally
                            // by falling through (pending already taken)
                        }
                    }
                }
                // Unknown chord in visual mode or unmatched normal-mode chord:
                // fall through and re-dispatch the second key.
            }

            // Stop macro recording: q while recording (Normal mode only, no second char)
            if !in_visual && c == 'q' && app.macro_recording.is_some() {
                app.macro_recording = None;
                return;
            }

            // Set pending chord starters.
            // 'g' starts a pending chord in all modes (for 'gg').
            // Starting a chord discards any accumulated count.
            // Other chord starters are Normal-mode only.
            if c == 'g' {
                app.editor.clear_count();
                app.pending_key = Some('g');
                return;
            }
            if !in_visual {
                match c {
                    'd' | 'm' | '\'' | '"' | '@' => {
                        app.editor.clear_count();
                        app.pending_key = Some(c);
                        return;
                    }
                    // q: if recording, stop; otherwise start pending for register char
                    'q' => {
                        app.editor.clear_count();
                        app.pending_key = Some('q');
                        return;
                    }
                    _ => {}
                }
            }
        } else {
            app.pending_key = None;
        }
    } else {
        app.pending_key = None;
    }

    let action = map_key_to_action(key, app.editor_mode());

    // Consume the count prefix. Any action clears it; motions may repeat.
    let count = app.editor.take_count();

    // In InstrumentList, preview notes instead of editing the pattern.
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
            Action::EnterNoteOff | Action::EnterNoteCut => return,
            _ => {}
        }
    }

    // nG / ngg: with a count > 1, go to that specific row (1-indexed).
    // This mirrors vim's behaviour where `5G` jumps to row 5.
    if count > 1 && matches!(action, Action::GoToBottom | Action::GoToTop) {
        app.editor.go_to_row(count.saturating_sub(1));
        return;
    }

    // Repeatable motions run count times.
    let is_repeatable = matches!(
        action,
        Action::MoveDown
            | Action::MoveUp
            | Action::MoveLeft
            | Action::MoveRight
            | Action::PageUp
            | Action::PageDown
            | Action::NextTrack
            | Action::PrevTrack
            | Action::JumpNextPattern
            | Action::JumpPrevPattern
            | Action::JumpToNextNote
            | Action::JumpToPrevNote
    );

    if is_repeatable && count > 1 {
        // Record a single action entry to the macro (not count copies).
        if let Some(slot) = app.macro_recording {
            if !matches!(action, Action::None) {
                app.macros.entry(slot).or_default().push(action);
            }
        }
        for _ in 0..count {
            actions::handle_action(app, action, key);
        }
    } else {
        dispatch_action(app, action, key);
    }
}

/// Dispatch an action, recording it into the active macro if recording.
fn dispatch_action(app: &mut App, action: Action, key: KeyEvent) {
    // Record non-trivial actions to the active macro buffer.
    if let Some(slot) = app.macro_recording {
        if !matches!(action, Action::None) {
            app.macros.entry(slot).or_default().push(action);
        }
    }
    actions::handle_action(app, action, key);
}

/// Replay a macro stored in the given register slot.
fn replay_macro(app: &mut App, slot: char, original_key: KeyEvent) {
    if app.replaying_macro {
        return; // Prevent infinite loops
    }
    let actions_to_run: Vec<Action> = match app.macros.get(&slot) {
        Some(v) if !v.is_empty() => v.clone(),
        _ => return,
    };
    app.last_macro = Some(slot);
    app.replaying_macro = true;
    for action in actions_to_run {
        actions::handle_action(app, action, original_key);
    }
    app.replaying_macro = false;
}
