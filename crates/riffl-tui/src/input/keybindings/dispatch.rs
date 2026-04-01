use super::action::Action;
use crate::app::AppView;
use crate::editor::EditorMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Map a keyboard event to an action, aware of the current editor mode
pub fn map_key_to_action(key: KeyEvent, mode: EditorMode) -> Action {
    match mode {
        EditorMode::Normal => map_normal_mode(key),
        EditorMode::Insert | EditorMode::Replace => map_insert_mode(key),
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
            // Track volume and pan (Alt+Arrow)
            KeyCode::Up => Action::TrackVolumeUp,
            KeyCode::Down => Action::TrackVolumeDown,
            KeyCode::Left => Action::TrackPanLeft,
            KeyCode::Right => Action::TrackPanRight,
            // Block transforms
            KeyCode::Char('f') => Action::FillSelection,
            KeyCode::Char('z') => Action::RandomizeNotes,
            // Bookmarks
            KeyCode::Char('m') => Action::AddBookmark,
            KeyCode::Char('n') => Action::NextBookmark,
            KeyCode::Char('p') => Action::PrevBookmark,
            _ => Action::None,
        };
    }

    // Handle Ctrl-modified bindings
    if key.modifiers == KeyModifiers::CONTROL {
        return match key.code {
            KeyCode::Char('j') => Action::ArrangementMoveEntryDown,
            KeyCode::Char('k') => Action::ArrangementMoveEntryUp,
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
            KeyCode::Char('p') => Action::OpenLenPrompt,
            KeyCode::Char('m') => Action::ToggleMetronome,
            KeyCode::Enter => Action::ExecuteScript,
            KeyCode::Delete => Action::DeleteRow,
            KeyCode::Left => Action::ResetHorizontalView,
            _ => Action::None,
        };
    }

    // Handle Shift-modified bindings
    if key.modifiers == KeyModifiers::SHIFT {
        return match key.code {
            // Toggle play from cursor (Shift+Space or Shift+Enter)
            KeyCode::Char(' ') | KeyCode::Enter => Action::PlayFromCursor,
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
            KeyCode::Char('G') => Action::GoToBottom,
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
        KeyCode::Char('R') => Action::EnterReplaceMode,

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

        // Go to bottom (Shift+G), quantize
        KeyCode::Char('G') => Action::GoToBottom,
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
        KeyCode::Char('w') => Action::ToggleInstrumentMiniPanel,

        // Pattern management (when in PatternList view)
        KeyCode::Char('c') => Action::ClonePattern,

        // Follow mode
        KeyCode::Char('f') => Action::ToggleFollowMode,

        // BPM tap-tempo
        KeyCode::Char('t') => Action::TapTempo,

        // Application
        KeyCode::Char('m') => Action::GoToRow,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('K') => Action::ToggleEffectHelp,
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

    // Track volume / pan (Alt+Arrow) — same as Normal mode
    if key.modifiers == KeyModifiers::ALT {
        return match key.code {
            KeyCode::Up => Action::TrackVolumeUp,
            KeyCode::Down => Action::TrackVolumeDown,
            KeyCode::Left => Action::TrackPanLeft,
            KeyCode::Right => Action::TrackPanRight,
            _ => Action::None,
        };
    }

    // Allow SHIFT through for shifted symbol keys (parentheses, tilde, etc.).
    if key.modifiers == KeyModifiers::SHIFT {
        return match key.code {
            KeyCode::Char(' ') | KeyCode::Enter => Action::PlayFromCursor,
            _ => {
                match key.code {
                    // Note-off (tilde)
                    KeyCode::Char('~') => Action::EnterNoteOff,
                    // Octave jump (parenthesis)
                    KeyCode::Char('(') => Action::OctaveDown,
                    KeyCode::Char(')') => Action::OctaveUp,
                    // Draw mode toggle
                    KeyCode::Char('D') => Action::ToggleDrawMode,
                    _ => Action::None,
                }
            }
        };
    }

    if key.modifiers != KeyModifiers::NONE {
        return Action::None;
    }

    match key.code {
        // Escape returns to Normal mode
        KeyCode::Esc => Action::EnterNormalMode,

        // Note-off (tilde)
        KeyCode::Char('~') => Action::EnterNoteOff,

        // Piano keyboard layout (FT2/IT tracker style):
        //   Lower row (white keys): a=C  s=D  d=E  f=F  g=G  h=A  j=B  k=C+1oct
        //   Upper row (black keys): w=C# e=D#     t=F# y=G# u=A#
        KeyCode::Char(
            c @ ('a' | 'w' | 's' | 'e' | 'd' | 'f' | 't' | 'g' | 'y' | 'h' | 'u' | 'j' | 'k'),
        ) => Action::EnterNote(c),

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

        // Draw mode toggle
        KeyCode::Char('D') => Action::ToggleDrawMode,

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
            // Execute script scoped to the current visual selection
            KeyCode::Enter => Action::ExecuteScriptOnSelection,
            _ => Action::None,
        };
    }

    // Handle Alt-modified bindings in visual mode
    if key.modifiers == KeyModifiers::ALT {
        return match key.code {
            KeyCode::Char('f') => Action::FillSelection,
            KeyCode::Char('z') => Action::RandomizeNotes,
            KeyCode::Char('m') => Action::AddBookmark,
            KeyCode::Char('n') => Action::NextBookmark,
            KeyCode::Char('p') => Action::PrevBookmark,
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
