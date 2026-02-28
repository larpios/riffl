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
}
