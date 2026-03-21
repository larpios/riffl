/// Project save/load functionality for the tracker.
///
/// Serializes and deserializes the Song data model to/from JSON files
/// with the `.trs` extension. Sample audio data is not embedded; only
/// file path references are stored.
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::song::Song;

/// Save a project (Song) to a JSON file.
///
/// The song is serialized to pretty-printed JSON and written to the specified path.
/// By convention, riffl project files use the `.trs` extension.
pub fn save_project(path: &Path, song: &Song) -> Result<()> {
    let json = serde_json::to_string_pretty(song).context("Failed to serialize project")?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write project file: {}", path.display()))?;
    Ok(())
}

/// Load a project (Song) from a JSON file.
///
/// Reads the specified file and deserializes it into a Song struct.
/// Returns an error if the file cannot be read or contains invalid data.
pub fn load_project(path: &Path) -> Result<Song> {
    let json = fs::read_to_string(path)
        .with_context(|| format!("Failed to read project file: {}", path.display()))?;
    let song: Song = serde_json::from_str(&json).context("Failed to deserialize project data")?;
    Ok(song)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Cell, Effect, Note, NoteEvent, Pattern, Pitch};
    use crate::song::Instrument;
    use std::io::Write;

    #[test]
    fn test_save_load_roundtrip_default_song() {
        let song = Song::default();
        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip_default.trs");

        save_project(&path, &song).unwrap();
        let loaded = load_project(&path).unwrap();

        assert_eq!(song.name, loaded.name);
        assert_eq!(song.artist, loaded.artist);
        assert_eq!(song.bpm, loaded.bpm);
        assert_eq!(song.patterns.len(), loaded.patterns.len());
        assert_eq!(song.arrangement, loaded.arrangement);
        assert_eq!(song.tracks.len(), loaded.tracks.len());
        assert_eq!(song.instruments.len(), loaded.instruments.len());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_load_roundtrip_with_notes() {
        let mut song = Song::new("Test Song", 140.0);
        song.artist = "Test Artist".to_string();

        // Add notes to the first pattern
        let note = Note::new(Pitch::CSharp, 4, 100, 1);
        song.patterns[0].set_cell(
            0,
            0,
            Cell {
                note: Some(NoteEvent::On(note)),
                instrument: Some(1),
                volume: Some(0x40),
                effects: vec![Effect::new(0xC, 0x20)],
            },
        );
        song.patterns[0].set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        // Add a second pattern with different data
        let mut pattern2 = Pattern::new(32, 8);
        pattern2.set_note(0, 0, Note::simple(Pitch::A, 5));
        pattern2.set_note(4, 1, Note::new(Pitch::FSharp, 3, 80, 2));
        song.add_pattern(pattern2);

        // Set up arrangement
        song.arrangement = vec![0, 1, 0];

        // Add instruments
        let mut inst = Instrument::new("Kick Drum");
        inst.sample_index = Some(0);
        inst.volume = 0.8;
        song.instruments.push(inst);

        let mut inst2 = Instrument::new("Synth Lead");
        inst2.sample_index = Some(1);
        inst2.base_note = Note::simple(Pitch::A, 4);
        song.instruments.push(inst2);

        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip_notes.trs");

        save_project(&path, &song).unwrap();
        let loaded = load_project(&path).unwrap();

        // Verify full equality
        assert_eq!(song, loaded);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_load_roundtrip_all_note_data_survives() {
        let mut song = Song::new("Data Integrity", 120.0);

        // Test various note types
        let notes_to_test = [
            Note::new(Pitch::C, 0, 0, 0),        // minimum values
            Note::new(Pitch::B, 9, 127, 255),    // maximum values
            Note::new(Pitch::FSharp, 4, 64, 42), // mid-range values
        ];

        for (i, note) in notes_to_test.iter().enumerate() {
            song.patterns[0].set_note(i, 0, *note);
        }

        // Test note-off
        song.patterns[0].set_cell(3, 0, Cell::with_note(NoteEvent::Off));

        // Test cell with only effect
        song.patterns[0].set_cell(
            4,
            0,
            Cell {
                note: None,
                instrument: None,
                volume: None,
                effects: vec![Effect::new(0xF, 0xFF)],
            },
        );

        // Test cell with only volume
        song.patterns[0].set_cell(
            5,
            0,
            Cell {
                note: None,
                instrument: None,
                volume: Some(0x7F),
                effects: Vec::new(),
            },
        );

        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip_data_integrity.trs");

        save_project(&path, &song).unwrap();
        let loaded = load_project(&path).unwrap();

        assert_eq!(song, loaded);

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_missing_file() {
        let path = Path::new("/tmp/nonexistent_file_12345.trs");
        let result = load_project(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_corrupt_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_corrupt.trs");

        // Write invalid JSON
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{ this is not valid json }").unwrap();

        let result = load_project(&path);
        assert!(result.is_err());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_empty_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty.trs");

        fs::write(&path, "").unwrap();

        let result = load_project(&path);
        assert!(result.is_err());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_load_incomplete_json() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_incomplete.trs");

        // Write JSON that's valid but missing required fields
        fs::write(&path, r#"{"name": "Test"}"#).unwrap();

        let result = load_project(&path);
        assert!(result.is_err());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_creates_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_save_creates.trs");

        // Ensure file doesn't exist
        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        let song = Song::default();
        save_project(&path, &song).unwrap();

        assert!(path.exists());

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_save_overwrites_existing() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_save_overwrites.trs");

        let song1 = Song::new("First", 120.0);
        save_project(&path, &song1).unwrap();

        let mut song2 = Song::new("Second", 140.0);
        song2.artist = "Artist 2".to_string();
        save_project(&path, &song2).unwrap();

        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.name, "Second");
        assert_eq!(loaded.bpm, 140.0);
        assert_eq!(loaded.artist, "Artist 2");

        // Clean up
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_roundtrip_track_metadata() {
        let mut song = Song::default();

        // Modify track metadata
        if let Some(track) = song.patterns[0].get_track_mut(0) {
            track.name = "Kick".to_string();
            track.set_volume(0.8);
            track.set_pan(-0.5);
            track.muted = true;
            track.instrument_index = Some(0);
        }
        if let Some(track) = song.patterns[0].get_track_mut(1) {
            track.name = "Bass".to_string();
            track.solo = true;
        }

        let dir = std::env::temp_dir();
        let path = dir.join("test_roundtrip_tracks.trs");

        save_project(&path, &song).unwrap();
        let loaded = load_project(&path).unwrap();

        assert_eq!(song, loaded);

        // Clean up
        let _ = fs::remove_file(&path);
    }
}
