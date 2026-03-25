use super::App;
use tracker_core::pattern::Pattern;

impl App {
    pub fn pattern_selection(&self) -> Option<usize> {
        self.pattern_selection
    }

    /// Set the selected pattern index.
    pub fn set_pattern_selection(&mut self, index: Option<usize>) {
        self.pattern_selection = index;
    }

    /// Move pattern selection up.
    pub fn pattern_selection_up(&mut self) {
        let count = self.song.patterns.len();
        if count == 0 {
            self.pattern_selection = None;
            return;
        }
        match self.pattern_selection {
            None => self.pattern_selection = Some(count - 1),
            Some(0) => self.pattern_selection = Some(count - 1),
            Some(i) => self.pattern_selection = Some(i - 1),
        }
    }

    /// Move pattern selection down.
    pub fn pattern_selection_down(&mut self) {
        let count = self.song.patterns.len();
        if count == 0 {
            self.pattern_selection = None;
            return;
        }
        match self.pattern_selection {
            None => self.pattern_selection = Some(0),
            Some(i) if i >= count - 1 => self.pattern_selection = Some(0),
            Some(i) => self.pattern_selection = Some(i + 1),
        }
    }

    /// Add a new empty pattern.
    pub fn add_pattern(&mut self) {
        if let Some(idx) = self.song.add_pattern(Pattern::default()) {
            self.pattern_selection = Some(idx);
        }
    }

    /// Delete the selected pattern.
    pub fn delete_pattern(&mut self) -> bool {
        if let Some(idx) = self.pattern_selection {
            if self.song.remove_pattern(idx) {
                // Adjust selection
                if self.song.patterns.is_empty() {
                    // Add a default pattern if none remain
                    self.song.add_pattern(Pattern::default());
                    self.pattern_selection = Some(0);
                } else if idx >= self.song.patterns.len() {
                    self.pattern_selection = Some(self.song.patterns.len() - 1);
                }
                return true;
            }
        }
        false
    }

    /// Duplicate (clone) the selected pattern.
    pub fn duplicate_pattern(&mut self) -> bool {
        if let Some(idx) = self.pattern_selection {
            if let Some(new_idx) = self.song.duplicate_pattern(idx) {
                self.pattern_selection = Some(new_idx);
                return true;
            }
        }
        false
    }

    /// Select pattern for editing (load it into the editor).
    pub fn select_pattern(&mut self) {
        if let Some(idx) = self.pattern_selection {
            if idx < self.song.patterns.len() {
                // Replace editor's pattern with the selected one
                self.editor.set_pattern(self.song.patterns[idx].clone());
                // Update transport to match the new pattern's row count
                self.transport
                    .set_num_rows(self.song.patterns[idx].num_rows());
            }
        }
    }

    /// Toggle mute on the current track (channel under cursor).
    /// Syncs with the global song track state so it persists across pattern changes.
    pub fn toggle_mute_current_track(&mut self) {
        let ch = self.editor.cursor_channel();
        // Toggle in current pattern for immediate feedback
        if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
            track.toggle_mute();
        }
        // Sync to song tracks for persistence
        if let Some(track) = self.song.tracks.get_mut(ch) {
            track.toggle_mute();
        }
        self.mark_dirty();
    }

    /// Toggle solo on the current track (channel under cursor).
    /// Syncs with the global song track state so it persists across pattern changes.
    pub fn toggle_solo_current_track(&mut self) {
        let ch = self.editor.cursor_channel();
        // Toggle in current pattern for immediate feedback
        if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
            track.toggle_solo();
        }
        // Sync to song tracks for persistence
        if let Some(track) = self.song.tracks.get_mut(ch) {
            track.toggle_solo();
        }
        self.mark_dirty();
    }
}
