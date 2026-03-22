/// Keybinding handling with vim-style navigation and mode-aware dispatch
///
/// This module provides keybinding infrastructure for the application,
/// with support for vim-style navigation keys (h/j/k/l) and modal editing
/// (Normal, Insert, Visual modes).
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppView;
use crate::editor::EditorMode;
use crate::registry::{ActionCategory, ActionMetadata, Keybinding};

/// Registry for discovering keybindings
pub struct KeybindingRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KeyMapping {
    key: &'static str,
    action: Action,
    mode: EditorMode,
}

const KEY_MAPPINGS: &[KeyMapping] = &[
    // Normal Mode
    KeyMapping {
        key: "h / ←",
        action: Action::MoveLeft,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "j / ↓",
        action: Action::MoveDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "k / ↑",
        action: Action::MoveUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "l / →",
        action: Action::MoveRight,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "g",
        action: Action::GoToStart,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "G",
        action: Action::GoToBottom,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Home",
        action: Action::GoToTop,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "End",
        action: Action::GoToBottom,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "PgUp",
        action: Action::PageUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "PgDn",
        action: Action::PageDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Tab",
        action: Action::NextTrack,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "i",
        action: Action::EnterInsertMode,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "v",
        action: Action::EnterVisualMode,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "y",
        action: Action::Copy,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "p",
        action: Action::Paste,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "x / Del",
        action: Action::DeleteCell,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ins",
        action: Action::InsertRow,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "o",
        action: Action::InsertRowBelow,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "u",
        action: Action::Undo,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: ":",
        action: Action::EnterCommandMode,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Space",
        action: Action::TogglePlay,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "f",
        action: Action::ToggleFollowMode,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "t",
        action: Action::TapTempo,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "?",
        action: Action::ToggleHelp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "1-6",
        action: Action::SwitchView(AppView::PatternEditor),
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+C",
        action: Action::Copy,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+V",
        action: Action::Paste,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+X",
        action: Action::Cut,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+R",
        action: Action::Redo,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+S",
        action: Action::SaveProject,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+B",
        action: Action::OpenBpmPrompt,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+P",
        action: Action::OpenLenPrompt,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+[",
        action: Action::SetLoopStart,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+]",
        action: Action::SetLoopEnd,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+Shift+L",
        action: Action::ToggleLoopRegion,
        mode: EditorMode::Normal,
    },
    // Insert Mode
    KeyMapping {
        key: "Esc",
        action: Action::EnterNormalMode,
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "~",
        action: Action::EnterNoteOff,
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "a-k",
        action: Action::EnterNote('a'),
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "0-9",
        action: Action::SetOctave(0),
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "Backspace",
        action: Action::DeleteCell,
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "Delete",
        action: Action::EnterNoteCut,
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "Space",
        action: Action::TogglePlay,
        mode: EditorMode::Insert,
    },
    KeyMapping {
        key: "Tab",
        action: Action::NextTrack,
        mode: EditorMode::Insert,
    },
    // Visual Mode
    KeyMapping {
        key: "Esc / v",
        action: Action::EnterNormalMode,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "y",
        action: Action::Copy,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "p",
        action: Action::Paste,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "d / x",
        action: Action::Cut,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "i",
        action: Action::Interpolate,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "Shift+Up/Down",
        action: Action::TransposeUp,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "Ctrl+Shift+Up/Down",
        action: Action::TransposeOctaveUp,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "Ctrl+Enter",
        action: Action::ExecuteScript,
        mode: EditorMode::Normal,
    },
];

/// Chord prefix mappings for which-key display
struct ChordMapping {
    prefix: char,
    completion: char,
    action: Action,
}

const CHORD_MAPPINGS: &[ChordMapping] = &[
    ChordMapping {
        prefix: 'd',
        completion: 'd',
        action: Action::DeleteRow,
    },
    ChordMapping {
        prefix: 'g',
        completion: 'g',
        action: Action::GoToTop,
    },
];

impl KeybindingRegistry {
    /// Get all available keybindings for a given editor mode
    pub fn get_bindings_for_mode(mode: EditorMode) -> Vec<Keybinding> {
        KEY_MAPPINGS
            .iter()
            .filter(|m| m.mode == mode)
            .map(|m| Keybinding {
                key: m.key.to_string(),
                action: m.action.name().to_string(),
                description: m.action.description().to_string(),
                category: m.action.category(),
            })
            .collect()
    }

    /// Get which-key entries for a given chord prefix
    pub fn get_which_key_entries(prefix: char) -> Vec<(String, String)> {
        CHORD_MAPPINGS
            .iter()
            .filter(|m| m.prefix == prefix)
            .map(|m| {
                let key = format!("{}{}", prefix, m.completion);
                let desc = m.action.description().to_string();
                (key, desc)
            })
            .collect()
    }
}

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
    GoToTop,
    GoToBottom,
    GoToStart,
    GoToEnd,

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

    // Pattern length
    OpenLenPrompt,

    // Loop region
    SetLoopStart,
    SetLoopEnd,
    ToggleLoopRegion,

    // Draw mode
    ToggleDrawMode,

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

impl ActionMetadata for Action {
    fn name(&self) -> &str {
        match self {
            Action::MoveLeft => "Move Left",
            Action::MoveDown => "Move Down",
            Action::MoveUp => "Move Up",
            Action::MoveRight => "Move Right",
            Action::PageUp => "Page Up",
            Action::PageDown => "Page Down",
            Action::EnterInsertMode => "Insert Mode",
            Action::EnterNormalMode => "Normal Mode",
            Action::EnterVisualMode => "Visual Mode",
            Action::EnterNote(_) => "Enter Note",
            Action::SetOctave(_) => "Set Octave",
            Action::Copy => "Copy",
            Action::Paste => "Paste",
            Action::Cut => "Cut",
            Action::Redo => "Redo",
            Action::EnterNoteOff => "Note Off",
            Action::EnterNoteCut => "Note Cut",
            Action::StepUp => "Step Up",
            Action::StepDown => "Step Down",
            Action::OctaveUp => "Octave Up",
            Action::OctaveDown => "Octave Down",
            Action::GoToRow => "Go to Row",
            Action::GoToTop => "Go to Top",
            Action::GoToBottom => "Go to Bottom",
            Action::AddTrack => "Add Track",
            Action::DeleteTrack => "Delete Track",
            Action::CloneTrack => "Clone Track",
            Action::Quantize => "Quantize",
            Action::TransposeUp => "Transpose Up",
            Action::TransposeDown => "Transpose Down",
            Action::TransposeOctaveUp => "Transpose Octave Up",
            Action::TransposeOctaveDown => "Transpose Octave Down",
            Action::Interpolate => "Interpolate",
            Action::DeleteCell => "Delete Cell",
            Action::InsertRow => "Insert Row",
            Action::InsertRowBelow => "Insert Row Below",
            Action::DeleteRow => "Delete Row",
            Action::Undo => "Undo",
            Action::EnterCommandMode => "Command Mode",
            Action::TogglePlay => "Play/Pause",
            Action::Stop => "Stop",
            Action::BpmUp => "BPM Up",
            Action::BpmDown => "BPM Down",
            Action::BpmUpLarge => "BPM Up Large",
            Action::BpmDownLarge => "BPM Down Large",
            Action::ToggleLoop => "Toggle Loop",
            Action::TogglePlaybackMode => "Playback Mode",
            Action::JumpNextPattern => "Next Pattern",
            Action::JumpPrevPattern => "Prev Pattern",
            Action::ToggleMute => "Toggle Mute",
            Action::ToggleSolo => "Toggle Solo",
            Action::NextTrack => "Next Track",
            Action::SwitchView(_) => "Switch View",
            Action::SaveProject => "Save Project",
            Action::LoadProject => "Load Project",
            Action::OpenExportDialog => "Export Dialog",
            Action::ToggleSplitView => "Split View",
            Action::ExecuteScript => "Execute Script",
            Action::OpenTemplates => "Templates",
            Action::ToggleLiveMode => "Live Mode",
            Action::ToggleFollowMode => "Follow Mode",
            Action::OpenBpmPrompt => "BPM Prompt",
            Action::TapTempo => "Tap Tempo",
            Action::OpenLenPrompt => "Pattern Length",
            Action::SetLoopStart => "Set Loop Start",
            Action::SetLoopEnd => "Set Loop End",
            Action::ToggleLoopRegion => "Toggle Loop Region",
            Action::ToggleDrawMode => "Draw Mode",
            Action::Quit => "Quit",
            Action::Confirm => "Confirm",
            Action::Cancel => "Cancel",
            Action::OpenModal => "Open Modal",
            Action::ToggleHelp => "Help",
            Action::OpenFileBrowser => "File Browser",
            Action::AddInstrument => "Add Instrument",
            Action::DeleteInstrument => "Delete Instrument",
            Action::RenameInstrument => "Rename Instrument",
            Action::EditInstrument => "Edit Instrument",
            Action::SelectInstrument => "Select Instrument",
            Action::AddPattern => "Add Pattern",
            Action::DeletePattern => "Delete Pattern",
            Action::ClonePattern => "Clone Pattern",
            Action::SelectPattern => "Select Pattern",
            Action::None => "None",
        }
    }

    fn description(&self) -> &str {
        match self {
            Action::MoveLeft => "Move cursor left",
            Action::MoveDown => "Move cursor down",
            Action::MoveUp => "Move cursor up",
            Action::MoveRight => "Move cursor right",
            Action::PageUp => "Move page up",
            Action::PageDown => "Move page down",
            Action::EnterInsertMode => "Enter insert mode for note entry",
            Action::EnterNormalMode => "Return to normal mode",
            Action::EnterVisualMode => "Enter visual mode for selection",
            Action::EnterNote(_) => "Enter musical note",
            Action::SetOctave(_) => "Set current octave (0-9)",
            Action::Copy => "Copy selection to clipboard",
            Action::Paste => "Paste from clipboard",
            Action::Cut => "Cut selection to clipboard",
            Action::Redo => "Redo last undone change",
            Action::EnterNoteOff => "Enter a note-off (release)",
            Action::EnterNoteCut => "Enter a note-cut (hard silence)",
            Action::StepUp => "Increase row step size",
            Action::StepDown => "Decrease row step size",
            Action::OctaveUp => "Increase current octave",
            Action::OctaveDown => "Decrease current octave",
            Action::GoToRow => "Jump to specific row",
            Action::GoToTop => "Jump to top of pattern",
            Action::GoToBottom => "Jump to bottom of pattern",
            Action::AddTrack => "Add a new track",
            Action::DeleteTrack => "Delete current track",
            Action::CloneTrack => "Clone current track",
            Action::Quantize => "Quantize selection",
            Action::TransposeUp => "Transpose semitone up",
            Action::TransposeDown => "Transpose semitone down",
            Action::TransposeOctaveUp => "Transpose octave up",
            Action::TransposeOctaveDown => "Transpose octave down",
            Action::Interpolate => "Interpolate selection between values",
            Action::DeleteCell => "Delete cell content at cursor",
            Action::InsertRow => "Insert a blank row",
            Action::InsertRowBelow => "Insert a blank row below",
            Action::DeleteRow => "Delete current row",
            Action::Undo => "Undo last change",
            Action::EnterCommandMode => "Enter command line mode",
            Action::TogglePlay => "Toggle audio playback",
            Action::Stop => "Stop playback",
            Action::BpmUp => "Increase BPM by 1",
            Action::BpmDown => "Decrease BPM by 1",
            Action::BpmUpLarge => "Increase BPM by 10",
            Action::BpmDownLarge => "Decrease BPM by 10",
            Action::ToggleLoop => "Toggle playback looping",
            Action::TogglePlaybackMode => "Toggle song/pattern mode",
            Action::JumpNextPattern => "Jump to next pattern",
            Action::JumpPrevPattern => "Jump to previous pattern",
            Action::ToggleMute => "Mute/unmute current track",
            Action::ToggleSolo => "Solo current track",
            Action::NextTrack => "Jump to next track",
            Action::SwitchView(_) => "Switch to another application view",
            Action::SaveProject => "Save current project",
            Action::LoadProject => "Load project from file",
            Action::OpenExportDialog => "Open audio export dialog",
            Action::ToggleSplitView => "Toggle code editor split view",
            Action::ExecuteScript => "Execute current script",
            Action::OpenTemplates => "Open code template menu",
            Action::ToggleLiveMode => "Toggle live script execution",
            Action::ToggleFollowMode => "Follow playback position",
            Action::OpenBpmPrompt => "Set BPM via prompt",
            Action::TapTempo => "Set BPM via tap tempo",
            Action::OpenLenPrompt => "Set pattern length via prompt",
            Action::SetLoopStart => "Set loop region start",
            Action::SetLoopEnd => "Set loop region end",
            Action::ToggleLoopRegion => "Toggle loop region active",
            Action::ToggleDrawMode => "Toggle parameter draw mode",
            Action::Quit => "Quit application",
            Action::Confirm => "Confirm action or choice",
            Action::Cancel => "Cancel action or choice",
            Action::OpenModal => "Open a modal dialog",
            Action::ToggleHelp => "Show help screen",
            Action::OpenFileBrowser => "Open file browser",
            Action::AddInstrument => "Add a new instrument",
            Action::DeleteInstrument => "Delete current instrument",
            Action::RenameInstrument => "Rename current instrument",
            Action::EditInstrument => "Enter instrument editor",
            Action::SelectInstrument => "Select current instrument",
            Action::AddPattern => "Add a new pattern",
            Action::DeletePattern => "Delete current pattern",
            Action::ClonePattern => "Clone current pattern",
            Action::SelectPattern => "Select current pattern",
            Action::None => "No operation",
        }
    }

    fn category(&self) -> ActionCategory {
        match self {
            Action::MoveLeft
            | Action::MoveDown
            | Action::MoveUp
            | Action::MoveRight
            | Action::PageUp
            | Action::PageDown
            | Action::GoToRow
            | Action::GoToTop
            | Action::GoToBottom
            | Action::NextTrack => ActionCategory::Navigation,

            Action::EnterInsertMode | Action::EnterNormalMode | Action::EnterVisualMode => {
                ActionCategory::Application
            }

            Action::EnterNote(_)
            | Action::SetOctave(_)
            | Action::EnterNoteOff
            | Action::EnterNoteCut
            | Action::StepUp
            | Action::StepDown
            | Action::OctaveUp
            | Action::OctaveDown
            | Action::Quantize
            | Action::TransposeUp
            | Action::TransposeDown
            | Action::TransposeOctaveUp
            | Action::TransposeOctaveDown
            | Action::Interpolate
            | Action::DeleteCell
            | Action::InsertRow
            | Action::InsertRowBelow
            | Action::DeleteRow
            | Action::Undo
            | Action::ToggleDrawMode => ActionCategory::Editing,

            Action::Copy | Action::Paste | Action::Cut | Action::Redo => ActionCategory::Clipboard,

            Action::AddTrack
            | Action::DeleteTrack
            | Action::CloneTrack
            | Action::ToggleMute
            | Action::ToggleSolo => ActionCategory::Track,

            Action::AddInstrument
            | Action::DeleteInstrument
            | Action::RenameInstrument
            | Action::EditInstrument
            | Action::SelectInstrument => ActionCategory::Instrument,

            Action::AddPattern
            | Action::DeletePattern
            | Action::ClonePattern
            | Action::SelectPattern => ActionCategory::Pattern,

            Action::TogglePlay
            | Action::Stop
            | Action::BpmUp
            | Action::BpmDown
            | Action::BpmUpLarge
            | Action::BpmDownLarge
            | Action::ToggleLoop
            | Action::TogglePlaybackMode
            | Action::JumpNextPattern
            | Action::JumpPrevPattern
            | Action::ToggleFollowMode
            | Action::OpenBpmPrompt
            | Action::TapTempo
            | Action::OpenLenPrompt
            | Action::SetLoopStart
            | Action::SetLoopEnd
            | Action::ToggleLoopRegion => ActionCategory::Transport,

            Action::SwitchView(_) | Action::ToggleSplitView | Action::ToggleLiveMode => {
                ActionCategory::View
            }

            Action::SaveProject | Action::LoadProject | Action::OpenExportDialog => {
                ActionCategory::Project
            }

            Action::ExecuteScript | Action::OpenTemplates => ActionCategory::Editing,

            Action::EnterCommandMode
            | Action::Quit
            | Action::Confirm
            | Action::Cancel
            | Action::OpenModal
            | Action::ToggleHelp
            | Action::OpenFileBrowser => ActionCategory::Application,

            Action::None => ActionCategory::None,
        }
    }
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
            KeyCode::Char('p') => Action::OpenLenPrompt,
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

    // Allow SHIFT through for shifted symbol keys (parentheses, tilde, etc.).
    if key.modifiers != KeyModifiers::NONE && key.modifiers != KeyModifiers::SHIFT {
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
}
