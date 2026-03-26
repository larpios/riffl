/// Pattern editor state machine
///
/// Provides a vim-inspired modal editor for the tracker pattern grid.
/// Supports Normal (navigation), Insert (note entry), and Visual (selection) modes.
use tracker_core::pattern::effect::Effect;
use tracker_core::pattern::note::{Note, NoteEvent, Pitch};
use tracker_core::pattern::pattern::Pattern;
use tracker_core::pattern::row::Cell;

/// Clipboard for copy/paste operations.
///
/// Stores a rectangular grid of cells copied from the pattern.
/// A single cell copy is a 1×1 grid.
#[derive(Debug, Clone)]
pub struct Clipboard {
    /// 2D grid of cells: rows × columns.
    cells: Vec<Vec<Cell>>,
    /// Number of rows in the clipboard.
    num_rows: usize,
    /// Number of columns (channels) in the clipboard.
    num_cols: usize,
}

impl Clipboard {
    /// Create a clipboard from a rectangular grid of cells.
    pub fn new(cells: Vec<Vec<Cell>>) -> Self {
        let num_rows = cells.len();
        let num_cols = cells.first().map_or(0, |r| r.len());
        Self {
            cells,
            num_rows,
            num_cols,
        }
    }

    /// Create a clipboard holding a single cell.
    pub fn single(cell: Cell) -> Self {
        Self::new(vec![vec![cell]])
    }

    /// Check if the clipboard is empty.
    pub fn is_empty(&self) -> bool {
        self.num_rows == 0 || self.num_cols == 0
    }

    /// Get the dimensions of the clipboard content.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.num_rows, self.num_cols)
    }

    /// Get a reference to the cell grid.
    pub fn cells(&self) -> &Vec<Vec<Cell>> {
        &self.cells
    }
}

/// The sub-column within a channel that the cursor is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubColumn {
    Note,
    Instrument,
    Volume,
    Effect,
}

impl SubColumn {
    /// Move to the next sub-column (wraps around).
    pub fn next(self) -> Self {
        match self {
            SubColumn::Note => SubColumn::Instrument,
            SubColumn::Instrument => SubColumn::Volume,
            SubColumn::Volume => SubColumn::Effect,
            SubColumn::Effect => SubColumn::Note,
        }
    }

    /// Move to the previous sub-column (wraps around).
    pub fn prev(self) -> Self {
        match self {
            SubColumn::Note => SubColumn::Effect,
            SubColumn::Instrument => SubColumn::Note,
            SubColumn::Volume => SubColumn::Instrument,
            SubColumn::Effect => SubColumn::Volume,
        }
    }
}

/// Editor mode (vim-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Navigation mode — move cursor, no editing.
    Normal,
    /// Note entry mode — typing inserts notes/data.
    Insert,
    /// Selection mode — select ranges of cells.
    Visual,
    /// Replace mode — overwrite cells without advancing cursor on each keystroke.
    Replace,
}

impl EditorMode {
    /// Display label for the mode.
    pub fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
            EditorMode::Replace => "REPLACE",
        }
    }
}

/// A snapshot of the pattern for undo support.
#[derive(Debug, Clone)]
struct HistoryEntry {
    pattern: Pattern,
    cursor_row: usize,
    cursor_channel: usize,
}

/// Number of rows to jump with page up/down.
const PAGE_SIZE: usize = 16;

/// Maximum undo history depth.
const MAX_HISTORY: usize = 100;

/// Width of a single channel column (including separator): "│ C#4 01 40 C20 " = 2 + 14 + 1 = 17
const CHANNEL_COL_WIDTH: u16 = 17;

/// Width of the row number column: "  XX  " = 6
const ROW_NUM_WIDTH: u16 = 6;

/// The pattern editor wrapping a `Pattern` with cursor, mode, and edit history.
#[derive(Debug, Clone)]
pub struct Editor {
    /// The pattern being edited.
    pattern: Pattern,
    /// Current cursor row.
    cursor_row: usize,
    /// Current cursor channel.
    cursor_channel: usize,
    /// Current sub-column within the channel.
    sub_column: SubColumn,
    /// Current editor mode.
    mode: EditorMode,
    /// Current default octave for note entry.
    current_octave: u8,
    /// Current default instrument index.
    current_instrument: u8,
    /// Undo history (snapshots before edits).
    history: Vec<HistoryEntry>,
    /// Redo stack (snapshots restored by undo, cleared on new edits).
    redo_history: Vec<HistoryEntry>,
    /// Visual mode anchor (row, channel) — starting position of selection.
    visual_anchor: Option<(usize, usize)>,
    /// Clipboard for copy/paste operations.
    clipboard: Option<Clipboard>,
    /// Current hex digit entry position for the effect column (0=command, 1=param_hi, 2=param_lo).
    effect_digit_position: u8,
    /// Current hex digit entry position for the Instrument sub-column (0=hi nibble, 1=lo nibble).
    instrument_digit_pos: u8,
    /// Current hex digit entry position for the Volume sub-column (0=hi nibble, 1=lo nibble).
    volume_digit_pos: u8,
    /// Number of rows to advance after entering a note/event (step size, default 1).
    step_size: usize,
}


pub mod clipboard;
pub mod editing;
pub mod navigation;
#[cfg(test)]
mod tests;

impl Editor {
    /// Get a reference to the clipboard contents.
    pub fn get_clipboard(&self) -> Option<&Clipboard> {
        self.clipboard.as_ref()
    }

    /// Create a new editor wrapping the given pattern.
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            cursor_row: 0,
            cursor_channel: 0,
            sub_column: SubColumn::Note,
            mode: EditorMode::Normal,
            current_octave: 4,
            current_instrument: 0,
            history: Vec::new(),
            redo_history: Vec::new(),
            visual_anchor: None,
            clipboard: None,
            effect_digit_position: 0,
            instrument_digit_pos: 0,
            volume_digit_pos: 0,
            step_size: 1,
        }
    }

    // --- Accessors ---

    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn pattern_mut(&mut self) -> &mut Pattern {
        &mut self.pattern
    }

    pub fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    pub fn cursor_channel(&self) -> usize {
        self.cursor_channel
    }

    pub fn sub_column(&self) -> SubColumn {
        self.sub_column
    }

    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    pub fn current_octave(&self) -> u8 {
        self.current_octave
    }

    pub fn current_instrument(&self) -> u8 {
        self.current_instrument
    }

    pub fn set_instrument(&mut self, instrument: usize) {
        self.current_instrument = instrument as u8;
    }

    /// Increase current octave by 1 (max 9).
    pub fn octave_up(&mut self) {
        if self.current_octave < 9 {
            self.current_octave += 1;
        }
    }

    /// Decrease current octave by 1 (min 0).
    pub fn octave_down(&mut self) {
        if self.current_octave > 0 {
            self.current_octave -= 1;
        }
    }

    /// Go to specific row.
    pub fn go_to_row(&mut self, row: usize) {
        let max_row = self.pattern.num_rows().saturating_sub(1);
        self.cursor_row = row.min(max_row);
    }

    /// Set cursor channel.
    pub fn set_cursor_channel(&mut self, channel: usize) {
        let max_channel = self.pattern.num_channels().saturating_sub(1);
        self.cursor_channel = channel.min(max_channel);
    }

    /// Quantize: snap all notes in selection to grid (4-row intervals).
    pub fn quantize(&mut self) {
        if let Some(((r0, c0), (r1, c1))) = self.visual_selection() {
            self.save_history();
            for row in r0..=r1 {
                for ch in c0..=c1 {
                    if let Some(cell) = self.pattern.get_cell(row, ch) {
                        if cell.note.is_some() {
                            let quantized_row = (row / 4) * 4;
                            if quantized_row != row {
                                let cell_clone = cell.clone();
                                self.pattern.set_cell(quantized_row, ch, cell_clone);
                                self.pattern.set_cell(row, ch, Cell::empty());
                            }
                        }
                    }
                }
            }
        }
    }

    /// Replace the pattern being edited.
    pub fn set_pattern(&mut self, pattern: Pattern) {
        self.pattern = pattern;
        self.cursor_row = 0;
        self.cursor_channel = 0;
        self.history.clear();
    }

    /// Add a new track at the end.
    pub fn add_track(&mut self) {
        self.save_history();
        self.pattern.add_track();
    }

    /// Delete the track at the current cursor channel.
    pub fn delete_track(&mut self) -> bool {
        if self.pattern.remove_track(self.cursor_channel) {
            if self.cursor_channel >= self.pattern.num_channels() {
                self.cursor_channel = self.pattern.num_channels().saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    /// Clone the track at the current cursor channel.
    pub fn clone_track(&mut self) -> bool {
        self.save_history();
        self.pattern.clone_track(self.cursor_channel)
    }

}
