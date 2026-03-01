/// Pattern editor state machine
///
/// Provides a vim-inspired modal editor for the tracker pattern grid.
/// Supports Normal (navigation), Insert (note entry), and Visual (selection) modes.

use crate::pattern::note::{Note, NoteEvent, Pitch};
use crate::pattern::pattern::Pattern;
use crate::pattern::row::Cell;

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
}

impl EditorMode {
    /// Display label for the mode.
    pub fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
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
    /// Visual mode anchor (row, channel) — starting position of selection.
    visual_anchor: Option<(usize, usize)>,
}

impl Editor {
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
            visual_anchor: None,
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

    // --- Navigation ---

    /// Move cursor up by one row.
    pub fn move_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(1);
    }

    /// Move cursor down by one row.
    pub fn move_down(&mut self) {
        let max = self.pattern.num_rows().saturating_sub(1);
        if self.cursor_row < max {
            self.cursor_row += 1;
        }
    }

    /// Move cursor left. In Normal mode, moves by channel. In Insert mode,
    /// moves by sub-column first, then wraps to previous channel.
    pub fn move_left(&mut self) {
        if self.mode == EditorMode::Insert {
            if self.sub_column != SubColumn::Note {
                self.sub_column = self.sub_column.prev();
            } else if self.cursor_channel > 0 {
                self.cursor_channel -= 1;
                self.sub_column = SubColumn::Effect;
            }
        } else {
            self.cursor_channel = self.cursor_channel.saturating_sub(1);
        }
    }

    /// Move cursor right. In Normal mode, moves by channel. In Insert mode,
    /// moves by sub-column first, then wraps to next channel.
    pub fn move_right(&mut self) {
        let max_ch = self.pattern.num_channels().saturating_sub(1);
        if self.mode == EditorMode::Insert {
            if self.sub_column != SubColumn::Effect {
                self.sub_column = self.sub_column.next();
            } else if self.cursor_channel < max_ch {
                self.cursor_channel += 1;
                self.sub_column = SubColumn::Note;
            }
        } else if self.cursor_channel < max_ch {
            self.cursor_channel += 1;
        }
    }

    /// Move cursor up by a page (PAGE_SIZE rows).
    pub fn page_up(&mut self) {
        self.cursor_row = self.cursor_row.saturating_sub(PAGE_SIZE);
    }

    /// Move cursor down by a page (PAGE_SIZE rows).
    pub fn page_down(&mut self) {
        let max = self.pattern.num_rows().saturating_sub(1);
        self.cursor_row = (self.cursor_row + PAGE_SIZE).min(max);
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
    }

    // --- Mode Transitions ---

    /// Enter Insert mode.
    pub fn enter_insert_mode(&mut self) {
        self.mode = EditorMode::Insert;
    }

    /// Enter Normal mode (from any mode).
    pub fn enter_normal_mode(&mut self) {
        self.mode = EditorMode::Normal;
        self.visual_anchor = None;
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
    }

    /// Undo the last edit, restoring the previous pattern and cursor.
    pub fn undo(&mut self) -> bool {
        if let Some(entry) = self.history.pop() {
            self.pattern = entry.pattern;
            self.cursor_row = entry.cursor_row;
            self.cursor_channel = entry.cursor_channel;
            true
        } else {
            false
        }
    }

    /// Enter a note pitch at the current cursor position.
    /// Only works in Insert mode on the Note sub-column.
    pub fn enter_note(&mut self, pitch: Pitch) {
        if self.mode != EditorMode::Insert {
            return;
        }
        self.save_history();
        let note = Note::new(pitch, self.current_octave, 100, self.current_instrument);
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::On(note)),
        );
        // Advance cursor down after entering a note
        self.move_down();
    }

    /// Enter a note-off event at the current cursor position.
    pub fn enter_note_off(&mut self) {
        if self.mode != EditorMode::Insert {
            return;
        }
        self.save_history();
        self.pattern.set_cell(
            self.cursor_row,
            self.cursor_channel,
            Cell::with_note(NoteEvent::Off),
        );
        self.move_down();
    }

    /// Set the current octave (0-9). Used when typing a digit in Insert mode.
    pub fn set_octave(&mut self, octave: u8) {
        if octave <= 9 {
            self.current_octave = octave;
        }
    }

    /// Delete (clear) the current cell.
    pub fn delete_cell(&mut self) {
        self.save_history();
        self.pattern.clear_cell(self.cursor_row, self.cursor_channel);
    }

    /// Insert a new empty row at the cursor position, pushing rows down.
    pub fn insert_row(&mut self) {
        self.save_history();
        self.pattern.insert_row(self.cursor_row);
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

    /// Parse a character as a note pitch (A-G) for note entry.
    pub fn char_to_pitch(c: char) -> Option<Pitch> {
        match c.to_ascii_uppercase() {
            'C' => Some(Pitch::C),
            'D' => Some(Pitch::D),
            'E' => Some(Pitch::E),
            'F' => Some(Pitch::F),
            'G' => Some(Pitch::G),
            'A' => Some(Pitch::A),
            'B' => Some(Pitch::B),
            _ => None,
        }
    }

    /// Clamp cursor positions to valid bounds (useful after pattern resize).
    pub fn clamp_cursor(&mut self) {
        let max_row = self.pattern.num_rows().saturating_sub(1);
        let max_ch = self.pattern.num_channels().saturating_sub(1);
        self.cursor_row = self.cursor_row.min(max_row);
        self.cursor_channel = self.cursor_channel.min(max_ch);
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
        editor.cursor_channel = 2;
        editor.move_left();
        assert_eq!(editor.cursor_channel(), 1);
    }

    #[test]
    fn test_move_left_at_zero() {
        let mut editor = test_editor();
        editor.move_left();
        assert_eq!(editor.cursor_channel(), 0);
    }

    #[test]
    fn test_move_right_normal_mode() {
        let mut editor = test_editor();
        editor.move_right();
        assert_eq!(editor.cursor_channel(), 1);
    }

    #[test]
    fn test_move_right_at_max_channel() {
        let mut editor = test_editor();
        editor.cursor_channel = 3;
        editor.move_right();
        assert_eq!(editor.cursor_channel(), 3);
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
    fn test_char_to_pitch() {
        assert_eq!(Editor::char_to_pitch('c'), Some(Pitch::C));
        assert_eq!(Editor::char_to_pitch('C'), Some(Pitch::C));
        assert_eq!(Editor::char_to_pitch('G'), Some(Pitch::G));
        assert_eq!(Editor::char_to_pitch('a'), Some(Pitch::A));
        assert_eq!(Editor::char_to_pitch('x'), None);
        assert_eq!(Editor::char_to_pitch('1'), None);
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
        let mut editor = Editor::new(Pattern::new(2, 1));
        editor.enter_insert_mode();
        editor.cursor_row = 1;
        editor.enter_note(Pitch::C);
        // Should stay at last row since move_down can't go past it
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
}
