use crate::app::{App, AppView};
use crate::editor::Editor;
use crate::input::handler::keyboard::export::hex_char_to_digit;
use crate::input::keybindings::Action;

pub(super) fn handle(app: &mut App, action: &Action) -> bool {
    match action {
        // Mode transitions
        Action::EnterInsertMode => app.editor.enter_insert_mode(),
        Action::EnterNormalMode => app.editor.enter_normal_mode(),
        Action::EnterVisualMode => app.editor.enter_visual_mode(),
        Action::EnterVisualLineMode => app.editor.enter_visual_line_mode(),
        Action::EnterReplaceMode => app.editor.enter_replace_mode(),

        // Note entry
        Action::EnterNote(c) => {
            if app.current_view == AppView::Arrangement {
                if let Some(digit) = hex_char_to_digit(*c) {
                    app.arrangement_set_pattern_digit(digit);
                }
                return true;
            }
            if let Some((pitch, oct_offset)) = Editor::piano_key_to_pitch(*c) {
                let base_octave = app.editor.current_octave();
                let octave = (base_octave as i8 + oct_offset).clamp(0, 9) as u8;
                app.editor.enter_note_with_octave(pitch, octave);
                {
                    use riffl_core::pattern::note::{Note, NoteEvent};
                    let inst = app.editor.current_instrument();
                    app.draw_note = Some(NoteEvent::On(Note::new(pitch, octave, 127, inst)));
                }
                app.mark_dirty();
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
        Action::SetOctave(oct) => app.editor.set_octave(*oct),
        Action::StepUp => app.editor.step_up(),
        Action::StepDown => app.editor.step_down(),
        Action::OctaveUp => app.editor.octave_up(),
        Action::OctaveDown => app.editor.octave_down(),

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
        Action::Undo => {
            app.editor.undo();
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

        // Pattern editing
        Action::Quantize => {
            app.editor.quantize();
            app.mark_dirty();
        }
        Action::Interpolate => {
            app.editor.interpolate();
            app.mark_dirty();
        }
        Action::FillSelection => {
            use riffl_core::pattern::note::{Note, NoteEvent, Pitch};
            let note = app.draw_note.unwrap_or_else(|| {
                NoteEvent::On(Note::new(Pitch::C, 4, 127, app.editor.current_instrument()))
            });
            app.editor.fill_selection_with_note(note);
            app.mark_dirty();
        }
        Action::RandomizeNotes => {
            app.editor.randomize_notes();
            app.mark_dirty();
        }
        Action::ReverseSelection => {
            app.editor.reverse_selection();
            app.mark_dirty();
        }
        Action::HumanizeNotes => {
            app.editor.humanize_notes(8);
            app.mark_dirty();
        }

        // Macro actions are handled in the keyboard handler layer, not here.
        Action::SetMark(_)
        | Action::GotoMark(_)
        | Action::SetRegister(_)
        | Action::StartMacroRecord(_)
        | Action::StopMacroRecord
        | Action::ReplayMacro(_)
        | Action::ReplayLastMacro => {}

        // Bookmarks
        Action::AddBookmark => app.editor.add_bookmark(None),
        Action::NextBookmark => app.editor.goto_next_bookmark(),
        Action::PrevBookmark => app.editor.goto_prev_bookmark(),

        // Row operations
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

        _ => return false,
    }
    true
}
