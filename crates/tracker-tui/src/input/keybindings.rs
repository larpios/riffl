/// Keybinding handling with vim-style navigation and mode-aware dispatch
///
/// This module provides keybinding infrastructure for the application,
/// with support for vim-style navigation keys (h/j/k/l) and modal editing
/// (Normal, Insert, Visual modes).
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppView;
use crate::editor::EditorMode;

/// Actions that can be triggered by keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // Navigation
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    PageUp,
    PageDown,

    // Mode transitions
    EnterInsertMode,
    EnterNormalMode,
    EnterVisualMode,

    // Editing (Insert mode)
    EnterNote(char),
    SetOctave(u8),

    // Clipboard
    Copy,
    Paste,
    Cut,
    Redo,

    // Note-off entry
    EnterNoteOff,
    // Note-cut entry (hard-silence, ^^^)
    EnterNoteCut,

    // Step size
    StepUp,
    StepDown,

    // Octave navigation
    OctaveUp,
    OctaveDown,

    // Go to row
    GoToRow,

    // Track management
    AddTrack,
    DeleteTrack,
    CloneTrack,

    // Quantize
    Quantize,

    // Transpose
    TransposeUp,
    TransposeDown,
    TransposeOctaveUp,
    TransposeOctaveDown,

    // Interpolation
    Interpolate,

    // Editing (Normal mode)
    DeleteCell,
    InsertRow,
    InsertRowBelow,
    DeleteRow,
    Undo,
    EnterCommandMode,

    // Transport
    TogglePlay,
    Stop,
    BpmUp,
    BpmDown,
    BpmUpLarge,
    BpmDownLarge,
    ToggleLoop,
    TogglePlaybackMode,
    JumpNextPattern,
    JumpPrevPattern,

    // Track operations
    ToggleMute,
    ToggleSolo,
    NextTrack,

    // View switching
    SwitchView(AppView),

    // Project
    SaveProject,
    LoadProject,

    // Export
    OpenExportDialog,

    // Code editor
    ToggleSplitView,
    ExecuteScript,
    OpenTemplates,
    ToggleLiveMode,
    ToggleFollowMode,

    // BPM
    OpenBpmPrompt,
    TapTempo,

    // Loop region
    SetLoopStart,
    SetLoopEnd,
    ToggleLoopRegion,

    // Application
    Quit,
    Confirm,
    Cancel,
    OpenModal,
    ToggleHelp,
    OpenFileBrowser,

    // Instrument management
    AddInstrument,
    DeleteInstrument,
    RenameInstrument,
    EditInstrument,
    SelectInstrument,

    // Pattern management
    AddPattern,
    DeletePattern,
    ClonePattern,
    SelectPattern,

    /// No action (unmapped key)
    None,
}

/// Map a keyboard event to an action, aware of the current editor mode
pub fn map_key_to_action(key: KeyEvent, mode: EditorMode) -> Action {
    match mode {
        EditorMode::Normal => map_normal_mode(key),
        EditorMode::Insert => map_insert_mode(key),
        EditorMode::Visual => map_visual_mode(key),
    }
}

fn map_normal_mode(key: KeyEvent) -> Action {
    // Handle Ctrl+Shift modified bindings
    if key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT {
        return match key.code {
            KeyCode::Up => Action::TransposeOctaveUp,
            KeyCode::Down => Action::TransposeOctaveDown,
            // Toggle loop region active (Ctrl+Shift+L)
            KeyCode::Char('l') | KeyCode::Char('L') => Action::ToggleLoopRegion,
            _ => Action::None,
        };
    }

    // Handle Alt modified bindings
    if key.modifiers == KeyModifiers::ALT {
        return match key.code {
            KeyCode::Char('[') => Action::SetLoopStart,
            KeyCode::Char(']') => Action::SetLoopEnd,
            _ => Action::None,
        };
    }

    // Handle Ctrl-modified bindings
    if key.modifiers == KeyModifiers::CONTROL {
        return match key.code {
            KeyCode::Char('c') => Action::Copy,
            KeyCode::Char('v') => Action::Paste,
            KeyCode::Char('x') => Action::Cut,
            KeyCode::Char('r') => Action::Redo,
            KeyCode::Char('s') => Action::SaveProject,
            KeyCode::Char('o') => Action::LoadProject,
            KeyCode::Char('f') => Action::OpenFileBrowser,
            KeyCode::Char('e') => Action::OpenExportDialog,
            KeyCode::Char('\\') => Action::ToggleSplitView,
            KeyCode::Char('t') => Action::OpenTemplates,
            KeyCode::Char('l') => Action::ToggleLiveMode,
            KeyCode::Char('b') => Action::OpenBpmPrompt,
            KeyCode::Enter => Action::ExecuteScript,
            KeyCode::Delete => Action::DeleteRow,
            _ => Action::None,
        };
    }

    // Handle Shift-modified bindings
    if key.modifiers == KeyModifiers::SHIFT {
        return match key.code {
            // Toggle loop mode (Shift+L = 'L')
            KeyCode::Char('L') => Action::ToggleLoop,
            // Toggle playback mode (Shift+P = 'P')
            KeyCode::Char('P') => Action::TogglePlaybackMode,
            // Track operations (Shift+M = 'M', Shift+S = 'S')
            KeyCode::Char('M') => Action::ToggleMute,
            KeyCode::Char('S') => Action::ToggleSolo,
            // '+' key (Shift+'=' on US keyboards) for BPM up
            KeyCode::Char('+') => Action::BpmUp,
            // ':' (Shift+';' on US keyboards) — command mode
            // Some terminals on Windows report ':' with SHIFT modifier
            KeyCode::Char(':') => Action::EnterCommandMode,
            // Transpose by semitone
            KeyCode::Up => Action::TransposeUp,
            KeyCode::Down => Action::TransposeDown,
            // Uppercase shortcuts also need to work when SHIFT is reported
            KeyCode::Char('G') => Action::GoToRow,
            KeyCode::Char('Q') => Action::Quantize,
            KeyCode::Char('T') => Action::AddTrack,
            KeyCode::Char('D') => Action::DeleteTrack,
            KeyCode::Char('C') => Action::CloneTrack,
            _ => Action::None,
        };
    }

    // All other modified keys are ignored
    if key.modifiers != KeyModifiers::NONE {
        return Action::None;
    }

    match key.code {
        // Vim navigation
        KeyCode::Char('h') => Action::MoveLeft,
        KeyCode::Char('j') => Action::MoveDown,
        KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Char('l') => Action::MoveRight,

        // Arrow keys
        KeyCode::Left => Action::MoveLeft,
        KeyCode::Down => Action::MoveDown,
        KeyCode::Up => Action::MoveUp,
        KeyCode::Right => Action::MoveRight,

        // Page navigation
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,

        // Track navigation
        KeyCode::Tab => Action::NextTrack,

        // Mode transitions
        KeyCode::Char('i') => Action::EnterInsertMode,
        KeyCode::Char('v') => Action::EnterVisualMode,

        // Clipboard
        KeyCode::Char('y') => Action::Copy,
        KeyCode::Char('p') => Action::Paste,

        // Editing
        KeyCode::Char('x') | KeyCode::Delete => Action::DeleteCell,
        KeyCode::Insert => Action::InsertRow,
        KeyCode::Char('o') => Action::InsertRowBelow,
        KeyCode::Char(':') => Action::EnterCommandMode,
        KeyCode::Char('u') => Action::Undo,

        // Octave navigation (parenthesis keys)
        KeyCode::Char('(') => Action::OctaveDown,
        KeyCode::Char(')') => Action::OctaveUp,

        // Step size (braces: { = smaller step, } = larger step)
        KeyCode::Char('{') => Action::StepDown,
        KeyCode::Char('}') => Action::StepUp,

        // Go to row (Shift+G), quantize
        KeyCode::Char('G') => Action::GoToRow,
        KeyCode::Char('Q') => Action::Quantize,

        // Track management (Shift+T for new, Shift+D delete, Shift+C clone)
        KeyCode::Char('T') => Action::AddTrack,
        KeyCode::Char('D') => Action::DeleteTrack,
        KeyCode::Char('C') => Action::CloneTrack,

        // Transport
        KeyCode::Char(' ') => Action::TogglePlay,
        KeyCode::Char('=') => Action::BpmUp,
        KeyCode::Char('-') => Action::BpmDown,
        KeyCode::Char(']') => Action::JumpNextPattern,
        KeyCode::Char('[') => Action::JumpPrevPattern,

        // View switching
        KeyCode::Char('1') => Action::SwitchView(AppView::PatternEditor),
        KeyCode::Char('2') => Action::SwitchView(AppView::Arrangement),
        KeyCode::Char('3') => Action::SwitchView(AppView::InstrumentList),
        KeyCode::Char('4') => Action::SwitchView(AppView::CodeEditor),
        KeyCode::Char('5') => Action::SwitchView(AppView::PatternList),
        KeyCode::Char('6') => Action::SwitchView(AppView::SampleBrowser),

        // Instrument management (when in InstrumentList view)
        KeyCode::Char('n') => Action::AddInstrument,
        KeyCode::Char('d') => Action::DeleteInstrument,
        KeyCode::Char('r') => Action::RenameInstrument,
        KeyCode::Char('a') => Action::EditInstrument,
        KeyCode::Char('s') => Action::SelectInstrument,

        // Pattern management (when in PatternList view)
        KeyCode::Char('c') => Action::ClonePattern,

        // Follow mode
        KeyCode::Char('f') => Action::ToggleFollowMode,

        // BPM tap-tempo
        KeyCode::Char('t') => Action::TapTempo,

        // Application
        KeyCode::Char('m') => Action::OpenModal,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Enter => Action::Confirm,
        KeyCode::Esc => Action::Cancel,

        _ => Action::None,
    }
}

fn map_insert_mode(key: KeyEvent) -> Action {
    // Handle Ctrl-modified bindings in insert mode
    if key.modifiers == KeyModifiers::CONTROL {
        return match key.code {
            KeyCode::Char('\\') => Action::ToggleSplitView,
            KeyCode::Char('t') => Action::OpenTemplates,
            KeyCode::Char('l') => Action::ToggleLiveMode,
            KeyCode::Enter => Action::ExecuteScript,
            _ => Action::None,
        };
    }

    // Allow SHIFT through — uppercase A-G enters sharps, octave parens, etc.
    if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
        return Action::None;
    }

    match key.code {
        // Escape returns to Normal mode
        KeyCode::Esc => Action::EnterNormalMode,

        // Note-off (tilde)
        KeyCode::Char('~') => Action::EnterNoteOff,

        // Note entry: lowercase a-g = natural, uppercase A-G = sharp equivalent
        KeyCode::Char(c @ ('a'..='g' | 'A'..='G')) => Action::EnterNote(c),

        // Octave setting (0-9)
        KeyCode::Char(c @ '0'..='9') => Action::SetOctave(c as u8 - b'0'),

        // Octave jump (parenthesis)
        KeyCode::Char('(') => Action::OctaveDown,
        KeyCode::Char(')') => Action::OctaveUp,

        // Navigation still works in Insert mode
        KeyCode::Left => Action::MoveLeft,
        KeyCode::Down => Action::MoveDown,
        KeyCode::Up => Action::MoveUp,
        KeyCode::Right => Action::MoveRight,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,

        // Track navigation
        KeyCode::Tab => Action::NextTrack,

        // Space for play/pause even in insert mode
        KeyCode::Char(' ') => Action::TogglePlay,

        // Del enters a note-cut (^^^); Backspace deletes the cell
        KeyCode::Delete => Action::EnterNoteCut,
        KeyCode::Backspace => Action::DeleteCell,

        // Insert a new row at cursor
        KeyCode::Insert => Action::InsertRow,

        _ => Action::None,
    }
}

fn map_visual_mode(key: KeyEvent) -> Action {
    // Handle Ctrl+Shift modified bindings (transpose by octave)
    if key.modifiers == KeyModifiers::CONTROL | KeyModifiers::SHIFT {
        return match key.code {
            KeyCode::Up => Action::TransposeOctaveUp,
            KeyCode::Down => Action::TransposeOctaveDown,
            _ => Action::None,
        };
    }

    // Handle Ctrl-modified bindings
    if key.modifiers == KeyModifiers::CONTROL {
        return match key.code {
            KeyCode::Char('c') => Action::Copy,
            KeyCode::Char('v') => Action::Paste,
            KeyCode::Char('x') => Action::Cut,
            _ => Action::None,
        };
    }

    // Handle Shift-modified bindings (transpose by semitone)
    if key.modifiers == KeyModifiers::SHIFT {
        return match key.code {
            KeyCode::Up => Action::TransposeUp,
            KeyCode::Down => Action::TransposeDown,
            // ':' (Shift+';') — command mode, same as Normal mode
            KeyCode::Char(':') => Action::EnterCommandMode,
            _ => Action::None,
        };
    }

    if key.modifiers != KeyModifiers::NONE {
        return Action::None;
    }

    match key.code {
        // Escape or v returns to Normal mode
        KeyCode::Esc | KeyCode::Char('v') => Action::EnterNormalMode,

        // Command mode
        KeyCode::Char(':') => Action::EnterCommandMode,

        // Navigation in Visual mode
        KeyCode::Char('h') | KeyCode::Left => Action::MoveLeft,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Char('l') | KeyCode::Right => Action::MoveRight,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::PageDown => Action::PageDown,

        // Clipboard operations
        KeyCode::Char('y') => Action::Copy,
        KeyCode::Char('p') => Action::Paste,
        KeyCode::Char('d') => Action::Cut,

        // Interpolate
        KeyCode::Char('i') => Action::Interpolate,

        // Delete/cut selection (mirrors 'd' — both cut in Visual mode)
        KeyCode::Char('x') | KeyCode::Delete => Action::Cut,

        _ => Action::None,
    }
}

/// Check if an action represents a navigation movement
pub fn is_navigation_action(action: Action) -> bool {
    matches!(
        action,
        Action::MoveLeft
            | Action::MoveDown
            | Action::MoveUp
            | Action::MoveRight
            | Action::PageUp
            | Action::PageDown
    )
}

/// Check if an action dismisses modal dialogs
pub fn is_modal_dismiss_action(action: Action) -> bool {
    matches!(
        action,
        Action::Cancel | Action::Confirm | Action::EnterNormalMode
    )
}

#[cfg(test)]
mod tests {
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
    fn test_normal_mode_view_switching_1_2_3() {
        use crate::app::AppView;
        let k1 = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        let k2 = KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE);
        let k3 = KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE);
        assert_eq!(
            map_key_to_action(k1, EditorMode::Normal),
            Action::SwitchView(AppView::PatternEditor)
        );
        assert_eq!(
            map_key_to_action(k2, EditorMode::Normal),
            Action::SwitchView(AppView::Arrangement)
        );
        assert_eq!(
            map_key_to_action(k3, EditorMode::Normal),
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
        let c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE);
        let a_upper = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE);
        assert_eq!(
            map_key_to_action(c, EditorMode::Insert),
            Action::EnterNote('c')
        );
        assert_eq!(
            map_key_to_action(g, EditorMode::Insert),
            Action::EnterNote('g')
        );
        assert_eq!(
            map_key_to_action(a_upper, EditorMode::Insert),
            Action::EnterNote('A')
        );
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
        let ctrl_shift_down =
            KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
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
    fn test_normal_mode_4_switches_to_code_editor() {
        let k4 = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE);
        assert_eq!(
            map_key_to_action(k4, EditorMode::Normal),
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
}
