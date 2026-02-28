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
}
