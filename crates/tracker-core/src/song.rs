/// Song and arrangement data model for the tracker.
///
/// A Song is the top-level container that holds a pool of patterns,
/// an arrangement (ordered sequence of pattern indices), global track
/// metadata, and instrument definitions.
use serde::{Deserialize, Serialize};

use crate::pattern::{Note, Pattern, Pitch, Track};

/// An instrument definition linking a name to a sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    /// Display name for this instrument.
    pub name: String,
    /// Index into the sample pool, if a sample is assigned.
    pub sample_index: Option<usize>,
    /// File path to the audio sample, relative to the project or absolute.
    pub sample_path: Option<String>,
    /// Base note for sample pitch mapping (default C-4).
    pub base_note: Note,
    /// Volume multiplier for this instrument (0.0 to 1.0).
    pub volume: f32,
    /// Finetune adjustment (-8 to +7).
    /// Each unit is 1/8th of a semitone (12.5 cents).
    pub finetune: i8,
}

impl Instrument {
    /// Create a new instrument with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sample_index: None,
            sample_path: None,
            base_note: Note::simple(Pitch::C, 4),
            volume: 1.0,
            finetune: 0,
        }
    }

    /// Set the finetune value for the instrument.
    pub fn with_finetune(mut self, finetune: i8) -> Self {
        self.finetune = finetune.clamp(-8, 7);
        self
    }

    /// Set the default volume for the instrument (0.0 to 1.0).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

impl Default for Instrument {
    fn default() -> Self {
        Self::new("Instrument")
    }
}

/// A song containing patterns, arrangement, tracks, and instruments.
///
/// The song is the top-level data structure for a tracker project.
/// Patterns are stored in a pool (up to 256) and referenced by index
/// in the arrangement sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// Song title.
    pub name: String,
    /// Artist name.
    pub artist: String,
    /// Tempo in beats per minute.
    pub bpm: f64,
    /// Lines per beat.
    pub lpb: u32,
    /// Ticks per row (line), defines sub-row resolution.
    pub tpl: u32,
    /// Pattern pool (up to 256 patterns).
    pub patterns: Vec<Pattern>,
    /// Arrangement: ordered list of pattern indices forming the song sequence.
    pub arrangement: Vec<usize>,
    /// Global track metadata (volume, pan, mute, solo, instrument).
    pub tracks: Vec<Track>,
    /// Instrument definitions linking to samples.
    pub instruments: Vec<Instrument>,
}

/// Maximum number of patterns in the pool.
pub const MAX_PATTERNS: usize = 256;

impl Song {
    /// Create a new empty song with one default pattern.
    pub fn new(name: impl Into<String>, bpm: f64) -> Self {
        let default_pattern = Pattern::default();
        let tracks: Vec<Track> = (1..=default_pattern.num_channels())
            .map(Track::with_number)
            .collect();
        Self {
            name: name.into(),
            artist: String::new(),
            bpm,
            lpb: 4,
            tpl: 6,
            patterns: vec![default_pattern],
            arrangement: vec![0],
            tracks,
            instruments: Vec::new(),
        }
    }

    /// Add a new pattern to the pool.
    ///
    /// Returns the index of the new pattern, or None if the pool is full (256 patterns).
    pub fn add_pattern(&mut self, pattern: Pattern) -> Option<usize> {
        if self.patterns.len() >= MAX_PATTERNS {
            return None;
        }
        let index = self.patterns.len();
        self.patterns.push(pattern);
        Some(index)
    }

    /// Remove a pattern from the pool by index.
    ///
    /// Updates the arrangement to remove references to the deleted pattern
    /// and adjusts indices of patterns that come after it.
    /// Returns false if the index is out of bounds or it's the last pattern.
    pub fn remove_pattern(&mut self, index: usize) -> bool {
        if index >= self.patterns.len() || self.patterns.len() <= 1 {
            return false;
        }
        self.patterns.remove(index);

        // Remove arrangement entries that referenced the deleted pattern
        // and adjust indices for patterns after the removed one
        self.arrangement.retain(|&i| i != index);
        for entry in &mut self.arrangement {
            if *entry > index {
                *entry -= 1;
            }
        }

        // If arrangement is now empty, point to pattern 0
        if self.arrangement.is_empty() {
            self.arrangement.push(0);
        }

        true
    }

    /// Duplicate a pattern and add the copy to the pool.
    ///
    /// Returns the index of the new pattern, or None if the pool is full
    /// or the source index is out of bounds.
    pub fn duplicate_pattern(&mut self, index: usize) -> Option<usize> {
        if index >= self.patterns.len() || self.patterns.len() >= MAX_PATTERNS {
            return None;
        }
        let cloned = self.patterns[index].clone();
        self.add_pattern(cloned)
    }

    /// Reorder an arrangement entry from one position to another.
    ///
    /// Returns false if either position is out of bounds.
    pub fn reorder_arrangement(&mut self, from: usize, to: usize) -> bool {
        if from >= self.arrangement.len() || to >= self.arrangement.len() {
            return false;
        }
        let entry = self.arrangement.remove(from);
        self.arrangement.insert(to, entry);
        true
    }

    /// Insert a pattern reference into the arrangement at the given position.
    ///
    /// Returns false if the pattern_index is out of bounds or position > arrangement length.
    pub fn insert_in_arrangement(&mut self, position: usize, pattern_index: usize) -> bool {
        if pattern_index >= self.patterns.len() || position > self.arrangement.len() {
            return false;
        }
        self.arrangement.insert(position, pattern_index);
        true
    }

    /// Remove an entry from the arrangement at the given position.
    ///
    /// Returns the pattern index that was removed, or None if out of bounds.
    pub fn remove_from_arrangement(&mut self, position: usize) -> Option<usize> {
        if position >= self.arrangement.len() {
            return None;
        }
        Some(self.arrangement.remove(position))
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new("Untitled", 120.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_new() {
        let song = Song::new("Test Song", 140.0);
        assert_eq!(song.name, "Test Song");
        assert_eq!(song.bpm, 140.0);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.arrangement, vec![0]);
        assert!(!song.tracks.is_empty());
    }

    #[test]
    fn test_song_default() {
        let song = Song::default();
        assert_eq!(song.name, "Untitled");
        assert_eq!(song.bpm, 120.0);
    }

    #[test]
    fn test_instrument_new() {
        let inst = Instrument::new("Kick");
        assert_eq!(inst.name, "Kick");
        assert_eq!(inst.sample_index, None);
        assert_eq!(inst.base_note.pitch, Pitch::C);
        assert_eq!(inst.base_note.octave, 4);
        assert_eq!(inst.volume, 1.0);
    }

    #[test]
    fn test_instrument_default() {
        let inst = Instrument::default();
        assert_eq!(inst.name, "Instrument");
    }

    #[test]
    fn test_add_pattern() {
        let mut song = Song::default();
        assert_eq!(song.patterns.len(), 1);

        let idx = song.add_pattern(Pattern::default());
        assert_eq!(idx, Some(1));
        assert_eq!(song.patterns.len(), 2);
    }

    #[test]
    fn test_add_pattern_pool_full() {
        let mut song = Song::default();
        // Fill up to 256
        for _ in 1..MAX_PATTERNS {
            song.add_pattern(Pattern::default());
        }
        assert_eq!(song.patterns.len(), MAX_PATTERNS);
        assert_eq!(song.add_pattern(Pattern::default()), None);
    }

    #[test]
    fn test_remove_pattern() {
        let mut song = Song::default();
        song.add_pattern(Pattern::new(32, 4));
        song.add_pattern(Pattern::new(16, 4));
        assert_eq!(song.patterns.len(), 3);

        // Arrangement references pattern 0, 1, 2
        song.arrangement = vec![0, 1, 2, 1];

        // Remove pattern 1
        assert!(song.remove_pattern(1));
        assert_eq!(song.patterns.len(), 2);

        // Arrangement should have removed refs to 1 and decremented refs > 1
        assert_eq!(song.arrangement, vec![0, 1]);
    }

    #[test]
    fn test_remove_pattern_last_remaining() {
        let mut song = Song::default();
        // Cannot remove the last pattern
        assert!(!song.remove_pattern(0));
        assert_eq!(song.patterns.len(), 1);
    }

    #[test]
    fn test_remove_pattern_out_of_bounds() {
        let mut song = Song::default();
        assert!(!song.remove_pattern(5));
    }

    #[test]
    fn test_remove_pattern_arrangement_becomes_empty() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        // Arrangement only references pattern 0
        song.arrangement = vec![0];

        // Remove pattern 0 → arrangement loses its only entry → should default to [0]
        assert!(song.remove_pattern(0));
        assert_eq!(song.arrangement, vec![0]);
    }

    #[test]
    fn test_duplicate_pattern() {
        let mut song = Song::default();
        let idx = song.duplicate_pattern(0);
        assert_eq!(idx, Some(1));
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.patterns[1].num_rows(), song.patterns[0].num_rows());
        assert_eq!(
            song.patterns[1].num_channels(),
            song.patterns[0].num_channels()
        );
    }

    #[test]
    fn test_duplicate_pattern_out_of_bounds() {
        let mut song = Song::default();
        assert_eq!(song.duplicate_pattern(5), None);
    }

    #[test]
    fn test_reorder_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0, 1, 2];

        assert!(song.reorder_arrangement(0, 2));
        assert_eq!(song.arrangement, vec![1, 2, 0]);
    }

    #[test]
    fn test_reorder_arrangement_out_of_bounds() {
        let mut song = Song::default();
        song.arrangement = vec![0];
        assert!(!song.reorder_arrangement(0, 5));
        assert!(!song.reorder_arrangement(5, 0));
    }

    #[test]
    fn test_insert_in_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0];

        assert!(song.insert_in_arrangement(1, 1));
        assert_eq!(song.arrangement, vec![0, 1]);

        assert!(song.insert_in_arrangement(0, 1));
        assert_eq!(song.arrangement, vec![1, 0, 1]);
    }

    #[test]
    fn test_insert_in_arrangement_invalid_pattern() {
        let mut song = Song::default();
        assert!(!song.insert_in_arrangement(0, 5)); // pattern 5 doesn't exist
    }

    #[test]
    fn test_insert_in_arrangement_invalid_position() {
        let mut song = Song::default();
        assert!(!song.insert_in_arrangement(10, 0)); // position beyond length
    }

    #[test]
    fn test_remove_from_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0, 1, 0];

        assert_eq!(song.remove_from_arrangement(1), Some(1));
        assert_eq!(song.arrangement, vec![0, 0]);
    }

    #[test]
    fn test_remove_from_arrangement_out_of_bounds() {
        let mut song = Song::default();
        assert_eq!(song.remove_from_arrangement(5), None);
    }

    #[test]
    fn test_instrument_assignment() {
        let mut song = Song::default();
        let inst = Instrument::new("Bass");
        song.instruments.push(inst);

        assert_eq!(song.instruments.len(), 1);
        assert_eq!(song.instruments[0].name, "Bass");

        song.instruments[0].sample_index = Some(0);
        assert_eq!(song.instruments[0].sample_index, Some(0));
    }

    #[test]
    fn test_song_tracks_match_pattern_channels() {
        let song = Song::default();
        assert_eq!(song.tracks.len(), song.patterns[0].num_channels());
    }
}
