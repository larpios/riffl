/// Pattern editor state machine
///
/// Provides a vim-inspired modal editor for the tracker pattern grid.
/// Supports Normal (navigation), Insert (note entry), and Visual (selection) modes.
use riffl_core::pattern::effect::Effect;
use riffl_core::pattern::note::{Note, NoteEvent, Pitch};
use riffl_core::pattern::pattern::Pattern;
use riffl_core::pattern::row::Cell;
use std::collections::HashMap;

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
    /// First effect slot (effects[0]).
    Effect,
    /// Second effect slot (effects[1]).
    Effect2,
}

impl SubColumn {
    /// Move to the next sub-column (wraps around).
    pub fn next(self) -> Self {
        match self {
            SubColumn::Note => SubColumn::Instrument,
            SubColumn::Instrument => SubColumn::Volume,
            SubColumn::Volume => SubColumn::Effect,
            SubColumn::Effect => SubColumn::Effect2,
            SubColumn::Effect2 => SubColumn::Note,
        }
    }

    /// Move to the previous sub-column (wraps around).
    pub fn prev(self) -> Self {
        match self {
            SubColumn::Note => SubColumn::Effect2,
            SubColumn::Instrument => SubColumn::Note,
            SubColumn::Volume => SubColumn::Instrument,
            SubColumn::Effect => SubColumn::Volume,
            SubColumn::Effect2 => SubColumn::Effect,
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
    /// Selection mode — select rectangular blocks of cells.
    Visual,
    /// Selection mode — selects full rows across all channels.
    VisualLine,
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
            EditorMode::VisualLine => "V-LINE",
            EditorMode::Replace => "REPLACE",
        }
    }

    /// True if this is any visual selection mode.
    pub fn is_visual(self) -> bool {
        matches!(self, EditorMode::Visual | EditorMode::VisualLine)
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

/// Width of a single channel column (including separator):
/// "│ C#4 01 40 0C20 0A04 " = 2 + 3+1+2+1+2+1+4+1+4 + 1 = 22
const CHANNEL_COL_WIDTH: u16 = 22;

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
    /// Named registers for copy/paste (0-9, a-z). Default register is '0'.
    registers: HashMap<char, Clipboard>,
    /// The register that the next yank/paste/cut will use (defaults to '0').
    active_register: char,
    /// Named marks: maps a char to (row, channel) within the current pattern.
    marks: HashMap<char, (usize, usize)>,
    /// Current hex digit entry position for the effect column (0=command, 1=param_hi, 2=param_lo).
    effect_digit_position: u8,
    /// Current hex digit entry position for the Instrument sub-column (0=hi nibble, 1=lo nibble).
    instrument_digit_pos: u8,
    /// Current hex digit entry position for the Volume sub-column (0=hi nibble, 1=lo nibble).
    volume_digit_pos: u8,
    /// Number of rows to advance after entering a note/event (step size, default 1).
    step_size: usize,
    /// Named cursor bookmarks: (row, channel, label) tuples for quick navigation.
    bookmarks: Vec<(usize, usize, String)>,
    /// Index of the last-visited bookmark (for cycling through bookmarks).
    bookmark_cursor: usize,
    /// Pending count prefix digits accumulated before a motion (e.g. "10" before "j").
    count_prefix: String,
}

pub mod clipboard;
pub mod editing;
pub mod navigation;
#[cfg(test)]
mod tests;

impl Editor {
    /// Append a digit to the count prefix buffer (called from input handler).
    pub fn push_count_digit(&mut self, c: char) {
        self.count_prefix.push(c);
    }

    /// Drain the count prefix buffer and parse it as a count (defaults to 1 if empty).
    pub fn take_count(&mut self) -> usize {
        if self.count_prefix.is_empty() {
            1
        } else {
            let s = std::mem::take(&mut self.count_prefix);
            s.parse::<usize>().unwrap_or(1).max(1)
        }
    }

    /// Peek at the accumulated count prefix string (for status bar display).
    pub fn count_prefix(&self) -> &str {
        &self.count_prefix
    }

    /// Discard the count prefix (e.g. on Esc or unknown key).
    pub fn clear_count(&mut self) {
        self.count_prefix.clear();
    }

    /// Get the contents of the active register (for paste preview in UI).
    pub fn get_clipboard(&self) -> Option<&Clipboard> {
        self.registers.get(&self.active_register)
    }

    /// Set the active register (used before yank/paste/cut). Resets to '0' after use.
    pub fn set_active_register(&mut self, c: char) {
        self.active_register = c;
    }

    /// Read the active register name.
    pub fn active_register(&self) -> char {
        self.active_register
    }

    /// Set a mark at the current cursor position.
    pub fn set_mark(&mut self, name: char) {
        self.marks
            .insert(name, (self.cursor_row, self.cursor_channel));
    }

    /// Jump to a named mark. Returns false if the mark doesn't exist.
    pub fn goto_mark(&mut self, name: char) -> bool {
        if let Some(&(row, ch)) = self.marks.get(&name) {
            let max_row = self.pattern.num_rows().saturating_sub(1);
            let max_ch = self.pattern.num_channels().saturating_sub(1);
            self.cursor_row = row.min(max_row);
            self.cursor_channel = ch.min(max_ch);
            true
        } else {
            false
        }
    }

    /// Return all current marks as (name, row, channel) triples.
    pub fn marks(&self) -> impl Iterator<Item = (char, usize, usize)> + '_ {
        self.marks.iter().map(|(&name, &(row, ch))| (name, row, ch))
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
            registers: HashMap::new(),
            active_register: '0',
            marks: HashMap::new(),
            effect_digit_position: 0,
            instrument_digit_pos: 0,
            volume_digit_pos: 0,
            step_size: 1,
            bookmarks: Vec::new(),
            bookmark_cursor: 0,
            count_prefix: String::new(),
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

    /// Add a bookmark at the current cursor position with an optional label.
    /// If a bookmark at the same (row, channel) already exists, removes it (toggle).
    pub fn add_bookmark(&mut self, label: Option<String>) {
        let row = self.cursor_row;
        let ch = self.cursor_channel;
        if let Some(idx) = self
            .bookmarks
            .iter()
            .position(|(r, c, _)| *r == row && *c == ch)
        {
            self.bookmarks.remove(idx);
            if self.bookmark_cursor > 0 && self.bookmark_cursor >= self.bookmarks.len() {
                self.bookmark_cursor = self.bookmarks.len().saturating_sub(1);
            }
        } else {
            let label = label.unwrap_or_else(|| format!("B{}", self.bookmarks.len() + 1));
            self.bookmarks.push((row, ch, label));
            self.bookmarks.sort_by_key(|(r, c, _)| (*r, *c));
            self.bookmark_cursor = self
                .bookmarks
                .iter()
                .position(|(r, c, _)| *r == row && *c == ch)
                .unwrap_or(0);
        }
    }

    /// Move to the next bookmark (wraps around).
    pub fn goto_next_bookmark(&mut self) {
        if self.bookmarks.is_empty() {
            return;
        }
        self.bookmark_cursor = (self.bookmark_cursor + 1) % self.bookmarks.len();
        let (row, ch, _) = self.bookmarks[self.bookmark_cursor].clone();
        self.cursor_row = row;
        self.cursor_channel = ch;
    }

    /// Move to the previous bookmark (wraps around).
    pub fn goto_prev_bookmark(&mut self) {
        if self.bookmarks.is_empty() {
            return;
        }
        self.bookmark_cursor = if self.bookmark_cursor == 0 {
            self.bookmarks.len() - 1
        } else {
            self.bookmark_cursor - 1
        };
        let (row, ch, _) = self.bookmarks[self.bookmark_cursor].clone();
        self.cursor_row = row;
        self.cursor_channel = ch;
    }

    /// Get all current bookmarks as (row, channel, label) slices.
    pub fn bookmarks(&self) -> &[(usize, usize, String)] {
        &self.bookmarks
    }

    /// Clear all bookmarks.
    pub fn clear_bookmarks(&mut self) {
        self.bookmarks.clear();
        self.bookmark_cursor = 0;
    }
}
