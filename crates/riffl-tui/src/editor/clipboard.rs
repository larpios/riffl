use super::*;

impl Editor {
    // --- Clipboard Operations ---

    /// Copy the current cell (Normal mode) or visual selection (Visual mode) to the clipboard.
    pub fn copy(&mut self) {
        if self.mode == EditorMode::Visual {
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
                self.clipboard = Some(Clipboard::new(rows));
            }
        } else {
            // Copy single cell at cursor
            let cell = self
                .pattern
                .get_cell(self.cursor_row, self.cursor_channel)
                .cloned()
                .unwrap_or_else(Cell::empty);
            self.clipboard = Some(Clipboard::single(cell));
        }
    }

    /// Paste clipboard contents at the current cursor position.
    pub fn paste(&mut self) {
        let clipboard = match &self.clipboard {
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

    /// Cut: copy the selection to clipboard and clear the source cells.
    /// In Visual mode, copies and clears the selection. Otherwise copies and clears the current cell.
    pub fn cut(&mut self) {
        self.copy();
        self.save_history();
        if self.mode == EditorMode::Visual {
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
        let selection = if self.mode == EditorMode::Visual {
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

    /// Interpolate volume values in the visual selection.
    ///
    /// For each channel column in the selection, finds the first and last cells
    /// that have volume values, then fills intermediate cells with linearly
    /// interpolated volume values.
    pub fn interpolate(&mut self) {
        let selection = if self.mode == EditorMode::Visual {
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
}
