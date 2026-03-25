use super::App;

impl App {
    /// Move arrangement view cursor up.
    pub fn arrangement_selection_up(&mut self) {
        self.arrangement_view.move_up();
    }

    /// Move arrangement view cursor down.
    pub fn arrangement_selection_down(&mut self) {
        self.arrangement_view.move_down(self.song.arrangement.len());
    }

    /// Add the currently selected pattern to the arrangement at the current cursor position.
    pub fn arrangement_add_at_cursor(&mut self) {
        if let Some(idx) = self.pattern_selection {
            self.arrangement_view.append_pattern(&mut self.song, idx);
            self.mark_dirty();
        } else if !self.song.patterns.is_empty() {
            // Default to pattern 0 if no selection
            self.arrangement_view.append_pattern(&mut self.song, 0);
            self.mark_dirty();
        }
    }

    /// Delete the arrangement entry at the current cursor position.
    pub fn arrangement_delete_at_cursor(&mut self) {
        if self.arrangement_view.remove_at_cursor(&mut self.song) {
            self.mark_dirty();
        }
    }

    /// Create a new empty pattern and insert it into the arrangement.
    pub fn arrangement_create_pattern(&mut self) {
        if let Some(idx) = self.arrangement_view.create_new_pattern(&mut self.song) {
            self.pattern_selection = Some(idx);
            self.mark_dirty();
        }
    }

    /// Change the pattern index at the current arrangement cursor (typed hex digits).
    pub fn arrangement_set_pattern_digit(&mut self, digit: u8) {
        let cursor = self.arrangement_view.cursor();
        if let Some(entry) = self.song.arrangement.get_mut(cursor) {
            // Hex entry: shift left 4 bits and add new digit, mask to 8 bits (max 255 patterns)
            *entry = ((*entry << 4) | (digit as usize)) & 0xFF;

            // Ensure the pattern exists, if not, clamp to max available
            if *entry >= self.song.patterns.len() {
                *entry = self.song.patterns.len().saturating_sub(1);
            }
            self.mark_dirty();
        }
    }

    /// Jump to the next pattern in the arrangement
    pub fn jump_next_pattern(&mut self) {
        self.transport
            .set_arrangement_length(self.song.arrangement.len());
        let current = self.transport.arrangement_position();
        let next = current + 1;
        if next < self.song.arrangement.len() {
            self.flush_editor_pattern(current);
            self.transport.jump_to_arrangement_position(next);
            self.load_arrangement_pattern(next);
        }
    }

    /// Jump to the previous pattern in the arrangement
    pub fn jump_prev_pattern(&mut self) {
        self.transport
            .set_arrangement_length(self.song.arrangement.len());
        let current = self.transport.arrangement_position();
        if current > 0 {
            let prev = current - 1;
            self.flush_editor_pattern(current);
            self.transport.jump_to_arrangement_position(prev);
            self.load_arrangement_pattern(prev);
        }
    }

    /// Jump to the very beginning of the song (Pattern 0, Row 0).
    pub fn jump_to_start(&mut self) {
        let current = self.transport.arrangement_position();
        self.flush_editor_pattern(current);
        self.transport.jump_to_arrangement_position(0);
        self.load_arrangement_pattern(0);
        self.editor.go_to_row(0);
    }

    /// Jump to the very end of the song (Last pattern in arrangement, last row).
    pub fn jump_to_end(&mut self) {
        let current = self.transport.arrangement_position();
        let last_pos = self.song.arrangement.len().saturating_sub(1);
        self.flush_editor_pattern(current);
        self.transport.jump_to_arrangement_position(last_pos);
        self.load_arrangement_pattern(last_pos);
        self.editor.go_to_row(usize::MAX);
    }
}
