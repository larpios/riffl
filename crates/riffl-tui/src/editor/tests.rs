use super::*;

fn test_editor() -> Editor {
    Editor::new(Pattern::new(16, 4))
}

// --- Mode Tests ---

#[test]
fn test_initial_mode_is_normal() {
    let editor = test_editor();
    assert_eq!(editor.mode(), EditorMode::Normal);
}

#[test]
fn test_enter_insert_mode() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    assert_eq!(editor.mode(), EditorMode::Insert);
}

#[test]
fn test_enter_normal_mode_from_insert() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_normal_mode();
    assert_eq!(editor.mode(), EditorMode::Normal);
}

#[test]
fn test_enter_visual_mode() {
    let mut editor = test_editor();
    editor.move_down();
    editor.move_down();
    editor.enter_visual_mode();
    assert_eq!(editor.mode(), EditorMode::Visual);
    assert_eq!(editor.visual_anchor, Some((2, 0)));
}

#[test]
fn test_visual_mode_returns_to_normal() {
    let mut editor = test_editor();
    editor.enter_visual_mode();
    editor.enter_normal_mode();
    assert_eq!(editor.mode(), EditorMode::Normal);
    assert!(editor.visual_anchor.is_none());
}

#[test]
fn test_mode_labels() {
    assert_eq!(EditorMode::Normal.label(), "NORMAL");
    assert_eq!(EditorMode::Insert.label(), "INSERT");
    assert_eq!(EditorMode::Visual.label(), "VISUAL");
    assert_eq!(EditorMode::Replace.label(), "REPLACE");
}

#[test]
fn test_replace_mode_enter_and_exit() {
    let mut editor = test_editor();
    assert_eq!(editor.mode(), EditorMode::Normal);
    editor.enter_replace_mode();
    assert_eq!(editor.mode(), EditorMode::Replace);
    assert!(editor.is_entry_mode());
    editor.enter_normal_mode();
    assert_eq!(editor.mode(), EditorMode::Normal);
}

#[test]
fn test_is_entry_mode() {
    let mut editor = test_editor();
    assert!(!editor.is_entry_mode());
    editor.enter_insert_mode();
    assert!(editor.is_entry_mode());
    editor.enter_replace_mode();
    assert!(editor.is_entry_mode());
    editor.enter_visual_mode();
    assert!(!editor.is_entry_mode());
}

// --- Navigation Tests ---

#[test]
fn test_move_up() {
    let mut editor = test_editor();
    editor.cursor_row = 5;
    editor.move_up();
    assert_eq!(editor.cursor_row(), 4);
}

#[test]
fn test_move_up_at_top() {
    let mut editor = test_editor();
    editor.move_up();
    assert_eq!(editor.cursor_row(), 0);
}

#[test]
fn test_move_down() {
    let mut editor = test_editor();
    editor.move_down();
    assert_eq!(editor.cursor_row(), 1);
}

#[test]
fn test_move_down_at_bottom() {
    let mut editor = test_editor();
    editor.cursor_row = 15;
    editor.move_down();
    assert_eq!(editor.cursor_row(), 15);
}

#[test]
fn test_move_left_normal_mode() {
    let mut editor = test_editor();
    // move_left retreats sub-column, not channel
    editor.sub_column = SubColumn::Effect;
    editor.move_left();
    assert_eq!(editor.sub_column(), SubColumn::Volume);
    assert_eq!(editor.cursor_channel(), 0);
}

#[test]
fn test_move_left_wraps_to_prev_channel() {
    let mut editor = test_editor();
    editor.cursor_channel = 2;
    // at Note sub-column, moving left wraps to Effect2 of previous channel
    editor.move_left();
    assert_eq!(editor.cursor_channel(), 1);
    assert_eq!(editor.sub_column(), SubColumn::Effect2);
}

#[test]
fn test_move_left_at_zero() {
    let mut editor = test_editor();
    // at channel 0, Note sub-column — can't go further left
    editor.move_left();
    assert_eq!(editor.cursor_channel(), 0);
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

#[test]
fn test_move_right_normal_mode() {
    let mut editor = test_editor();
    // move_right advances sub-column, not channel
    assert_eq!(editor.sub_column(), SubColumn::Note);
    editor.move_right();
    assert_eq!(editor.sub_column(), SubColumn::Instrument);
    assert_eq!(editor.cursor_channel(), 0);
}

#[test]
fn test_move_right_wraps_to_next_channel() {
    let mut editor = test_editor();
    editor.sub_column = SubColumn::Effect2;
    editor.move_right();
    assert_eq!(editor.cursor_channel(), 1);
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

#[test]
fn test_move_right_at_max_channel_effect2() {
    let mut editor = test_editor();
    editor.cursor_channel = 3;
    editor.sub_column = SubColumn::Effect2;
    editor.move_right();
    assert_eq!(editor.cursor_channel(), 3);
    assert_eq!(editor.sub_column(), SubColumn::Effect2);
}

#[test]
fn test_page_up() {
    let mut editor = Editor::new(Pattern::new(64, 4));
    editor.cursor_row = 20;
    editor.page_up();
    assert_eq!(editor.cursor_row(), 4);
}

#[test]
fn test_page_up_at_top() {
    let mut editor = Editor::new(Pattern::new(64, 4));
    editor.cursor_row = 5;
    editor.page_up();
    assert_eq!(editor.cursor_row(), 0);
}

#[test]
fn test_page_down() {
    let mut editor = Editor::new(Pattern::new(64, 4));
    editor.page_down();
    assert_eq!(editor.cursor_row(), 16);
}

#[test]
fn test_page_down_at_bottom() {
    let mut editor = Editor::new(Pattern::new(64, 4));
    editor.cursor_row = 60;
    editor.page_down();
    assert_eq!(editor.cursor_row(), 63);
}

#[test]
fn test_home() {
    let mut editor = test_editor();
    editor.cursor_row = 10;
    editor.home();
    assert_eq!(editor.cursor_row(), 0);
}

#[test]
fn test_end() {
    let mut editor = test_editor();
    editor.end();
    assert_eq!(editor.cursor_row(), 15);
}

// --- Sub-column Navigation in Insert Mode ---

#[test]
fn test_insert_mode_move_right_sub_columns() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    assert_eq!(editor.sub_column(), SubColumn::Note);
    editor.move_right();
    assert_eq!(editor.sub_column(), SubColumn::Instrument);
    editor.move_right();
    assert_eq!(editor.sub_column(), SubColumn::Volume);
    editor.move_right();
    assert_eq!(editor.sub_column(), SubColumn::Effect);
}

#[test]
fn test_insert_mode_move_right_wraps_channel() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect2;
    editor.move_right();
    assert_eq!(editor.cursor_channel(), 1);
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

#[test]
fn test_insert_mode_move_left_sub_columns() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.move_left();
    assert_eq!(editor.sub_column(), SubColumn::Volume);
    editor.move_left();
    assert_eq!(editor.sub_column(), SubColumn::Instrument);
    editor.move_left();
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

#[test]
fn test_insert_mode_move_left_wraps_channel() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.cursor_channel = 1;
    editor.sub_column = SubColumn::Note;
    editor.move_left();
    assert_eq!(editor.cursor_channel(), 0);
    assert_eq!(editor.sub_column(), SubColumn::Effect2);
}

#[test]
fn test_insert_mode_move_left_at_start_no_wrap() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    // At channel 0, sub_column Note — cannot go further left
    editor.move_left();
    assert_eq!(editor.cursor_channel(), 0);
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

// --- Note Entry Tests ---

#[test]
fn test_enter_note() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    // After entering a note, cursor should advance down
    assert_eq!(editor.cursor_row(), 1);
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    match cell.note {
        Some(NoteEvent::On(note)) => {
            assert_eq!(note.pitch, Pitch::C);
            assert_eq!(note.octave, 4);
        }
        _ => panic!("Expected note-on event"),
    }
}

#[test]
fn test_enter_note_in_normal_mode_does_nothing() {
    let mut editor = test_editor();
    editor.enter_note(Pitch::C);
    assert_eq!(editor.cursor_row(), 0);
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

#[test]
fn test_enter_note_off() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note_off();
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert_eq!(cell.note, Some(NoteEvent::Off));
}

#[test]
fn test_enter_note_cut() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note_cut();
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert_eq!(cell.note, Some(NoteEvent::Cut));
}

#[test]
fn test_enter_note_cut_in_normal_mode_does_nothing() {
    let mut editor = test_editor();
    // mode is Normal by default
    editor.enter_note_cut();
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

#[test]
fn test_set_octave() {
    let mut editor = test_editor();
    editor.set_octave(7);
    assert_eq!(editor.current_octave(), 7);
    editor.enter_insert_mode();
    editor.enter_note(Pitch::A);
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    match cell.note {
        Some(NoteEvent::On(note)) => assert_eq!(note.octave, 7),
        _ => panic!("Expected note-on"),
    }
}

#[test]
fn test_set_octave_out_of_range() {
    let mut editor = test_editor();
    editor.set_octave(10);
    assert_eq!(editor.current_octave(), 4); // unchanged
}

#[test]
fn test_piano_key_to_pitch() {
    // Lower row — white keys
    assert_eq!(Editor::piano_key_to_pitch('a'), Some((Pitch::C, 0)));
    assert_eq!(Editor::piano_key_to_pitch('s'), Some((Pitch::D, 0)));
    assert_eq!(Editor::piano_key_to_pitch('d'), Some((Pitch::E, 0)));
    assert_eq!(Editor::piano_key_to_pitch('f'), Some((Pitch::F, 0)));
    assert_eq!(Editor::piano_key_to_pitch('g'), Some((Pitch::G, 0)));
    assert_eq!(Editor::piano_key_to_pitch('h'), Some((Pitch::A, 0)));
    assert_eq!(Editor::piano_key_to_pitch('j'), Some((Pitch::B, 0)));
    // k = C in next octave
    assert_eq!(Editor::piano_key_to_pitch('k'), Some((Pitch::C, 1)));
    // Upper row — black keys
    assert_eq!(Editor::piano_key_to_pitch('w'), Some((Pitch::CSharp, 0)));
    assert_eq!(Editor::piano_key_to_pitch('e'), Some((Pitch::DSharp, 0)));
    assert_eq!(Editor::piano_key_to_pitch('t'), Some((Pitch::FSharp, 0)));
    assert_eq!(Editor::piano_key_to_pitch('y'), Some((Pitch::GSharp, 0)));
    assert_eq!(Editor::piano_key_to_pitch('u'), Some((Pitch::ASharp, 0)));
    // Non-piano keys
    assert_eq!(Editor::piano_key_to_pitch('c'), None);
    assert_eq!(Editor::piano_key_to_pitch('b'), None);
    assert_eq!(Editor::piano_key_to_pitch('x'), None);
    assert_eq!(Editor::piano_key_to_pitch('1'), None);
}

// --- Delete/Clear Tests ---

#[test]
fn test_delete_cell() {
    let mut editor = test_editor();
    // First set a note
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0; // go back to the cell
    editor.enter_normal_mode();
    // Delete it
    editor.delete_cell();
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

// --- Row Operations Tests ---

#[test]
fn test_insert_row() {
    let mut editor = test_editor();
    assert_eq!(editor.pattern().num_rows(), 16);
    editor.insert_row();
    assert_eq!(editor.pattern().num_rows(), 17);
}

#[test]
fn test_insert_row_shifts_data() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    // Note is now at row 0
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.insert_row();
    // Row 0 should now be empty (inserted), note moved to row 1
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
    assert!(!editor.pattern().get_cell(1, 0).unwrap().is_empty());
}

#[test]
fn test_delete_row() {
    let mut editor = test_editor();
    assert_eq!(editor.pattern().num_rows(), 16);
    editor.delete_row();
    assert_eq!(editor.pattern().num_rows(), 15);
}

#[test]
fn test_delete_row_clamps_cursor() {
    let mut editor = Editor::new(Pattern::new(2, 1));
    editor.cursor_row = 1;
    editor.delete_row();
    // Only 1 row left, cursor should be at 0
    assert_eq!(editor.cursor_row(), 0);
}

// --- Undo Tests ---

#[test]
fn test_undo_restores_pattern() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    assert!(!editor.pattern().get_cell(0, 0).unwrap().is_empty());
    editor.undo();
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

#[test]
fn test_undo_restores_cursor() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C); // cursor moved to row 1
    assert_eq!(editor.cursor_row(), 1);
    editor.undo();
    assert_eq!(editor.cursor_row(), 0);
}

#[test]
fn test_undo_empty_returns_false() {
    let mut editor = test_editor();
    assert!(!editor.undo());
}

#[test]
fn test_multiple_undos() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.enter_note(Pitch::E);
    // Two edits, both should be undoable
    assert!(editor.undo());
    assert!(editor.undo());
    assert!(!editor.undo()); // nothing left
}

// --- Visual Selection Tests ---

#[test]
fn test_visual_selection() {
    let mut editor = test_editor();
    editor.cursor_row = 2;
    editor.cursor_channel = 1;
    editor.enter_visual_mode();
    editor.cursor_row = 5;
    editor.cursor_channel = 3;
    let sel = editor.visual_selection().unwrap();
    assert_eq!(sel, ((2, 1), (5, 3)));
}

#[test]
fn test_visual_selection_reverse() {
    let mut editor = test_editor();
    editor.cursor_row = 5;
    editor.cursor_channel = 3;
    editor.enter_visual_mode();
    editor.cursor_row = 2;
    editor.cursor_channel = 1;
    let sel = editor.visual_selection().unwrap();
    assert_eq!(sel, ((2, 1), (5, 3)));
}

#[test]
fn test_no_visual_selection_in_normal_mode() {
    let editor = test_editor();
    assert!(editor.visual_selection().is_none());
}

// --- Sub-column Tests ---

#[test]
fn test_sub_column_next_cycle() {
    assert_eq!(SubColumn::Note.next(), SubColumn::Instrument);
    assert_eq!(SubColumn::Instrument.next(), SubColumn::Volume);
    assert_eq!(SubColumn::Volume.next(), SubColumn::Effect);
    assert_eq!(SubColumn::Effect.next(), SubColumn::Effect2);
    assert_eq!(SubColumn::Effect2.next(), SubColumn::Note);
}

#[test]
fn test_sub_column_prev_cycle() {
    assert_eq!(SubColumn::Note.prev(), SubColumn::Effect2);
    assert_eq!(SubColumn::Effect2.prev(), SubColumn::Effect);
    assert_eq!(SubColumn::Effect.prev(), SubColumn::Volume);
    assert_eq!(SubColumn::Volume.prev(), SubColumn::Instrument);
    assert_eq!(SubColumn::Instrument.prev(), SubColumn::Note);
}

// --- Clamp Cursor Tests ---

#[test]
fn test_clamp_cursor() {
    let mut editor = Editor::new(Pattern::new(4, 2));
    editor.cursor_row = 10;
    editor.cursor_channel = 5;
    editor.clamp_cursor();
    assert_eq!(editor.cursor_row(), 3);
    assert_eq!(editor.cursor_channel(), 1);
}

// --- Edge Cases ---

#[test]
fn test_enter_note_at_last_row_stays() {
    // Pattern length is fixed; cursor clamps at last row after entry
    let mut editor = Editor::new(Pattern::new(2, 1));
    editor.enter_insert_mode();
    editor.cursor_row = 1;
    editor.enter_note(Pitch::C);
    // Pattern does NOT grow; cursor stays at last row
    assert_eq!(editor.pattern().num_rows(), 2);
    assert_eq!(editor.cursor_row(), 1);
}

// --- Next Track (Tab) Tests ---

#[test]
fn test_next_track() {
    let mut editor = test_editor(); // 4 channels
    assert_eq!(editor.cursor_channel(), 0);
    editor.next_track();
    assert_eq!(editor.cursor_channel(), 1);
    editor.next_track();
    assert_eq!(editor.cursor_channel(), 2);
    editor.next_track();
    assert_eq!(editor.cursor_channel(), 3);
}

#[test]
fn test_next_track_wraps() {
    let mut editor = test_editor(); // 4 channels
    editor.cursor_channel = 3;
    editor.next_track();
    assert_eq!(editor.cursor_channel(), 0);
}

#[test]
fn test_next_track_resets_sub_column() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Volume;
    editor.next_track();
    assert_eq!(editor.sub_column(), SubColumn::Note);
}

#[test]
fn test_default_octave_is_4() {
    let editor = test_editor();
    assert_eq!(editor.current_octave(), 4);
}

#[test]
fn test_default_instrument_is_0() {
    let editor = test_editor();
    assert_eq!(editor.current_instrument(), 0);
}

#[test]
fn test_delete_cell_saves_history() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.delete_cell();
    // Should be able to undo the delete
    assert!(editor.undo());
    assert!(!editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

// --- Clipboard Tests ---

#[test]
fn test_clipboard_single_cell() {
    let cell = Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
    let cb = Clipboard::single(cell);
    assert_eq!(cb.dimensions(), (1, 1));
    assert!(!cb.is_empty());
}

#[test]
fn test_clipboard_rectangular() {
    let cells = vec![
        vec![Cell::empty(), Cell::empty()],
        vec![Cell::empty(), Cell::empty()],
        vec![Cell::empty(), Cell::empty()],
    ];
    let cb = Clipboard::new(cells);
    assert_eq!(cb.dimensions(), (3, 2));
}

#[test]
fn test_copy_single_cell_normal_mode() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.copy();
    let cb = editor.get_clipboard().unwrap();
    assert_eq!(cb.dimensions(), (1, 1));
    assert!(cb.cells()[0][0].note.is_some());
}

#[test]
fn test_copy_visual_selection() {
    let mut editor = test_editor();
    // Set some notes
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C); // row 0 → moves to row 1
    editor.enter_note(Pitch::E); // row 1 → moves to row 2
    editor.enter_normal_mode();
    // Select rows 0-1, channel 0
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 1;
    editor.copy();
    let cb = editor.get_clipboard().unwrap();
    assert_eq!(cb.dimensions(), (2, 1));
    // First cell should have C-4
    match cb.cells()[0][0].note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::C),
        _ => panic!("Expected C note"),
    }
    // Second cell should have E-4
    match cb.cells()[1][0].note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::E),
        _ => panic!("Expected E note"),
    }
}

#[test]
fn test_paste_single_cell() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.copy();
    // Paste at row 5
    editor.cursor_row = 5;
    editor.paste();
    match editor.pattern().get_cell(5, 0).unwrap().note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::C),
        _ => panic!("Expected C note at paste location"),
    }
}

#[test]
fn test_paste_rectangular() {
    let mut editor = Editor::new(Pattern::new(16, 4));
    // Place notes at (0,0) and (1,1)
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C); // (0,0)
    editor.cursor_row = 1;
    editor.cursor_channel = 1;
    editor.enter_note(Pitch::E); // (1,1)
    editor.enter_normal_mode();
    // Select from (0,0) to (1,1)
    editor.cursor_row = 0;
    editor.cursor_channel = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 1;
    editor.cursor_channel = 1;
    editor.copy();
    // Paste at (4,2)
    editor.enter_normal_mode();
    editor.cursor_row = 4;
    editor.cursor_channel = 2;
    editor.paste();
    // Verify pasted content
    match editor.pattern().get_cell(4, 2).unwrap().note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::C),
        _ => panic!("Expected C at (4,2)"),
    }
    match editor.pattern().get_cell(5, 3).unwrap().note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::E),
        _ => panic!("Expected E at (5,3)"),
    }
}

#[test]
fn test_paste_clips_to_pattern_bounds() {
    let mut editor = Editor::new(Pattern::new(4, 2));
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.copy();
    // Paste at last row — should work
    editor.cursor_row = 3;
    editor.paste();
    assert!(editor.pattern().get_cell(3, 0).unwrap().note.is_some());
}

#[test]
fn test_paste_without_clipboard_is_noop() {
    let mut editor = test_editor();
    let original = editor.pattern().clone();
    editor.paste();
    // Pattern should be unchanged
    for r in 0..original.num_rows() {
        for c in 0..original.num_channels() {
            assert_eq!(
                editor.pattern().get_cell(r, c).unwrap().is_empty(),
                original.get_cell(r, c).unwrap().is_empty()
            );
        }
    }
}

#[test]
fn test_cut_normal_mode() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.cut();
    // Cell should be cleared
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
    // Clipboard should have the note
    let cb = editor.get_clipboard().unwrap();
    assert!(cb.cells()[0][0].note.is_some());
}

#[test]
fn test_cut_visual_mode() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.enter_note(Pitch::E);
    editor.enter_normal_mode();
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 1;
    editor.cut();
    // Both cells should be cleared
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
    assert!(editor.pattern().get_cell(1, 0).unwrap().is_empty());
    // Clipboard should have both notes
    let cb = editor.get_clipboard().unwrap();
    assert_eq!(cb.dimensions(), (2, 1));
}

#[test]
fn test_cut_is_undoable() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.cut();
    assert!(editor.pattern().get_cell(0, 0).unwrap().is_empty());
    editor.undo(); // undo the clear
    assert!(!editor.pattern().get_cell(0, 0).unwrap().is_empty());
}

// --- Transpose Tests ---

#[test]
fn test_transpose_single_cell_up() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.transpose_selection(1);
    match editor.pattern().get_cell(0, 0).unwrap().note {
        Some(NoteEvent::On(note)) => {
            assert_eq!(note.pitch, Pitch::CSharp);
            assert_eq!(note.octave, 4);
        }
        _ => panic!("Expected transposed note"),
    }
}

#[test]
fn test_transpose_visual_selection() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.enter_note(Pitch::E);
    editor.enter_normal_mode();
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 1;
    editor.transpose_selection(12); // up one octave
    match editor.pattern().get_cell(0, 0).unwrap().note {
        Some(NoteEvent::On(note)) => {
            assert_eq!(note.pitch, Pitch::C);
            assert_eq!(note.octave, 5);
        }
        _ => panic!("Expected C-5"),
    }
    match editor.pattern().get_cell(1, 0).unwrap().note {
        Some(NoteEvent::On(note)) => {
            assert_eq!(note.pitch, Pitch::E);
            assert_eq!(note.octave, 5);
        }
        _ => panic!("Expected E-5"),
    }
}

#[test]
fn test_transpose_skips_empty_cells() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.enter_normal_mode();
    // Select rows 0-3 (row 0 has note, rest empty)
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 3;
    editor.transpose_selection(1);
    // Row 0 should be transposed
    assert!(editor.pattern().get_cell(0, 0).unwrap().note.is_some());
    // Row 1-3 should still be empty
    assert!(editor.pattern().get_cell(1, 0).unwrap().is_empty());
}

#[test]
fn test_transpose_out_of_range_leaves_note_unchanged() {
    let mut editor = Editor::new(Pattern::new(4, 1));
    editor.enter_insert_mode();
    // Enter B-9 (highest possible)
    editor.set_octave(9);
    editor.enter_note(Pitch::B);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.transpose_selection(1); // can't go higher
    match editor.pattern().get_cell(0, 0).unwrap().note {
        Some(NoteEvent::On(note)) => {
            assert_eq!(note.pitch, Pitch::B);
            assert_eq!(note.octave, 9); // unchanged
        }
        _ => panic!("Expected unchanged note"),
    }
}

#[test]
fn test_transpose_is_undoable() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.enter_note(Pitch::C);
    editor.cursor_row = 0;
    editor.enter_normal_mode();
    editor.transpose_selection(1);
    editor.undo();
    match editor.pattern().get_cell(0, 0).unwrap().note {
        Some(NoteEvent::On(note)) => assert_eq!(note.pitch, Pitch::C),
        _ => panic!("Expected original C"),
    }
}

// --- Interpolation Tests ---

#[test]
fn test_interpolate_volume_ramp() {
    let mut editor = Editor::new(Pattern::new(8, 1));
    // Set volume at row 0 = 0, row 4 = 64
    editor.pattern_mut().set_cell(
        0,
        0,
        Cell {
            volume: Some(0),
            ..Cell::empty()
        },
    );
    editor.pattern_mut().set_cell(
        4,
        0,
        Cell {
            volume: Some(64),
            ..Cell::empty()
        },
    );
    // Select rows 0-4
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 4;
    editor.interpolate();
    // Check interpolated values
    assert_eq!(editor.pattern().get_cell(0, 0).unwrap().volume, Some(0));
    assert_eq!(editor.pattern().get_cell(1, 0).unwrap().volume, Some(16));
    assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(32));
    assert_eq!(editor.pattern().get_cell(3, 0).unwrap().volume, Some(48));
    assert_eq!(editor.pattern().get_cell(4, 0).unwrap().volume, Some(64));
}

#[test]
fn test_interpolate_requires_visual_mode() {
    let mut editor = Editor::new(Pattern::new(8, 1));
    editor.pattern_mut().set_cell(
        0,
        0,
        Cell {
            volume: Some(0),
            ..Cell::empty()
        },
    );
    editor.pattern_mut().set_cell(
        4,
        0,
        Cell {
            volume: Some(64),
            ..Cell::empty()
        },
    );
    // Normal mode — interpolate should be a no-op
    editor.interpolate();
    assert!(editor.pattern().get_cell(2, 0).unwrap().volume.is_none());
}

#[test]
fn test_interpolate_needs_two_endpoints() {
    let mut editor = Editor::new(Pattern::new(8, 1));
    // Only one volume value
    editor.pattern_mut().set_cell(
        0,
        0,
        Cell {
            volume: Some(50),
            ..Cell::empty()
        },
    );
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 4;
    editor.interpolate();
    // Middle rows should still have no volume
    assert!(editor.pattern().get_cell(2, 0).unwrap().volume.is_none());
}

#[test]
fn test_interpolate_is_undoable() {
    let mut editor = Editor::new(Pattern::new(8, 1));
    editor.pattern_mut().set_cell(
        0,
        0,
        Cell {
            volume: Some(0),
            ..Cell::empty()
        },
    );
    editor.pattern_mut().set_cell(
        4,
        0,
        Cell {
            volume: Some(64),
            ..Cell::empty()
        },
    );
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 4;
    editor.interpolate();
    assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(32));
    editor.undo();
    assert!(editor.pattern().get_cell(2, 0).unwrap().volume.is_none());
}

#[test]
fn test_interpolate_descending_ramp() {
    let mut editor = Editor::new(Pattern::new(4, 1));
    editor.pattern_mut().set_cell(
        0,
        0,
        Cell {
            volume: Some(60),
            ..Cell::empty()
        },
    );
    editor.pattern_mut().set_cell(
        3,
        0,
        Cell {
            volume: Some(0),
            ..Cell::empty()
        },
    );
    editor.cursor_row = 0;
    editor.enter_visual_mode();
    editor.cursor_row = 3;
    editor.interpolate();
    assert_eq!(editor.pattern().get_cell(0, 0).unwrap().volume, Some(60));
    assert_eq!(editor.pattern().get_cell(1, 0).unwrap().volume, Some(40));
    assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(20));
    assert_eq!(editor.pattern().get_cell(3, 0).unwrap().volume, Some(0));
}

// --- Effect Digit Entry Tests ---

#[test]
fn test_effect_digit_position_starts_at_zero() {
    let editor = test_editor();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_enter_effect_digit_sets_command() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    // Enter command nibble 0xA (volume slide)
    editor.enter_effect_digit(0xA);

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(eff.command, 0xA);
    assert_eq!(eff.param, 0x00);
    assert_eq!(editor.effect_digit_position(), 1);
}

#[test]
fn test_enter_effect_digit_sets_param_high_nibble() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    editor.enter_effect_digit(0xA); // command
    editor.enter_effect_digit(0x0); // param hi

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(eff.command, 0xA);
    assert_eq!(eff.param, 0x00);
    assert_eq!(editor.effect_digit_position(), 2);
}

#[test]
fn test_enter_effect_digit_sets_param_low_nibble_and_advances() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    editor.enter_effect_digit(0xA); // command = A
    editor.enter_effect_digit(0x0); // param hi = 0
    editor.enter_effect_digit(0x4); // param lo = 4 → "A04"

    // After 3 digits, cursor should advance to next row and reset position
    assert_eq!(editor.cursor_row(), 1);
    assert_eq!(editor.effect_digit_position(), 0);

    // Check the effect on row 0
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(eff.command, 0xA);
    assert_eq!(eff.param, 0x04);
    assert_eq!(format!("{}", eff), "0A04");
}

#[test]
fn test_enter_effect_digit_full_sequence_c40() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    editor.enter_effect_digit(0xC); // Set Volume command
    editor.enter_effect_digit(0x4); // param hi
    editor.enter_effect_digit(0x0); // param lo → "C40"

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(format!("{}", eff), "0C40");
}

#[test]
fn test_enter_effect_digit_full_sequence_fff() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    editor.enter_effect_digit(0xF);
    editor.enter_effect_digit(0xF);
    editor.enter_effect_digit(0xF);

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(format!("{}", eff), "0FFF");
}

#[test]
fn test_enter_effect_digit_only_in_insert_mode() {
    let mut editor = test_editor();
    // Normal mode — should not enter effect
    editor.sub_column = SubColumn::Effect;
    editor.enter_effect_digit(0xA);

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert!(cell.first_effect().is_none());
}

#[test]
fn test_enter_effect_digit_only_on_effect_column() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    // Note column — should not enter effect
    editor.sub_column = SubColumn::Note;
    editor.enter_effect_digit(0xA);

    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert!(cell.first_effect().is_none());
}

#[test]
fn test_effect_digit_position_resets_on_mode_change() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.enter_effect_digit(0xA); // position now 1

    editor.enter_normal_mode();
    assert_eq!(editor.effect_digit_position(), 0);

    editor.enter_insert_mode();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_effect_digit_position_resets_on_cursor_move() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.enter_effect_digit(0xA); // position now 1

    // Moving up should reset
    editor.move_up();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_effect_digit_position_resets_on_move_left() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.enter_effect_digit(0xA); // position now 1

    editor.move_left();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_effect_digit_position_resets_on_move_right() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.cursor_channel = 0;
    editor.enter_effect_digit(0xA); // position now 1

    // Move right wraps to next channel since we're on Effect
    editor.move_right();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_effect_digit_position_resets_on_page_up() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.cursor_row = 10;
    editor.enter_effect_digit(0xA);

    editor.page_up();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_effect_digit_position_resets_on_next_track() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;
    editor.enter_effect_digit(0xA);

    editor.next_track();
    assert_eq!(editor.effect_digit_position(), 0);
}

#[test]
fn test_enter_effect_digit_clamps_to_nibble() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    // Pass a value > 0xF — should be clamped
    editor.enter_effect_digit(0xFF);
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    let eff = cell.first_effect().unwrap();
    assert_eq!(eff.command, 0x0F); // 0xFF & 0x0F = 0x0F
}

#[test]
fn test_enter_effect_digit_supports_undo() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    editor.enter_effect_digit(0xC);
    editor.enter_effect_digit(0x4);
    editor.enter_effect_digit(0x0);

    // Verify effect was placed
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert!(cell.first_effect().is_some());

    // Undo all three edits
    editor.undo();
    editor.undo();
    editor.undo();

    // Effect should be gone
    let cell = editor.pattern().get_cell(0, 0).unwrap();
    assert!(cell.first_effect().is_none());
}

#[test]
fn test_enter_multiple_effects_on_consecutive_rows() {
    let mut editor = test_editor();
    editor.enter_insert_mode();
    editor.sub_column = SubColumn::Effect;

    // Enter A04 on row 0
    editor.enter_effect_digit(0xA);
    editor.enter_effect_digit(0x0);
    editor.enter_effect_digit(0x4);
    assert_eq!(editor.cursor_row(), 1);

    // Enter C40 on row 1
    editor.enter_effect_digit(0xC);
    editor.enter_effect_digit(0x4);
    editor.enter_effect_digit(0x0);
    assert_eq!(editor.cursor_row(), 2);

    // Verify both
    let eff0 = editor
        .pattern()
        .get_cell(0, 0)
        .unwrap()
        .first_effect()
        .unwrap();
    assert_eq!(format!("{}", eff0), "0A04");

    let eff1 = editor
        .pattern()
        .get_cell(1, 0)
        .unwrap()
        .first_effect()
        .unwrap();
    assert_eq!(format!("{}", eff1), "0C40");
}
