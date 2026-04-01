use super::*;

impl Editor {
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
    pub(crate) fn advance_row(&mut self) {
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

    /// Set the visual anchor position directly (used during mouse drag selection).
    pub fn set_visual_anchor(&mut self, row: usize, channel: usize) {
        self.visual_anchor = Some((row, channel));
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
}
