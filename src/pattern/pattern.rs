/// Pattern grid structure for the tracker.
///
/// A pattern is a 2D grid of cells organized by rows (time steps) and
/// channels (parallel voices). The default size is 64 rows x 4 channels,
/// matching classic tracker conventions.

use super::note::{Note, NoteEvent};
use super::row::{Cell, Row, new_row};

/// Default number of rows in a pattern.
pub const DEFAULT_ROWS: usize = 64;

/// Default number of channels in a pattern.
pub const DEFAULT_CHANNELS: usize = 4;

/// A tracker pattern containing a grid of cells.
///
/// Patterns are the fundamental unit of composition in a tracker.
/// Each pattern contains a fixed number of rows and channels.
/// Rows represent time steps; channels represent parallel voices.
#[derive(Debug, Clone)]
pub struct Pattern {
    /// The pattern data: rows × channels.
    rows: Vec<Row>,
    /// Number of channels per row.
    channels: usize,
}

impl Pattern {
    /// Create a new empty pattern with the given dimensions.
    pub fn new(num_rows: usize, channels: usize) -> Self {
        assert!(num_rows > 0, "Pattern must have at least 1 row");
        assert!(channels > 0, "Pattern must have at least 1 channel");
        let rows = (0..num_rows).map(|_| new_row(channels)).collect();
        Self { rows, channels }
    }

    /// Get the number of rows in this pattern.
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of channels in this pattern.
    pub fn num_channels(&self) -> usize {
        self.channels
    }

    /// Get a reference to a cell at the given position.
    ///
    /// Returns None if the row or channel is out of bounds.
    pub fn get_cell(&self, row: usize, channel: usize) -> Option<&Cell> {
        self.rows.get(row).and_then(|r| r.get(channel))
    }

    /// Get a mutable reference to a cell at the given position.
    ///
    /// Returns None if the row or channel is out of bounds.
    pub fn get_cell_mut(&mut self, row: usize, channel: usize) -> Option<&mut Cell> {
        self.rows.get_mut(row).and_then(|r| r.get_mut(channel))
    }

    /// Set a cell at the given position.
    ///
    /// Returns true if the position was valid and the cell was set,
    /// false if out of bounds.
    pub fn set_cell(&mut self, row: usize, channel: usize, cell: Cell) -> bool {
        if let Some(existing) = self.get_cell_mut(row, channel) {
            *existing = cell;
            true
        } else {
            false
        }
    }

    /// Set a note at the given position, creating a cell with just the note.
    ///
    /// Returns true if the position was valid, false if out of bounds.
    pub fn set_note(&mut self, row: usize, channel: usize, note: Note) -> bool {
        self.set_cell(row, channel, Cell::with_note(NoteEvent::On(note)))
    }

    /// Clear a cell at the given position (reset to empty).
    ///
    /// Returns true if the position was valid, false if out of bounds.
    pub fn clear_cell(&mut self, row: usize, channel: usize) -> bool {
        self.set_cell(row, channel, Cell::empty())
    }

    /// Insert a new empty row at the given position.
    ///
    /// Rows after the insertion point are shifted down.
    /// If `at` is beyond the current row count, the row is appended.
    pub fn insert_row(&mut self, at: usize) {
        let at = at.min(self.rows.len());
        self.rows.insert(at, new_row(self.channels));
    }

    /// Delete the row at the given position.
    ///
    /// Returns true if the row was deleted, false if out of bounds.
    /// Will not delete the last row (patterns must have at least 1 row).
    pub fn delete_row(&mut self, at: usize) -> bool {
        if at < self.rows.len() && self.rows.len() > 1 {
            self.rows.remove(at);
            true
        } else {
            false
        }
    }

    /// Get a reference to a full row.
    pub fn get_row(&self, row: usize) -> Option<&Row> {
        self.rows.get(row)
    }
}

impl Default for Pattern {
    /// Create a default pattern (64 rows, 4 channels).
    fn default() -> Self {
        Self::new(DEFAULT_ROWS, DEFAULT_CHANNELS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::note::Pitch;
    use crate::pattern::row::Effect;

    #[test]
    fn test_pattern_default() {
        let pat = Pattern::default();
        assert_eq!(pat.num_rows(), 64);
        assert_eq!(pat.num_channels(), 4);
    }

    #[test]
    fn test_pattern_custom_dimensions() {
        let pat = Pattern::new(32, 8);
        assert_eq!(pat.num_rows(), 32);
        assert_eq!(pat.num_channels(), 8);
    }

    #[test]
    #[should_panic(expected = "at least 1 row")]
    fn test_pattern_zero_rows() {
        Pattern::new(0, 4);
    }

    #[test]
    #[should_panic(expected = "at least 1 channel")]
    fn test_pattern_zero_channels() {
        Pattern::new(64, 0);
    }

    #[test]
    fn test_get_cell() {
        let pat = Pattern::new(16, 4);
        assert!(pat.get_cell(0, 0).is_some());
        assert!(pat.get_cell(15, 3).is_some());
        assert!(pat.get_cell(0, 0).unwrap().is_empty());

        // Out of bounds
        assert!(pat.get_cell(16, 0).is_none());
        assert!(pat.get_cell(0, 4).is_none());
    }

    #[test]
    fn test_set_cell() {
        let mut pat = Pattern::new(16, 4);
        let note = Note::simple(Pitch::C, 4);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(0),
            volume: Some(0x40),
            effect: Some(Effect::new(0, 0)),
        };

        assert!(pat.set_cell(0, 0, cell));
        let retrieved = pat.get_cell(0, 0).unwrap();
        assert!(!retrieved.is_empty());
        assert_eq!(retrieved.instrument, Some(0));
    }

    #[test]
    fn test_set_cell_out_of_bounds() {
        let mut pat = Pattern::new(16, 4);
        let cell = Cell::empty();
        assert!(!pat.set_cell(16, 0, cell));
        assert!(!pat.set_cell(0, 4, cell));
    }

    #[test]
    fn test_set_note() {
        let mut pat = Pattern::new(16, 4);
        let note = Note::simple(Pitch::E, 4);
        assert!(pat.set_note(0, 0, note));

        let cell = pat.get_cell(0, 0).unwrap();
        assert_eq!(cell.note, Some(NoteEvent::On(note)));
    }

    #[test]
    fn test_clear_cell() {
        let mut pat = Pattern::new(16, 4);
        let note = Note::simple(Pitch::G, 4);
        pat.set_note(0, 0, note);
        assert!(!pat.get_cell(0, 0).unwrap().is_empty());

        assert!(pat.clear_cell(0, 0));
        assert!(pat.get_cell(0, 0).unwrap().is_empty());
    }

    #[test]
    fn test_insert_row() {
        let mut pat = Pattern::new(16, 4);
        assert_eq!(pat.num_rows(), 16);

        pat.insert_row(8);
        assert_eq!(pat.num_rows(), 17);

        // Insert beyond end appends
        pat.insert_row(100);
        assert_eq!(pat.num_rows(), 18);

        // Data integrity: set a note, insert before it, verify it shifted
        pat.set_note(5, 0, Note::simple(Pitch::C, 4));
        pat.insert_row(5);
        // The note should now be at row 6
        assert!(pat.get_cell(5, 0).unwrap().is_empty());
        assert!(!pat.get_cell(6, 0).unwrap().is_empty());
    }

    #[test]
    fn test_delete_row() {
        let mut pat = Pattern::new(16, 4);
        assert!(pat.delete_row(0));
        assert_eq!(pat.num_rows(), 15);

        // Out of bounds
        assert!(!pat.delete_row(15));

        // Cannot delete last row
        let mut pat = Pattern::new(1, 4);
        assert!(!pat.delete_row(0));
        assert_eq!(pat.num_rows(), 1);
    }

    #[test]
    fn test_delete_row_shifts_data() {
        let mut pat = Pattern::new(4, 1);
        pat.set_note(2, 0, Note::simple(Pitch::A, 4));
        pat.delete_row(1);
        // Row 2 should now be at row 1
        assert!(!pat.get_cell(1, 0).unwrap().is_empty());
    }

    #[test]
    fn test_all_cells_initially_empty() {
        let pat = Pattern::new(64, 4);
        for r in 0..64 {
            for c in 0..4 {
                assert!(pat.get_cell(r, c).unwrap().is_empty());
            }
        }
    }

    #[test]
    fn test_get_row() {
        let pat = Pattern::new(16, 4);
        let row = pat.get_row(0).unwrap();
        assert_eq!(row.len(), 4);
        assert!(pat.get_row(16).is_none());
    }

    #[test]
    fn test_set_note_out_of_bounds() {
        let mut pat = Pattern::new(16, 4);
        let note = Note::simple(Pitch::C, 4);
        assert!(!pat.set_note(16, 0, note));
        assert!(!pat.set_note(0, 4, note));
        assert!(!pat.set_note(100, 100, note));
    }

    #[test]
    fn test_clear_cell_out_of_bounds() {
        let mut pat = Pattern::new(16, 4);
        assert!(!pat.clear_cell(16, 0));
        assert!(!pat.clear_cell(0, 4));
    }

    #[test]
    fn test_overwrite_cell() {
        let mut pat = Pattern::new(16, 4);
        let note_c = Note::simple(Pitch::C, 4);
        let note_e = Note::simple(Pitch::E, 4);

        pat.set_note(0, 0, note_c);
        assert_eq!(pat.get_cell(0, 0).unwrap().note, Some(NoteEvent::On(note_c)));

        // Overwrite with a different note
        pat.set_note(0, 0, note_e);
        assert_eq!(pat.get_cell(0, 0).unwrap().note, Some(NoteEvent::On(note_e)));
    }

    #[test]
    fn test_get_cell_mut() {
        let mut pat = Pattern::new(16, 4);
        if let Some(cell) = pat.get_cell_mut(0, 0) {
            cell.instrument = Some(5);
            cell.volume = Some(0x40);
        }
        let cell = pat.get_cell(0, 0).unwrap();
        assert_eq!(cell.instrument, Some(5));
        assert_eq!(cell.volume, Some(0x40));
    }

    #[test]
    fn test_get_cell_mut_out_of_bounds() {
        let mut pat = Pattern::new(16, 4);
        assert!(pat.get_cell_mut(16, 0).is_none());
        assert!(pat.get_cell_mut(0, 4).is_none());
    }

    #[test]
    fn test_pattern_single_row_single_channel() {
        let pat = Pattern::new(1, 1);
        assert_eq!(pat.num_rows(), 1);
        assert_eq!(pat.num_channels(), 1);
        assert!(pat.get_cell(0, 0).unwrap().is_empty());
    }

    #[test]
    fn test_insert_row_at_beginning() {
        let mut pat = Pattern::new(4, 1);
        pat.set_note(0, 0, Note::simple(Pitch::C, 4));
        pat.insert_row(0);
        assert_eq!(pat.num_rows(), 5);
        // Original row 0 should now be at row 1
        assert!(pat.get_cell(0, 0).unwrap().is_empty());
        assert!(!pat.get_cell(1, 0).unwrap().is_empty());
    }

    #[test]
    fn test_insert_row_at_end() {
        let mut pat = Pattern::new(4, 1);
        pat.insert_row(4); // at the end
        assert_eq!(pat.num_rows(), 5);
        assert!(pat.get_cell(4, 0).unwrap().is_empty());
    }

    #[test]
    fn test_delete_all_but_one_row() {
        let mut pat = Pattern::new(3, 1);
        assert!(pat.delete_row(0));
        assert_eq!(pat.num_rows(), 2);
        assert!(pat.delete_row(0));
        assert_eq!(pat.num_rows(), 1);
        // Cannot delete the last row
        assert!(!pat.delete_row(0));
        assert_eq!(pat.num_rows(), 1);
    }

    #[test]
    fn test_multiple_channels_independent() {
        let mut pat = Pattern::new(4, 4);
        pat.set_note(0, 0, Note::simple(Pitch::C, 4));
        pat.set_note(0, 1, Note::simple(Pitch::E, 4));
        pat.set_note(0, 2, Note::simple(Pitch::G, 4));

        // Each channel is independent
        let c0 = pat.get_cell(0, 0).unwrap();
        let c1 = pat.get_cell(0, 1).unwrap();
        let c2 = pat.get_cell(0, 2).unwrap();
        let c3 = pat.get_cell(0, 3).unwrap();

        assert_eq!(c0.note, Some(NoteEvent::On(Note::simple(Pitch::C, 4))));
        assert_eq!(c1.note, Some(NoteEvent::On(Note::simple(Pitch::E, 4))));
        assert_eq!(c2.note, Some(NoteEvent::On(Note::simple(Pitch::G, 4))));
        assert!(c3.is_empty());
    }

    #[test]
    fn test_pattern_large_dimensions() {
        let pat = Pattern::new(256, 16);
        assert_eq!(pat.num_rows(), 256);
        assert_eq!(pat.num_channels(), 16);
        assert!(pat.get_cell(255, 15).is_some());
        assert!(pat.get_cell(256, 0).is_none());
    }

    #[test]
    fn test_set_cell_with_full_data() {
        let mut pat = Pattern::new(16, 4);
        let cell = Cell {
            note: Some(NoteEvent::On(Note::new(Pitch::A, 4, 127, 3))),
            instrument: Some(3),
            volume: Some(0x7F),
            effect: Some(Effect::new(0xC, 0x40)),
        };
        assert!(pat.set_cell(5, 2, cell));
        let retrieved = pat.get_cell(5, 2).unwrap();
        assert_eq!(retrieved.instrument, Some(3));
        assert_eq!(retrieved.volume, Some(0x7F));
        assert_eq!(retrieved.effect, Some(Effect::new(0xC, 0x40)));
    }
}
