//! Row and cell types for the tracker pattern grid.
//!
//! A row represents a single time step across all channels. Each channel
//! position in the row contains a Cell with optional note, instrument,
//! volume, and effect data.

use serde::{Deserialize, Serialize};
use std::fmt;

pub use super::effect::{Effect, MAX_EFFECTS_PER_CELL};
use super::note::NoteEvent;

/// A single cell in the tracker pattern grid.
///
/// Each cell represents one channel at one row and can contain:
/// - A note event (note-on or note-off)
/// - An instrument number
/// - A volume value
/// - Up to 2 effect commands
///
/// All fields are optional; an empty cell has all fields as None/empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// The note event (note-on, note-off, or empty).
    pub note: Option<NoteEvent>,
    /// Instrument index (0-255).
    pub instrument: Option<u8>,
    /// Volume override (0-127, independent of note velocity).
    pub volume: Option<u8>,
    /// Effect commands (up to 2 per cell).
    pub effects: Vec<Effect>,
}

impl Cell {
    /// Create an empty cell with no data.
    pub fn empty() -> Self {
        Self {
            note: None,
            instrument: None,
            volume: None,
            effects: Vec::new(),
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
            && self.effects.is_empty()
    }

    /// Get the first effect, if any.
    pub fn first_effect(&self) -> Option<&Effect> {
        self.effects.first()
    }

    /// Get the second effect, if any.
    pub fn second_effect(&self) -> Option<&Effect> {
        self.effects.get(1)
    }

    /// Add an effect to this cell.
    ///
    /// Returns true if the effect was added, false if the cell already
    /// has the maximum number of effects.
    pub fn add_effect(&mut self, effect: Effect) -> bool {
        if self.effects.len() < MAX_EFFECTS_PER_CELL {
            self.effects.push(effect);
            true
        } else {
            false
        }
    }

    /// Set the first effect, replacing any existing first effect.
    pub fn set_effect(&mut self, effect: Effect) {
        if self.effects.is_empty() {
            self.effects.push(effect);
        } else {
            self.effects[0] = effect;
        }
    }

    /// Clear all effects from this cell.
    pub fn clear_effects(&mut self) {
        self.effects.clear();
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Cell {
    /// Display in tracker format: "C#4 01 40 0A04" or "--- .. .. ...."
    ///
    /// Shows the first effect column. If two effects are present,
    /// only the first is shown in the standard display.
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

        // Effect column (4 chars) — shows first effect
        match self.first_effect() {
            Some(eff) => write!(f, "{}", eff)?,
            None => write!(f, "....")?,
        }

        Ok(())
    }
}

/// A row in the pattern, containing cells for each channel.
pub type Row = Vec<Cell>;

/// Create a new empty row with the given number of channels.
pub fn new_row(channels: usize) -> Row {
    (0..channels).map(|_| Cell::empty()).collect()
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
        assert!(cell.effects.is_empty());
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
        assert_eq!(format!("{}", cell), "--- .. .. ....");
    }

    #[test]
    fn test_cell_display_with_note() {
        let note = Note::new(Pitch::CSharp, 4, 100, 1);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(1),
            volume: Some(0x40),
            effects: vec![Effect::new(0xC, 0x20)],
        };
        assert_eq!(format!("{}", cell), "C#4 01 40 0C20");
    }

    #[test]
    fn test_cell_display_note_off() {
        let cell = Cell::with_note(NoteEvent::Off);
        assert_eq!(format!("{}", cell), "=== .. .. ....");
    }

    #[test]
    fn test_effect_display() {
        assert_eq!(format!("{}", Effect::new(0, 0)), "0000");
        assert_eq!(format!("{}", Effect::new(0xF, 0xFF)), "0FFF");
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "0C40");
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
            effects: vec![Effect::new(0xF, 0x06)],
        };
        assert!(!cell.is_empty());
        assert_eq!(cell.instrument, Some(2));
        assert_eq!(cell.volume, Some(64));
        assert_eq!(format!("{}", cell), "A-4 02 40 0F06");
    }

    #[test]
    fn test_cell_partial_fields() {
        let cell = Cell {
            note: None,
            instrument: Some(5),
            volume: None,
            effects: Vec::new(),
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- 05 .. ....");
    }

    #[test]
    fn test_cell_only_volume() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: Some(0x7F),
            effects: Vec::new(),
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- .. 7F ....");
    }

    #[test]
    fn test_cell_only_effect() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xA, 0x0F)],
        };
        assert!(!cell.is_empty());
        assert_eq!(format!("{}", cell), "--- .. .. 0A0F");
    }

    #[test]
    fn test_effect_boundary_values() {
        let min = Effect::new(0, 0);
        assert_eq!(format!("{}", min), "0000");

        let max = Effect::new(0xF, 0xFF);
        assert_eq!(format!("{}", max), "0FFF");
    }

    #[test]
    fn test_cell_clone_eq() {
        let note = Note::simple(Pitch::C, 4);
        let cell = Cell::with_note(NoteEvent::On(note));
        let cloned = cell.clone();
        assert_eq!(cell, cloned);
    }

    // --- Multi-effect tests ---

    #[test]
    fn test_cell_two_effects() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xA, 0x04), Effect::new(0xC, 0x40)],
        };
        assert!(!cell.is_empty());
        assert_eq!(cell.effects.len(), 2);
        assert_eq!(cell.first_effect(), Some(&Effect::new(0xA, 0x04)));
        assert_eq!(cell.second_effect(), Some(&Effect::new(0xC, 0x40)));
    }

    #[test]
    fn test_cell_add_effect() {
        let mut cell = Cell::empty();
        assert!(cell.add_effect(Effect::new(0xA, 0x04)));
        assert_eq!(cell.effects.len(), 1);
        assert!(cell.add_effect(Effect::new(0xC, 0x40)));
        assert_eq!(cell.effects.len(), 2);
        // Third effect should fail
        assert!(!cell.add_effect(Effect::new(0xF, 0x06)));
        assert_eq!(cell.effects.len(), 2);
    }

    #[test]
    fn test_cell_set_effect() {
        let mut cell = Cell::empty();
        cell.set_effect(Effect::new(0xA, 0x04));
        assert_eq!(cell.effects.len(), 1);
        assert_eq!(cell.first_effect(), Some(&Effect::new(0xA, 0x04)));

        // Replace first effect
        cell.set_effect(Effect::new(0xC, 0x40));
        assert_eq!(cell.effects.len(), 1);
        assert_eq!(cell.first_effect(), Some(&Effect::new(0xC, 0x40)));
    }

    #[test]
    fn test_cell_clear_effects() {
        let mut cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xA, 0x04), Effect::new(0xC, 0x40)],
        };
        cell.clear_effects();
        assert!(cell.effects.is_empty());
    }

    #[test]
    fn test_cell_display_shows_first_effect_only() {
        let cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xA, 0x04), Effect::new(0xC, 0x40)],
        };
        // Display should show first effect
        assert_eq!(format!("{}", cell), "--- .. .. 0A04");
    }

    #[test]
    fn test_cell_first_effect_none_when_empty() {
        let cell = Cell::empty();
        assert_eq!(cell.first_effect(), None);
        assert_eq!(cell.second_effect(), None);
    }

    // --- Effect + Cell Integration Tests ---

    #[test]
    fn test_cell_serde_roundtrip_with_effects() {
        let note = Note::new(Pitch::CSharp, 4, 100, 1);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(1),
            volume: Some(0x40),
            effects: vec![Effect::new(0xA, 0x04), Effect::new(0xC, 0x40)],
        };
        let json = serde_json::to_string(&cell).unwrap();
        let decoded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, decoded);
        assert_eq!(decoded.effects.len(), 2);
        assert_eq!(decoded.first_effect(), Some(&Effect::new(0xA, 0x04)));
        assert_eq!(decoded.second_effect(), Some(&Effect::new(0xC, 0x40)));
    }

    #[test]
    fn test_cell_serde_roundtrip_empty() {
        let cell = Cell::empty();
        let json = serde_json::to_string(&cell).unwrap();
        let decoded: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, decoded);
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_cell_effects_stored_and_retrieved_by_type() {
        let mut cell = Cell::empty();

        // Add volume slide effect
        let vol_slide = Effect::from_type(super::super::effect::EffectType::VolumeSlide, 0x04);
        cell.add_effect(vol_slide);

        // Add set speed effect
        let set_speed = Effect::from_type(super::super::effect::EffectType::SetSpeed, 0x06);
        cell.add_effect(set_speed);

        // Verify effects are correctly stored and retrievable
        let first = cell.first_effect().unwrap();
        assert_eq!(
            first.effect_type(),
            Some(super::super::effect::EffectType::VolumeSlide)
        );
        assert_eq!(first.param, 0x04);
        assert_eq!(format!("{}", first), "0A04");

        let second = cell.second_effect().unwrap();
        assert_eq!(
            second.effect_type(),
            Some(super::super::effect::EffectType::SetSpeed)
        );
        assert_eq!(second.param, 0x06);
        assert_eq!(format!("{}", second), "0F06");
    }

    #[test]
    fn test_cell_set_effect_replaces_first_preserves_second() {
        let mut cell = Cell::empty();
        cell.add_effect(Effect::new(0xA, 0x04));
        cell.add_effect(Effect::new(0xC, 0x40));

        // set_effect replaces the first effect only
        cell.set_effect(Effect::new(0xF, 0x06));
        assert_eq!(cell.effects.len(), 2);
        assert_eq!(cell.first_effect(), Some(&Effect::new(0xF, 0x06)));
        assert_eq!(cell.second_effect(), Some(&Effect::new(0xC, 0x40)));
    }

    #[test]
    fn test_cell_display_with_all_effect_types() {
        // Verify display with each known effect type as first effect
        let test_cases = vec![
            (Effect::new(0x0, 0x37), "--- .. .. 0037"),
            (Effect::new(0x1, 0x10), "--- .. .. 0110"),
            (Effect::new(0x2, 0x20), "--- .. .. 0220"),
            (Effect::new(0x3, 0x08), "--- .. .. 0308"),
            (Effect::new(0x4, 0x46), "--- .. .. 0446"),
            (Effect::new(0xA, 0x0F), "--- .. .. 0A0F"),
            (Effect::new(0xB, 0x02), "--- .. .. 0B02"),
            (Effect::new(0xC, 0x40), "--- .. .. 0C40"),
            (Effect::new(0xD, 0x00), "--- .. .. 0D00"),
            (Effect::new(0xF, 0x06), "--- .. .. 0F06"),
        ];
        for (effect, expected) in test_cases {
            let cell = Cell {
                note: None,
                instrument: None,
                volume: None,
                effects: vec![effect],
            };
            assert_eq!(
                format!("{}", cell),
                expected,
                "Display mismatch for effect {}",
                effect
            );
        }
    }

    #[test]
    fn test_cell_clear_then_add_effects() {
        let mut cell = Cell::empty();
        cell.add_effect(Effect::new(0xA, 0x04));
        cell.add_effect(Effect::new(0xC, 0x40));
        assert_eq!(cell.effects.len(), 2);

        cell.clear_effects();
        assert!(cell.effects.is_empty());
        assert_eq!(cell.first_effect(), None);

        // Can add effects again after clearing
        assert!(cell.add_effect(Effect::new(0xF, 0x06)));
        assert_eq!(cell.effects.len(), 1);
        assert_eq!(cell.first_effect(), Some(&Effect::new(0xF, 0x06)));
    }

    #[test]
    fn test_cell_with_note_and_effects_display() {
        let note = Note::new(Pitch::A, 4, 100, 5);
        let cell = Cell {
            note: Some(NoteEvent::On(note)),
            instrument: Some(5),
            volume: Some(0x64),
            effects: vec![Effect::new(0x4, 0x37)],
        };
        assert_eq!(format!("{}", cell), "A-4 05 64 0437");
    }

    #[test]
    fn test_row_with_effects_across_channels() {
        let mut row = new_row(4);

        // Set effects on different channels
        row[0].add_effect(Effect::new(0xC, 0x40));
        row[1].add_effect(Effect::new(0xA, 0x04));
        row[2].add_effect(Effect::new(0xF, 0x06));

        assert_eq!(row[0].first_effect(), Some(&Effect::new(0xC, 0x40)));
        assert_eq!(row[1].first_effect(), Some(&Effect::new(0xA, 0x04)));
        assert_eq!(row[2].first_effect(), Some(&Effect::new(0xF, 0x06)));
        assert_eq!(row[3].first_effect(), None);
    }
}
