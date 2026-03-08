use crate::app::{App, AppView};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keys when the instrument editor panel is focused.
/// Returns true if the key was consumed.
pub fn handle_instrument_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
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
                InstrumentField::LoopMode => {}
                InstrumentField::KeyzoneList => app.adjust_keyzone_selection(1),
                InstrumentField::KeyzoneSample => app.adjust_keyzone_sample(1),
                InstrumentField::KeyzoneBaseNote => app.adjust_keyzone_base_note(1),
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
                InstrumentField::LoopMode => {}
                InstrumentField::KeyzoneList => app.adjust_keyzone_selection(-1),
                InstrumentField::KeyzoneSample => app.adjust_keyzone_sample(-1),
                InstrumentField::KeyzoneBaseNote => app.adjust_keyzone_base_note(-1),
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
pub fn handle_envelope_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
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

pub fn handle_waveform_editor_key(app: &mut App, key: crossterm::event::KeyEvent) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    use riffl_core::audio::sample::LoopMode;

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
