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
}
