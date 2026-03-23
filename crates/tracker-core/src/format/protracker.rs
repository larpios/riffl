//! ProTracker MOD file importer.
//!
//! Supports the classic 4-channel M.K. format and common variants with
//! 2, 6, 8 or more channels. Converts MOD patterns, instruments, and
//! sample data into tracker-core's native types.

use super::{FormatData, ModuleLoader};
use crate::audio::sample::{LoopMode, Sample, C4_MIDI};
use crate::pattern::effect::{Effect, EffectMode};
use crate::pattern::note::{Note, NoteEvent, Pitch};
use crate::pattern::pattern::Pattern;
use crate::pattern::{Cell, Track};
use crate::song::{Instrument, Song};

pub struct ModLoader;

impl ModuleLoader for ModLoader {
    fn name(&self) -> &'static str {
        "Protracker"
    }

    fn extensions(&self) -> &[&str] {
        &["mod"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        if data.len() < 1084 {
            return false;
        }
        let tag = &data[1080..1084];
        match tag {
            b"M.K." | b"M!K!" | b"4CHN" | b"FLT4" | b"FLT8" | b"6CHN" | b"8CHN" | b"CD81"
            | b"OCTA" => true,
            _ => {
                tag[2] == b'C'
                    && tag[3] == b'H'
                    && tag[0].is_ascii_digit()
                    && tag[1].is_ascii_digit()
            }
        }
    }

    fn load(&self, data: &[u8]) -> Result<FormatData, String> {
        import_mod(data)
    }
}

/// The ProTracker period value for a C-2 note (which we map to our C4_MIDI base note).
/// Period 428 is the standard PAL Amiga period for C-2.
const C4_PERIOD: f64 = 428.0;

/// Sample rate assigned to imported MOD samples.
/// The baseline sample rate for a C-2 note on a PAL Amiga clock.
/// Period 428 maps to 8287.14 Hz. Using 8287 aligns pitches perfectly with our C4_MIDI base playback.
const MOD_SAMPLE_RATE: u32 = 8287;

/// Default note velocity for imported pattern cells.
const DEFAULT_VELOCITY: u8 = 100;

/// Default BPM when no tempo info is found in the MOD file.
const _DEFAULT_BPM: f64 = 125.0;

/// Extract tempo information from MOD patterns by scanning for SetSpeed effects.
/// Returns (speed, bpm) indicating the first Speed (TPL) and BPM found, defaulting to 6 and 125.0.
fn extract_tempo_from_patterns(patterns: &[Pattern]) -> (u8, f64) {
    use crate::pattern::effect::EffectType;
    let mut out_speed = 6u8;
    let mut out_bpm = 125.0;

    // We just want to find the first occurrence of each to set the initial Song tempo.
    let mut found_speed = false;
    let mut found_bpm = false;

    for pattern in patterns {
        for row_idx in 0..pattern.num_rows() {
            if let Some(row) = pattern.get_row(row_idx) {
                for cell in row.iter() {
                    if let Some(effect) = cell.effects.first() {
                        if effect.effect_type() == Some(EffectType::SetSpeed) {
                            if !found_bpm && effect.param >= 32 {
                                out_bpm = effect.param as f64;
                                found_bpm = true;
                            } else if !found_speed && effect.param > 0 && effect.param < 32 {
                                out_speed = effect.param;
                                found_speed = true;
                            }
                        }
                    }
                }
            }
            if found_speed && found_bpm {
                return (out_speed, out_bpm);
            }
        }
    }

    (out_speed, out_bpm)
}

/// Export a Song and sample data to ProTracker-compatible MOD format bytes.
///
/// The `samples` vec must have exactly 31 entries (one per instrument slot).
/// Returns an error if the song has unsupported channel counts or data shapes.
pub fn export_mod(song: &Song, samples: &[Sample]) -> Result<Vec<u8>, String> {
    if samples.len() != 31 {
        return Err(format!(
            "export_mod requires exactly 31 samples (got {})",
            samples.len()
        ));
    }

    let num_channels = song.patterns.first().map(|p| p.num_channels()).unwrap_or(4);
    if !(2..=32).contains(&num_channels) {
        return Err(format!(
            "export_mod requires 2-32 channels (got {})",
            num_channels
        ));
    }

    let tag = match num_channels {
        4 => *b"M.K.",
        2 => *b"2CHN",
        6 => *b"6CHN",
        8 => *b"8CHN",
        n => {
            let mut t = [b' ', b' ', b'c', b'h'];
            if n >= 10 {
                t[0] = (n / 10) as u8 + b'0';
                t[1] = (n % 10) as u8 + b'0';
            } else {
                t[0] = n as u8 + b'0';
                t[1] = b' ';
            }
            t
        }
    };

    let max_pattern_idx = song.arrangement.iter().copied().max().unwrap_or(0);
    let total_patterns = (max_pattern_idx + 1).max(1);

    let mut buf = Vec::with_capacity(1084 + total_patterns * 64 * num_channels * 4);

    // ── Song title (20 bytes) ───────────────────────────────────────────────
    let title_bytes: [u8; 20] = {
        let mut t = [b' '; 20];
        let bytes = song.name.as_bytes();
        let len = bytes.len().min(20);
        t[..len].copy_from_slice(&bytes[..len]);
        t
    };
    buf.extend_from_slice(&title_bytes);

    // ── 31 sample headers (30 bytes each) ──────────────────────────────────
    for (i, sample) in samples.iter().enumerate() {
        let inst_name = song
            .instruments
            .get(i)
            .map(|inst| inst.name.as_str())
            .unwrap_or("");

        let inst_finetune = song
            .instruments
            .get(i)
            .map(|inst| inst.finetune.clamp(-8, 7))
            .unwrap_or(0);

        let inst_volume = song
            .instruments
            .get(i)
            .map(|inst| (inst.volume.clamp(0.0, 1.0) * 64.0).round() as u8)
            .unwrap_or(64)
            .min(64);

        let name_bytes: [u8; 22] = {
            let mut n = [0u8; 22];
            let bytes = inst_name.as_bytes();
            let len = bytes.len().min(22);
            n[..len].copy_from_slice(&bytes[..len]);
            n
        };
        buf.extend_from_slice(&name_bytes);

        let frame_count = sample.frame_count();
        let byte_len = frame_count.saturating_sub(sample.header_size() as usize);
        let word_len = byte_len / 2;

        buf.extend_from_slice(&u16::to_be_bytes(word_len as u16));
        buf.push((inst_finetune & 0x0F) as u8);
        buf.push(inst_volume.min(64));
        let (loop_start, loop_end) = if let Some((mode, s, e)) = sample.loop_info() {
            if matches!(mode, LoopMode::Forward | LoopMode::PingPong) {
                (s as u32, e as u32)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };
        buf.extend_from_slice(&u16::to_be_bytes((loop_start / 2) as u16));
        let loop_len = if loop_end > loop_start {
            ((loop_end - loop_start) / 2) as u16
        } else {
            0u16
        };
        buf.extend_from_slice(&u16::to_be_bytes(loop_len));
    }

    // ── Song length + restart byte ─────────────────────────────────────────
    let song_length = song.arrangement.len().min(128) as u8;
    buf.push(song_length);
    buf.push(0u8);

    // ── Pattern order table (128 bytes) ────────────────────────────────────
    for &idx in song.arrangement.iter().take(128) {
        buf.push(idx as u8);
    }
    let pad_count = 128usize.saturating_sub(song.arrangement.len());
    buf.resize(buf.len() + pad_count, 0u8);

    // ── Format tag (4 bytes) ───────────────────────────────────────────────
    buf.extend_from_slice(&tag);

    // ── Pattern data ───────────────────────────────────────────────────────
    for pat_idx in 0..total_patterns {
        if let Some(pat) = song.patterns.get(pat_idx) {
            for row in 0..64.min(pat.num_rows()) {
                for ch in 0..num_channels {
                    let (period, instrument, effect_cmd, effect_param) =
                        if let Some(cell) = pat.get_cell(row, ch) {
                            let inst_nibble = cell.instrument.map(|i| i + 1).unwrap_or(0);

                            let (period, inst) = if let Some(ref note_event) = cell.note {
                                match note_event {
                                    NoteEvent::On(note) => {
                                        let p = note_to_period(note.pitch, note.octave);
                                        (p, inst_nibble)
                                    }
                                    NoteEvent::Off | NoteEvent::Cut => (0, inst_nibble),
                                }
                            } else {
                                (0, inst_nibble)
                            };

                            let (eff_cmd, eff_param) = if let Some(eff) = cell.effects.first() {
                                (
                                    eff.effect_type().map(|et| et.protracker_cmd()).unwrap_or(0),
                                    eff.param,
                                )
                            } else {
                                (0, 0)
                            };

                            (period, inst, eff_cmd, eff_param)
                        } else {
                            (0, 0, 0, 0)
                        };

                    let inst_hi = (instrument & 0xF0) >> 4;
                    let inst_lo = instrument & 0x0F;
                    let byte0 = (((period >> 8) & 0x0F) as u8) | inst_hi;
                    buf.push(byte0);
                    buf.push(period as u8);
                    buf.push((inst_lo << 4) | effect_cmd);
                    buf.push(effect_param);
                }
            }
            // Pad empty rows if pattern is shorter than 64
            for _row in pat.num_rows()..64 {
                for _ in 0..num_channels {
                    buf.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        } else {
            for _ in 0..64 {
                for _ in 0..num_channels {
                    buf.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }
    }

    // ── Sample data ────────────────────────────────────────────────────────
    for sample in samples {
        let frame_count = sample.frame_count();
        if frame_count <= sample.header_size() as usize {
            continue;
        }
        let data_start = sample.header_size() as usize;
        let data = &sample.data_ref()[data_start..];
        let byte_len = data.len();
        let encoded: Vec<u8> = data
            .iter()
            .map(|&s| {
                let clamped = s.clamp(-1.0, 1.0);
                (clamped * 127.0).round() as i8 as u8
            })
            .collect();
        buf.extend_from_slice(&encoded[..byte_len.min(encoded.len())]);
    }

    Ok(buf)
}

/// Import a ProTracker-compatible MOD file from raw bytes.
///
/// Returns an error string if the data is too small or structurally invalid.
pub fn import_mod(data: &[u8]) -> Result<super::FormatData, String> {
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

    let (speed, bpm) = extract_tempo_from_patterns(&patterns);

    let mut song = Song::new(
        if title.is_empty() {
            "Untitled".to_string()
        } else {
            title
        },
        bpm,
    );

    // Initialize tracks with Amiga panning
    // Amiga hardware: Ch 0(L), 1(R), 2(R), 3(L)
    // Stereo separation is roughly 80/127 (around 63%), matching Furnace's default
    let mut tracks = Vec::with_capacity(num_channels);
    for i in 0..num_channels {
        let mut t = Track::with_number(i + 1);
        let pan = match i % 4 {
            0 | 3 => -0.63, // Left
            1 | 2 => 0.63,  // Right
            _ => 0.0,
        };
        t.pan = pan;
        tracks.push(t);
    }
    song.tracks = tracks;

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
    song.tpl = speed as u32;

    song.effect_mode = EffectMode::Compatible;

    Ok(super::FormatData { song, samples })
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
    let semitones = (12.0 * (C4_PERIOD / period as f64).log2()).round() as i32;
    let midi = 48i32 + semitones;
    if !(0..=119).contains(&midi) {
        return None;
    }
    let pitch = Pitch::from_semitone(midi.rem_euclid(12) as u8)?;
    let octave = (midi / 12) as u8;
    Some((pitch, octave))
}

/// Inverse of `period_to_note`: convert (Pitch, octave) back to a ProTracker period value.
///
/// Returns 0 if the note is out of MOD's supported range.
fn note_to_period(pitch: Pitch, octave: u8) -> u16 {
    let semitone = pitch.semitone() as i32;
    let midi = (octave as i32) * 12 + semitone;
    if midi <= 0 {
        return 0;
    }
    let semitones_from_c4 = midi - 48;
    let period = C4_PERIOD / 2.0_f64.powf(semitones_from_c4 as f64 / 12.0);
    let rounded = period.round() as u16;
    rounded.max(1)
}

/// Convert a MOD effect command and parameter to our Effect, if non-trivial.
fn decode_effect(cmd: u8, param: u8) -> Option<Effect> {
    // Arpeggio with param 0 is a no-op in ProTracker
    if cmd == 0 && param == 0 {
        return None;
    }
    // Map MOD 8xx panning (0-128) to our 0-255 range
    if cmd == 0x8 {
        let mapped = (param.min(128) as u16 * 255 / 128) as u8;
        return Some(Effect::new(0x8, mapped));
    }
    // Map MOD E8x panning (0-15) to our 0-255 range
    if cmd == 0xE && (param >> 4) == 0x8 {
        let mapped = (param & 0x0F) * 17;
        return Some(Effect::new(0x8, mapped));
    }
    // MOD Fxx effect: param < 32 sets tick speed, param >= 32 sets BPM.
    // Both use command 0xF — SetSpeed handler checks the param range.
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
    fn test_extract_tempo_no_patterns() {
        let patterns: Vec<Pattern> = vec![];
        let (speed, bpm) = extract_tempo_from_patterns(&patterns);
        assert_eq!(speed, 6);
        assert_eq!(bpm, 125.0);
    }

    #[test]
    fn test_extract_tempo_pattern_with_fxx_effect() {
        let mut pattern = Pattern::new(64, 4);
        let cell = Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xF, 0x05)], // F05 = speed 5
        };
        pattern.set_cell(0, 0, cell);
        let patterns = vec![pattern];

        let (speed, bpm) = extract_tempo_from_patterns(&patterns);
        assert_eq!(speed, 5);
        assert_eq!(bpm, 125.0);
    }

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
        let (pitch, octave) = period_to_note(254).unwrap();
        assert_eq!(pitch, Pitch::A);
        assert_eq!(octave, 4);
    }

    #[test]
    fn test_period_to_note_roundtrip_all_notes() {
        use crate::pattern::note::Pitch;

        let notes: &[(Pitch, u8)] = &[
            (Pitch::C, 1),
            (Pitch::C, 2),
            (Pitch::C, 3),
            (Pitch::C, 4),
            (Pitch::C, 5),
            (Pitch::C, 6),
            (Pitch::C, 7),
            (Pitch::C, 8),
            (Pitch::C, 9),
            (Pitch::B, 0),
            (Pitch::B, 1),
            (Pitch::B, 2),
            (Pitch::B, 3),
            (Pitch::B, 4),
            (Pitch::B, 5),
            (Pitch::B, 6),
            (Pitch::B, 7),
            (Pitch::B, 8),
            (Pitch::B, 9),
            (Pitch::G, 4),
            (Pitch::A, 4),
            (Pitch::D, 4),
            (Pitch::E, 4),
        ];

        let mut failures = Vec::new();
        for &(pitch, octave) in notes {
            let period = note_to_period(pitch, octave);
            if let Some((round_pitch, round_octave)) = period_to_note(period) {
                if round_pitch != pitch || round_octave != octave {
                    failures.push(format!(
                        "{:?}-{}: period={} -> ({:?}, {}), expected ({:?}, {})",
                        pitch, octave, period, round_pitch, round_octave, pitch, octave
                    ));
                }
            } else {
                failures.push(format!(
                    "{:?}-{}: period_to_note({}) returned None",
                    pitch, octave, period
                ));
            }
        }

        if !failures.is_empty() {
            panic!("Roundtrip failures:\n{}", failures.join("\n"));
        }
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

    #[test]
    fn test_note_to_period_c4() {
        let p = note_to_period(Pitch::C, 4);
        assert_eq!(p, 428);
    }

    #[test]
    fn test_note_to_period_c5_one_octave_up() {
        let p = note_to_period(Pitch::C, 5);
        assert_eq!(p, 214);
    }

    #[test]
    fn test_note_to_period_a4_roundtrip() {
        let period = note_to_period(Pitch::A, 4);
        let (pitch, octave) = period_to_note(period).unwrap();
        assert_eq!(pitch, Pitch::A);
        assert_eq!(octave, 4);
    }

    #[test]
    fn test_period_to_note_formula_consistency() {
        let test_periods: &[u16] = &[
            856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453, 428, 404, 381, 360, 340,
            320, 303, 286, 269, 240, 214, 202, 190, 180, 170, 160,
        ];

        for &period in test_periods {
            if let Some((pitch, octave)) = period_to_note(period) {
                let back = note_to_period(pitch, octave);
                let diff = (period as i32 - back as i32).abs();
                assert!(
                    diff <= 2,
                    "period {} roundtrip via {:?}-{} gave {} (diff={})",
                    period,
                    pitch,
                    octave,
                    back,
                    diff
                );
            }
        }
    }

    #[test]
    fn test_export_mod_requires_31_samples() {
        let song = Song::new("Test".to_string(), 125.0);
        let samples: Vec<Sample> = vec![Sample::default(); 15];
        assert!(export_mod(&song, &samples).is_err());
    }

    #[test]
    fn test_export_mod_empty_song_produces_valid_file() {
        let mut song = Song::new("Empty Song".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let result = export_mod(&song, &samples);
        assert!(result.is_ok(), "empty export failed: {:?}", result.err());
        let data = result.unwrap();
        assert!(data.len() >= 1084, "file too small: {}", data.len());
        assert_eq!(&data[1080..1084], b"M.K.");
    }

    #[test]
    fn test_export_mod_roundtrip_empty() {
        let mut song = Song::new("Roundtrip".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported);
        assert!(
            imported.is_ok(),
            "round-trip import failed: {:?}",
            imported.err()
        );
        let result = imported.unwrap();
        assert_eq!(result.song.name, "Roundtrip");
        assert_eq!(result.song.patterns.len(), 1);
        assert_eq!(result.song.patterns[0].num_channels(), 4);
        assert_eq!(result.song.arrangement, vec![0]);
        assert_eq!(result.samples.len(), 31);
    }

    #[test]
    fn test_export_mod_encoding_debug() {
        let mut song = Song::new("Debug".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);
        pat.set_cell(
            0,
            0,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 100, 0))),
                instrument: Some(0),
                volume: None,
                effects: vec![],
            },
        );
        song.patterns = vec![pat];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        assert_eq!(exported[1084], 0x01); // period hi nibble for 428 = 0x1AC
        assert_eq!(exported[1085], 0xAC); // period lo byte
        let imported = import_mod(&exported).unwrap();
        let ip = &imported.song.patterns[0];
        let cell = ip.get_cell(0, 0).unwrap();
        assert!(matches!(cell.note.as_ref(), Some(NoteEvent::On(n)) if n.pitch == Pitch::C));
    }

    #[test]
    fn test_export_mod_roundtrip_with_notes() {
        let mut song = Song::new("Note Test".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);
        pat.set_cell(
            0,
            0,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 100, 0))),
                instrument: Some(0),
                volume: None,
                effects: vec![],
            },
        );
        pat.set_cell(
            1,
            0,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::E, 4, 100, 0))),
                instrument: Some(0),
                volume: None,
                effects: vec![],
            },
        );
        pat.set_cell(
            2,
            1,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::G, 4, 100, 2))),
                instrument: Some(2),
                volume: None,
                effects: vec![],
            },
        );
        song.patterns = vec![pat];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();

        let imported_pat = &imported.song.patterns[0];
        assert!(matches!(
            imported_pat.get_cell(0, 0).unwrap().note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::C
        ));
        assert!(matches!(
            imported_pat.get_cell(1, 0).unwrap().note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::E
        ));
        let cell21 = imported_pat.get_cell(2, 1).unwrap();
        assert!(matches!(
            cell21.note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::G
        ));
        assert_eq!(cell21.instrument, Some(2));
        assert!(cell21.effects.is_empty());
    }

    #[test]
    fn test_export_mod_roundtrip_all_notes() {
        use crate::pattern::note::Pitch;

        let notes: &[(Pitch, u8)] = &[
            (Pitch::C, 1),
            (Pitch::CSharp, 1),
            (Pitch::D, 1),
            (Pitch::DSharp, 1),
            (Pitch::E, 1),
            (Pitch::F, 1),
            (Pitch::FSharp, 1),
            (Pitch::G, 1),
            (Pitch::GSharp, 1),
            (Pitch::A, 1),
            (Pitch::ASharp, 1),
            (Pitch::B, 1),
            (Pitch::C, 2),
            (Pitch::CSharp, 2),
            (Pitch::D, 2),
            (Pitch::DSharp, 2),
            (Pitch::E, 2),
            (Pitch::F, 2),
            (Pitch::FSharp, 2),
            (Pitch::G, 2),
            (Pitch::GSharp, 2),
            (Pitch::A, 2),
            (Pitch::ASharp, 2),
            (Pitch::B, 2),
            (Pitch::C, 3),
            (Pitch::CSharp, 3),
            (Pitch::D, 3),
            (Pitch::DSharp, 3),
            (Pitch::E, 3),
            (Pitch::F, 3),
            (Pitch::FSharp, 3),
            (Pitch::G, 3),
            (Pitch::GSharp, 3),
            (Pitch::A, 3),
            (Pitch::ASharp, 3),
            (Pitch::B, 3),
        ];

        let mut song = Song::new("All Notes Test".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);

        for (i, &(pitch, octave)) in notes.iter().enumerate() {
            let row = i % 64;
            let ch = (i / 64) % 4;
            if ch == 0 && row >= 64 {
                break;
            }
            let actual_row = row.min(63);
            let actual_ch = ch.min(3);
            pat.set_cell(
                actual_row,
                actual_ch,
                Cell {
                    note: Some(NoteEvent::On(Note::new(pitch, octave, 100, 0))),
                    instrument: Some(0),
                    volume: None,
                    effects: vec![],
                },
            );
        }

        song.patterns = vec![pat];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];

        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let imported_pat = &imported.song.patterns[0];

        let mut failures = Vec::new();
        for (i, &(expected_pitch, expected_octave)) in notes.iter().enumerate() {
            let row = i % 64;
            let ch = (i / 64) % 4;
            if ch == 0 && row >= 64 {
                break;
            }
            let actual_row = row.min(63);
            let actual_ch = ch.min(3);

            if let Some(cell) = imported_pat.get_cell(actual_row, actual_ch) {
                if let Some(NoteEvent::On(note)) = &cell.note {
                    if note.pitch != expected_pitch || note.octave != expected_octave {
                        failures.push(format!(
                            "Note {} at row={} ch={}: got ({:?}, {}) expected ({:?}, {})",
                            i,
                            actual_row,
                            actual_ch,
                            note.pitch,
                            note.octave,
                            expected_pitch,
                            expected_octave
                        ));
                    }
                } else {
                    failures.push(format!(
                        "Note {} at row={} ch={}: no note event",
                        i, actual_row, actual_ch
                    ));
                }
            } else {
                failures.push(format!(
                    "Note {} at row={} ch={}: no cell",
                    i, actual_row, actual_ch
                ));
            }
        }

        if !failures.is_empty() {
            panic!("Roundtrip failures:\n{}", failures.join("\n"));
        }
    }

    #[test]
    fn test_export_mod_respects_channel_count() {
        let mut song = Song::new("8ch".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 8)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        assert_eq!(&exported[1080..1084], b"8CHN");
    }

    #[test]
    fn test_export_mod_title_encoding() {
        let mut song = Song::new("Test Song Title".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        assert_eq!(imported.song.name, "Test Song Title");
    }

    #[test]
    fn test_export_mod_instrument_volume_preserved() {
        let mut song = Song::new("Volume Test".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        for i in 0..31 {
            song.instruments
                .push(Instrument::new(format!("Inst {}", i)));
        }
        song.instruments[0].volume = 0.75;
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let imported_vol = imported.song.instruments[0].volume;
        assert!(
            (imported_vol - 0.75).abs() < 0.02,
            "volume not preserved: {}",
            imported_vol
        );
    }

    #[test]
    fn test_export_mod_instrument_finetune_preserved() {
        let mut song = Song::new("Finetune Test".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        for i in 0..31 {
            song.instruments
                .push(Instrument::new(format!("Inst {}", i)));
        }
        song.instruments[0].finetune = -5;
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        assert_eq!(imported.song.instruments[0].finetune, -5);
    }

    #[test]
    fn test_export_mod_effect_preserved() {
        let mut song = Song::new("Effect Test".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);
        pat.set_cell(
            0,
            0,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 100, 0))),
                instrument: Some(0),
                volume: None,
                effects: vec![Effect::new(0x0C, 64)], // Set volume C-4 to 64
            },
        );
        song.patterns = vec![pat];
        song.arrangement = vec![0];
        for i in 0..31 {
            song.instruments
                .push(Instrument::new(format!("Inst {}", i)));
        }
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let cell = imported.song.patterns[0].get_cell(0, 0).unwrap();
        assert!(!cell.effects.is_empty(), "effect was lost in round-trip");
        assert_eq!(cell.effects[0].param, 64);
    }

    #[test]
    fn test_export_mod_multiple_effects_preserved() {
        let mut song = Song::new("Multi Effect Test".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);
        pat.set_cell(
            5,
            2,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::A, 3, 100, 1))),
                instrument: Some(1),
                volume: None,
                effects: vec![
                    Effect::new(0x09, 0x20), // Sample offset 0x20
                    Effect::new(0x0D, 0x08), // Note delay 8
                ],
            },
        );
        song.patterns = vec![pat];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let cell = imported.song.patterns[0].get_cell(5, 2).unwrap();
        assert!(
            cell.effects.len() >= 1,
            "at least one effect should be preserved"
        );
    }

    #[test]
    fn test_export_mod_sample_data_roundtrip() {
        use crate::audio::sample::Sample;
        let mut song = Song::new("Sample Data".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        let sample_data: Vec<f32> = (0..1000).map(|i| (i as f32 / 100.0).sin()).collect();
        let sample = Sample::new(sample_data.clone(), 8363, 1, Some("Test".to_string()));
        let mut samples: Vec<Sample> = vec![Sample::default(); 31];
        samples[0] = sample;
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let imported_sample = &imported.samples[0];
        assert!(
            imported_sample.frame_count() > 0,
            "sample data should be preserved"
        );
    }

    #[test]
    fn test_export_mod_multi_pattern_arrangement() {
        let mut song = Song::new("Multi Pattern".to_string(), 125.0);
        song.patterns = vec![
            Pattern::new(64, 4),
            Pattern::new(64, 4),
            Pattern::new(64, 4),
        ];
        song.arrangement = vec![0, 1, 2, 1, 0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        assert_eq!(imported.song.arrangement, vec![0, 1, 2, 1, 0]);
        assert_eq!(imported.song.patterns.len(), 3);
    }

    #[test]
    fn test_export_mod_pattern_data_in_different_patterns() {
        let mut song = Song::new("Pattern Data Test".to_string(), 125.0);
        let mut pat0 = Pattern::new(64, 4);
        pat0.set_cell(
            0,
            0,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 100, 0))),
                instrument: Some(0),
                volume: None,
                effects: vec![],
            },
        );
        let mut pat1 = Pattern::new(64, 4);
        pat1.set_cell(
            10,
            1,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::E, 4, 100, 0))),
                instrument: Some(1),
                volume: None,
                effects: vec![],
            },
        );
        pat1.set_cell(
            20,
            3,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::G, 4, 100, 2))),
                instrument: Some(2),
                volume: None,
                effects: vec![],
            },
        );
        song.patterns = vec![pat0, pat1];
        song.arrangement = vec![0, 1];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let pat0_cell = imported.song.patterns[0].get_cell(0, 0).unwrap();
        assert!(matches!(
            pat0_cell.note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::C
        ));
        let pat1_cell_1 = imported.song.patterns[1].get_cell(10, 1).unwrap();
        assert!(matches!(
            pat1_cell_1.note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::E
        ));
        let pat1_cell_2 = imported.song.patterns[1].get_cell(20, 3).unwrap();
        assert!(matches!(
            pat1_cell_2.note.as_ref(),
            Some(NoteEvent::On(n)) if n.pitch == Pitch::G
        ));
        assert_eq!(pat1_cell_2.instrument, Some(2));
    }

    #[test]
    fn test_export_mod_empty_cells_preserved() {
        let mut song = Song::new("Empty Cells".to_string(), 125.0);
        let mut pat = Pattern::new(64, 4);
        pat.set_cell(
            5,
            2,
            Cell {
                note: Some(NoteEvent::On(Note::new(Pitch::D, 4, 100, 1))),
                instrument: Some(1),
                volume: None,
                effects: vec![],
            },
        );
        song.patterns = vec![pat];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        let empty_cell = imported.song.patterns[0].get_cell(4, 2).unwrap();
        assert!(empty_cell.note.is_none(), "empty cell should remain empty");
        let empty_cell_2 = imported.song.patterns[0].get_cell(6, 2).unwrap();
        assert!(
            empty_cell_2.note.is_none(),
            "empty cell should remain empty"
        );
    }

    #[test]
    fn test_export_mod_2channel_format() {
        let mut song = Song::new("2 Channel".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 2)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        assert_eq!(&exported[1080..1084], b"2CHN");
        let imported = import_mod(&exported).unwrap();
        assert_eq!(imported.song.patterns[0].num_channels(), 2);
    }

    #[test]
    fn test_export_mod_8channel_format() {
        let mut song = Song::new("8 Channel".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 8)];
        song.arrangement = vec![0];
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        assert_eq!(&exported[1080..1084], b"8CHN");
        let imported = import_mod(&exported).unwrap();
        assert_eq!(imported.song.patterns[0].num_channels(), 8);
    }

    #[test]
    fn test_export_mod_all_instruments_initialized() {
        let mut song = Song::new("All Instruments".to_string(), 125.0);
        song.patterns = vec![Pattern::new(64, 4)];
        song.arrangement = vec![0];
        for i in 0..31 {
            song.instruments
                .push(Instrument::new(format!("Inst {}", i)));
        }
        for inst in &mut song.instruments {
            inst.volume = 0.5;
        }
        let samples: Vec<Sample> = vec![Sample::default(); 31];
        let exported = export_mod(&song, &samples).unwrap();
        let imported = import_mod(&exported).unwrap();
        for (i, inst) in imported.song.instruments.iter().enumerate() {
            assert!(
                (inst.volume - 0.5).abs() < 0.02,
                "instrument {} volume not preserved",
                i
            );
        }
    }
}
