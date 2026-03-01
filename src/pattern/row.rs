/// Row and cell types for the tracker pattern grid.
///
/// A row represents a single time step across all channels. Each channel
/// position in the row contains a Cell with optional note, instrument,
/// volume, and effect data.

use std::fmt;
use serde::{Serialize, Deserialize};

use super::note::NoteEvent;

/// An effect command applied to a channel at a specific row.
///
/// Effect commands modify playback behavior (e.g., pitch slides, vibrato,
/// volume changes). Each effect has a type byte and a parameter byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    /// Effect type identifier.
    pub command: u8,
    /// Effect parameter value.
    pub param: u8,
}

impl Effect {
    /// Create a new effect command.
    pub fn new(command: u8, param: u8) -> Self {
        Self { command, param }
    }
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:01X}{:02X}", self.command, self.param)
    }
}

/// A single cell in the tracker pattern grid.
///
/// Each cell represents one channel at one row and can contain:
/// - A note event (note-on or note-off)
/// - An instrument number
/// - A volume value
/// - An effect command
///
/// All fields are optional; an empty cell has all fields as None.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The note event (note-on, note-off, or empty).
    pub note: Option<NoteEvent>,
    /// Instrument index (0-255).
    pub instrument: Option<u8>,
    /// Volume override (0-127, independent of note velocity).
    pub volume: Option<u8>,
    /// Effect command.
    pub effect: Option<Effect>,
}

impl Cell {
    /// Create an empty cell with no data.
    pub fn empty() -> Self {
        Self {
            note: None,
            instrument: None,
            volume: None,
            effect: None,
        }
    }

    /// Create a cell with just a note event.
    pub fn with_note(note: NoteEvent) -> Self {
        Self {
            note: Some(note),
            ..Self::empty()
        }
    }

    /// Returns true if the cell contains no data.
    pub fn is_empty(&self) -> bool {
        self.note.is_none()
            && self.instrument.is_none()
            && self.volume.is_none()
            && self.effect.is_none()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Cell {
    /// Display in tracker format: "C#4 01 40 000" or "--- .. .. ..."
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Note column (3 chars)
        match &self.note {
            Some(event) => write!(f, "{}", event)?,
            None => write!(f, "---")?,
        }

        write!(f, " ")?;

        // Instrument column (2 chars)
        match self.instrument {
            Some(inst) => write!(f, "{:02X}", inst)?,
            None => write!(f, "..")?,
        }

        write!(f, " ")?;

        // Volume column (2 chars)
        match self.volume {
            Some(vol) => write!(f, "{:02X}", vol)?,
            None => write!(f, "..")?,
        }

        write!(f, " ")?;

        // Effect column (3 chars)
        match &self.effect {
            Some(eff) => write!(f, "{}", eff)?,
            None => write!(f, "...")?,
        }

        Ok(())
    }
}

/// A row in the pattern, containing cells for each channel.
pub type Row = Vec<Cell>;

/// Create a new empty row with the given number of channels.
pub fn new_row(channels: usize) -> Row {
    vec![Cell::empty(); channels]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::note::{Note, NoteEvent, Pitch};

    #[test]
    fn test_cell_empty() {
        let cell = Cell::empty();
        assert!(cell.is_empty());
        assert_eq!(cell.note, None);
        assert_eq!(cell.instrument, None);
        assert_eq!(cell.volume, None);
        assert_eq!(cell.effect, None);
    }

    #[test]
    fn test_cell_with_note() {
        let note = Note::simple(Pitch::C, 4);
        let cell = Cell::with_note(NoteEvent::On(note));
        assert!(!cell.is_empty());
        assert!(cell.note.is_some());
    }

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert!(cell.is_empty());
    }

    #[test]
    fn test_cell_display_empty() {
        let cell = Cell::empty();
        assert_eq!(format!("{}", cell), "--- .. .. ...");
    }

    #[test]
    fn test_cell_display_with_note() {
        let note = Note::new(Pitch::CSharp, 4, 100, 1);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(1),
            volume: Some(0x40),
            effect: Some(Effect::new(0xC, 0x20)),
        };
        assert_eq!(format!("{}", cell), "C#4 01 40 C20");
    }

    #[test]
    fn test_cell_display_note_off() {
        let cell = Cell::with_note(NoteEvent::Off);
        assert_eq!(format!("{}", cell), "=== .. .. ...");
    }

    #[test]
    fn test_effect_display() {
        assert_eq!(format!("{}", Effect::new(0, 0)), "000");
        assert_eq!(format!("{}", Effect::new(0xF, 0xFF)), "FFF");
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "C40");
    }

    #[test]
    fn test_new_row() {
        let row = new_row(4);
        assert_eq!(row.len(), 4);
        assert!(row.iter().all(|c| c.is_empty()));
    }

    #[test]
    fn test_new_row_single_channel() {
        let row = new_row(1);
        assert_eq!(row.len(), 1);
        assert!(row[0].is_empty());
    }

    #[test]
    fn test_cell_with_all_fields() {
        let note = Note::new(Pitch::A, 4, 64, 2);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(2),
            volume: Some(64),
            effect: Some(Effect::new(0xF, 0x06)),
        };
        assert!(!cell.is_empty());
        assert_eq!(cell.instrument, Some(2));
        assert_eq!(cell.volume, Some(64));
        assert_eq!(format!("{}", cell), "A-4 02 40 F06");
    }

    #[test]
    fn test_cell_partial_fields() {
        // Cell with only instrument set (no note, no volume, no effect)
        let cell = Cell {
            note: None,
            instrument: Some(5),
            volume: None,
            effect: None,
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- 05 .. ...");
    }

    #[test]
    fn test_cell_only_volume() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: Some(0x7F),
            effect: None,
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- .. 7F ...");
    }

    #[test]
    fn test_cell_only_effect() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effect: Some(Effect::new(0xA, 0x0F)),
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- .. .. A0F");
    }

    #[test]
    fn test_effect_boundary_values() {
        let min = Effect::new(0, 0);
        assert_eq!(format!("{}", min), "000");

        let max = Effect::new(0xF, 0xFF);
        assert_eq!(format!("{}", max), "FFF");
    }

    #[test]
    fn test_cell_clone_eq() {
        let note = Note::simple(Pitch::C, 4);
        let cell = Cell::with_note(NoteEvent::On(note));
        let cloned = cell;
        assert_eq!(cell, cloned);
    }
}
