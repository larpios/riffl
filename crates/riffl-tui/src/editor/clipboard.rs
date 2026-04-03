use super::*;

impl Editor {
    // --- Clipboard Operations ---

    /// Copy the current cell (Normal mode) or visual selection (Visual/VisualLine mode) to the
    /// active register ('0' by default, or whatever was set with `set_active_register`).
    /// The active register is reset to '0' after the operation.
    pub fn copy(&mut self) {
        let cb = if self.mode.is_visual() {
            if let Some(((r0, c0), (r1, c1))) = self.visual_selection() {
                let mut rows = Vec::new();
                for r in r0..=r1 {
                    let mut row = Vec::new();
                    for c in c0..=c1 {
                        let cell = self
                            .pattern
                            .get_cell(r, c)
                            .cloned()
                            .unwrap_or_else(Cell::empty);
                        row.push(cell);
                    }
                    rows.push(row);
                }
                Some(Clipboard::new(rows))
            } else {
                None
            }
        } else {
            // Copy single cell at cursor
            let cell = self
                .pattern
                .get_cell(self.cursor_row, self.cursor_channel)
                .cloned()
                .unwrap_or_else(Cell::empty);
            Some(Clipboard::single(cell))
        };
        if let Some(cb) = cb {
            let reg = self.active_register;
            self.registers.insert(reg, cb);
        }
        self.active_register = '0';
    }

    /// Paste the active register contents at the current cursor position.
    /// The active register is reset to '0' after the operation.
    pub fn paste(&mut self) {
        let reg = self.active_register;
        self.active_register = '0';
        let clipboard = match self.registers.get(&reg) {
            Some(cb) if !cb.is_empty() => cb.clone(),
            _ => return,
        };
        self.save_history();
        let (num_rows, num_cols) = clipboard.dimensions();
        for dr in 0..num_rows {
            for dc in 0..num_cols {
                let target_row = self.cursor_row + dr;
                let target_ch = self.cursor_channel + dc;
                if target_row < self.pattern.num_rows() && target_ch < self.pattern.num_channels() {
                    self.pattern
                        .set_cell(target_row, target_ch, clipboard.cells()[dr][dc].clone());
                }
            }
        }
    }

    /// Cut: copy selection to the active register and clear the source cells.
    /// In Visual/VisualLine mode, copies and clears the selection. Otherwise copies and clears the
    /// current cell. The active register is reset to '0' after the operation.
    pub fn cut(&mut self) {
        self.copy();
        self.save_history();
        if self.mode.is_visual() {
            if let Some(((r0, c0), (r1, c1))) = self.visual_selection() {
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        self.pattern.clear_cell(r, c);
                    }
                }
            }
        } else {
            self.pattern
                .clear_cell(self.cursor_row, self.cursor_channel);
        }
    }

    /// Transpose notes in the visual selection by a number of semitones.
    /// Only affects cells that contain NoteEvent::On events.
    /// If any note would go out of range, it is left unchanged.
    pub fn transpose_selection(&mut self, semitones: i32) {
        let selection = if self.mode.is_visual() {
            self.visual_selection()
        } else {
            // Single cell at cursor
            Some((
                (self.cursor_row, self.cursor_channel),
                (self.cursor_row, self.cursor_channel),
            ))
        };

        let ((r0, c0), (r1, c1)) = match selection {
            Some(s) => s,
            None => return,
        };

        self.save_history();
        for r in r0..=r1 {
            for c in c0..=c1 {
                if let Some(cell) = self.pattern.get_cell(r, c).cloned() {
                    if let Some(NoteEvent::On(note)) = cell.note {
                        if let Some(transposed) = note.transpose(semitones) {
                            let new_cell = Cell {
                                note: Some(NoteEvent::On(transposed)),
                                ..cell
                            };
                            self.pattern.set_cell(r, c, new_cell);
                        }
                    }
                }
            }
        }
    }

    /// Fill the visual selection (or current channel if no selection) with the given note.
    ///
    /// Fills every row in the selection range for each selected channel with `note`.
    /// Pushes an undo snapshot before making changes.
    pub fn fill_selection_with_note(&mut self, note: riffl_core::pattern::note::NoteEvent) {
        let ((r0, c0), (r1, c1)) = if self.mode.is_visual() {
            match self.visual_selection() {
                Some(s) => s,
                None => return,
            }
        } else {
            let c = self.cursor_channel;
            let max_r = self.pattern.num_rows().saturating_sub(1);
            ((0, c), (max_r, c))
        };

        self.save_history();
        for row in r0..=r1 {
            for ch in c0..=c1 {
                self.pattern
                    .set_cell(row, ch, riffl_core::pattern::row::Cell::with_note(note));
            }
        }
    }

    /// Randomize the pitch of all notes in the visual selection (or current cell).
    ///
    /// Each cell that contains a NoteEvent::On has its pitch replaced with a randomly
    /// chosen pitch from the chromatic scale. Octave and other note properties are
    /// preserved. Uses a simple LCG seeded from the current row position for
    /// reproducibility within a session.
    pub fn randomize_notes(&mut self) {
        let ((r0, c0), (r1, c1)) = if self.mode.is_visual() {
            match self.visual_selection() {
                Some(s) => s,
                None => return,
            }
        } else {
            let r = self.cursor_row;
            let c = self.cursor_channel;
            ((r, c), (r, c))
        };

        self.save_history();

        // Simple LCG pseudo-random seeded from cursor position to avoid rand dependency
        let mut state: u64 = (r0 as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add((c0 as u64).wrapping_mul(1442695040888963407))
            .wrapping_add(1);
        let mut next_pitch = || -> riffl_core::pattern::note::Pitch {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let idx = ((state >> 33) as usize) % 12;
            riffl_core::pattern::note::Pitch::ALL[idx]
        };

        for row in r0..=r1 {
            for ch in c0..=c1 {
                if let Some(cell) = self.pattern.get_cell(row, ch).cloned() {
                    if let Some(riffl_core::pattern::note::NoteEvent::On(note)) = cell.note {
                        let new_pitch = next_pitch();
                        let new_note = riffl_core::pattern::note::Note::new(
                            new_pitch,
                            note.octave,
                            note.velocity,
                            note.instrument,
                        );
                        let new_cell = riffl_core::pattern::row::Cell {
                            note: Some(riffl_core::pattern::note::NoteEvent::On(new_note)),
                            ..cell
                        };
                        self.pattern.set_cell(row, ch, new_cell);
                    }
                }
            }
        }
    }

    /// Interpolate volume values in the visual selection.
    ///
    /// For each channel column in the selection, finds the first and last cells
    /// that have volume values, then fills intermediate cells with linearly
    /// interpolated volume values.
    pub fn interpolate(&mut self) {
        let selection = if self.mode.is_visual() {
            self.visual_selection()
        } else {
            return; // Interpolation only makes sense with a selection
        };

        let ((r0, c0), (r1, c1)) = match selection {
            Some(s) if s.0 .0 != s.1 .0 => s, // need at least 2 rows
            _ => return,
        };

        self.save_history();

        for c in c0..=c1 {
            // Find first and last volume values in this column
            let mut first_vol: Option<(usize, u8)> = None;
            let mut last_vol: Option<(usize, u8)> = None;

            for r in r0..=r1 {
                if let Some(cell) = self.pattern.get_cell(r, c) {
                    if let Some(vol) = cell.volume {
                        if first_vol.is_none() {
                            first_vol = Some((r, vol));
                        }
                        last_vol = Some((r, vol));
                    }
                }
            }

            // Need both endpoints to interpolate
            let (start_row, start_val) = match first_vol {
                Some(v) => v,
                None => continue,
            };
            let (end_row, end_val) = match last_vol {
                Some(v) if v.0 > start_row => v,
                _ => continue,
            };

            let span = (end_row - start_row) as f64;
            for r in start_row..=end_row {
                let t = (r - start_row) as f64 / span;
                let interpolated = start_val as f64 + t * (end_val as f64 - start_val as f64);
                let vol = interpolated.round() as u8;
                if let Some(cell) = self.pattern.get_cell(r, c).cloned() {
                    let new_cell = Cell {
                        volume: Some(vol),
                        ..cell
                    };
                    self.pattern.set_cell(r, c, new_cell);
                }
            }
        }
    }

    /// Reverse the order of rows in the visual selection.
    ///
    /// Flips the selection upside-down: the last row becomes the first, etc.
    /// Only works in visual mode (requires a selection of at least 2 rows).
    pub fn reverse_selection(&mut self) {
        let selection = if self.mode.is_visual() {
            self.visual_selection()
        } else {
            return;
        };

        let ((r0, c0), (r1, c1)) = match selection {
            Some(s) if s.0 .0 != s.1 .0 => s, // need at least 2 rows
            _ => return,
        };

        self.save_history();

        let rows: Vec<Vec<Cell>> = (r0..=r1)
            .map(|r| {
                (c0..=c1)
                    .map(|c| {
                        self.pattern
                            .get_cell(r, c)
                            .cloned()
                            .unwrap_or_else(Cell::empty)
                    })
                    .collect()
            })
            .collect();

        for (i, r) in (r0..=r1).enumerate() {
            let src = &rows[rows.len() - 1 - i];
            for (j, c) in (c0..=c1).enumerate() {
                self.pattern.set_cell(r, c, src[j].clone());
            }
        }
    }

    /// Add a small random velocity offset (±`amount`) to all NoteEvent::On cells in the selection.
    ///
    /// Velocity is clamped to 1–127. Uses the same LCG PRNG as `randomize_notes`.
    /// Works on the visual selection or the single cell at the cursor.
    pub fn humanize_notes(&mut self, amount: u8) {
        let ((r0, c0), (r1, c1)) = if self.mode.is_visual() {
            match self.visual_selection() {
                Some(s) => s,
                None => return,
            }
        } else {
            let r = self.cursor_row;
            let c = self.cursor_channel;
            ((r, c), (r, c))
        };

        if amount == 0 {
            return;
        }

        self.save_history();

        let mut state: u64 = (r0 as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add((c0 as u64).wrapping_mul(1442695040888963407))
            .wrapping_add(17);

        for row in r0..=r1 {
            for ch in c0..=c1 {
                if let Some(cell) = self.pattern.get_cell(row, ch).cloned() {
                    if let Some(riffl_core::pattern::note::NoteEvent::On(note)) = cell.note {
                        state = state
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        // Offset in range [-amount, +amount]
                        let range = (amount as u64) * 2 + 1;
                        let offset = ((state >> 33) % range) as i32 - amount as i32;
                        let new_vel = (note.velocity as i32 + offset).clamp(1, 127) as u8;
                        let new_note = riffl_core::pattern::note::Note::new(
                            note.pitch,
                            note.octave,
                            new_vel,
                            note.instrument,
                        );
                        let new_cell = Cell {
                            note: Some(riffl_core::pattern::note::NoteEvent::On(new_note)),
                            ..cell
                        };
                        self.pattern.set_cell(row, ch, new_cell);
                    }
                }
            }
        }
    }
}
