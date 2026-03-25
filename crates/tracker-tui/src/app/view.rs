use super::{App, AppView};
use crate::ui::code_editor;

impl App {
    /// Switch to a different top-level view.
    pub fn set_view(&mut self, view: AppView) {
        // Always start code editor in Normal mode when entering/leaving it
        self.code_editor.mode = code_editor::ModeKind::Normal;
        self.current_view = view;
        // When switching to CodeEditor view, activate the code editor
        self.code_editor.active = view == AppView::CodeEditor;
    }

    /// Toggle split view mode (pattern left, code editor right).
    pub fn toggle_split_view(&mut self) {
        self.split_view = !self.split_view;
        if self.split_view {
            self.code_editor.active = true;
            // Ensure we're in pattern editor view for the split
            if self.current_view == AppView::CodeEditor {
                self.current_view = AppView::PatternEditor;
            }
        } else {
            self.code_editor.active = false;
        }
    }

    /// Check if the code editor is active (either full-screen or split).
    pub fn is_code_editor_active(&self) -> bool {
        self.code_editor.active
    }

    /// Toggle instrument mini panel in the main view.
    pub fn toggle_instrument_mini_panel(&mut self) {
        self.instrument_mini_panel = !self.instrument_mini_panel;
    }

    /// Toggle instrument expanded view (full-screen deep editing).
    pub fn toggle_instrument_expanded(&mut self) {
        self.instrument_expanded = !self.instrument_expanded;
    }

    /// Reset horizontal view to the leftmost channel.
    pub fn reset_horizontal_view(&mut self) {
        self.channel_scroll = 0;
    }

    /// Adjust `channel_scroll` so that the editor cursor's channel is always
    /// visible within the available terminal width.
    ///
    /// `term_width` should be the full terminal width in columns. Borders and
    /// the row-number gutter are accounted for internally.
    pub fn ensure_cursor_visible_horizontally(&mut self, term_width: u16) {
        use crate::ui::{CHANNEL_COL_WIDTH, ROW_NUM_WIDTH};

        // Account for the pattern block's 2-column border + row number column
        let inner_width = term_width.saturating_sub(2); // 1 border each side
        let channel_space = inner_width.saturating_sub(ROW_NUM_WIDTH);
        let visible_channels = ((channel_space / CHANNEL_COL_WIDTH) as usize)
            .max(1)
            .min(self.editor.pattern().num_channels());

        if visible_channels == 0 {
            return;
        }

        let cursor = self.editor.cursor_channel();
        let num_channels = self.editor.pattern().num_channels();

        // Clamp scroll so it never goes past the last page
        let max_scroll = num_channels.saturating_sub(visible_channels);

        if cursor < self.channel_scroll {
            // Cursor went left of view — snap scroll left
            self.channel_scroll = cursor;
        } else if cursor >= self.channel_scroll + visible_channels {
            // Cursor went right of view — snap scroll right
            self.channel_scroll = cursor + 1 - visible_channels;
        }

        self.channel_scroll = self.channel_scroll.min(max_scroll);
    }

    /// Scroll the view one channel to the right without moving the cursor,
    /// used in follow mode where `h`/`l` pan the view, not the cursor.
    pub fn scroll_view_right(&mut self, term_width: u16) {
        use crate::ui::{CHANNEL_COL_WIDTH, ROW_NUM_WIDTH};

        let inner_width = term_width.saturating_sub(2);
        let channel_space = inner_width.saturating_sub(ROW_NUM_WIDTH);
        let visible_channels = ((channel_space / CHANNEL_COL_WIDTH) as usize).max(1);
        let num_channels = self.editor.pattern().num_channels();
        let max_scroll = num_channels.saturating_sub(visible_channels);
        self.channel_scroll = (self.channel_scroll + 1).min(max_scroll);
    }

    /// Scroll the view one channel to the left without moving the cursor,
    /// used in follow mode where `h`/`l` pan the view, not the cursor.
    pub fn scroll_view_left(&mut self) {
        self.channel_scroll = self.channel_scroll.saturating_sub(1);
    }
}
