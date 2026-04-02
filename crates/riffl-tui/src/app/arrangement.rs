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

    /// Move the arrangement entry at the cursor one position up (swap with predecessor).
    pub fn arrangement_move_entry_up(&mut self) {
        if self.arrangement_view.move_entry_up(&mut self.song) {
            self.mark_dirty();
        }
    }

    /// Move the arrangement entry at the cursor one position down (swap with successor).
    pub fn arrangement_move_entry_down(&mut self) {
        if self.arrangement_view.move_entry_down(&mut self.song) {
            self.mark_dirty();
        }
    }

    /// Clone the pattern at the current arrangement cursor and insert it after the cursor.
    ///
    /// The cloned pattern is a deep copy of the source, inserted immediately after the
    /// current entry. The cursor advances to the new entry, and the editor is flushed
    /// so any unsaved edits are preserved in the source pattern first.
    pub fn arrangement_clone_pattern(&mut self) {
        let cursor = self.arrangement_view.cursor();
        let Some(&pattern_idx) = self.song.arrangement.get(cursor) else {
            return;
        };
        // Flush current editor state into the source pattern before cloning.
        let transport_pos = self.transport.arrangement_position();
        self.flush_editor_pattern(transport_pos);

        if let Some(new_idx) = self.song.duplicate_pattern(pattern_idx) {
            let insert_pos = (cursor + 1).min(self.song.arrangement.len());
            self.song.insert_in_arrangement(insert_pos, new_idx);
            self.arrangement_view
                .clamp_cursor(self.song.arrangement.len());
            // Advance cursor to the new entry
            self.arrangement_view.move_down(self.song.arrangement.len());
            self.pattern_selection = Some(new_idx);
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
            if self.transport.is_playing() {
                self.chase_notes();
            }
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
            if self.transport.is_playing() {
                self.chase_notes();
            }
        }
    }

    /// Jump to the very beginning of the song (Pattern 0, Row 0).
    pub fn jump_to_start(&mut self) {
        let current = self.transport.arrangement_position();
        self.flush_editor_pattern(current);
        self.transport.jump_to_arrangement_position(0);
        self.load_arrangement_pattern(0);
        self.editor.go_to_row(0);
        if self.transport.is_playing() {
            self.chase_notes();
        }
    }

    /// Jump to the very end of the song (Last pattern in arrangement, last row).
    pub fn jump_to_end(&mut self) {
        let current = self.transport.arrangement_position();
        let last_pos = self.song.arrangement.len().saturating_sub(1);
        self.flush_editor_pattern(current);
        self.transport.jump_to_arrangement_position(last_pos);
        self.load_arrangement_pattern(last_pos);
        self.editor.go_to_row(usize::MAX);
        if self.transport.is_playing() {
            self.chase_notes();
        }
    }
}

use crate::editor::Editor;

impl App {
    /// Add or update a section marker at the current arrangement cursor.
    pub fn arrangement_add_marker(&mut self, label: String) {
        let pos = self.arrangement_view.cursor();
        self.song.add_section_marker(pos, label);
        self.mark_dirty();
    }

    /// Remove the section marker at the current arrangement cursor.
    pub fn arrangement_remove_marker(&mut self) {
        let pos = self.arrangement_view.cursor();
        self.song.remove_section_marker(pos);
        self.mark_dirty();
    }
}

impl App {
    pub fn flush_editor_pattern(&mut self, arrangement_pos: usize) {
        if let Some(&pattern_idx) = self.song.arrangement.get(arrangement_pos) {
            if let Some(pattern) = self.song.patterns.get_mut(pattern_idx) {
                *pattern = self.editor.pattern().clone();
            }
        }
    }

    /// Load the pattern at the given arrangement position into the editor.
    /// Syncs global track state into the pattern so mixing settings persist.
    pub fn load_arrangement_pattern(&mut self, arrangement_pos: usize) {
        if let Some(&pattern_idx) = self.song.arrangement.get(arrangement_pos) {
            if let Some(pattern) = self.song.patterns.get(pattern_idx) {
                let mut p = pattern.clone();
                // Sync tracks from song
                for (ch, track) in p.tracks_mut().iter_mut().enumerate() {
                    if let Some(song_track) = self.song.tracks.get(ch) {
                        track.muted = song_track.muted;
                        track.solo = song_track.solo;
                        track.volume = song_track.volume;
                        track.pan = song_track.pan;
                    }
                }
                self.editor = Editor::new(p);
                self.transport.set_num_rows(pattern.num_rows());
            }
        }
    }
}
