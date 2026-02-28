/// Represents a musical pitch (C, D, E, F, G, A, B)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pitch {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Pitch {
    /// Parse a pitch from a character
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'C' => Some(Pitch::C),
            'D' => Some(Pitch::D),
            'E' => Some(Pitch::E),
            'F' => Some(Pitch::F),
            'G' => Some(Pitch::G),
            'A' => Some(Pitch::A),
            'B' => Some(Pitch::B),
            _ => None,
        }
    }

    /// Convert pitch to character
    pub fn to_char(&self) -> char {
        match self {
            Pitch::C => 'C',
            Pitch::D => 'D',
            Pitch::E => 'E',
            Pitch::F => 'F',
            Pitch::G => 'G',
            Pitch::A => 'A',
            Pitch::B => 'B',
        }
    }
}

/// Represents a note in the tracker pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub pitch: Pitch,
    pub octave: u8,
    pub velocity: Option<u8>,
    pub instrument: Option<u8>,
}

impl Note {
    /// Create a new note with pitch and octave
    pub fn new(pitch: Pitch, octave: u8) -> Self {
        Self {
            pitch,
            octave,
            velocity: None,
            instrument: None,
        }
    }

    /// Create a new note with all fields
    pub fn with_all(pitch: Pitch, octave: u8, velocity: u8, instrument: u8) -> Self {
        Self {
            pitch,
            octave,
            velocity: Some(velocity),
            instrument: Some(instrument),
        }
    }

    /// Parse a note from string (e.g., "C4", "A5")
    pub fn from_str(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        let pitch_char = s.chars().next()?;
        let octave_char = s.chars().nth(1)?;

        let pitch = Pitch::from_char(pitch_char)?;
        let octave = octave_char.to_digit(10)? as u8;

        if octave > 9 {
            return None;
        }

        Some(Note::new(pitch, octave))
    }

    /// Convert note to string representation
    pub fn to_string(&self) -> String {
        format!("{}{}", self.pitch.to_char(), self.octave)
    }
}

/// Represents a row in the pattern (collection of notes across channels)
pub type Row = Vec<Option<Note>>;

/// Represents a tracker pattern with multiple rows
#[derive(Debug, Clone)]
pub struct Pattern {
    rows: Vec<Row>,
    channels: usize,
}

impl Pattern {
    /// Create a new pattern with specified number of rows and channels
    pub fn new(num_rows: usize, channels: usize) -> Self {
        let rows = vec![vec![None; channels]; num_rows];
        Self { rows, channels }
    }

    /// Get the number of rows in the pattern
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of channels in the pattern
    pub fn num_channels(&self) -> usize {
        self.channels
    }

    /// Get a reference to a row
    pub fn get_row(&self, row: usize) -> Option<&Row> {
        self.rows.get(row)
    }

    /// Get a mutable reference to a row
    pub fn get_row_mut(&mut self, row: usize) -> Option<&mut Row> {
        self.rows.get_mut(row)
    }

    /// Set a note at a specific row and channel
    pub fn set_note(&mut self, row: usize, channel: usize, note: Option<Note>) -> bool {
        if let Some(row_data) = self.rows.get_mut(row) {
            if let Some(cell) = row_data.get_mut(channel) {
                *cell = note;
                return true;
            }
        }
        false
    }

    /// Get a note at a specific row and channel
    pub fn get_note(&self, row: usize, channel: usize) -> Option<&Option<Note>> {
        self.rows.get(row).and_then(|r| r.get(channel))
    }

    /// Insert a new empty row at the specified position
    pub fn insert_row(&mut self, row: usize) {
        if row <= self.rows.len() {
            self.rows.insert(row, vec![None; self.channels]);
        }
    }

    /// Delete a row at the specified position
    pub fn delete_row(&mut self, row: usize) -> bool {
        if row < self.rows.len() {
            self.rows.remove(row);
            true
        } else {
            false
        }
    }

    /// Clear all notes in a specific row
    pub fn clear_row(&mut self, row: usize) {
        if let Some(row_data) = self.rows.get_mut(row) {
            row_data.iter_mut().for_each(|cell| *cell = None);
        }
    }

    /// Clear a specific note at row and channel
    pub fn clear_note(&mut self, row: usize, channel: usize) {
        self.set_note(row, channel, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_creation() {
        let note = Note::new(Pitch::C, 4);
        assert_eq!(note.pitch, Pitch::C);
        assert_eq!(note.octave, 4);
        assert_eq!(note.velocity, None);
        assert_eq!(note.instrument, None);
    }

    #[test]
    fn test_note_with_all_fields() {
        let note = Note::with_all(Pitch::A, 5, 127, 1);
        assert_eq!(note.pitch, Pitch::A);
        assert_eq!(note.octave, 5);
        assert_eq!(note.velocity, Some(127));
        assert_eq!(note.instrument, Some(1));
    }

    #[test]
    fn test_note_from_string() {
        let note = Note::from_str("C4").unwrap();
        assert_eq!(note.pitch, Pitch::C);
        assert_eq!(note.octave, 4);

        let note2 = Note::from_str("G7").unwrap();
        assert_eq!(note2.pitch, Pitch::G);
        assert_eq!(note2.octave, 7);
    }

    #[test]
    fn test_note_to_string() {
        let note = Note::new(Pitch::D, 3);
        assert_eq!(note.to_string(), "D3");
    }

    #[test]
    fn test_pitch_from_char() {
        assert_eq!(Pitch::from_char('C'), Some(Pitch::C));
        assert_eq!(Pitch::from_char('c'), Some(Pitch::C));
        assert_eq!(Pitch::from_char('G'), Some(Pitch::G));
        assert_eq!(Pitch::from_char('X'), None);
    }

    #[test]
    fn test_pitch_to_char() {
        assert_eq!(Pitch::C.to_char(), 'C');
        assert_eq!(Pitch::G.to_char(), 'G');
        assert_eq!(Pitch::A.to_char(), 'A');
    }

    #[test]
    fn test_note_equality() {
        let note1 = Note::new(Pitch::C, 4);
        let note2 = Note::new(Pitch::C, 4);
        let note3 = Note::new(Pitch::D, 4);

        assert_eq!(note1, note2);
        assert_ne!(note1, note3);
    }

    #[test]
    fn test_note_octave_range() {
        for octave in 0..=9 {
            let note = Note::new(Pitch::C, octave);
            assert_eq!(note.octave, octave);
        }
    }

    #[test]
    fn test_note_invalid_string() {
        assert!(Note::from_str("").is_none());
        assert!(Note::from_str("C").is_none());
        assert!(Note::from_str("X4").is_none());
    }

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(16, 4);
        assert_eq!(pattern.num_rows(), 16);
        assert_eq!(pattern.num_channels(), 4);
    }

    #[test]
    fn test_pattern_set_and_get_note() {
        let mut pattern = Pattern::new(16, 4);
        let note = Note::new(Pitch::C, 4);

        assert!(pattern.set_note(0, 0, Some(note)));

        let retrieved = pattern.get_note(0, 0);
        assert!(retrieved.is_some());
        assert_eq!(*retrieved.unwrap(), Some(note));
    }

    #[test]
    fn test_pattern_set_note_out_of_bounds() {
        let mut pattern = Pattern::new(16, 4);
        let note = Note::new(Pitch::C, 4);

        assert!(!pattern.set_note(20, 0, Some(note)));
        assert!(!pattern.set_note(0, 10, Some(note)));
    }

    #[test]
    fn test_pattern_get_row() {
        let pattern = Pattern::new(16, 4);
        let row = pattern.get_row(0);
        assert!(row.is_some());
        assert_eq!(row.unwrap().len(), 4);

        let invalid_row = pattern.get_row(20);
        assert!(invalid_row.is_none());
    }

    #[test]
    fn test_pattern_insert_row() {
        let mut pattern = Pattern::new(4, 2);
        assert_eq!(pattern.num_rows(), 4);

        pattern.insert_row(2);
        assert_eq!(pattern.num_rows(), 5);

        let row = pattern.get_row(2).unwrap();
        assert_eq!(row.len(), 2);
        assert!(row.iter().all(|n| n.is_none()));
    }

    #[test]
    fn test_pattern_delete_row() {
        let mut pattern = Pattern::new(4, 2);
        let note = Note::new(Pitch::A, 5);
        pattern.set_note(1, 0, Some(note));

        assert!(pattern.delete_row(1));
        assert_eq!(pattern.num_rows(), 3);

        assert!(!pattern.delete_row(10));
    }

    #[test]
    fn test_pattern_clear_row() {
        let mut pattern = Pattern::new(4, 2);
        pattern.set_note(0, 0, Some(Note::new(Pitch::C, 4)));
        pattern.set_note(0, 1, Some(Note::new(Pitch::D, 5)));

        pattern.clear_row(0);

        let row = pattern.get_row(0).unwrap();
        assert!(row.iter().all(|n| n.is_none()));
    }

    #[test]
    fn test_pattern_clear_note() {
        let mut pattern = Pattern::new(4, 2);
        pattern.set_note(0, 0, Some(Note::new(Pitch::C, 4)));

        pattern.clear_note(0, 0);

        let note = pattern.get_note(0, 0);
        assert!(note.is_some());
        assert!(note.unwrap().is_none());
    }

    #[test]
    fn test_pattern_insert_row_at_end() {
        let mut pattern = Pattern::new(4, 2);
        pattern.insert_row(4);
        assert_eq!(pattern.num_rows(), 5);
    }

    #[test]
    fn test_pattern_insert_row_at_start() {
        let mut pattern = Pattern::new(4, 2);
        pattern.set_note(0, 0, Some(Note::new(Pitch::C, 4)));

        pattern.insert_row(0);
        assert_eq!(pattern.num_rows(), 5);

        // Original first row should now be at index 1
        let note = pattern.get_note(1, 0);
        assert!(note.is_some());
        assert_eq!(note.unwrap().as_ref().unwrap().pitch, Pitch::C);
    }
}
