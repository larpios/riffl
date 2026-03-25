//! XM (FastTracker II Extended Module) binary parser.
//!
//! Parses XM files directly from raw bytes, bypassing the xmrs crate's
//! buggy float normalization pipeline. Produces our internal `FormatData`
//! with direct byte-to-Effect mapping.

use crate::audio::sample::{LoopMode, Sample};
use crate::pattern::effect::{Effect, EffectMode};
use crate::pattern::note::{Note, Pitch};
use crate::pattern::{Cell, NoteEvent, Pattern};
use crate::song::{Envelope, EnvelopePoint, Instrument, PanningLaw, Song};

use super::{FormatData, FormatError, FormatResult, ModuleLoader};

pub struct XmLoader;

impl ModuleLoader for XmLoader {
    fn name(&self) -> &'static str {
        "FastTracker II"
    }

    fn extensions(&self) -> &[&str] {
        &["xm"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        data.starts_with(b"Extended Module: ")
    }

    fn load(&self, data: &[u8]) -> FormatResult<FormatData> {
        import_xm(data).map_err(FormatError::from)
    }
}

// ─── Binary Helpers ──────────────────────────────────────────────────────────

fn read_u8(data: &[u8], offset: &mut usize) -> u8 {
    let v = data[*offset];
    *offset += 1;
    v
}

fn read_u16_le(data: &[u8], offset: &mut usize) -> u16 {
    let v = u16::from_le_bytes([data[*offset], data[*offset + 1]]);
    *offset += 2;
    v
}

fn read_u32_le(data: &[u8], offset: &mut usize) -> u32 {
    let v = u32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    v
}

fn read_string(data: &[u8], offset: &mut usize, len: usize) -> String {
    let end = (*offset + len).min(data.len());
    let s = String::from_utf8_lossy(&data[*offset..end])
        .trim_end_matches('\0')
        .trim()
        .to_string();
    *offset += len;
    s
}

fn read_i8(data: &[u8], offset: &mut usize) -> i8 {
    let v = data[*offset] as i8;
    *offset += 1;
    v
}

// ─── XM Header ───────────────────────────────────────────────────────────────

struct XmHeader {
    name: String,
    #[allow(dead_code)]
    song_length: u16,
    #[allow(dead_code)]
    restart_position: u16,
    num_channels: u16,
    num_patterns: u16,
    num_instruments: u16,
    #[allow(dead_code)]
    linear_frequencies: bool,
    default_tempo: u16,
    default_bpm: u16,
    pattern_order: Vec<u8>,
}

fn parse_xm_header(data: &[u8]) -> Result<(XmHeader, usize), String> {
    if data.len() < 80 {
        return Err("XM file too short for header".into());
    }

    let id = String::from_utf8_lossy(&data[0..17]);
    if !id.starts_with("Extended Module:") {
        return Err(format!("Not an XM file (got '{}')", id));
    }

    let mut off = 17;
    let name = read_string(data, &mut off, 20);
    let _right_arrow = read_u8(data, &mut off); // 0x1A
    let _tracker_name = read_string(data, &mut off, 20);
    let _version = read_u16_le(data, &mut off);
    let header_size = read_u32_le(data, &mut off);

    // Header fields after header_size (offset 64)
    let song_length = read_u16_le(data, &mut off);
    let restart_position = read_u16_le(data, &mut off);
    let num_channels = read_u16_le(data, &mut off);
    let num_patterns = read_u16_le(data, &mut off);
    let num_instruments = read_u16_le(data, &mut off);
    let flags = read_u16_le(data, &mut off);
    let default_tempo = read_u16_le(data, &mut off);
    let default_bpm = read_u16_le(data, &mut off);

    // Pattern order table starts at offset 80
    let order_start = 80;
    let order_len = song_length.min(256) as usize;
    if data.len() < order_start + order_len {
        return Err("XM file too short for pattern order".into());
    }
    let pattern_order = data[order_start..order_start + order_len].to_vec();

    // Skip to end of header (header_size is measured from offset 60)
    let header_end = 60 + header_size as usize;

    Ok((
        XmHeader {
            name,
            song_length,
            restart_position,
            num_channels,
            num_patterns,
            num_instruments,
            linear_frequencies: flags & 1 != 0,
            default_tempo,
            default_bpm,
            pattern_order,
        },
        header_end,
    ))
}

// ─── XM Pattern ──────────────────────────────────────────────────────────────

/// Raw pattern slot as it comes from the XM file.
struct XmSlot {
    note: u8,       // 0=none, 1-96=note, 97=off
    instrument: u8, // 0=none, 1-128=instrument
    volume: u8,     // volume column byte
    effect_type: u8,
    effect_param: u8,
}

/// Rows × Channels of raw slots.
struct XmPattern {
    num_rows: u16,
    slots: Vec<Vec<XmSlot>>,
}

fn parse_xm_patterns(
    data: &[u8],
    offset: &mut usize,
    num_patterns: u16,
    num_channels: u16,
) -> Result<Vec<XmPattern>, String> {
    let mut patterns = Vec::with_capacity(num_patterns as usize);

    for pat_idx in 0..num_patterns as usize {
        if *offset + 9 > data.len() {
            return Err(format!("XM pattern {} header truncated", pat_idx));
        }

        let header_len = read_u32_le(data, offset);
        let _packing_type = read_u8(data, offset);
        let num_rows = read_u16_le(data, offset);
        let packed_size = read_u16_le(data, offset);

        // Skip any extra header bytes beyond the 9 we already read
        if header_len > 9 {
            *offset += (header_len - 9) as usize;
        }

        if packed_size == 0 {
            // Empty pattern
            let mut rows = Vec::with_capacity(num_rows as usize);
            for _ in 0..num_rows {
                let mut row = Vec::with_capacity(num_channels as usize);
                for _ in 0..num_channels {
                    row.push(XmSlot {
                        note: 0,
                        instrument: 0,
                        volume: 0,
                        effect_type: 0,
                        effect_param: 0,
                    });
                }
                rows.push(row);
            }
            patterns.push(XmPattern {
                num_rows,
                slots: rows,
            });
            continue;
        }

        let pattern_end = *offset + packed_size as usize;
        if pattern_end > data.len() {
            return Err(format!("XM pattern {} data truncated", pat_idx));
        }

        let mut rows = Vec::with_capacity(num_rows as usize);
        for _ in 0..num_rows {
            let mut row = Vec::with_capacity(num_channels as usize);
            for _ in 0..num_channels {
                if *offset >= pattern_end {
                    row.push(XmSlot {
                        note: 0,
                        instrument: 0,
                        volume: 0,
                        effect_type: 0,
                        effect_param: 0,
                    });
                    continue;
                }

                let byte0 = read_u8(data, offset);
                let (note, instr, vol, eff_type, eff_param);

                if byte0 & 0x80 != 0 {
                    // Packed format
                    note = if byte0 & 0x01 != 0 {
                        read_u8(data, offset)
                    } else {
                        0
                    };
                    instr = if byte0 & 0x02 != 0 {
                        read_u8(data, offset)
                    } else {
                        0
                    };
                    vol = if byte0 & 0x04 != 0 {
                        read_u8(data, offset)
                    } else {
                        0
                    };
                    eff_type = if byte0 & 0x08 != 0 {
                        read_u8(data, offset)
                    } else {
                        0
                    };
                    eff_param = if byte0 & 0x10 != 0 {
                        read_u8(data, offset)
                    } else {
                        0
                    };
                } else {
                    // Unpacked: 5 consecutive bytes, first is note
                    note = byte0;
                    instr = read_u8(data, offset);
                    vol = read_u8(data, offset);
                    eff_type = read_u8(data, offset);
                    eff_param = read_u8(data, offset);
                }

                row.push(XmSlot {
                    note,
                    instrument: instr,
                    volume: vol,
                    effect_type: eff_type,
                    effect_param: eff_param,
                });
            }
            rows.push(row);
        }

        *offset = pattern_end;
        patterns.push(XmPattern {
            num_rows,
            slots: rows,
        });
    }

    Ok(patterns)
}

// ─── XM Instrument + Sample ─────────────────────────────────────────────────

#[allow(dead_code)]
struct XmSampleHeader {
    length: u32,
    loop_start: u32,
    loop_length: u32,
    volume: u8,
    finetune: i8,
    flags: u8,
    panning: u8,
    relative_pitch: i8,
    name: String,
}

#[allow(dead_code)]
struct XmInstrData {
    name: String,
    sample_for_pitch: [u8; 96],
    volume_envelope: Vec<EnvelopePoint>,
    volume_envelope_flags: u8,
    volume_sustain_point: u8,
    volume_loop_start: u8,
    volume_loop_end: u8,
    panning_envelope: Vec<EnvelopePoint>,
    panning_envelope_flags: u8,
    panning_sustain_point: u8,
    panning_loop_start: u8,
    panning_loop_end: u8,
    volume_fadeout: u16,
    samples: Vec<XmSampleHeader>,
    sample_data: Vec<Vec<f32>>,
}

fn parse_envelope_points(data: &[u8], offset: &mut usize, num_points: u8) -> Vec<EnvelopePoint> {
    let mut points = Vec::new();
    for _ in 0..12 {
        let frame = read_u16_le(data, offset);
        let value = read_u16_le(data, offset);
        points.push(EnvelopePoint {
            frame,
            value: value as f32 / 64.0,
        });
    }
    points.truncate(num_points as usize);
    points
}

fn parse_xm_instruments(
    data: &[u8],
    offset: &mut usize,
    num_instruments: u16,
) -> Result<Vec<XmInstrData>, String> {
    let mut instruments = Vec::with_capacity(num_instruments as usize);

    for inst_idx in 0..num_instruments as usize {
        if *offset + 4 > data.len() {
            return Err(format!("XM instrument {} header truncated", inst_idx));
        }

        let inst_header_len = read_u32_le(data, offset);
        let inst_start = *offset - 4; // include the header_len field

        if *offset + 25 > data.len() {
            return Err(format!("XM instrument {} header truncated", inst_idx));
        }

        let name = read_string(data, offset, 22);
        let _instr_type = read_u8(data, offset);
        let num_samples = read_u16_le(data, offset);

        if num_samples == 0 {
            // Skip to end of instrument header
            *offset = inst_start + inst_header_len as usize;
            instruments.push(XmInstrData {
                name,
                sample_for_pitch: [0; 96],
                volume_envelope: Vec::new(),
                volume_envelope_flags: 0,
                volume_sustain_point: 0,
                volume_loop_start: 0,
                volume_loop_end: 0,
                panning_envelope: Vec::new(),
                panning_envelope_flags: 0,
                panning_sustain_point: 0,
                panning_loop_start: 0,
                panning_loop_end: 0,
                volume_fadeout: 0,
                samples: Vec::new(),
                sample_data: Vec::new(),
            });
            continue;
        }

        // Read sample header size
        let _sample_header_size = read_u32_le(data, offset);

        // Read sample_for_pitch (96 bytes)
        let mut sample_for_pitch = [0u8; 96];
        if *offset + 96 <= data.len() {
            sample_for_pitch.copy_from_slice(&data[*offset..*offset + 96]);
        }
        *offset += 96;

        // Volume envelope points (48 bytes = 12 points × 4 bytes)
        let vol_env_start = *offset;
        *offset += 48;

        // Panning envelope points (48 bytes)
        let pan_env_start = *offset;
        *offset += 48;

        let num_vol_points = read_u8(data, offset);
        let num_pan_points = read_u8(data, offset);

        let volume_sustain_point = read_u8(data, offset);
        let volume_loop_start = read_u8(data, offset);
        let volume_loop_end = read_u8(data, offset);
        let panning_sustain_point = read_u8(data, offset);
        let panning_loop_start = read_u8(data, offset);
        let panning_loop_end = read_u8(data, offset);

        let volume_envelope_flags = read_u8(data, offset);
        let panning_envelope_flags = read_u8(data, offset);

        let _vibrato_type = read_u8(data, offset);
        let _vibrato_sweep = read_u8(data, offset);
        let _vibrato_depth = read_u8(data, offset);
        let _vibrato_rate = read_u8(data, offset);

        let volume_fadeout = read_u16_le(data, offset);

        // Now parse envelope points from saved positions
        let mut ve_off = vol_env_start;
        let volume_envelope = parse_envelope_points(data, &mut ve_off, num_vol_points.min(12));
        let mut pe_off = pan_env_start;
        let panning_envelope = parse_envelope_points(data, &mut pe_off, num_pan_points.min(12));

        // Skip to end of instrument header
        *offset = inst_start + inst_header_len as usize;

        // Read sample headers
        let mut sample_headers = Vec::with_capacity(num_samples as usize);
        for _ in 0..num_samples {
            if *offset + 40 > data.len() {
                return Err(format!(
                    "XM instrument {} sample header truncated",
                    inst_idx
                ));
            }
            let length = read_u32_le(data, offset);
            let loop_start = read_u32_le(data, offset);
            let loop_length = read_u32_le(data, offset);
            let volume = read_u8(data, offset);
            let finetune = read_i8(data, offset);
            let flags = read_u8(data, offset);
            let panning = read_u8(data, offset);
            let relative_pitch = read_i8(data, offset);
            let _reserved = read_u8(data, offset);
            let sname = read_string(data, offset, 22);

            sample_headers.push(XmSampleHeader {
                length,
                loop_start,
                loop_length,
                volume,
                finetune,
                flags,
                panning,
                relative_pitch,
                name: sname,
            });
        }

        // Read sample data (delta-encoded)
        let mut sample_data_vec = Vec::with_capacity(num_samples as usize);
        for sh in &sample_headers {
            let is_16bit = sh.flags & 0x10 != 0;
            let data_len = sh.length as usize;

            if *offset + data_len > data.len() {
                return Err(format!("XM instrument {} sample data truncated", inst_idx));
            }

            let float_data = if is_16bit {
                // 16-bit delta encoding
                let num_samples_count = data_len / 2;
                let mut decoded = Vec::with_capacity(num_samples_count);
                let mut old: u16 = 0;
                for i in 0..num_samples_count {
                    let raw =
                        u16::from_le_bytes([data[*offset + i * 2], data[*offset + i * 2 + 1]]);
                    let new = raw.wrapping_add(old);
                    decoded.push(new as i16 as f32 / 32768.0);
                    old = new;
                }
                decoded
            } else {
                // 8-bit delta encoding
                let mut decoded = Vec::with_capacity(data_len);
                let mut old: u8 = 0;
                for i in 0..data_len {
                    let new = data[*offset + i].wrapping_add(old);
                    decoded.push(new as i8 as f32 / 128.0);
                    old = new;
                }
                decoded
            };

            *offset += data_len;
            sample_data_vec.push(float_data);
        }

        instruments.push(XmInstrData {
            name,
            sample_for_pitch,
            volume_envelope,
            volume_envelope_flags,
            volume_sustain_point,
            volume_loop_start,
            volume_loop_end,
            panning_envelope,
            panning_envelope_flags,
            panning_sustain_point,
            panning_loop_start,
            panning_loop_end,
            volume_fadeout,
            samples: sample_headers,
            sample_data: sample_data_vec,
        });
    }

    Ok(instruments)
}

// ─── Effect Conversion ───────────────────────────────────────────────────────

/// Convert XM volume column byte to an optional Effect and/or volume value.
fn convert_xm_volume_column(vol: u8) -> (Option<u8>, Option<Effect>) {
    match vol >> 4 {
        0x1..=0x5 => {
            // Set volume (0x10-0x50 → volume 0-64)
            let v = (vol - 0x10).min(64);
            (Some(v), None)
        }
        0x6 => {
            // Volume slide down
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x0A, param)))
        }
        0x7 => {
            // Volume slide up
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x0A, param << 4)))
        }
        0x8 => {
            // Fine volume slide down
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x0E, 0xB0 | param)))
        }
        0x9 => {
            // Fine volume slide up
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x0E, 0xA0 | param)))
        }
        0xA => {
            // Set vibrato speed
            let param = vol & 0x0F;
            if param > 0 {
                (None, Some(Effect::new(0x04, param << 4)))
            } else {
                (None, None)
            }
        }
        0xB => {
            // Vibrato depth
            let param = vol & 0x0F;
            if param > 0 {
                (None, Some(Effect::new(0x04, param)))
            } else {
                (None, None)
            }
        }
        0xC => {
            // Set panning
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x08, param * 17)))
        }
        0xD => {
            // Panning slide left
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x12, param)))
        }
        0xE => {
            // Panning slide right
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x12, param << 4)))
        }
        0xF => {
            // Tone portamento
            let param = vol & 0x0F;
            (None, Some(Effect::new(0x03, param << 4)))
        }
        _ => (None, None),
    }
}

/// Convert XM effect column directly to our Effect — no float intermediary.
fn convert_xm_effect(eff_type: u8, param: u8) -> Option<Effect> {
    match eff_type {
        // 0xy: Arpeggio
        0x00 => {
            if param != 0 {
                Some(Effect::new(0x00, param))
            } else {
                None
            }
        }
        // 1xx-Axx: Direct passthrough to our effect system
        0x01 => Some(Effect::new(0x01, param)), // Portamento up
        0x02 => Some(Effect::new(0x02, param)), // Portamento down
        0x03 => Some(Effect::new(0x03, param)), // Tone portamento
        0x04 => Some(Effect::new(0x04, param)), // Vibrato
        0x05 => Some(Effect::new(0x05, param)), // Tone porta + vol slide
        0x06 => Some(Effect::new(0x06, param)), // Vibrato + vol slide
        0x07 => Some(Effect::new(0x07, param)), // Tremolo
        0x08 => Some(Effect::new(0x08, param)), // Set panning
        0x09 => Some(Effect::new(0x09, param)), // Sample offset
        0x0A => Some(Effect::new(0x0A, param)), // Volume slide
        0x0B => Some(Effect::new(0x0B, param)), // Position jump
        0x0C => Some(Effect::new(0x0C, param)), // Set volume
        0x0D => Some(Effect::new(0x0D, param)), // Pattern break
        0x0E => {
            // Extended effects (Exy)
            // E8x is Set Panning
            if (param >> 4) == 0x8 {
                Some(Effect::new(0x08, (param & 0x0F) * 17))
            } else {
                Some(Effect::new(0x0E, param))
            }
        }
        0x0F => Some(Effect::new(0x0F, param)), // Set speed/BPM
        // Extended XM effects (0x10+)
        0x10 => Some(Effect::new(0x10, param)), // Set global volume (Gxx)
        0x11 => Some(Effect::new(0x11, param)), // Global volume slide (Hxx)
        // 0x14 = Key off (handled at note level, not as effect)
        0x14 => None,
        // 0x15 = Set envelope position (Lxx)
        0x15 => Some(Effect::new(0x17, param)),
        // 0x19 = Panning slide (Pxy)
        0x19 => Some(Effect::new(0x12, param)),
        // 0x1B = Multi retrig note (Rxy)
        0x1B => Some(Effect::new(0x16, param)),
        // 0x1D = Tremor (Txy)
        0x1D => Some(Effect::new(0x15, param)),
        // 0x21 = Extra fine portamento (X1y / X2y)
        0x21 => {
            let sub = param >> 4;
            let val = param & 0x0F;
            match sub {
                1 => Some(Effect::new(0x21, val)), // Extra fine porta up
                2 => Some(Effect::new(0x22, val)), // Extra fine porta down
                _ => None,
            }
        }
        _ => None,
    }
}

// ─── Main Import ─────────────────────────────────────────────────────────────

/// Import an XM file from raw bytes, producing our internal FormatData.
pub fn import_xm(data: &[u8]) -> Result<FormatData, String> {
    // ── Parse header ──
    let (header, mut offset) = parse_xm_header(data)?;
    let num_channels = header.num_channels as usize;

    // ── Parse patterns ──
    let xm_patterns =
        parse_xm_patterns(data, &mut offset, header.num_patterns, header.num_channels)?;

    // ── Parse instruments ──
    let xm_instruments = parse_xm_instruments(data, &mut offset, header.num_instruments)?;

    // ── Build samples and instruments ──
    let mut out_samples: Vec<Sample> = Vec::new();
    let mut out_instruments: Vec<Instrument> = Vec::new();
    // Maps: xm_inst_idx → [sample_idx → tracker_instrument_idx]
    let mut inst_to_tracker_inst: Vec<Vec<Option<usize>>> = Vec::new();

    for xm_inst in &xm_instruments {
        let mut sample_map = vec![None; xm_inst.samples.len()];

        for (s_idx, sh) in xm_inst.samples.iter().enumerate() {
            if s_idx >= xm_inst.sample_data.len() {
                continue;
            }
            let float_data = &xm_inst.sample_data[s_idx];
            if float_data.is_empty() {
                continue;
            }

            let is_16bit = sh.flags & 0x10 != 0;
            let channels = 1u16; // XM samples are always mono

            let mut sample = Sample::new(float_data.clone(), 8363, channels, Some(sh.name.clone()));
            sample.volume = sh.volume as f32 / 64.0;
            sample.finetune = (sh.finetune as f32 / 127.0 * 100.0) as i32;

            // Loop parameters
            let mut loop_start = sh.loop_start as usize;
            let mut loop_length = sh.loop_length as usize;
            if is_16bit {
                loop_start /= 2;
                loop_length /= 2;
            }

            // Fix invalid loops
            let sample_len = float_data.len();
            if sample_len > 0 {
                if loop_start >= sample_len {
                    loop_start = sample_len - 1;
                }
                if loop_length > sample_len - loop_start {
                    loop_length = sample_len - loop_start;
                }
            } else {
                loop_start = 0;
                loop_length = 0;
            }

            let loop_end = loop_start + loop_length.saturating_sub(1);
            match sh.flags & 0x03 {
                1 if loop_end > loop_start => {
                    sample = sample.with_loop(LoopMode::Forward, loop_start, loop_end);
                }
                2 | 3 if loop_end > loop_start => {
                    sample = sample.with_loop(LoopMode::PingPong, loop_start, loop_end);
                }
                _ => {}
            }

            // Base note
            let base_note = (48_i32 - sh.relative_pitch as i32).clamp(0, 119) as u8;
            sample = sample.with_base_note(base_note);

            // Instrument
            let inst_name = if sh.name.is_empty() {
                xm_inst.name.clone()
            } else if xm_inst.name.is_empty() {
                sh.name.clone()
            } else {
                format!("{} - {}", xm_inst.name, sh.name)
            };

            let mut inst = Instrument::new(inst_name);
            inst.sample_index = Some(out_samples.len());
            inst.volume = sh.volume as f32 / 64.0;
            inst.panning = Some((sh.panning as f32 - 128.0) / 128.0);

            // Volume envelope
            if xm_inst.volume_envelope_flags & 0x01 != 0 && !xm_inst.volume_envelope.is_empty() {
                inst.volume_envelope = Some(Envelope {
                    enabled: true,
                    points: xm_inst.volume_envelope.clone(),
                    sustain_enabled: xm_inst.volume_envelope_flags & 0x02 != 0,
                    sustain_start_point: xm_inst.volume_sustain_point as usize,
                    sustain_end_point: xm_inst.volume_sustain_point as usize,
                    loop_enabled: xm_inst.volume_envelope_flags & 0x04 != 0,
                    loop_start_point: xm_inst.volume_loop_start as usize,
                    loop_end_point: xm_inst.volume_loop_end as usize,
                });
            }

            // Panning envelope
            if xm_inst.panning_envelope_flags & 0x01 != 0 && !xm_inst.panning_envelope.is_empty() {
                inst.panning_envelope = Some(Envelope {
                    enabled: true,
                    points: xm_inst.panning_envelope.iter().map(|p| EnvelopePoint {
                        frame: p.frame,
                        // p.value is already normalized to 0..1 by parse_envelope_points;
                        // remap to -1..1 (-1=left, 0=center, +1=right).
                        value: p.value * 2.0 - 1.0,
                    }).collect(),
                    sustain_enabled: xm_inst.panning_envelope_flags & 0x02 != 0,
                    sustain_start_point: xm_inst.panning_sustain_point as usize,
                    sustain_end_point: xm_inst.panning_sustain_point as usize,
                    loop_enabled: xm_inst.panning_envelope_flags & 0x04 != 0,
                    loop_start_point: xm_inst.panning_loop_start as usize,
                    loop_end_point: xm_inst.panning_loop_end as usize,
                });
            }
            inst.fadeout = xm_inst.volume_fadeout;
            sample_map[s_idx] = Some(out_instruments.len());
            out_samples.push(sample);
            out_instruments.push(inst);
        }

        inst_to_tracker_inst.push(sample_map);
    }

    // ── Build song ──
    let mut song = Song::new(header.name.clone(), header.default_bpm as f64);
    song.tpl = header.default_tempo as u32;
    song.instruments = out_instruments;

    // ── Convert patterns ──
    song.patterns.clear();
    let mut last_instrument: Vec<Option<usize>> = vec![None; num_channels];
    let mut last_sample: Vec<Option<u8>> = vec![None; num_channels];

    for xm_pat in &xm_patterns {
        let num_rows = xm_pat.num_rows as usize;
        let mut pat = Pattern::new(num_rows.max(1), num_channels);

        for (r_idx, row) in xm_pat.slots.iter().enumerate() {
            for (c_idx, slot) in row.iter().enumerate() {
                if c_idx >= num_channels {
                    continue;
                }

                let mut cell = Cell::empty();

                // ── Note ──
                let mut note_event = None;
                if slot.note == 97 {
                    note_event = Some(NoteEvent::Off);
                } else if slot.note >= 1 && slot.note <= 96 {
                    let midi_note = slot.note - 1; // 0-95
                    let octave = midi_note / 12;
                    let semitone = midi_note % 12;
                    if let Some(pitch) = Pitch::from_semitone(semitone) {
                        let vel = 127;
                        note_event = Some(NoteEvent::On(Note::new(pitch, octave, vel, 0)));
                    }
                }

                // Key off from effect column 0x14 (XM extended key off)
                if slot.effect_type == 0x14 {
                    note_event = Some(NoteEvent::Off);
                }

                // ── Instrument resolution ──
                let mut explicit_inst = false;
                let resolved_inst = if slot.instrument > 0 {
                    explicit_inst = true;
                    let i = (slot.instrument as usize).saturating_sub(1);
                    last_instrument[c_idx] = Some(i);
                    Some(i)
                } else if note_event.is_some() {
                    last_instrument[c_idx]
                } else {
                    None
                };

                let mut mapped_sample_idx = None;
                if let Some(i) = resolved_inst {
                    if i < xm_instruments.len() {
                        let xm_inst = &xm_instruments[i];
                        let mut sample_idx = 0u8;
                        if let Some(NoteEvent::On(n)) = &note_event {
                            let midi_pitch = n.octave as usize * 12 + n.pitch.semitone() as usize;
                            if midi_pitch < 96 {
                                sample_idx = xm_inst.sample_for_pitch[midi_pitch];
                            }
                        }
                        if i < inst_to_tracker_inst.len()
                            && (sample_idx as usize) < inst_to_tracker_inst[i].len()
                        {
                            if let Some(mapped_idx) = inst_to_tracker_inst[i][sample_idx as usize] {
                                mapped_sample_idx = Some(mapped_idx as u8);
                                last_sample[c_idx] = mapped_sample_idx;
                                if explicit_inst {
                                    cell.instrument = mapped_sample_idx;
                                }
                            }
                        }
                    }
                }

                // ── Finalize note event ──
                if let Some(NoteEvent::On(mut n)) = note_event {
                    if let Some(idx) = mapped_sample_idx {
                        n.instrument = idx;
                        cell.note = Some(NoteEvent::On(n));
                    } else if let Some(fallback_idx) = last_sample[c_idx] {
                        n.instrument = fallback_idx;
                        cell.note = Some(NoteEvent::On(n));
                    } else {
                        cell.note = None;
                    }
                } else {
                    cell.note = note_event;
                }

                // ── Volume column ──
                let (vol_set, vol_effect) = convert_xm_volume_column(slot.volume);
                if let Some(v) = vol_set {
                    cell.volume = Some(v);
                }
                if let Some(eff) = vol_effect {
                    if cell.effects.len() < crate::pattern::effect::MAX_EFFECTS_PER_CELL {
                        cell.effects.push(eff);
                    }
                }

                // ── Effect column ──
                if let Some(eff) = convert_xm_effect(slot.effect_type, slot.effect_param) {
                    if cell.effects.len() < crate::pattern::effect::MAX_EFFECTS_PER_CELL {
                        cell.effects.push(eff);
                    }
                }

                pat.set_cell(r_idx, c_idx, cell);
            }
        }
        song.add_pattern(pat);
    }

    // ── Arrangement ──
    song.arrangement.clear();
    for &pat_idx in &header.pattern_order {
        let idx = pat_idx as usize;
        if idx < song.patterns.len() {
            song.arrangement.push(idx);
        }
    }
    if song.arrangement.is_empty() {
        song.arrangement.push(0);
    }

    song.effect_mode = EffectMode::Compatible;
    // FT2/XM uses linear panning (amplitude proportional to pan position).
    song.panning_law = PanningLaw::Linear;
    if header.linear_frequencies {
        song.slide_mode = crate::audio::pitch::SlideMode::Linear;
        song.format_is_s3m = false;
    } else {
        song.slide_mode = crate::audio::pitch::SlideMode::AmigaPeriod;
        // XM Amiga mode uses standard PAL clock (3.5MHz) and MOD units.
        song.format_is_s3m = false;
    };

    Ok(FormatData {
        song,
        samples: out_samples,
    })
}
