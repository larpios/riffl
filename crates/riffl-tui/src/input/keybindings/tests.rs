use super::*;

#[test]
fn test_normal_mode_vim_navigation() {
    let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    let l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);

    assert_eq!(map_key_to_action(h, EditorMode::Normal), Action::MoveLeft);
    assert_eq!(map_key_to_action(j, EditorMode::Normal), Action::MoveDown);
    assert_eq!(map_key_to_action(k, EditorMode::Normal), Action::MoveUp);
    assert_eq!(map_key_to_action(l, EditorMode::Normal), Action::MoveRight);
}

#[test]
fn test_normal_mode_arrow_keys() {
    let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

    assert_eq!(
        map_key_to_action(left, EditorMode::Normal),
        Action::MoveLeft
    );
    assert_eq!(
        map_key_to_action(down, EditorMode::Normal),
        Action::MoveDown
    );
    assert_eq!(map_key_to_action(up, EditorMode::Normal), Action::MoveUp);
    assert_eq!(
        map_key_to_action(right, EditorMode::Normal),
        Action::MoveRight
    );
}

#[test]
fn test_normal_mode_enter_insert() {
    let i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(i, EditorMode::Normal),
        Action::EnterInsertMode
    );
}

#[test]
fn test_normal_mode_enter_visual() {
    let v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(v, EditorMode::Normal),
        Action::EnterVisualMode
    );
}

#[test]
fn test_normal_mode_delete_cell() {
    let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    let del = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
    assert_eq!(map_key_to_action(x, EditorMode::Normal), Action::DeleteCell);
    assert_eq!(
        map_key_to_action(del, EditorMode::Normal),
        Action::DeleteCell
    );
}

#[test]
fn test_normal_mode_undo() {
    let u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(u, EditorMode::Normal), Action::Undo);
}

#[test]
fn test_normal_mode_standard_keys() {
    let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

    // q no longer quits directly — use :q in command mode instead
    assert_eq!(map_key_to_action(q, EditorMode::Normal), Action::None);
    assert_eq!(
        map_key_to_action(enter, EditorMode::Normal),
        Action::Confirm
    );
    assert_eq!(map_key_to_action(esc, EditorMode::Normal), Action::Cancel);
}

#[test]
fn test_normal_mode_toggle_play() {
    let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(space, EditorMode::Normal),
        Action::TogglePlay
    );
}

#[test]
fn test_normal_mode_page_navigation() {
    let pgup = KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE);
    let pgdn = KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE);
    assert_eq!(map_key_to_action(pgup, EditorMode::Normal), Action::PageUp);
    assert_eq!(
        map_key_to_action(pgdn, EditorMode::Normal),
        Action::PageDown
    );
}

#[test]
fn test_normal_mode_modified_keys_ignored() {
    let ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_h, EditorMode::Normal), Action::None);
}

// --- BPM and Transport Tests ---

#[test]
fn test_normal_mode_bpm_up() {
    let eq = KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(eq, EditorMode::Normal), Action::BpmUp);
}

#[test]
fn test_normal_mode_bpm_down() {
    let minus = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(minus, EditorMode::Normal),
        Action::BpmDown
    );
}

#[test]
fn test_normal_mode_view_switching_f1_f2_f3() {
    use crate::app::AppView;
    let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
    let f2 = KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE);
    let f3 = KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(f1, EditorMode::Normal),
        Action::SwitchView(AppView::PatternEditor)
    );
    assert_eq!(
        map_key_to_action(f2, EditorMode::Normal),
        Action::SwitchView(AppView::Arrangement)
    );
    assert_eq!(
        map_key_to_action(f3, EditorMode::Normal),
        Action::SwitchView(AppView::InstrumentList)
    );
}

#[test]
fn test_normal_mode_bpm_shift_plus() {
    let plus = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT);
    assert_eq!(map_key_to_action(plus, EditorMode::Normal), Action::BpmUp);
}

#[test]
fn test_normal_mode_ctrl_f_opens_file_browser() {
    let ctrl_f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_f, EditorMode::Normal),
        Action::OpenFileBrowser
    );
}

#[test]
fn test_normal_mode_toggle_loop() {
    let shift_l = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_l, EditorMode::Normal),
        Action::ToggleLoop
    );
}

// --- Insert Mode Tests ---

#[test]
fn test_insert_mode_note_entry() {
    // Piano keyboard layout — lower row (white keys)
    let a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
    let s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
    let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    // Piano keyboard layout — upper row (black keys)
    let w = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE);
    let t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(a, EditorMode::Insert),
        Action::EnterNote('a')
    );
    assert_eq!(
        map_key_to_action(s, EditorMode::Insert),
        Action::EnterNote('s')
    );
    assert_eq!(
        map_key_to_action(h, EditorMode::Insert),
        Action::EnterNote('h')
    );
    assert_eq!(
        map_key_to_action(k, EditorMode::Insert),
        Action::EnterNote('k')
    );
    assert_eq!(
        map_key_to_action(w, EditorMode::Insert),
        Action::EnterNote('w')
    );
    assert_eq!(
        map_key_to_action(t, EditorMode::Insert),
        Action::EnterNote('t')
    );
    // Old a-g alphabetical note keys (b, c) should NOT trigger note entry
    let b = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE);
    let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(b, EditorMode::Insert), Action::None);
    assert_eq!(map_key_to_action(c, EditorMode::Insert), Action::None);
}

#[test]
fn test_insert_mode_octave_entry() {
    let zero = KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE);
    let five = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
    let nine = KeyEvent::new(KeyCode::Char('9'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(zero, EditorMode::Insert),
        Action::SetOctave(0)
    );
    assert_eq!(
        map_key_to_action(five, EditorMode::Insert),
        Action::SetOctave(5)
    );
    assert_eq!(
        map_key_to_action(nine, EditorMode::Insert),
        Action::SetOctave(9)
    );
}

#[test]
fn test_insert_mode_escape_to_normal() {
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(esc, EditorMode::Insert),
        Action::EnterNormalMode
    );
}

#[test]
fn test_insert_mode_arrow_navigation() {
    let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(map_key_to_action(up, EditorMode::Insert), Action::MoveUp);
    assert_eq!(
        map_key_to_action(down, EditorMode::Insert),
        Action::MoveDown
    );
}

#[test]
fn test_insert_mode_delete() {
    let del = KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE);
    let bs = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
    // Del enters a note-cut (^^^); Backspace deletes the cell
    assert_eq!(
        map_key_to_action(del, EditorMode::Insert),
        Action::EnterNoteCut
    );
    assert_eq!(
        map_key_to_action(bs, EditorMode::Insert),
        Action::DeleteCell
    );
}

#[test]
fn test_insert_mode_toggle_play() {
    let space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(space, EditorMode::Insert),
        Action::TogglePlay
    );
}

// --- Visual Mode Tests ---

#[test]
fn test_visual_mode_escape_to_normal() {
    let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(esc, EditorMode::Visual),
        Action::EnterNormalMode
    );
}

#[test]
fn test_visual_mode_navigation() {
    let h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(h, EditorMode::Visual), Action::MoveLeft);
    assert_eq!(map_key_to_action(j, EditorMode::Visual), Action::MoveDown);
}

#[test]
fn test_visual_mode_delete() {
    let x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(x, EditorMode::Visual), Action::Cut);
}

// --- Utility Function Tests ---

#[test]
fn test_is_navigation_action() {
    assert!(is_navigation_action(Action::MoveLeft));
    assert!(is_navigation_action(Action::MoveDown));
    assert!(is_navigation_action(Action::MoveUp));
    assert!(is_navigation_action(Action::MoveRight));
    assert!(is_navigation_action(Action::PageUp));
    assert!(is_navigation_action(Action::PageDown));

    assert!(!is_navigation_action(Action::Quit));
    assert!(!is_navigation_action(Action::EnterInsertMode));
    assert!(!is_navigation_action(Action::EnterNote('c')));
    assert!(!is_navigation_action(Action::None));
}

#[test]
fn test_normal_mode_o_inserts_row_below() {
    let o = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(o, EditorMode::Normal),
        Action::InsertRowBelow
    );
}

#[test]
fn test_normal_mode_ctrl_f_opens_file_browser2() {
    let ctrl_f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_f, EditorMode::Normal),
        Action::OpenFileBrowser
    );
}

// --- Track Operation Tests ---

#[test]
fn test_normal_mode_toggle_mute() {
    let shift_m = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_m, EditorMode::Normal),
        Action::ToggleMute
    );
}

#[test]
fn test_normal_mode_toggle_solo() {
    let shift_s = KeyEvent::new(KeyCode::Char('S'), KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_s, EditorMode::Normal),
        Action::ToggleSolo
    );
}

#[test]
fn test_normal_mode_next_track() {
    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(tab, EditorMode::Normal),
        Action::NextTrack
    );
}

#[test]
fn test_insert_mode_next_track() {
    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(tab, EditorMode::Insert),
        Action::NextTrack
    );
}

#[test]
fn test_is_modal_dismiss_action() {
    assert!(is_modal_dismiss_action(Action::Cancel));
    assert!(is_modal_dismiss_action(Action::Confirm));
    assert!(is_modal_dismiss_action(Action::EnterNormalMode));

    assert!(!is_modal_dismiss_action(Action::Quit));
    assert!(!is_modal_dismiss_action(Action::MoveLeft));
    assert!(!is_modal_dismiss_action(Action::TogglePlay));
    assert!(!is_modal_dismiss_action(Action::None));
}

// --- Clipboard Keybinding Tests ---

#[test]
fn test_normal_mode_copy_y() {
    let y = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(y, EditorMode::Normal), Action::Copy);
}

#[test]
fn test_normal_mode_paste_p() {
    let p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(p, EditorMode::Normal), Action::Paste);
}

#[test]
fn test_normal_mode_copy_ctrl_c() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_c, EditorMode::Normal), Action::Copy);
}

#[test]
fn test_normal_mode_paste_ctrl_v() {
    let ctrl_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_v, EditorMode::Normal), Action::Paste);
}

#[test]
fn test_normal_mode_cut_ctrl_x() {
    let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_x, EditorMode::Normal), Action::Cut);
}

// --- Transpose Keybinding Tests ---

#[test]
fn test_normal_mode_transpose_up_shift_up() {
    let shift_up = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_up, EditorMode::Normal),
        Action::TransposeUp
    );
}

#[test]
fn test_normal_mode_transpose_down_shift_down() {
    let shift_down = KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_down, EditorMode::Normal),
        Action::TransposeDown
    );
}

#[test]
fn test_normal_mode_transpose_octave_up() {
    let ctrl_shift_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(ctrl_shift_up, EditorMode::Normal),
        Action::TransposeOctaveUp
    );
}

#[test]
fn test_normal_mode_transpose_octave_down() {
    let ctrl_shift_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(ctrl_shift_down, EditorMode::Normal),
        Action::TransposeOctaveDown
    );
}

// --- Visual Mode Clipboard/Transpose Tests ---

#[test]
fn test_visual_mode_copy_y() {
    let y = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(y, EditorMode::Visual), Action::Copy);
}

#[test]
fn test_visual_mode_paste_p() {
    let p = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(p, EditorMode::Visual), Action::Paste);
}

#[test]
fn test_visual_mode_cut_d() {
    let d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(map_key_to_action(d, EditorMode::Visual), Action::Cut);
}

#[test]
fn test_visual_mode_interpolate_i() {
    let i = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(i, EditorMode::Visual),
        Action::Interpolate
    );
}

#[test]
fn test_visual_mode_transpose_up() {
    let shift_up = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_up, EditorMode::Visual),
        Action::TransposeUp
    );
}

#[test]
fn test_visual_mode_transpose_down() {
    let shift_down = KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_down, EditorMode::Visual),
        Action::TransposeDown
    );
}

#[test]
fn test_visual_mode_ctrl_c_copy() {
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_c, EditorMode::Visual), Action::Copy);
}

#[test]
fn test_visual_mode_ctrl_x_cut() {
    let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(map_key_to_action(ctrl_x, EditorMode::Visual), Action::Cut);
}

// --- Song Playback Keybinding Tests ---

#[test]
fn test_normal_mode_toggle_playback_mode() {
    let shift_p = KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT);
    assert_eq!(
        map_key_to_action(shift_p, EditorMode::Normal),
        Action::TogglePlaybackMode
    );
}

#[test]
fn test_normal_mode_jump_next_pattern() {
    let rb = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(rb, EditorMode::Normal),
        Action::JumpNextPattern
    );
}

#[test]
fn test_normal_mode_jump_prev_pattern() {
    let lb = KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(lb, EditorMode::Normal),
        Action::JumpPrevPattern
    );
}

// --- Export Dialog Tests ---

#[test]
fn test_normal_mode_ctrl_e_opens_export() {
    let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_e, EditorMode::Normal),
        Action::OpenExportDialog
    );
}

// --- Code Editor Keybinding Tests ---

#[test]
fn test_normal_mode_f4_switches_to_code_editor() {
    let f4 = KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE);
    assert_eq!(
        map_key_to_action(f4, EditorMode::Normal),
        Action::SwitchView(AppView::CodeEditor)
    );
}

#[test]
fn test_normal_mode_ctrl_backslash_toggles_split() {
    let ctrl_bs = KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_bs, EditorMode::Normal),
        Action::ToggleSplitView
    );
}

#[test]
fn test_normal_mode_ctrl_enter_executes_script() {
    let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_enter, EditorMode::Normal),
        Action::ExecuteScript
    );
}

#[test]
fn test_insert_mode_ctrl_backslash_toggles_split() {
    let ctrl_bs = KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_bs, EditorMode::Insert),
        Action::ToggleSplitView
    );
}

#[test]
fn test_insert_mode_ctrl_enter_executes_script() {
    let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_enter, EditorMode::Insert),
        Action::ExecuteScript
    );
}

// --- Template Menu Keybinding Tests ---

#[test]
fn test_normal_mode_ctrl_t_opens_templates() {
    let ctrl_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_t, EditorMode::Normal),
        Action::OpenTemplates
    );
}

#[test]
fn test_insert_mode_ctrl_t_opens_templates() {
    let ctrl_t = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_t, EditorMode::Insert),
        Action::OpenTemplates
    );
}

// --- Live Mode Keybinding Tests ---

#[test]
fn test_normal_mode_ctrl_l_toggles_live_mode() {
    let ctrl_l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_l, EditorMode::Normal),
        Action::ToggleLiveMode
    );
}

#[test]
fn test_insert_mode_ctrl_l_toggles_live_mode() {
    let ctrl_l = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL);
    assert_eq!(
        map_key_to_action(ctrl_l, EditorMode::Insert),
        Action::ToggleLiveMode
    );
}

#[test]
fn test_get_bindings_for_normal_mode() {
    let bindings = KeybindingRegistry::get_bindings_for_mode(EditorMode::Normal);
    assert!(!bindings.is_empty());
    assert!(bindings.iter().any(|b| b.key == "h / ←"));
}

#[test]
fn test_get_bindings_for_insert_mode() {
    let bindings = KeybindingRegistry::get_bindings_for_mode(EditorMode::Insert);
    assert!(!bindings.is_empty());
    assert!(bindings.iter().any(|b| b.key == "Esc"));
}

#[test]
fn test_get_which_key_entries_d() {
    let entries = KeybindingRegistry::get_which_key_entries('d');
    assert!(!entries.is_empty());
    // desc must not duplicate the key — just the action description
    assert!(entries
        .iter()
        .any(|(key, desc)| key == "dd" && desc.contains("Delete") && !desc.contains("dd")));
}

#[test]
fn test_get_which_key_entries_g() {
    let entries = KeybindingRegistry::get_which_key_entries('g');
    assert!(!entries.is_empty());
    // key must be "gg", desc must say "top" (not "Row") and must not duplicate the key
    assert!(entries.iter().any(|(key, desc)| {
        key == "gg" && desc.to_lowercase().contains("top") && !desc.contains("gg")
    }));
}

#[test]
fn test_get_which_key_entries_no_match() {
    let entries = KeybindingRegistry::get_which_key_entries('x');
    assert!(entries.is_empty());
}
