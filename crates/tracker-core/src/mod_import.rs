//! ProTracker MOD file importer.
//!
//! Supports the classic 4-channel M.K. format and common variants with
//! 2, 6, 8 or more channels. Converts MOD patterns, instruments, and
//! sample data into tracker-core's native types.

use crate::audio::sample::{LoopMode, Sample, C4_MIDI};
use crate::pattern::effect::Effect;
use crate::pattern::note::{Note, NoteEvent, Pitch};
use crate::pattern::pattern::Pattern;
use crate::pattern::row::Cell;
use crate::song::{Instrument, Song};

/// Reference period for C-4 in our pitch system. Period 428 on a PAL
/// Amiga produces ~8287 Hz; we treat it as our middle-C (261.63 Hz)
/// so that relative intervals are preserved regardless of absolute pitch.
const C4_PERIOD: f64 = 428.0;

/// Sample rate assigned to imported MOD samples.
/// The de-facto standard used by most modern MOD players.
const MOD_SAMPLE_RATE: u32 = 8363;

/// Default note velocity for imported pattern cells.
const DEFAULT_VELOCITY: u8 = 100;

/// Result of a successful MOD import.
pub struct ModImportResult {
    /// Song structure: patterns, arrangement, instrument definitions.
    pub song: Song,
    /// Raw audio data for each instrument slot (31 entries, matching
    /// `song.instruments` index-for-index).
    pub samples: Vec<Sample>,
}

/// Import a ProTracker-compatible MOD file from raw bytes.
///
/// Returns an error string if the data is too small or structurally invalid.
pub fn import_mod(data: &[u8]) -> Result<ModImportResult, String> {
    // Minimum: 20 title + 31*30 headers + 1 length + 1 restart + 128 order + 4 tag
    if data.len() < 1084 {
        return Err(format!(
            "File too small: {} bytes (minimum 1084)",
            data.len()
        ));
    }

    let mut pos = 0usize;

    // ── Song title (20 bytes) ───────────────────────────────────────────────
    let title = read_string(data, pos, 20);
    pos += 20;

    // ── 31 sample headers (30 bytes each) ──────────────────────────────────
    let mut headers: Vec<SampleHeader> = Vec::with_capacity(31);
    for _ in 0..31 {
        headers.push(read_sample_header(data, pos)?);
        pos += 30;
    }

    // ── Song length + restart byte ─────────────────────────────────────────
    let song_length = (data[pos] as usize).min(128);
    pos += 1;
    pos += 1; // restart position — ignored

    // ── Pattern order table (128 bytes) ────────────────────────────────────
    let pattern_order = data[pos..pos + 128].to_vec();
    pos += 128;

    // ── Format tag → channel count ─────────────────────────────────────────
    let tag: [u8; 4] = data[pos..pos + 4].try_into().unwrap();
    let num_channels = detect_channels(&tag);
    pos += 4;

    // ── Pattern decode ─────────────────────────────────────────────────────
    let num_patterns = pattern_order[..song_length]
        .iter()
        .copied()
        .max()
        .unwrap_or(0) as usize
        + 1;

    let pattern_bytes = num_patterns * 64 * num_channels * 4;
    if data.len() < pos + pattern_bytes {
        return Err(format!(
            "File truncated in pattern data (need {pattern_bytes} bytes at offset {pos}, \
             have {} remaining)",
            data.len().saturating_sub(pos)
        ));
    }

    let mut patterns: Vec<Pattern> = Vec::with_capacity(num_patterns);
    for _ in 0..num_patterns {
        patterns.push(decode_pattern(data, pos, num_channels)?);
        pos += 64 * num_channels * 4;
    }

    // ── Sample data decode ─────────────────────────────────────────────────
    let mut samples: Vec<Sample> = Vec::with_capacity(31);
    for hdr in &headers {
        let byte_len = hdr.length_words * 2;
        if byte_len == 0 {
            samples.push(Sample::default());
            continue;
        }
        let available = data.len().saturating_sub(pos);
        let slice = &data[pos..pos + byte_len.min(available)];
        samples.push(decode_sample_data(slice, hdr));
        pos += byte_len.min(available);
    }

    // ── Build Song ─────────────────────────────────────────────────────────
    let arrangement: Vec<usize> = pattern_order[..song_length]
        .iter()
        .map(|&idx| idx as usize)
        .collect();

    let instruments: Vec<Instrument> = headers
        .iter()
        .enumerate()
        .map(|(i, hdr)| {
            let name = if hdr.name.is_empty() {
                format!("Sample {:02}", i + 1)
            } else {
                hdr.name.clone()
            };
            let mut inst = Instrument::new(name);
            inst.sample_index = Some(i);
            inst.volume = hdr.volume_f32();
            inst.finetune = hdr.finetune_signed();
            inst
        })
        .collect();

    let mut song = Song::new(
        if title.is_empty() {
            "Untitled".to_string()
        } else {
            title
        },
        125.0,
    );
    song.patterns = if patterns.is_empty() {
        vec![Pattern::default()]
    } else {
        patterns
    };
    song.arrangement = if arrangement.is_empty() {
        vec![0]
    } else {
        arrangement
    };
    song.instruments = instruments;

    Ok(ModImportResult { song, samples })
}

// ── Internal helpers ────────────────────────────────────────────────────────

struct SampleHeader {
    name: String,
    length_words: usize,
    finetune: u8,
    volume: u8,
    loop_start_words: usize,
    loop_length_words: usize,
}

impl SampleHeader {
    fn volume_f32(&self) -> f32 {
        self.volume.min(64) as f32 / 64.0
    }

    /// Decode ProTracker's signed 4-bit finetune nibble (-8 to +7).
    fn finetune_signed(&self) -> i8 {
        let nibble = (self.finetune & 0x0F) as i8;
        if nibble > 7 {
            nibble - 16
        } else {
            nibble
        }
    }

    fn has_loop(&self) -> bool {
        self.loop_length_words > 1
    }
}

fn read_sample_header(data: &[u8], pos: usize) -> Result<SampleHeader, String> {
    if data.len() < pos + 30 {
        return Err("File truncated in sample header".to_string());
    }
    Ok(SampleHeader {
        name: read_string(data, pos, 22),
        length_words: u16::from_be_bytes([data[pos + 22], data[pos + 23]]) as usize,
        finetune: data[pos + 24] & 0x0F,
        volume: data[pos + 25].min(64),
        loop_start_words: u16::from_be_bytes([data[pos + 26], data[pos + 27]]) as usize,
        loop_length_words: u16::from_be_bytes([data[pos + 28], data[pos + 29]]) as usize,
    })
}

fn detect_channels(tag: &[u8; 4]) -> usize {
    match tag {
        b"M.K." | b"M!K!" | b"FLT4" | b"4CHN" | b"4FLT" => 4,
        b"2CHN" => 2,
        b"6CHN" | b"6FLT" => 6,
        b"8CHN" | b"8FLT" | b"OCTA" | b"CD81" => 8,
        _ => {
            // Try "NNch" style tag (e.g. b"10ch", b"16ch")
            if tag[2] == b'c' && tag[3] == b'h' && tag[0].is_ascii_digit() {
                let n = if tag[1].is_ascii_digit() {
                    (tag[0] - b'0') * 10 + (tag[1] - b'0')
                } else {
                    tag[0] - b'0'
                } as usize;
                if (2..=32).contains(&n) {
                    return n;
                }
            }
            4
        }
    }
}

fn decode_pattern(data: &[u8], pos: usize, num_channels: usize) -> Result<Pattern, String> {
    let mut pattern = Pattern::new(64, num_channels);
    let mut offset = pos;

    for row in 0..64 {
        for ch in 0..num_channels {
            if offset + 4 > data.len() {
                return Err(format!("Pattern truncated at row {row} ch {ch}"));
            }
            let b = &data[offset..offset + 4];
            offset += 4;

            // ProTracker cell encoding:
            //   [iiiipppp pppppppp iiiieeee xxxxxxxx]
            //   i = instrument nibbles, p = period, e = effect, x = param
            let period = ((b[0] & 0x0F) as u16) << 8 | b[1] as u16;
            let instrument = (b[0] & 0xF0) | ((b[2] >> 4) & 0x0F);
            let effect_cmd = b[2] & 0x0F;
            let effect_param = b[3];

            let note_event = if period > 0 {
                period_to_note(period).map(|(pitch, octave)| {
                    NoteEvent::On(Note::new(
                        pitch,
                        octave,
                        DEFAULT_VELOCITY,
                        instrument.saturating_sub(1),
                    ))
                })
            } else {
                None
            };

            let cell = Cell {
                note: note_event,
                instrument: (instrument > 0).then_some(instrument.saturating_sub(1)),
                volume: None,
                effects: decode_effect(effect_cmd, effect_param)
                    .into_iter()
                    .collect(),
            };
            pattern.set_cell(row, ch, cell);
        }
    }
    Ok(pattern)
}

/// Convert a ProTracker period value to (Pitch, octave) in our system.
///
/// Period 428 → C-4 (our middle C). Each halving of the period = one octave up.
fn period_to_note(period: u16) -> Option<(Pitch, u8)> {
    if period == 0 {
        return None;
    }
    // semitones relative to C-4 (MIDI 48 in our numbering)
    let semitones = (12.0 * (C4_PERIOD / period as f64).log2()).round() as i32;
    let midi = 48i32 + semitones;
    if !(0..=119).contains(&midi) {
        return None;
    }
    let pitch = Pitch::from_semitone((midi % 12) as u8)?;
    let octave = (midi / 12) as u8;
    Some((pitch, octave))
}

/// Convert a MOD effect command and parameter to our Effect, if non-trivial.
fn decode_effect(cmd: u8, param: u8) -> Option<Effect> {
    // Arpeggio with param 0 is a no-op in ProTracker
    if cmd == 0 && param == 0 {
        return None;
    }
    Some(Effect::new(cmd, param))
}

/// Decode raw 8-bit signed MOD sample data into a normalised f32 Sample.
fn decode_sample_data(raw: &[u8], hdr: &SampleHeader) -> Sample {
    let data: Vec<f32> = raw.iter().map(|&b| b as i8 as f32 / 128.0).collect();
    let frame_count = data.len();

    let mut sample =
        Sample::new(data, MOD_SAMPLE_RATE, 1, Some(hdr.name.clone())).with_base_note(C4_MIDI);

    if hdr.has_loop() {
        let loop_start = hdr.loop_start_words * 2;
        let loop_end = ((hdr.loop_start_words + hdr.loop_length_words) * 2)
            .min(frame_count)
            .saturating_sub(1);
        if loop_start < loop_end {
            sample = sample.with_loop(LoopMode::Forward, loop_start, loop_end);
        }
    }
    sample
}

/// Read a null-terminated ASCII string from `data[pos..pos+len]`.
fn read_string(data: &[u8], pos: usize, len: usize) -> String {
    let end = (pos + len).min(data.len());
    let bytes = &data[pos..end];
    let trimmed: String = bytes
        .iter()
        .copied()
        .take_while(|&b| b != 0)
        .filter(|b| b.is_ascii_graphic() || *b == b' ')
        .map(|b| b as char)
        .collect();
    trimmed.trim().to_string()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_to_note_c4() {
        let (pitch, octave) = period_to_note(428).unwrap();
        assert_eq!(pitch, Pitch::C);
        assert_eq!(octave, 4);
    }

    #[test]
    fn test_period_to_note_c5_one_octave_up() {
        let (pitch, octave) = period_to_note(214).unwrap();
        assert_eq!(pitch, Pitch::C);
        assert_eq!(octave, 5);
    }

    #[test]
    fn test_period_to_note_c3_one_octave_down() {
        let (pitch, octave) = period_to_note(856).unwrap();
        assert_eq!(pitch, Pitch::C);
        assert_eq!(octave, 3);
    }

    #[test]
    fn test_period_to_note_a4() {
        // A-4 is 9 semitones above C-4. Period = 428 / 2^(9/12) ≈ 255
        let (pitch, octave) = period_to_note(254).unwrap();
        assert_eq!(pitch, Pitch::A);
        assert_eq!(octave, 4);
    }

    #[test]
    fn test_period_to_note_zero_returns_none() {
        assert!(period_to_note(0).is_none());
    }

    #[test]
    fn test_period_to_note_out_of_range() {
        // Very low period (very high pitch) should return None
        assert!(period_to_note(1).is_none());
    }

    #[test]
    fn test_detect_channels_mk() {
        assert_eq!(detect_channels(b"M.K."), 4);
    }

    #[test]
    fn test_detect_channels_8chn() {
        assert_eq!(detect_channels(b"8CHN"), 8);
    }

    #[test]
    fn test_detect_channels_octa() {
        assert_eq!(detect_channels(b"OCTA"), 8);
    }

    #[test]
    fn test_detect_channels_nch_style() {
        assert_eq!(detect_channels(b"12ch"), 12);
    }

    #[test]
    fn test_detect_channels_unknown_defaults_to_4() {
        assert_eq!(detect_channels(b"????"), 4);
    }

    #[test]
    fn test_finetune_positive() {
        let hdr = SampleHeader {
            name: String::new(),
            length_words: 0,
            finetune: 3,
            volume: 64,
            loop_start_words: 0,
            loop_length_words: 0,
        };
        assert_eq!(hdr.finetune_signed(), 3);
    }

    #[test]
    fn test_finetune_negative() {
        // nibble 0xF = 15 → 15 - 16 = -1
        let hdr = SampleHeader {
            name: String::new(),
            length_words: 0,
            finetune: 0xF,
            volume: 64,
            loop_start_words: 0,
            loop_length_words: 0,
        };
        assert_eq!(hdr.finetune_signed(), -1);
    }

    #[test]
    fn test_volume_f32() {
        let hdr = SampleHeader {
            name: String::new(),
            length_words: 0,
            finetune: 0,
            volume: 32,
            loop_start_words: 0,
            loop_length_words: 0,
        };
        assert!((hdr.volume_f32() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_import_mod_too_small() {
        assert!(import_mod(&[0u8; 100]).is_err());
    }

    #[test]
    fn test_import_mod_minimal_valid() {
        // Build a minimal valid MOD: 1 pattern, 4 channels, no samples
        let mut data = vec![0u8; 1084];

        // Title (20 bytes, already zero)
        // 31 sample headers (30 bytes each, all zero = empty samples)
        // song_length = 1
        data[1080] = 1;
        // restart = 0
        // pattern_order[0] = 0 (use pattern 0)
        // tag = "M.K."
        data[1080] = 1;
        data[1082] = 0; // pattern order [0] = 0
        let tag_pos = 20 + 31 * 30 + 2 + 128;
        data[tag_pos] = b'M';
        data[tag_pos + 1] = b'.';
        data[tag_pos + 2] = b'K';
        data[tag_pos + 3] = b'.';

        // Pattern data: 64 rows * 4 channels * 4 bytes = 1024 bytes (all zero)
        let total = 1084 + 1024;
        data.resize(total, 0);

        let result = import_mod(&data);
        assert!(
            result.is_ok(),
            "minimal MOD import failed: {:?}",
            result.err()
        );
        let r = result.unwrap();
        assert_eq!(r.song.patterns.len(), 1);
        assert_eq!(r.song.patterns[0].num_channels(), 4);
        assert_eq!(r.song.patterns[0].num_rows(), 64);
        assert_eq!(r.song.instruments.len(), 31);
        assert_eq!(r.samples.len(), 31);
    }

    #[test]
    fn test_decode_effect_null_arpeggio_is_none() {
        assert!(decode_effect(0, 0).is_none());
    }

    #[test]
    fn test_decode_effect_volume_slide() {
        let eff = decode_effect(0xA, 0x30).unwrap();
        assert_eq!(
            eff.effect_type(),
            Some(crate::pattern::effect::EffectType::VolumeSlide)
        );
        assert_eq!(eff.param, 0x30);
    }

    #[test]
    fn test_read_string_trims_nulls() {
        let data = b"hello\0\0\0world";
        assert_eq!(read_string(data, 0, 13), "hello");
    }
}
