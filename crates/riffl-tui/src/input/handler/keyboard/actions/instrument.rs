use crate::app::{App, AppView};
use crate::editor::{EditorMode, SubColumn};
use crate::input::keybindings::Action;
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle(app: &mut App, action: &Action, key: KeyEvent) -> bool {
    match action {
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
            } else if app.current_view == AppView::PatternList {
                app.command_mode = true;
                app.command_input = "pname ".to_string();
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
        Action::PreviewInstrument => {
            if let Some(idx) = app.instrument_selection() {
                use riffl_core::pattern::note::Pitch;
                app.preview_instrument_note_pitch(idx, Pitch::C, 4);
            }
        }
        Action::ToggleInstrumentMiniPanel => app.toggle_instrument_mini_panel(),
        Action::ToggleInstrumentExpanded => app.toggle_instrument_expanded(),
        Action::InstrumentNextTab => app.inst_editor.next_tab(),
        Action::InstrumentPrevTab => app.inst_editor.prev_tab(),

        // Envelope editor
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
                let env_type = app.env_editor.envelope_type;
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.move_point_up(envelope, env_type);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvMoveDown => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let env_type = app.env_editor.envelope_type;
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.move_point_down(envelope, env_type);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvMoveLeft => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.move_point_left(envelope);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvMoveRight => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.move_point_right(envelope);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvAddPoint => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    let frame = env_editor
                        .selected_point
                        .and_then(|i| envelope.points.get(i).map(|p| p.frame))
                        .unwrap_or(0);
                    env_editor.add_point_at(envelope, frame.saturating_add(32), 0.5);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvDeletePoint => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.delete_selected_point(envelope);
                });
                app.env_editor = env_editor;
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
                let delta = if key.code == KeyCode::Char('+') || key.code == KeyCode::Char('=') {
                    0.05
                } else {
                    -0.05
                };
                let mut env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    let envelope = env_editor.get_envelope_mut(inst);
                    env_editor.change_value(envelope, delta);
                });
                app.env_editor = env_editor;
            }
        }
        Action::EnvToggleEnabled => {
            if app.current_view == AppView::InstrumentList && app.instrument_selection().is_some() {
                app.env_editor.focus();
                let env_editor = app.env_editor.clone();
                app.modify_instrument(true, |inst| {
                    env_editor.toggle_envelope_enabled(inst);
                });
            }
        }
        Action::Undo => {
            if app.current_view == AppView::InstrumentList {
                app.undo_global();
            } else {
                return false;
            }
        }
        Action::Redo => {
            if app.current_view == AppView::InstrumentList {
                app.redo_global();
            } else {
                return false;
            }
        }

        // Waveform editor
        Action::WfTogglePencil => {
            if app.instrument_selection().is_some() {
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
                            s.loop_mode = riffl_core::audio::sample::LoopMode::Forward;
                            s.loop_start = 0;
                            s.loop_end = frame_count.saturating_sub(1);
                        } else {
                            s.loop_mode = riffl_core::audio::sample::LoopMode::NoLoop;
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
                            riffl_core::audio::sample::LoopMode::Forward,
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
                            riffl_core::audio::sample::LoopMode::Forward,
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
            // Note: modify_sample should ideally handle WfValueUp directly
            // For now, we will handle this inside the Waveform Editor or app.sample methods if needed.
            // In the interest of keeping the plan simple, waveform pencil drawing is
            // handled through `draw_waveform_sample`. But WfValueUp just changes the pencil value, not the sample.
            app.waveform_editor.focus();
            app.waveform_editor.pencil_value_up();
        }
        Action::WfValueDown => {
            // Just changes pencil value, no sample modification.
            app.waveform_editor.focus();
            app.waveform_editor.pencil_value_down();
        }
        Action::WfDrawSample => {
            if app.draw_waveform_sample().is_ok() {
                app.waveform_editor.focus();
            }
        }
        Action::WfFocus => app.waveform_editor.focus(),
        Action::WfUnfocus => app.waveform_editor.unfocus(),

        _ => return false,
    }
    true
}
