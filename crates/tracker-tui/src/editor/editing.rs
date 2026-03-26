use super::*;

impl Editor {
    // --- Editing Operations ---

    /// Save a snapshot to the undo history before making an edit.
    pub(crate) fn save_history(&mut self) {
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

    /// Set cursor position from mouse coordinates within the pattern grid.
    ///
    /// `mouse_y` is the row offset within the visible pattern area (excluding header).
    /// `mouse_x` is the column offset within the pattern area (excluding row number).
    /// `scroll_offset` is the current vertical scroll position.
    /// `channel_scroll` is the current horizontal channel scroll position.
    /// Returns the actual row and channel the cursor was placed at.
    pub fn set_cursor_from_mouse(
        &mut self,
        mouse_y: u16,
        mouse_x: u16,
        scroll_offset: usize,
        channel_scroll: usize,
    ) -> (usize, usize) {
        let row_offset = mouse_y as usize;
        let new_row = scroll_offset.saturating_add(row_offset);
        self.cursor_row = new_row.min(self.pattern.num_rows().saturating_sub(1));
        self.cursor_row = self.cursor_row.max(0);

        let col_offset = mouse_x.saturating_sub(ROW_NUM_WIDTH);
        let channel_offset = (col_offset / CHANNEL_COL_WIDTH) as usize;
        let new_channel = channel_scroll.saturating_add(channel_offset);
        self.cursor_channel = new_channel.min(self.pattern.num_channels().saturating_sub(1));
        self.cursor_channel = self.cursor_channel.max(0);

        (self.cursor_row, self.cursor_channel)
    }

    /// Set cursor position directly to specific row and channel.
    pub fn set_cursor(&mut self, row: usize, channel: usize) {
        self.cursor_row = row.min(self.pattern.num_rows().saturating_sub(1));
        self.cursor_channel = channel.min(self.pattern.num_channels().saturating_sub(1));
    }

}
