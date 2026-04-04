use crate::app::AppView;
use crate::registry::{ActionCategory, ActionMetadata};

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
    EnterVisualLineMode,
    EnterReplaceMode,

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
    ResetHorizontalView,

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

    // Block transforms
    FillSelection,
    RandomizeNotes,
    ReverseSelection,
    HumanizeNotes,

    // Marks (vim-style: m{a-z} to set, '{a-z} to jump)
    SetMark(char),
    GotoMark(char),

    // Registers (vim-style: "{a-z} prefix before yank/paste/cut)
    SetRegister(char),

    // Macro recording/replay (vim-style: q{a-z} record, @{a-z} replay)
    StartMacroRecord(char),
    StopMacroRecord,
    ReplayMacro(char),
    ReplayLastMacro,

    // Bookmarks
    AddBookmark,
    NextBookmark,
    PrevBookmark,

    // Editing (Normal mode)
    DeleteCell,
    InsertRow,
    InsertRowBelow,
    DeleteRow,
    Undo,
    EnterCommandMode,

    // Transport
    TogglePlay,
    PlayFromCursor,
    Stop,
    BpmUp,
    BpmDown,
    BpmUpLarge,
    BpmDownLarge,
    ToggleLoop,
    ToggleMetronome,
    TogglePlaybackMode,
    JumpNextPattern,
    JumpPrevPattern,

    // Track operations
    ToggleMute,
    ToggleSolo,
    TrackVolumeUp,
    TrackVolumeDown,
    TrackPanLeft,
    TrackPanRight,
    NextTrack,
    PrevTrack,

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
    ExecuteScriptOnSelection,
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
    ToggleEffectHelp,
    ShowWhichKey,
    OpenFileBrowser,
    OpenModuleBrowser,

    // Instrument management
    AddInstrument,
    DeleteInstrument,
    RenameInstrument,
    EditInstrument,
    SelectInstrument,
    PreviewInstrument,
    ToggleInstrumentMiniPanel,
    ToggleInstrumentExpanded,
    InstrumentNextTab,
    InstrumentPrevTab,

    // Pattern management
    AddPattern,
    DeletePattern,
    ClonePattern,
    SelectPattern,

    // Arrangement entry reorder
    ArrangementMoveEntryUp,
    ArrangementMoveEntryDown,

    // Envelope editor
    EnvCycle,
    EnvPrev,
    EnvMoveUp,
    EnvMoveDown,
    EnvMoveLeft,
    EnvMoveRight,
    EnvAddPoint,
    EnvDeletePoint,
    EnvSelectFirst,
    EnvSelectLast,
    EnvChangeValue,
    EnvToggleEnabled,

    // Waveform editor
    WfTogglePencil,
    WfToggleLoop,
    WfSetLoopStart,
    WfSetLoopEnd,
    WfMoveCursorLeft,
    WfMoveCursorRight,
    WfDrawSample,
    WfValueUp,
    WfValueDown,
    WfFocus,
    WfUnfocus,

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
            Action::EnterVisualLineMode => "Visual Line Mode",
            Action::EnterReplaceMode => "Replace Mode",
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
            Action::GoToStart => "Go to Start",
            Action::GoToEnd => "Go to End",
            Action::ResetHorizontalView => "Reset Horizontal View",
            Action::AddTrack => "Add Track",
            Action::DeleteTrack => "Delete Track",
            Action::CloneTrack => "Clone Track",
            Action::Quantize => "Quantize",
            Action::TransposeUp => "Transpose Up",
            Action::TransposeDown => "Transpose Down",
            Action::TransposeOctaveUp => "Transpose Octave Up",
            Action::TransposeOctaveDown => "Transpose Octave Down",
            Action::Interpolate => "Interpolate",
            Action::FillSelection => "Fill Selection",
            Action::RandomizeNotes => "Randomize Notes",
            Action::ReverseSelection => "Reverse Selection",
            Action::HumanizeNotes => "Humanize Notes",
            Action::SetMark(_) => "Set Mark",
            Action::GotoMark(_) => "Go to Mark",
            Action::SetRegister(_) => "Set Register",
            Action::StartMacroRecord(_) => "Start Macro Record",
            Action::StopMacroRecord => "Stop Macro Record",
            Action::ReplayMacro(_) => "Replay Macro",
            Action::ReplayLastMacro => "Replay Last Macro",
            Action::AddBookmark => "Add Bookmark",
            Action::NextBookmark => "Next Bookmark",
            Action::PrevBookmark => "Prev Bookmark",
            Action::DeleteCell => "Delete Cell",
            Action::InsertRow => "Insert Row",
            Action::InsertRowBelow => "Insert Row Below",
            Action::DeleteRow => "Delete Row",
            Action::Undo => "Undo",
            Action::EnterCommandMode => "Command Mode",
            Action::TogglePlay => "Play/Pause",
            Action::PlayFromCursor => "Play from Cursor",
            Action::Stop => "Stop",
            Action::BpmUp => "BPM Up",
            Action::BpmDown => "BPM Down",
            Action::BpmUpLarge => "BPM Up Large",
            Action::BpmDownLarge => "BPM Down Large",
            Action::ToggleLoop => "Toggle Loop",
            Action::ToggleMetronome => "Toggle Metronome",
            Action::TogglePlaybackMode => "Playback Mode",
            Action::JumpNextPattern => "Next Pattern",
            Action::JumpPrevPattern => "Prev Pattern",
            Action::ToggleMute => "Toggle Mute",
            Action::ToggleSolo => "Toggle Solo",
            Action::TrackVolumeUp => "Track Volume +",
            Action::TrackVolumeDown => "Track Volume -",
            Action::TrackPanLeft => "Track Pan Left",
            Action::TrackPanRight => "Track Pan Right",
            Action::NextTrack => "Next Track",
            Action::PrevTrack => "Prev Track",
            Action::SwitchView(_) => "Switch View",
            Action::SaveProject => "Save Project",
            Action::LoadProject => "Load Project",
            Action::OpenExportDialog => "Export Dialog",
            Action::ToggleSplitView => "Split View",
            Action::ExecuteScript => "Execute Script",
            Action::ExecuteScriptOnSelection => "Execute Script (Selection)",
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
            Action::ToggleEffectHelp => "Effect Help",
            Action::OpenFileBrowser => "File Browser",
            Action::OpenModuleBrowser => "Module Browser",
            Action::AddInstrument => "Add Instrument",
            Action::DeleteInstrument => "Delete Instrument",
            Action::RenameInstrument => "Rename Instrument",
            Action::EditInstrument => "Edit Instrument",
            Action::SelectInstrument => "Select Instrument",
            Action::PreviewInstrument => "Preview Instrument",
            Action::ToggleInstrumentMiniPanel => "Instrument Mini Panel",
            Action::ToggleInstrumentExpanded => "Instrument Expand",
            Action::InstrumentNextTab => "Next Instrument Tab",
            Action::InstrumentPrevTab => "Prev Instrument Tab",
            Action::AddPattern => "Add Pattern",
            Action::DeletePattern => "Delete Pattern",
            Action::ClonePattern => "Clone Pattern",
            Action::SelectPattern => "Select Pattern",
            Action::ArrangementMoveEntryUp => "Move Entry Up",
            Action::ArrangementMoveEntryDown => "Move Entry Down",
            Action::EnvCycle => "Env Cycle",
            Action::EnvPrev => "Env Prev",
            Action::EnvMoveUp => "Env Move Up",
            Action::EnvMoveDown => "Env Move Down",
            Action::EnvMoveLeft => "Env Move Left",
            Action::EnvMoveRight => "Env Move Right",
            Action::EnvAddPoint => "Env Add Point",
            Action::EnvDeletePoint => "Env Delete Point",
            Action::EnvSelectFirst => "Env Select First",
            Action::EnvSelectLast => "Env Select Last",
            Action::EnvChangeValue => "Env Change Value",
            Action::EnvToggleEnabled => "Env Toggle Enabled",
            Action::WfTogglePencil => "Wf Toggle Pencil",
            Action::WfToggleLoop => "Wf Toggle Loop",
            Action::WfSetLoopStart => "Wf Set Loop Start",
            Action::WfSetLoopEnd => "Wf Set Loop End",
            Action::WfMoveCursorLeft => "Wf Move Cursor Left",
            Action::WfMoveCursorRight => "Wf Move Cursor Right",
            Action::WfDrawSample => "Wf Draw Sample",
            Action::WfValueUp => "Wf Value Up",
            Action::WfValueDown => "Wf Value Down",
            Action::WfFocus => "Wf Focus",
            Action::WfUnfocus => "Wf Unfocus",
            Action::None => "None",
            Action::ShowWhichKey => "Show Which-Key",
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
            Action::EnterVisualMode => "Enter visual mode for rectangular selection",
            Action::EnterVisualLineMode => "Enter visual line mode (selects full rows)",
            Action::EnterReplaceMode => "Enter replace mode for overwriting cells",
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
            Action::GoToStart => "Jump to start of row",
            Action::GoToEnd => "Jump to end of row",
            Action::ResetHorizontalView => "Reset horizontal view to leftmost channel",
            Action::AddTrack => "Add a new track",
            Action::DeleteTrack => "Delete current track",
            Action::CloneTrack => "Clone current track",
            Action::Quantize => "Quantize selection",
            Action::TransposeUp => "Transpose semitone up",
            Action::TransposeDown => "Transpose semitone down",
            Action::TransposeOctaveUp => "Transpose octave up",
            Action::TransposeOctaveDown => "Transpose octave down",
            Action::Interpolate => "Interpolate selection between values",
            Action::FillSelection => "Fill selection with last entered note",
            Action::RandomizeNotes => "Randomize pitches of notes in selection",
            Action::ReverseSelection => "Reverse row order of selection",
            Action::HumanizeNotes => "Add small random velocity offsets to notes in selection",
            Action::SetMark(_) => "Set a named mark at the cursor position",
            Action::GotoMark(_) => "Jump to a named mark",
            Action::SetRegister(_) => "Set active register for next yank/paste/cut",
            Action::StartMacroRecord(_) => "Start recording keystrokes into a macro register",
            Action::StopMacroRecord => "Stop recording macro",
            Action::ReplayMacro(_) => "Replay a recorded macro",
            Action::ReplayLastMacro => "Replay the last used macro",
            Action::AddBookmark => "Add/remove bookmark at cursor (toggle)",
            Action::NextBookmark => "Jump to next bookmark",
            Action::PrevBookmark => "Jump to previous bookmark",
            Action::DeleteCell => "Delete cell content at cursor",
            Action::InsertRow => "Insert a blank row",
            Action::InsertRowBelow => "Insert a blank row below",
            Action::DeleteRow => "Delete current row",
            Action::Undo => "Undo last change",
            Action::EnterCommandMode => "Enter command line mode",
            Action::TogglePlay => "Toggle audio playback",
            Action::PlayFromCursor => "Start playback from current row (with chasing)",
            Action::Stop => "Stop playback",
            Action::BpmUp => "Increase BPM by 1",
            Action::BpmDown => "Decrease BPM by 1",
            Action::BpmUpLarge => "Increase BPM by 10",
            Action::BpmDownLarge => "Decrease BPM by 10",
            Action::ToggleLoop => "Toggle playback looping",
            Action::ToggleMetronome => "Toggle metronome click during playback",
            Action::TogglePlaybackMode => "Toggle song/pattern mode",
            Action::JumpNextPattern => "Jump to next pattern",
            Action::JumpPrevPattern => "Jump to previous pattern",
            Action::ToggleMute => "Mute/unmute current track",
            Action::ToggleSolo => "Solo current track",
            Action::TrackVolumeUp => "Increase current track volume by 5%",
            Action::TrackVolumeDown => "Decrease current track volume by 5%",
            Action::TrackPanLeft => "Pan current track left by 10%",
            Action::TrackPanRight => "Pan current track right by 10%",
            Action::NextTrack => "Jump to next track",
            Action::PrevTrack => "Jump to previous track",
            Action::SwitchView(_) => "Switch to another application view",
            Action::SaveProject => "Save current project",
            Action::LoadProject => "Load project from file",
            Action::OpenExportDialog => "Open audio export dialog",
            Action::ToggleSplitView => "Toggle code editor split view",
            Action::ExecuteScript => "Execute current script",
            Action::ExecuteScriptOnSelection => "Execute script scoped to visual selection",
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
            Action::ToggleEffectHelp => "Show effect command explorer",
            Action::OpenFileBrowser => "Open file browser",
            Action::OpenModuleBrowser => "Open module browser",
            Action::AddInstrument => "Add a new instrument",
            Action::DeleteInstrument => "Delete current instrument",
            Action::RenameInstrument => "Rename current instrument",
            Action::EditInstrument => "Enter instrument editor",
            Action::SelectInstrument => "Select current instrument",
            Action::PreviewInstrument => "Play a preview note for the selected instrument",
            Action::ToggleInstrumentMiniPanel => "Toggle instrument mini panel",
            Action::ToggleInstrumentExpanded => "Toggle instrument expanded view",
            Action::InstrumentNextTab => "Switch to next instrument editor tab",
            Action::InstrumentPrevTab => "Switch to previous instrument editor tab",
            Action::AddPattern => "Add a new pattern",
            Action::DeletePattern => "Delete current pattern",
            Action::ClonePattern => "Clone current pattern",
            Action::SelectPattern => "Select current pattern",
            Action::ArrangementMoveEntryUp => "Move arrangement entry up",
            Action::ArrangementMoveEntryDown => "Move arrangement entry down",
            Action::EnvCycle => "Cycle envelope type",
            Action::EnvPrev => "Previous envelope type",
            Action::EnvMoveUp => "Move envelope point up",
            Action::EnvMoveDown => "Move envelope point down",
            Action::EnvMoveLeft => "Move envelope point left",
            Action::EnvMoveRight => "Move envelope point right",
            Action::EnvAddPoint => "Add envelope point",
            Action::EnvDeletePoint => "Delete envelope point",
            Action::EnvSelectFirst => "Select first envelope point",
            Action::EnvSelectLast => "Select last envelope point",
            Action::EnvChangeValue => "Change envelope point value",
            Action::EnvToggleEnabled => "Toggle envelope enabled",
            Action::WfTogglePencil => "Toggle pencil mode for waveform editing",
            Action::WfToggleLoop => "Toggle loop mode for sample",
            Action::WfSetLoopStart => "Set loop start at cursor",
            Action::WfSetLoopEnd => "Set loop end at cursor",
            Action::WfMoveCursorLeft => "Move waveform cursor left",
            Action::WfMoveCursorRight => "Move waveform cursor right",
            Action::WfDrawSample => "Draw sample value at cursor",
            Action::WfValueUp => "Increase pencil value",
            Action::WfValueDown => "Decrease pencil value",
            Action::WfFocus => "Focus waveform editor",
            Action::WfUnfocus => "Unfocus waveform editor",
            Action::None => "No operation",
            Action::ShowWhichKey => "Show which-key menu",
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
            | Action::GoToStart
            | Action::GoToEnd
            | Action::ResetHorizontalView
            | Action::NextTrack
            | Action::PrevTrack => ActionCategory::Navigation,

            Action::EnterInsertMode
            | Action::EnterNormalMode
            | Action::EnterVisualMode
            | Action::EnterVisualLineMode
            | Action::EnterReplaceMode => ActionCategory::Application,

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
            | Action::FillSelection
            | Action::RandomizeNotes
            | Action::ReverseSelection
            | Action::HumanizeNotes
            | Action::SetMark(_)
            | Action::GotoMark(_)
            | Action::AddBookmark
            | Action::NextBookmark
            | Action::PrevBookmark
            | Action::DeleteCell
            | Action::InsertRow
            | Action::InsertRowBelow
            | Action::DeleteRow
            | Action::Undo
            | Action::ToggleDrawMode => ActionCategory::Editing,

            Action::SetRegister(_)
            | Action::StartMacroRecord(_)
            | Action::StopMacroRecord
            | Action::ReplayMacro(_)
            | Action::ReplayLastMacro => ActionCategory::Application,

            Action::Copy | Action::Paste | Action::Cut | Action::Redo => ActionCategory::Clipboard,

            Action::AddTrack
            | Action::DeleteTrack
            | Action::CloneTrack
            | Action::ToggleMute
            | Action::ToggleSolo
            | Action::TrackVolumeUp
            | Action::TrackVolumeDown
            | Action::TrackPanLeft
            | Action::TrackPanRight => ActionCategory::Track,

            Action::AddInstrument
            | Action::DeleteInstrument
            | Action::RenameInstrument
            | Action::EditInstrument
            | Action::SelectInstrument
            | Action::PreviewInstrument
            | Action::ToggleInstrumentMiniPanel
            | Action::ToggleInstrumentExpanded
            | Action::InstrumentNextTab
            | Action::InstrumentPrevTab => ActionCategory::Instrument,

            Action::AddPattern
            | Action::DeletePattern
            | Action::ClonePattern
            | Action::SelectPattern
            | Action::ArrangementMoveEntryUp
            | Action::ArrangementMoveEntryDown => ActionCategory::Pattern,

            Action::EnvCycle
            | Action::EnvPrev
            | Action::EnvMoveUp
            | Action::EnvMoveDown
            | Action::EnvMoveLeft
            | Action::EnvMoveRight
            | Action::EnvAddPoint
            | Action::EnvDeletePoint
            | Action::EnvSelectFirst
            | Action::EnvSelectLast
            | Action::EnvChangeValue
            | Action::EnvToggleEnabled
            | Action::WfTogglePencil
            | Action::WfToggleLoop
            | Action::WfSetLoopStart
            | Action::WfSetLoopEnd
            | Action::WfMoveCursorLeft
            | Action::WfMoveCursorRight
            | Action::WfDrawSample
            | Action::WfValueUp
            | Action::WfValueDown
            | Action::WfFocus
            | Action::WfUnfocus => ActionCategory::Instrument,

            Action::TogglePlay
            | Action::PlayFromCursor
            | Action::Stop
            | Action::BpmUp
            | Action::BpmDown
            | Action::BpmUpLarge
            | Action::BpmDownLarge
            | Action::ToggleLoop
            | Action::ToggleMetronome
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

            Action::ExecuteScript | Action::ExecuteScriptOnSelection | Action::OpenTemplates => {
                ActionCategory::Editing
            }

            Action::EnterCommandMode
            | Action::Quit
            | Action::Confirm
            | Action::Cancel
            | Action::OpenModal
            | Action::ToggleHelp
            | Action::ToggleEffectHelp
            | Action::OpenFileBrowser
            | Action::OpenModuleBrowser
            | Action::ShowWhichKey => ActionCategory::Application,

            Action::None => ActionCategory::None,
        }
    }
}
