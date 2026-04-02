use super::action::Action;
use crate::app::AppView;
use crate::editor::EditorMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct KeyMapping {
    pub(super) key: &'static str,
    pub(super) action: Action,
    pub(super) mode: EditorMode,
}

pub(super) const KEY_MAPPINGS: &[KeyMapping] = &[
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
        key: "Shift+Space",
        action: Action::PlayFromCursor,
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
        key: "\\",
        action: Action::ShowWhichKey,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "K",
        action: Action::ToggleEffectHelp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Esc",
        action: Action::Cancel,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "1-6",
        action: Action::SwitchView(AppView::PatternEditor),
        mode: EditorMode::Normal,
    },
    // Clipboard (Normal)
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
    // Editing (Normal)
    KeyMapping {
        key: "{",
        action: Action::StepDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "}",
        action: Action::StepUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "(",
        action: Action::OctaveDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: ")",
        action: Action::OctaveUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+Del",
        action: Action::DeleteRow,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Shift+Up",
        action: Action::TransposeUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Shift+Down",
        action: Action::TransposeDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+Shift+Up",
        action: Action::TransposeOctaveUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+Shift+Down",
        action: Action::TransposeOctaveDown,
        mode: EditorMode::Normal,
    },
    // Track management (Normal)
    KeyMapping {
        key: "T",
        action: Action::AddTrack,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "D",
        action: Action::DeleteTrack,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "C",
        action: Action::CloneTrack,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "M",
        action: Action::ToggleMute,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "S",
        action: Action::ToggleSolo,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Q",
        action: Action::Quantize,
        mode: EditorMode::Normal,
    },
    // Transport (Normal)
    KeyMapping {
        key: "=",
        action: Action::BpmUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "-",
        action: Action::BpmDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "[",
        action: Action::JumpPrevPattern,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "]",
        action: Action::JumpNextPattern,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "L",
        action: Action::ToggleLoop,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "P",
        action: Action::TogglePlaybackMode,
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
    KeyMapping {
        key: "Ctrl+M",
        action: Action::ToggleMetronome,
        mode: EditorMode::Normal,
    },
    // Project (Normal)
    KeyMapping {
        key: "Ctrl+S",
        action: Action::SaveProject,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+O",
        action: Action::LoadProject,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+F",
        action: Action::OpenFileBrowser,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+E",
        action: Action::OpenExportDialog,
        mode: EditorMode::Normal,
    },
    // View (Normal)
    KeyMapping {
        key: "Ctrl+\\",
        action: Action::ToggleSplitView,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+L",
        action: Action::ToggleLiveMode,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Ctrl+T",
        action: Action::OpenTemplates,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+Up",
        action: Action::TrackVolumeUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+Down",
        action: Action::TrackVolumeDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+Left",
        action: Action::TrackPanLeft,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+Right",
        action: Action::TrackPanRight,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+F",
        action: Action::FillSelection,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+Z",
        action: Action::RandomizeNotes,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+M",
        action: Action::AddBookmark,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+N",
        action: Action::NextBookmark,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Alt+P",
        action: Action::PrevBookmark,
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
        key: "Shift+Space",
        action: Action::PlayFromCursor,
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
    KeyMapping {
        key: "Ctrl+Enter",
        action: Action::ExecuteScriptOnSelection,
        mode: EditorMode::Visual,
    },
    KeyMapping {
        key: "Tab",
        action: Action::EnvCycle,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Shift+Tab",
        action: Action::EnvPrev,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Up",
        action: Action::EnvMoveUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Down",
        action: Action::EnvMoveDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Left",
        action: Action::EnvMoveLeft,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Right",
        action: Action::EnvMoveRight,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Insert",
        action: Action::EnvAddPoint,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Delete",
        action: Action::EnvDeletePoint,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Home",
        action: Action::EnvSelectFirst,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "End",
        action: Action::EnvSelectLast,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "+",
        action: Action::EnvChangeValue,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "-",
        action: Action::EnvChangeValue,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "p",
        action: Action::WfTogglePencil,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "l",
        action: Action::WfToggleLoop,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Left",
        action: Action::WfMoveCursorLeft,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Right",
        action: Action::WfMoveCursorRight,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Up",
        action: Action::WfValueUp,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Down",
        action: Action::WfValueDown,
        mode: EditorMode::Normal,
    },
    KeyMapping {
        key: "Enter",
        action: Action::WfDrawSample,
        mode: EditorMode::Normal,
    },
];

/// Chord prefix mappings for which-key display
pub(super) struct ChordMapping {
    pub(super) prefix: char,
    pub(super) completion: char,
    pub(super) action: Action,
}

pub(super) const CHORD_MAPPINGS: &[ChordMapping] = &[
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
