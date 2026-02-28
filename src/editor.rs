use crate::pattern::Pattern;

/// The main editor state, managing the pattern data and cursor position
#[derive(Debug)]
pub struct Editor {
    /// The pattern being edited
    pattern: Pattern,
    /// Current cursor row position
    current_row: usize,
    /// Current cursor column (channel) position
    current_col: usize,
}

impl Editor {
    /// Create a new editor with a pattern of specified dimensions
    pub fn new(num_rows: usize, num_channels: usize) -> Self {
        Self {
            pattern: Pattern::new(num_rows, num_channels),
            current_row: 0,
            current_col: 0,
        }
    }

    /// Get the current row position
    pub fn current_row(&self) -> usize {
        self.current_row
    }

    /// Get the current column (channel) position
    pub fn current_col(&self) -> usize {
        self.current_col
    }

    /// Get a reference to the pattern
    pub fn pattern(&self) -> &Pattern {
        &self.pattern
    }

    /// Get a mutable reference to the pattern
    pub fn pattern_mut(&mut self) -> &mut Pattern {
        &mut self.pattern
    }

    /// Move cursor up by one row (k or up arrow)
    /// Returns true if the cursor moved, false if already at top
    pub fn move_up(&mut self) -> bool {
        if self.current_row > 0 {
            self.current_row -= 1;
            true
        } else {
            false
        }
    }

    /// Move cursor down by one row (j or down arrow)
    /// Returns true if the cursor moved, false if already at bottom
    pub fn move_down(&mut self) -> bool {
        if self.current_row + 1 < self.pattern.num_rows() {
            self.current_row += 1;
            true
        } else {
            false
        }
    }

    /// Move cursor left by one column (h or left arrow)
    /// Returns true if the cursor moved, false if already at leftmost
    pub fn move_left(&mut self) -> bool {
        if self.current_col > 0 {
            self.current_col -= 1;
            true
        } else {
            false
        }
    }

    /// Move cursor right by one column (l or right arrow)
    /// Returns true if the cursor moved, false if already at rightmost
    pub fn move_right(&mut self) -> bool {
        if self.current_col + 1 < self.pattern.num_channels() {
            self.current_col += 1;
            true
        } else {
            false
        }
    }

    /// Enter a note at the current cursor position
    /// Returns true if the note was entered successfully, false otherwise
    pub fn enter_note(&mut self, note: crate::pattern::Note) -> bool {
        self.pattern.set_note(self.current_row, self.current_col, Some(note))
    }

    /// Delete (clear) the note at the current cursor position
    pub fn delete_note(&mut self) {
        self.pattern.clear_note(self.current_row, self.current_col);
    }

    /// Insert a new empty row at the current cursor position, pushing existing rows down
    pub fn insert_row(&mut self) {
        self.pattern.insert_row(self.current_row);
    }

    /// Delete the row at the current cursor position
    /// Returns true if the row was deleted, false if it was the last row or out of bounds
    pub fn delete_row(&mut self) -> bool {
        // Don't allow deleting if it's the only row left
        if self.pattern.num_rows() <= 1 {
            return false;
        }

        let result = self.pattern.delete_row(self.current_row);

        // Adjust cursor if we deleted the last row
        if result && self.current_row >= self.pattern.num_rows() {
            self.current_row = self.pattern.num_rows().saturating_sub(1);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = Editor::new(16, 4);
        assert_eq!(editor.current_row(), 0);
        assert_eq!(editor.current_col(), 0);
        assert_eq!(editor.pattern().num_rows(), 16);
        assert_eq!(editor.pattern().num_channels(), 4);
    }

    #[test]
    fn test_editor_pattern_access() {
        let mut editor = Editor::new(8, 2);
        let pattern = editor.pattern_mut();
        assert_eq!(pattern.num_rows(), 8);
        assert_eq!(pattern.num_channels(), 2);
    }

    #[test]
    fn test_editor_default_cursor() {
        let editor = Editor::new(32, 8);
        assert_eq!(editor.current_row(), 0);
        assert_eq!(editor.current_col(), 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut editor = Editor::new(16, 4);

        // Test initial position
        assert_eq!(editor.current_row(), 0);
        assert_eq!(editor.current_col(), 0);

        // Test move down
        assert!(editor.move_down());
        assert_eq!(editor.current_row(), 1);

        // Test move right
        assert!(editor.move_right());
        assert_eq!(editor.current_col(), 1);

        // Test move up
        assert!(editor.move_up());
        assert_eq!(editor.current_row(), 0);

        // Test move left
        assert!(editor.move_left());
        assert_eq!(editor.current_col(), 0);

        // Test boundaries - can't move up from row 0
        assert!(!editor.move_up());
        assert_eq!(editor.current_row(), 0);

        // Test boundaries - can't move left from col 0
        assert!(!editor.move_left());
        assert_eq!(editor.current_col(), 0);

        // Move to bottom-right corner
        for _ in 0..15 {
            editor.move_down();
        }
        for _ in 0..3 {
            editor.move_right();
        }
        assert_eq!(editor.current_row(), 15);
        assert_eq!(editor.current_col(), 3);

        // Test boundaries - can't move down from last row
        assert!(!editor.move_down());
        assert_eq!(editor.current_row(), 15);

        // Test boundaries - can't move right from last column
        assert!(!editor.move_right());
        assert_eq!(editor.current_col(), 3);
    }

    #[test]
    fn test_cursor_movement_small_pattern() {
        let mut editor = Editor::new(1, 1);

        // With a 1x1 pattern, no movement should be possible
        assert!(!editor.move_up());
        assert!(!editor.move_down());
        assert!(!editor.move_left());
        assert!(!editor.move_right());

        assert_eq!(editor.current_row(), 0);
        assert_eq!(editor.current_col(), 0);
    }

    #[test]
    fn test_cursor_movement_vertical_only() {
        let mut editor = Editor::new(8, 1);

        // Can move down
        assert!(editor.move_down());
        assert_eq!(editor.current_row(), 1);

        // Cannot move left or right with only 1 channel
        assert!(!editor.move_left());
        assert!(!editor.move_right());
        assert_eq!(editor.current_col(), 0);
    }

    #[test]
    fn test_cursor_movement_horizontal_only() {
        let mut editor = Editor::new(1, 8);

        // Can move right
        assert!(editor.move_right());
        assert_eq!(editor.current_col(), 1);

        // Cannot move up or down with only 1 row
        assert!(!editor.move_up());
        assert!(!editor.move_down());
        assert_eq!(editor.current_row(), 0);
    }

    #[test]
    fn test_edit_operations() {
        use crate::pattern::{Note, Pitch};

        let mut editor = Editor::new(8, 4);

        // Test note entry at cursor position (0, 0)
        let note = Note::new(Pitch::C, 4);
        assert!(editor.enter_note(note));

        // Verify note was entered
        let retrieved = editor.pattern().get_note(0, 0);
        assert!(retrieved.is_some());
        assert_eq!(*retrieved.unwrap(), Some(note));

        // Move cursor and enter another note
        editor.move_down();
        editor.move_right();
        let note2 = Note::new(Pitch::A, 5);
        assert!(editor.enter_note(note2));
        assert_eq!(editor.current_row(), 1);
        assert_eq!(editor.current_col(), 1);

        let retrieved2 = editor.pattern().get_note(1, 1);
        assert!(retrieved2.is_some());
        assert_eq!(*retrieved2.unwrap(), Some(note2));

        // Test delete note
        editor.delete_note();
        let retrieved3 = editor.pattern().get_note(1, 1);
        assert!(retrieved3.is_some());
        assert_eq!(*retrieved3.unwrap(), None);

        // Test insert row
        let initial_rows = editor.pattern().num_rows();
        editor.insert_row();
        assert_eq!(editor.pattern().num_rows(), initial_rows + 1);

        // Verify the row was inserted at the cursor position
        // The note at (0, 0) should still be there
        let note_at_0_0 = editor.pattern().get_note(0, 0);
        assert!(note_at_0_0.is_some());
        assert_eq!(*note_at_0_0.unwrap(), Some(note));

        // Test delete row
        let current_rows = editor.pattern().num_rows();
        assert!(editor.delete_row());
        assert_eq!(editor.pattern().num_rows(), current_rows - 1);
    }

    #[test]
    fn test_delete_note_at_different_positions() {
        use crate::pattern::{Note, Pitch};

        let mut editor = Editor::new(4, 4);

        // Enter notes at multiple positions
        editor.enter_note(Note::new(Pitch::C, 4));
        editor.move_right();
        editor.enter_note(Note::new(Pitch::D, 4));
        editor.move_down();
        editor.enter_note(Note::new(Pitch::E, 4));

        // Delete the note at current position (1, 1)
        editor.delete_note();
        let note = editor.pattern().get_note(1, 1);
        assert!(note.is_some());
        assert!(note.unwrap().is_none());

        // Other notes should still exist
        let note1 = editor.pattern().get_note(0, 0);
        assert!(note1.is_some());
        assert!(note1.unwrap().is_some());

        let note2 = editor.pattern().get_note(0, 1);
        assert!(note2.is_some());
        assert!(note2.unwrap().is_some());
    }

    #[test]
    fn test_insert_row_at_cursor() {
        use crate::pattern::{Note, Pitch};

        let mut editor = Editor::new(4, 2);

        // Add a note at row 1
        editor.move_down();
        let note = Note::new(Pitch::G, 5);
        editor.enter_note(note);

        // Move to row 1 and insert a row
        let initial_rows = editor.pattern().num_rows();
        editor.insert_row();

        assert_eq!(editor.pattern().num_rows(), initial_rows + 1);

        // The original note should now be at row 2 (pushed down)
        let note_at_2 = editor.pattern().get_note(2, 0);
        assert!(note_at_2.is_some());
        assert_eq!(*note_at_2.unwrap(), Some(note));

        // The newly inserted row at position 1 should be empty
        let note_at_1 = editor.pattern().get_note(1, 0);
        assert!(note_at_1.is_some());
        assert!(note_at_1.unwrap().is_none());
    }

    #[test]
    fn test_delete_row_prevents_last_row_deletion() {
        let mut editor = Editor::new(1, 4);

        // Cannot delete the only row
        assert!(!editor.delete_row());
        assert_eq!(editor.pattern().num_rows(), 1);
    }

    #[test]
    fn test_delete_row_cursor_adjustment() {
        let mut editor = Editor::new(4, 2);

        // Move to the last row (row 3)
        for _ in 0..3 {
            editor.move_down();
        }
        assert_eq!(editor.current_row(), 3);

        // Delete the last row
        assert!(editor.delete_row());
        assert_eq!(editor.pattern().num_rows(), 3);

        // Cursor should be adjusted to row 2 (last valid row)
        assert_eq!(editor.current_row(), 2);
    }

    #[test]
    fn test_enter_note_with_velocity_and_instrument() {
        use crate::pattern::{Note, Pitch};

        let mut editor = Editor::new(4, 2);

        let note = Note::with_all(Pitch::B, 3, 100, 2);
        assert!(editor.enter_note(note));

        let retrieved = editor.pattern().get_note(0, 0);
        assert!(retrieved.is_some());
        let retrieved_note = retrieved.unwrap().as_ref().unwrap();
        assert_eq!(retrieved_note.pitch, Pitch::B);
        assert_eq!(retrieved_note.octave, 3);
        assert_eq!(retrieved_note.velocity, Some(100));
        assert_eq!(retrieved_note.instrument, Some(2));
    }
}
