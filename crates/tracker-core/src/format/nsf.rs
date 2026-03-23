//! NSF (NES Sound Format) loader using game-music-emu.
//!
//! NSF files contain 6502 machine code and music data for the NES 2A03 APU.
//! This module uses the game-music-emu library for emulation and playback.

use crate::audio::sample::Sample;
use crate::pattern::effect::EffectMode;
use crate::song::{Instrument, Song};

use super::{FormatData, ModuleLoader};

pub struct NsfLoader;

impl ModuleLoader for NsfLoader {
    fn name(&self) -> &'static str {
        "NES Sound Format"
    }

    fn extensions(&self) -> &[&str] {
        &["nsf", "nsfe"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        if data.len() < 5 {
            return false;
        }
        (&data[0..5] == b"NESM\x1A") || (&data[0..4] == b"NSFE")
    }

    fn load(&self, data: &[u8]) -> Result<FormatData, String> {
        import_nsf(data)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NsfMetadata {
    pub song_name: String,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub track_count: u8,
    pub ntsc_pal: bool,
    pub chip_flags: u8,
}

impl Default for NsfMetadata {
    fn default() -> Self {
        Self {
            song_name: String::from("Untitled"),
            artist: None,
            copyright: None,
            track_count: 1,
            ntsc_pal: true,
            chip_flags: 0,
        }
    }
}

fn parse_nsf_header(data: &[u8]) -> NsfMetadata {
    let mut meta = NsfMetadata::default();

    if data.len() < 0x80 {
        return meta;
    }

    if &data[0..5] == b"NESM\x1A" {
        // Song name: offset 0x1B (27), 32 bytes
        meta.song_name = read_nsf_string(&data[0x1B..0x1B + 32]);
        // Artist: offset 0x3F (63), 32 bytes
        meta.artist = Some(read_nsf_string(&data[0x3F..0x3F + 32]));
        // Copyright: offset 0x60 (96), 32 bytes
        meta.copyright = Some(read_nsf_string(&data[0x60..0x60 + 32]));

        // Track count is at offset 0x06
        meta.track_count = data[0x06];
    } else if &data[0..4] == b"NSFE" {
        meta.song_name = read_nsf_string(&data[4..36.min(data.len())]);
    }

    meta
}

fn read_nsf_string(slice: &[u8]) -> String {
    String::from_utf8_lossy(slice)
        .trim_end_matches('\0')
        .trim()
        .to_string()
}

pub fn import_nsf(data: &[u8]) -> Result<FormatData, String> {
    let meta = parse_nsf_header(data);

    let gme = game_music_emu::GameMusicEmu::new(game_music_emu::EmuType::Nsf, 48000);

    gme.load_data(data)
        .map_err(|e| format!("Failed to load NSF data: {:?}", e))?;

    let track_count = meta.track_count.max(1);
    let mut instrument = Instrument::new(format!("NSF Track ({})", meta.song_name));
    instrument.volume = 1.0;

    let mut song = Song::new(&meta.song_name, 60.0);
    song.global_volume = 1.0;

    instrument.nsf_data = Some(NsfInstrumentData {
        raw_data: data.to_vec(),
        track_count,
        metadata: meta.clone(),
    });

    song.instruments.push(instrument);
    song.patterns.push(crate::pattern::Pattern::new(64, 1));

    let sample = Sample::new(
        vec![0.0f32; 48000],
        48000,
        2,
        Some(format!("NSF: {}", meta.song_name)),
    );

    song.effect_mode = EffectMode::Compatible;

    Ok(FormatData {
        song,
        samples: vec![sample],
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct NsfInstrumentData {
    pub raw_data: Vec<u8>,
    pub track_count: u8,
    pub metadata: NsfMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_nsf() {
        let mut data = vec![0u8; 128];
        data[0..5].copy_from_slice(b"NESM\x1A");

        let loader = NsfLoader;
        assert!(loader.detect(&data));
    }

    #[test]
    fn test_detect_nsfe() {
        let data = b"NSFEM".to_vec();

        let loader = NsfLoader;
        assert!(loader.detect(&data));
    }

    #[test]
    fn test_detect_invalid() {
        let data = b"NOT VALID".to_vec();

        let loader = NsfLoader;
        assert!(!loader.detect(&data));
    }

    #[test]
    fn test_parse_nsf_header() {
        let mut data = vec![0u8; 256];
        data[0..5].copy_from_slice(b"NESM\x1A");
        // Total songs at offset 0x06
        data[0x06] = 5;
        // Song name at offset 0x1B, 32 bytes
        let song_name: [u8; 32] = [
            b'T', b'e', b's', b't', b' ', b'S', b'o', b'n', b'g', b' ', b'N', b'a', b'm', b'e',
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20,
        ];
        data[0x1B..0x1B + 32].copy_from_slice(&song_name);

        let meta = parse_nsf_header(&data);
        assert_eq!(meta.song_name, "Test Song Name");
        assert_eq!(meta.track_count, 5);
    }

    #[test]
    fn test_parse_nsf_short_data() {
        let data = b"short".to_vec();
        let meta = parse_nsf_header(&data);
        assert_eq!(meta.song_name, "Untitled");
    }
}
