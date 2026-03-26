use crate::app::{App, AppView};
use crate::editor::{Editor, EditorMode, SubColumn};
use crate::input::keybindings::Action;
use crate::ui;
use crate::ui::code_editor::ModeKind;
use crossterm::event::{KeyCode, KeyEvent};
use super::export::hex_char_to_digit;

pub(super) fn handle_action(app: &mut App, action: Action, key: KeyEvent) {
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
