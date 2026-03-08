//! Musical note representation for the tracker pattern grid.
//!
//! Provides pitch, octave, velocity, and instrument data for each note event.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Musical pitch with sharps and flats.
///
/// Covers all 12 semitones of the chromatic scale using sharp notation.
/// Flats are accepted when parsing but stored as their enharmonic sharp equivalent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pitch {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

impl Pitch {
    /// All pitches in chromatic order.
    pub const ALL: [Pitch; 12] = [
        Pitch::C,
        Pitch::CSharp,
        Pitch::D,
        Pitch::DSharp,
        Pitch::E,
        Pitch::F,
        Pitch::FSharp,
        Pitch::G,
        Pitch::GSharp,
        Pitch::A,
        Pitch::ASharp,
        Pitch::B,
    ];

    /// Parse a pitch from a string slice (e.g., "C", "C#", "Db").
    ///
    /// Accepts sharp (#) and flat (b) notation. Flats are converted to
    /// their enharmonic sharp equivalent.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "C" => Some(Pitch::C),
            "C#" | "Db" => Some(Pitch::CSharp),
            "D" => Some(Pitch::D),
            "D#" | "Eb" => Some(Pitch::DSharp),
            "E" | "Fb" => Some(Pitch::E),
            "F" | "E#" => Some(Pitch::F),
            "F#" | "Gb" => Some(Pitch::FSharp),
            "G" => Some(Pitch::G),
            "G#" | "Ab" => Some(Pitch::GSharp),
            "A" => Some(Pitch::A),
            "A#" | "Bb" => Some(Pitch::ASharp),
            "B" | "Cb" => Some(Pitch::B),
            _ => None,
        }
    }

    /// Display string for this pitch (e.g., "C-", "C#", "D-").
    ///
    /// Natural notes use a dash as padding to maintain fixed-width display.
    pub fn display_str(&self) -> &'static str {
        match self {
            Pitch::C => "C-",
            Pitch::CSharp => "C#",
            Pitch::D => "D-",
            Pitch::DSharp => "D#",
            Pitch::E => "E-",
            Pitch::F => "F-",
            Pitch::FSharp => "F#",
            Pitch::G => "G-",
            Pitch::GSharp => "G#",
            Pitch::A => "A-",
            Pitch::ASharp => "A#",
            Pitch::B => "B-",
        }
    }

    /// Create a Pitch from a semitone index (0-11).
    ///
    /// Returns None if the index is out of range.
    pub fn from_semitone(semitone: u8) -> Option<Self> {
        if (semitone as usize) < Self::ALL.len() {
            Some(Self::ALL[semitone as usize])
        } else {
            None
        }
    }

    /// MIDI note number offset (0-11) for this pitch.
    pub fn semitone(&self) -> u8 {
        match self {
            Pitch::C => 0,
            Pitch::CSharp => 1,
            Pitch::D => 2,
            Pitch::DSharp => 3,
            Pitch::E => 4,
            Pitch::F => 5,
            Pitch::FSharp => 6,
            Pitch::G => 7,
            Pitch::GSharp => 8,
            Pitch::A => 9,
            Pitch::ASharp => 10,
            Pitch::B => 11,
        }
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_str())
    }
}

/// A musical note with pitch, octave, velocity, and instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// The pitch of the note (C through B with sharps).
    pub pitch: Pitch,
    /// The octave (0-9).
    pub octave: u8,
    /// Velocity (0-127). Higher values = louder.
    pub velocity: u8,
    /// Instrument index for sample lookup.
    pub instrument: u8,
}

impl Note {
    /// Create a new note.
    ///
    /// # Panics
    /// Panics if octave > 9 or velocity > 127.
    pub fn new(pitch: Pitch, octave: u8, velocity: u8, instrument: u8) -> Self {
        assert!(octave <= 9, "Octave must be 0-9, got {}", octave);
        assert!(velocity <= 127, "Velocity must be 0-127, got {}", velocity);
        Self {
            pitch,
            octave,
            velocity,
            instrument,
        }
    }

    /// Create a note with default velocity (100) and instrument (0).
    pub fn simple(pitch: Pitch, octave: u8) -> Self {
        Self::new(pitch, octave, 100, 0)
    }

    /// Parse a note from a tracker-style string like "C#4", "A-5", "D#3".
    ///
    /// Format: `<pitch><sharp/dash><octave>`
    /// - Pitch: A-G
    /// - Sharp/dash: `#` for sharp, `-` for natural
    /// - Octave: 0-9
    ///
    /// Returns None if the string doesn't match the expected format.
    pub fn from_tracker_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.len() < 3 {
            return None;
        }

        let chars: Vec<char> = s.chars().collect();
        let pitch_char = chars[0].to_ascii_uppercase();
        let modifier = chars[1];
        let octave_char = chars[2];

        // Parse the pitch + modifier
        let pitch_str = if modifier == '#' {
            format!("{}#", pitch_char)
        } else if modifier == '-' || modifier == ' ' {
            format!("{}", pitch_char)
        } else if modifier == 'b' {
            format!("{}b", pitch_char)
        } else {
            return None;
        };

        let pitch = Pitch::parse_str(&pitch_str)?;
        let octave = octave_char.to_digit(10)? as u8;

        Some(Note::simple(pitch, octave))
    }

    /// Convert to MIDI note number (C-0 = 0, C-4 = 48, A-4 = 57).
    pub fn midi_note(&self) -> u8 {
        self.octave * 12 + self.pitch.semitone()
    }

    /// Calculate the frequency in Hz (A4 = 440Hz standard tuning).
    pub fn frequency(&self) -> f64 {
        // A4 is MIDI note 57 (octave 4, semitone 9)
        let a4_midi = 4 * 12 + 9; // = 57
        let semitone_diff = self.midi_note() as i32 - a4_midi;
        440.0 * 2.0_f64.powf(semitone_diff as f64 / 12.0)
    }

    /// Transpose this note by the given number of semitones.
    ///
    /// Returns None if the result would be out of the valid range (C-0 to B-9).
    pub fn transpose(&self, semitones: i32) -> Option<Self> {
        let midi = self.midi_note() as i32 + semitones;
        if !(0..=119).contains(&midi) {
            return None;
        }
        let midi = midi as u8;
        let octave = midi / 12;
        let semitone = midi % 12;
        let pitch = Pitch::from_semitone(semitone)?;
        Some(Note::new(pitch, octave, self.velocity, self.instrument))
    }

    /// Tracker-style display string (e.g., "C#4", "A-5").
    pub fn display_str(&self) -> String {
        format!("{}{}", self.pitch.display_str(), self.octave)
    }
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_str())
    }
}

/// Sentinel value representing a note-off event in the pattern.
///
/// In tracker notation, this is typically shown as "===" or "OFF".
/// When encountered during playback, it stops the currently playing note
/// on that channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteOff;

impl fmt::Display for NoteOff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "===")
    }
}

/// A note event that can be either a note-on or note-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteEvent {
    /// A note-on event with pitch, octave, velocity, and instrument.
    On(Note),
    /// A note-off event that stops the current note on the channel.
    Off,
}

impl fmt::Display for NoteEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoteEvent::On(note) => write!(f, "{}", note),
            NoteEvent::Off => write!(f, "==="),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_from_str() {
        assert_eq!(Pitch::parse_str("C"), Some(Pitch::C));
        assert_eq!(Pitch::parse_str("C#"), Some(Pitch::CSharp));
        assert_eq!(Pitch::parse_str("Db"), Some(Pitch::CSharp));
        assert_eq!(Pitch::parse_str("F#"), Some(Pitch::FSharp));
        assert_eq!(Pitch::parse_str("Bb"), Some(Pitch::ASharp));
        assert_eq!(Pitch::parse_str("X"), None);
        assert_eq!(Pitch::parse_str(""), None);
    }

    #[test]
    fn test_pitch_display() {
        assert_eq!(Pitch::C.display_str(), "C-");
        assert_eq!(Pitch::CSharp.display_str(), "C#");
        assert_eq!(Pitch::FSharp.display_str(), "F#");
        assert_eq!(format!("{}", Pitch::GSharp), "G#");
    }

    #[test]
    fn test_pitch_semitone() {
        assert_eq!(Pitch::C.semitone(), 0);
        assert_eq!(Pitch::CSharp.semitone(), 1);
        assert_eq!(Pitch::B.semitone(), 11);
    }

    #[test]
    fn test_note_creation() {
        let note = Note::new(Pitch::C, 4, 100, 0);
        assert_eq!(note.pitch, Pitch::C);
        assert_eq!(note.octave, 4);
        assert_eq!(note.velocity, 100);
        assert_eq!(note.instrument, 0);
    }

    #[test]
    fn test_note_simple() {
        let note = Note::simple(Pitch::A, 4);
        assert_eq!(note.pitch, Pitch::A);
        assert_eq!(note.octave, 4);
        assert_eq!(note.velocity, 100);
        assert_eq!(note.instrument, 0);
    }

    #[test]
    #[should_panic(expected = "Octave must be 0-9")]
    #[cfg_attr(
        target_os = "macos",
        ignore = "panic unwinding broken on macOS ARM64 toolchain"
    )]
    fn test_note_invalid_octave() {
        Note::new(Pitch::C, 10, 100, 0);
    }

    #[test]
    #[should_panic(expected = "Velocity must be 0-127")]
    #[cfg_attr(
        target_os = "macos",
        ignore = "panic unwinding broken on macOS ARM64 toolchain"
    )]
    fn test_note_invalid_velocity() {
        Note::new(Pitch::C, 4, 128, 0);
    }

    #[test]
    fn test_note_display() {
        assert_eq!(Note::simple(Pitch::C, 4).display_str(), "C-4");
        assert_eq!(Note::simple(Pitch::CSharp, 4).display_str(), "C#4");
        assert_eq!(Note::simple(Pitch::A, 5).display_str(), "A-5");
        assert_eq!(format!("{}", Note::simple(Pitch::FSharp, 3)), "F#3");
    }

    #[test]
    fn test_note_from_tracker_str() {
        let note = Note::from_tracker_str("C#4").unwrap();
        assert_eq!(note.pitch, Pitch::CSharp);
        assert_eq!(note.octave, 4);

        let note = Note::from_tracker_str("A-5").unwrap();
        assert_eq!(note.pitch, Pitch::A);
        assert_eq!(note.octave, 5);

        let note = Note::from_tracker_str("D#3").unwrap();
        assert_eq!(note.pitch, Pitch::DSharp);
        assert_eq!(note.octave, 3);

        let note = Note::from_tracker_str("Db3").unwrap();
        assert_eq!(note.pitch, Pitch::CSharp); // Db = C#

        assert!(Note::from_tracker_str("X-4").is_none());
        assert!(Note::from_tracker_str("C").is_none());
        assert!(Note::from_tracker_str("").is_none());
    }

    #[test]
    fn test_note_midi() {
        assert_eq!(Note::simple(Pitch::C, 0).midi_note(), 0);
        assert_eq!(Note::simple(Pitch::C, 4).midi_note(), 48);
        assert_eq!(Note::simple(Pitch::A, 4).midi_note(), 57);
        assert_eq!(Note::simple(Pitch::B, 9).midi_note(), 119);
    }

    #[test]
    fn test_note_frequency() {
        let a4 = Note::simple(Pitch::A, 4);
        assert!((a4.frequency() - 440.0).abs() < 0.01);

        let a5 = Note::simple(Pitch::A, 5);
        assert!((a5.frequency() - 880.0).abs() < 0.01);
    }

    #[test]
    fn test_note_off_display() {
        assert_eq!(format!("{}", NoteOff), "===");
    }

    #[test]
    fn test_note_event_display() {
        let on = NoteEvent::On(Note::simple(Pitch::C, 4));
        assert_eq!(format!("{}", on), "C-4");

        let off = NoteEvent::Off;
        assert_eq!(format!("{}", off), "===");
    }

    #[test]
    fn test_pitch_all_contains_12_semitones() {
        assert_eq!(Pitch::ALL.len(), 12);
        for (i, pitch) in Pitch::ALL.iter().enumerate() {
            assert_eq!(pitch.semitone() as usize, i);
        }
    }

    #[test]
    fn test_pitch_from_str_all_flats() {
        // Verify all flat notation parses correctly
        assert_eq!(Pitch::parse_str("Db"), Some(Pitch::CSharp));
        assert_eq!(Pitch::parse_str("Eb"), Some(Pitch::DSharp));
        assert_eq!(Pitch::parse_str("Fb"), Some(Pitch::E));
        assert_eq!(Pitch::parse_str("Gb"), Some(Pitch::FSharp));
        assert_eq!(Pitch::parse_str("Ab"), Some(Pitch::GSharp));
        assert_eq!(Pitch::parse_str("Bb"), Some(Pitch::ASharp));
        assert_eq!(Pitch::parse_str("Cb"), Some(Pitch::B));
    }

    #[test]
    fn test_pitch_from_str_enharmonic_sharps() {
        assert_eq!(Pitch::parse_str("E#"), Some(Pitch::F));
    }

    #[test]
    fn test_note_from_tracker_str_lowercase() {
        let note = Note::from_tracker_str("c#4").unwrap();
        assert_eq!(note.pitch, Pitch::CSharp);
        assert_eq!(note.octave, 4);
    }

    #[test]
    fn test_note_from_tracker_str_with_whitespace() {
        let note = Note::from_tracker_str("  C#4  ").unwrap();
        assert_eq!(note.pitch, Pitch::CSharp);
        assert_eq!(note.octave, 4);
    }

    #[test]
    fn test_note_from_tracker_str_all_octaves() {
        for octave in 0..=9 {
            let s = format!("C-{}", octave);
            let note = Note::from_tracker_str(&s).unwrap();
            assert_eq!(note.octave, octave);
        }
    }

    #[test]
    fn test_note_from_tracker_str_invalid_modifier() {
        assert!(Note::from_tracker_str("C*4").is_none());
        assert!(Note::from_tracker_str("C!4").is_none());
    }

    #[test]
    fn test_note_from_tracker_str_flat_notation() {
        let note = Note::from_tracker_str("Db3").unwrap();
        assert_eq!(note.pitch, Pitch::CSharp); // Db => C#
        assert_eq!(note.octave, 3);
    }

    #[test]
    fn test_note_display_roundtrip() {
        // Verify display_str output can be re-parsed
        let original = Note::simple(Pitch::CSharp, 4);
        let display = original.display_str();
        let parsed = Note::from_tracker_str(&display).unwrap();
        assert_eq!(parsed.pitch, original.pitch);
        assert_eq!(parsed.octave, original.octave);
    }

    #[test]
    fn test_note_display_all_pitches() {
        // Every pitch should produce a 3-character display string
        for pitch in &Pitch::ALL {
            let note = Note::simple(*pitch, 4);
            let display = note.display_str();
            assert_eq!(
                display.len(),
                3,
                "Display for {:?} was '{}' (len {})",
                pitch,
                display,
                display.len()
            );
        }
    }

    #[test]
    fn test_note_boundary_octaves() {
        let low = Note::simple(Pitch::C, 0);
        assert_eq!(low.midi_note(), 0);
        assert_eq!(low.display_str(), "C-0");

        let high = Note::simple(Pitch::B, 9);
        assert_eq!(high.midi_note(), 119);
        assert_eq!(high.display_str(), "B-9");
    }

    #[test]
    fn test_note_boundary_velocity() {
        let quiet = Note::new(Pitch::C, 4, 0, 0);
        assert_eq!(quiet.velocity, 0);

        let loud = Note::new(Pitch::C, 4, 127, 0);
        assert_eq!(loud.velocity, 127);
    }

    #[test]
    fn test_note_clone_and_eq() {
        let note = Note::new(Pitch::FSharp, 5, 80, 3);
        let cloned = note;
        assert_eq!(note, cloned);
    }

    #[test]
    fn test_note_event_equality() {
        let on1 = NoteEvent::On(Note::simple(Pitch::C, 4));
        let on2 = NoteEvent::On(Note::simple(Pitch::C, 4));
        let off = NoteEvent::Off;
        assert_eq!(on1, on2);
        assert_ne!(on1, off);
    }

    #[test]
    fn test_note_frequency_middle_c() {
        let c4 = Note::simple(Pitch::C, 4);
        // Middle C is approximately 261.63 Hz
        assert!((c4.frequency() - 261.63).abs() < 0.1);
    }

    // --- Pitch::from_semitone tests ---

    #[test]
    fn test_pitch_from_semitone_all() {
        for (i, pitch) in Pitch::ALL.iter().enumerate() {
            assert_eq!(Pitch::from_semitone(i as u8), Some(*pitch));
        }
    }

    #[test]
    fn test_pitch_from_semitone_out_of_range() {
        assert_eq!(Pitch::from_semitone(12), None);
        assert_eq!(Pitch::from_semitone(255), None);
    }

    // --- Note::transpose tests ---

    #[test]
    fn test_note_transpose_up_semitone() {
        let c4 = Note::simple(Pitch::C, 4);
        let result = c4.transpose(1).unwrap();
        assert_eq!(result.pitch, Pitch::CSharp);
        assert_eq!(result.octave, 4);
    }

    #[test]
    fn test_note_transpose_down_semitone() {
        let d4 = Note::simple(Pitch::D, 4);
        let result = d4.transpose(-1).unwrap();
        assert_eq!(result.pitch, Pitch::CSharp);
        assert_eq!(result.octave, 4);
    }

    #[test]
    fn test_note_transpose_up_octave() {
        let c4 = Note::simple(Pitch::C, 4);
        let result = c4.transpose(12).unwrap();
        assert_eq!(result.pitch, Pitch::C);
        assert_eq!(result.octave, 5);
    }

    #[test]
    fn test_note_transpose_down_octave() {
        let c4 = Note::simple(Pitch::C, 4);
        let result = c4.transpose(-12).unwrap();
        assert_eq!(result.pitch, Pitch::C);
        assert_eq!(result.octave, 3);
    }

    #[test]
    fn test_note_transpose_wraps_pitch_across_octave() {
        let b4 = Note::simple(Pitch::B, 4);
        let result = b4.transpose(1).unwrap();
        assert_eq!(result.pitch, Pitch::C);
        assert_eq!(result.octave, 5);
    }

    #[test]
    fn test_note_transpose_below_minimum_returns_none() {
        let c0 = Note::simple(Pitch::C, 0);
        assert!(c0.transpose(-1).is_none());
    }

    #[test]
    fn test_note_transpose_above_maximum_returns_none() {
        let b9 = Note::simple(Pitch::B, 9);
        assert!(b9.transpose(1).is_none());
    }

    #[test]
    fn test_note_transpose_preserves_velocity_and_instrument() {
        let note = Note::new(Pitch::C, 4, 80, 5);
        let result = note.transpose(7).unwrap();
        assert_eq!(result.velocity, 80);
        assert_eq!(result.instrument, 5);
    }

    #[test]
    fn test_note_transpose_zero_is_identity() {
        let note = Note::simple(Pitch::E, 4);
        let result = note.transpose(0).unwrap();
        assert_eq!(result.pitch, Pitch::E);
        assert_eq!(result.octave, 4);
    }
}
