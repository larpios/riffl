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

    // --- Navigation ---

    /// Move cursor up by one row.
    pub fn move_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(1);
        self.effect_digit_position = 0;
    }

    /// Move cursor down by one row.
    pub fn move_down(&mut self) {
        let max = self.pattern.num_rows().saturating_sub(1);
        if self.cursor_row < max {
            self.cursor_row += 1;
        }
        self.effect_digit_position = 0;
    }

    /// Advance cursor down after note/effect entry in Insert mode.
    /// Advances by `step_size` rows; does not extend the pattern past its end.
    fn advance_row(&mut self) {
        let num_rows = self.pattern.num_rows();
        let next = (self.cursor_row + self.step_size).min(num_rows.saturating_sub(1));
        self.cursor_row = next;
        self.effect_digit_position = 0;
    }

    /// Get the current step size (rows advanced after each note entry).
    pub fn step_size(&self) -> usize {
        self.step_size
    }

    /// Set the step size (0–8).
    pub fn set_step_size(&mut self, step: usize) {
        self.step_size = step.min(8);
    }

    /// Increase step size by 1 (max 8).
    pub fn step_up(&mut self) {
        self.step_size = (self.step_size + 1).min(8);
    }

    /// Decrease step size by 1 (min 0).
    pub fn step_down(&mut self) {
        self.step_size = self.step_size.saturating_sub(1);
    }

    /// Move down in Insert mode, extending the pattern if at the last row.
    pub fn extend_down(&mut self) {
        let last = self.pattern.num_rows().saturating_sub(1);
        if self.cursor_row >= last {
            self.pattern.insert_row(self.pattern.num_rows());
        }
        self.cursor_row += 1;
        self.effect_digit_position = 0;
    }

    /// Move cursor left. In Normal mode, moves by channel. In Insert mode,
    /// moves by sub-column first, then wraps to previous channel.
    pub fn move_left(&mut self) {
        self.effect_digit_position = 0;
        if self.sub_column != SubColumn::Note {
            self.sub_column = self.sub_column.prev();
        } else if self.cursor_channel > 0 {
            self.cursor_channel -= 1;
            self.sub_column = SubColumn::Effect;
        }
    }

    /// Move cursor right by sub-column, wrapping to the next channel after Effect.
    pub fn move_right(&mut self) {
        self.effect_digit_position = 0;
        let max_ch = self.pattern.num_channels().saturating_sub(1);
        if self.sub_column != SubColumn::Effect {
            self.sub_column = self.sub_column.next();
        } else if self.cursor_channel < max_ch {
            self.cursor_channel += 1;
            self.sub_column = SubColumn::Note;
        }
    }

    /// Move cursor to the next channel (track), keeping the sub-column.
    pub fn next_channel(&mut self) {
        let max_ch = self.pattern.num_channels().saturating_sub(1);
        if self.cursor_channel < max_ch {
            self.cursor_channel += 1;
        }
        self.effect_digit_position = 0;
    }

    /// Move cursor to the previous channel (track), keeping the sub-column.
    pub fn prev_channel(&mut self) {
        self.cursor_channel = self.cursor_channel.saturating_sub(1);
        self.effect_digit_position = 0;
    }

    /// Move cursor up by a page (PAGE_SIZE rows).
    pub fn page_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(PAGE_SIZE);
        self.effect_digit_position = 0;
    }

    /// Move cursor down by a page (PAGE_SIZE rows).
    pub fn page_down(&mut self) {
        let max = self.pattern.num_rows().saturating_sub(1);
        self.cursor_row = (self.cursor_row + PAGE_SIZE).min(max);
        self.effect_digit_position = 0;
    }

    /// Move cursor to the first row.
    pub fn home(&mut self) {
        self.cursor_row = 0;
    }

    /// Move cursor to the last row.
    pub fn end(&mut self) {
        self.cursor_row = self.pattern.num_rows().saturating_sub(1);
    }

    /// Move cursor to the next track (channel), wrapping around to 0.
    pub fn next_track(&mut self) {
        let max_ch = self.pattern.num_channels();
        self.cursor_channel = (self.cursor_channel + 1) % max_ch;
        // Reset sub-column to Note when jumping tracks
        self.sub_column = SubColumn::Note;
        self.effect_digit_position = 0;
    }

    // --- Mode Transitions ---

    /// Enter Insert mode.
    pub fn enter_insert_mode(&mut self) {
        self.mode = EditorMode::Insert;
        self.effect_digit_position = 0;
    }

    /// Enter Normal mode (from any mode).
    pub fn enter_normal_mode(&mut self) {
        self.mode = EditorMode::Normal;
        self.visual_anchor = None;
        self.effect_digit_position = 0;
    }

    /// Enter Replace mode — edits overwrite cells without cursor advancement.
    pub fn enter_replace_mode(&mut self) {
        self.mode = EditorMode::Replace;
        self.effect_digit_position = 0;
    }

    /// Whether the editor is in a data-entry mode (Insert or Replace).
    pub fn is_entry_mode(&self) -> bool {
        matches!(self.mode, EditorMode::Insert | EditorMode::Replace)
    }

    /// Enter Visual mode, anchoring at the current position.
    pub fn enter_visual_mode(&mut self) {
        self.mode = EditorMode::Visual;
        self.visual_anchor = Some((self.cursor_row, self.cursor_channel));
    }

    /// Get the visual selection range as ((start_row, start_ch), (end_row, end_ch)),
    /// normalized so start <= end.
    pub fn visual_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.visual_anchor.map(|(ar, ac)| {
            let r0 = ar.min(self.cursor_row);
            let r1 = ar.max(self.cursor_row);
            let c0 = ac.min(self.cursor_channel);
            let c1 = ac.max(self.cursor_channel);
            ((r0, c0), (r1, c1))
        })
    }

    // --- Editing Operations ---

    /// Save a snapshot to the undo history before making an edit.
    fn save_history(&mut self) {
        if self.history.len() >= MAX_HISTORY {
            self.history.remove(0);
        }
        self.history.push(HistoryEntry {
            pattern: self.pattern.clone(),
            cursor_row: self.cursor_row,
            cursor_channel: self.cursor_channel,
        });
        self.redo_history.clear();
    }

    /// Undo the last edit, restoring the previous pattern and cursor.
    pub fn undo(&mut self) -> bool {
        if let Some(entry) = self.history.pop() {
            self.redo_history.push(HistoryEntry {
                pattern: self.pattern.clone(),
                cursor_row: self.cursor_row,
                cursor_channel: self.cursor_channel,
            });
            self.pattern = entry.pattern;
            self.cursor_row = entry.cursor_row;
            self.cursor_channel = entry.cursor_channel;
            true
        } else {
            false
        }
    }

    /// Redo the last undone edit.
    pub fn redo(&mut self) -> bool {
        if let Some(entry) = self.redo_history.pop() {
            self.history.push(HistoryEntry {
                pattern: self.pattern.clone(),
                cursor_row: self.cursor_row,
                cursor_channel: self.cursor_channel,
            });
            self.pattern = entry.pattern;
            self.cursor_row = entry.cursor_row;
            self.cursor_channel = entry.cursor_channel;
            true
        } else {
            false
        }
    }

    /// Enter a note pitch at the current cursor position using the current octave.
    /// Only works in Insert mode on the Note sub-column.
    pub fn enter_note(&mut self, pitch: Pitch) {
        let octave = self.current_octave;
        self.enter_note_with_octave(pitch, octave);
    }

    /// Enter a note-off event at the current cursor position.
    pub fn enter_note_off(&mut self) {
        if !self.is_entry_mode() {
            return;
        }
        self.save_history();
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::Off),
        );
        self.advance_row();
    }

    /// Enter a note-cut event at the current cursor position.
    /// Hard-silences the voice immediately (no envelope release, displayed as ^^^).
    pub fn enter_note_cut(&mut self) {
        if !self.is_entry_mode() {
            return;
        }
        self.save_history();
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::Cut),
        );
        self.advance_row();
    }

    /// Replace the note at the cursor without advancing the cursor (r replace-once).
    /// Pushes an undo snapshot before the change.
    pub fn replace_once(&mut self, pitch: Pitch) {
        self.save_history();
        let note = Note::new(pitch, self.current_octave, 100, self.current_instrument);
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::On(note)),
        );
        // cursor_row intentionally NOT advanced
    }

    /// Replace the current cell with a note-off without advancing the cursor.
    pub fn replace_cell_note_off(&mut self) {
        self.save_history();
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::Off),
        );
    }

    /// Set the current octave (0-9). Used when typing a digit in Insert mode.
    pub fn set_octave(&mut self, octave: u8) {
        if octave <= 9 {
            self.current_octave = octave;
        }
    }

    /// Get the current effect digit entry position (0=command, 1=param_hi, 2=param_lo).
    pub fn effect_digit_position(&self) -> u8 {
        self.effect_digit_position
    }

    /// Enter a hex digit (0-15) at the current effect digit position.
    ///
    /// The effect column has 3 hex positions: command (1 nibble), param_hi (1 nibble),
    /// param_lo (1 nibble). Each call fills one position and advances. After the
    /// third digit, the cursor advances to the next row and resets the position.
    ///
    /// Only works in Insert mode on the Effect sub-column.
    pub fn enter_effect_digit(&mut self, digit: u8) {
        if !self.is_entry_mode() || self.sub_column != SubColumn::Effect {
            return;
        }
        let digit = digit & 0x0F; // clamp to single nibble
        self.save_history();

        if let Some(cell) = self
            .pattern
            .get_cell_mut(self.cursor_row, self.cursor_channel)
        {
            let current = cell.first_effect().copied().unwrap_or(Effect::new(0, 0));

            let new_effect = match self.effect_digit_position {
                0 => Effect::new(digit, current.param),
                1 => {
                    let new_param = (digit << 4) | (current.param & 0x0F);
                    Effect::new(current.command, new_param)
                }
                _ => {
                    let new_param = (current.param & 0xF0) | digit;
                    Effect::new(current.command, new_param)
                }
            };

            cell.set_effect(new_effect);

            self.effect_digit_position = (self.effect_digit_position + 1) % 3;

            // After completing all 3 digits, advance to next row
            if self.effect_digit_position == 0 {
                self.advance_row();
            }
        }
    }

    /// Reset the effect digit entry position to 0.
    pub fn reset_effect_digit_position(&mut self) {
        self.effect_digit_position = 0;
    }

    /// Enter a hex digit for the Instrument sub-column (2 nibbles = 1 byte, 0x00–0xFF).
    /// First digit sets the high nibble, second sets the low nibble and advances the row.
    /// Only works in Insert mode on the Instrument sub-column.
    pub fn enter_instrument_digit(&mut self, digit: u8) {
        if !self.is_entry_mode() || self.sub_column != SubColumn::Instrument {
            return;
        }
        let digit = digit & 0x0F;
        self.save_history();
        if let Some(cell) = self
            .pattern
            .get_cell_mut(self.cursor_row, self.cursor_channel)
        {
            let current = cell.instrument.unwrap_or(0);
            let new_val = if self.instrument_digit_pos == 0 {
                (digit << 4) | (current & 0x0F)
            } else {
                (current & 0xF0) | digit
            };
            cell.instrument = Some(new_val);
            self.instrument_digit_pos = (self.instrument_digit_pos + 1) % 2;
            if self.instrument_digit_pos == 0 {
                self.advance_row();
            }
        }
    }

    /// Enter a hex digit for the Volume sub-column (2 nibbles = 1 byte, 0x00–0xFF).
    /// First digit sets the high nibble, second sets the low nibble and advances the row.
    /// Only works in Insert mode on the Volume sub-column.
    pub fn enter_volume_digit(&mut self, digit: u8) {
        if !self.is_entry_mode() || self.sub_column != SubColumn::Volume {
            return;
        }
        let digit = digit & 0x0F;
        self.save_history();
        if let Some(cell) = self
            .pattern
            .get_cell_mut(self.cursor_row, self.cursor_channel)
        {
            let current = cell.volume.unwrap_or(0);
            let new_val = if self.volume_digit_pos == 0 {
                (digit << 4) | (current & 0x0F)
            } else {
                (current & 0xF0) | digit
            };
            cell.volume = Some(new_val);
            self.volume_digit_pos = (self.volume_digit_pos + 1) % 2;
            if self.volume_digit_pos == 0 {
                self.advance_row();
            }
        }
    }

    /// Delete (clear) the current cell.
    pub fn delete_cell(&mut self) {
        self.save_history();
        self.pattern
            .clear_cell(self.cursor_row, self.cursor_channel);
    }

    /// Insert a new empty row at the cursor position, pushing rows down.
    pub fn insert_row(&mut self) {
        self.save_history();
        self.pattern.insert_row(self.cursor_row);
    }

    /// Insert a new empty row below the cursor, move down, and enter Insert mode.
    pub fn insert_row_below(&mut self) {
        self.save_history();
        let insert_at = self.cursor_row + 1;
        self.pattern.insert_row(insert_at);
        self.cursor_row = insert_at;
        self.mode = EditorMode::Insert;
        self.effect_digit_position = 0;
    }

    /// Delete the row at the cursor position, pulling rows up.
    pub fn delete_row(&mut self) {
        self.save_history();
        if self.pattern.delete_row(self.cursor_row) {
            // Clamp cursor if we deleted the last row
            let max = self.pattern.num_rows().saturating_sub(1);
            if self.cursor_row > max {
                self.cursor_row = max;
            }
        }
    }

    /// Map a piano keyboard key to a pitch and octave offset.
    ///
    /// Implements the standard FT2/IT tracker layout:
    ///   Lower row (white keys): a=C, s=D, d=E, f=F, g=G, h=A, j=B, k=C+1oct
    ///   Upper row (black keys): w=C#, e=D#, t=F#, y=G#, u=A#
    ///
    /// Returns `(pitch, octave_offset)` where `octave_offset` is +1 for `k` (C in the
    /// next octave) and 0 for all other keys.
    pub fn piano_key_to_pitch(c: char) -> Option<(Pitch, i8)> {
        match c {
            // Lower row — white keys
            'a' => Some((Pitch::C, 0)),
            's' => Some((Pitch::D, 0)),
            'd' => Some((Pitch::E, 0)),
            'f' => Some((Pitch::F, 0)),
            'g' => Some((Pitch::G, 0)),
            'h' => Some((Pitch::A, 0)),
            'j' => Some((Pitch::B, 0)),
            'k' => Some((Pitch::C, 1)), // C in the next octave
            // Upper row — black keys
            'w' => Some((Pitch::CSharp, 0)),
            'e' => Some((Pitch::DSharp, 0)),
            't' => Some((Pitch::FSharp, 0)),
            'y' => Some((Pitch::GSharp, 0)),
            'u' => Some((Pitch::ASharp, 0)),
            _ => None,
        }
    }

    /// Enter a note at the current cursor position with an explicit octave.
    /// Only works in Insert mode on the Note sub-column.
    pub fn enter_note_with_octave(&mut self, pitch: Pitch, octave: u8) {
        if !self.is_entry_mode() {
            return;
        }
        self.save_history();
        let note = Note::new(pitch, octave, 100, self.current_instrument);
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::On(note)),
        );
        self.advance_row();
    }

    /// Clamp cursor positions to valid bounds (useful after pattern resize).
    pub fn clamp_cursor(&mut self) {
        let max_row = self.pattern.num_rows().saturating_sub(1);
        let max_ch = self.pattern.num_channels().saturating_sub(1);
        self.cursor_row = self.cursor_row.min(max_row);
        self.cursor_channel = self.cursor_channel.min(max_ch);
    }

    // --- Clipboard Operations ---

    /// Copy the current cell (Normal mode) or visual selection (Visual mode) to the clipboard.
    pub fn copy(&mut self) {
        if self.mode == EditorMode::Visual {
            if let Some(((r0, c0), (r1, c1))) = self.visual_selection() {
                let mut rows = Vec::new();
                for r in r0..=r1 {
                    let mut row = Vec::new();
                    for c in c0..=c1 {
                        let cell = self
                            .pattern
                            .get_cell(r, c)
                            .cloned()
                            .unwrap_or_else(Cell::empty);
                        row.push(cell);
                    }
                    rows.push(row);
                }
                self.clipboard = Some(Clipboard::new(rows));
            }
        } else {
            // Copy single cell at cursor
            let cell = self
                .pattern
                .get_cell(self.cursor_row, self.cursor_channel)
                .cloned()
                .unwrap_or_else(Cell::empty);
            self.clipboard = Some(Clipboard::single(cell));
        }
    }

    /// Paste clipboard contents at the current cursor position.
    pub fn paste(&mut self) {
        let clipboard = match &self.clipboard {
            Some(cb) if !cb.is_empty() => cb.clone(),
            _ => return,
        };
        self.save_history();
        let (num_rows, num_cols) = clipboard.dimensions();
        for dr in 0..num_rows {
            for dc in 0..num_cols {
                let target_row = self.cursor_row + dr;
                let target_ch = self.cursor_channel + dc;
                if target_row < self.pattern.num_rows() && target_ch < self.pattern.num_channels() {
                    self.pattern
                        .set_cell(target_row, target_ch, clipboard.cells()[dr][dc].clone());
                }
            }
        }
    }

    /// Cut: copy the selection to clipboard and clear the source cells.
    /// In Visual mode, copies and clears the selection. Otherwise copies and clears the current cell.
    pub fn cut(&mut self) {
        self.copy();
        self.save_history();
        if self.mode == EditorMode::Visual {
            if let Some(((r0, c0), (r1, c1))) = self.visual_selection() {
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        self.pattern.clear_cell(r, c);
                    }
                }
            }
        } else {
            self.pattern
                .clear_cell(self.cursor_row, self.cursor_channel);
        }
    }

    /// Transpose notes in the visual selection by a number of semitones.
    /// Only affects cells that contain NoteEvent::On events.
    /// If any note would go out of range, it is left unchanged.
    pub fn transpose_selection(&mut self, semitones: i32) {
        let selection = if self.mode == EditorMode::Visual {
            self.visual_selection()
        } else {
            // Single cell at cursor
            Some((
                (self.cursor_row, self.cursor_channel),
                (self.cursor_row, self.cursor_channel),
            ))
        };

        let ((r0, c0), (r1, c1)) = match selection {
            Some(s) => s,
            None => return,
        };

        self.save_history();
        for r in r0..=r1 {
            for c in c0..=c1 {
                if let Some(cell) = self.pattern.get_cell(r, c).cloned() {
                    if let Some(NoteEvent::On(note)) = cell.note {
                        if let Some(transposed) = note.transpose(semitones) {
                            let new_cell = Cell {
                                note: Some(NoteEvent::On(transposed)),
                                ..cell
                            };
                            self.pattern.set_cell(r, c, new_cell);
                        }
                    }
                }
            }
        }
    }

    /// Interpolate volume values in the visual selection.
    ///
    /// For each channel column in the selection, finds the first and last cells
    /// that have volume values, then fills intermediate cells with linearly
    /// interpolated volume values.
    pub fn interpolate(&mut self) {
        let selection = if self.mode == EditorMode::Visual {
            self.visual_selection()
        } else {
            return; // Interpolation only makes sense with a selection
        };

        let ((r0, c0), (r1, c1)) = match selection {
            Some(s) if s.0 .0 != s.1 .0 => s, // need at least 2 rows
            _ => return,
        };

        self.save_history();

        for c in c0..=c1 {
            // Find first and last volume values in this column
            let mut first_vol: Option<(usize, u8)> = None;
            let mut last_vol: Option<(usize, u8)> = None;

            for r in r0..=r1 {
                if let Some(cell) = self.pattern.get_cell(r, c) {
                    if let Some(vol) = cell.volume {
                        if first_vol.is_none() {
                            first_vol = Some((r, vol));
                        }
                        last_vol = Some((r, vol));
                    }
                }
            }

            // Need both endpoints to interpolate
            let (start_row, start_val) = match first_vol {
                Some(v) => v,
                None => continue,
            };
            let (end_row, end_val) = match last_vol {
                Some(v) if v.0 > start_row => v,
                _ => continue,
            };

            let span = (end_row - start_row) as f64;
            for r in start_row..=end_row {
                let t = (r - start_row) as f64 / span;
                let interpolated = start_val as f64 + t * (end_val as f64 - start_val as f64);
                let vol = interpolated.round() as u8;
                if let Some(cell) = self.pattern.get_cell(r, c).cloned() {
                    let new_cell = Cell {
                        volume: Some(vol),
                        ..cell
                    };
                    self.pattern.set_cell(r, c, new_cell);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
        // at Note sub-column, moving left wraps to Effect of previous channel
        editor.move_left();
        assert_eq!(editor.cursor_channel(), 1);
        assert_eq!(editor.sub_column(), SubColumn::Effect);
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
        editor.sub_column = SubColumn::Effect;
        editor.move_right();
        assert_eq!(editor.cursor_channel(), 1);
        assert_eq!(editor.sub_column(), SubColumn::Note);
    }

    #[test]
    fn test_move_right_at_max_channel_effect() {
        let mut editor = test_editor();
        editor.cursor_channel = 3;
        editor.sub_column = SubColumn::Effect;
        editor.move_right();
        assert_eq!(editor.cursor_channel(), 3);
        assert_eq!(editor.sub_column(), SubColumn::Effect);
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
        editor.sub_column = SubColumn::Effect;
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
        assert_eq!(editor.sub_column(), SubColumn::Effect);
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
        assert_eq!(SubColumn::Effect.next(), SubColumn::Note);
    }

    #[test]
    fn test_sub_column_prev_cycle() {
        assert_eq!(SubColumn::Note.prev(), SubColumn::Effect);
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
        // Set volume at row 0 = 0, row 4 = 100
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
                volume: Some(100),
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
        assert_eq!(editor.pattern().get_cell(1, 0).unwrap().volume, Some(25));
        assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(50));
        assert_eq!(editor.pattern().get_cell(3, 0).unwrap().volume, Some(75));
        assert_eq!(editor.pattern().get_cell(4, 0).unwrap().volume, Some(100));
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
                volume: Some(100),
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
                volume: Some(100),
                ..Cell::empty()
            },
        );
        editor.cursor_row = 0;
        editor.enter_visual_mode();
        editor.cursor_row = 4;
        editor.interpolate();
        assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(50));
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
                volume: Some(120),
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
        assert_eq!(editor.pattern().get_cell(0, 0).unwrap().volume, Some(120));
        assert_eq!(editor.pattern().get_cell(1, 0).unwrap().volume, Some(80));
        assert_eq!(editor.pattern().get_cell(2, 0).unwrap().volume, Some(40));
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
}
