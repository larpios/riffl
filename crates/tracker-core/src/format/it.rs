//! IT (Impulse Tracker) binary parser.
//!
//! Parses IT files directly from raw bytes, fixing xmrs's critical operator
//! precedence bug in volume slide parsing. Maps IT effect bytes directly to
//! our internal Effect format with no float intermediary.

use crate::audio::sample::{LoopMode, Sample};
use crate::pattern::effect::{Effect, EffectMode};
use crate::pattern::note::{Note, Pitch};
use crate::pattern::{Cell, NoteEvent, Pattern, Track};
use crate::song::{Envelope, EnvelopePoint, Instrument, PanningLaw, Song};

use super::{FormatData, FormatError, FormatResult, ModuleLoader};

pub struct ItLoader;

impl ModuleLoader for ItLoader {
    fn name(&self) -> &'static str {
        "Impulse Tracker"
    }

    fn extensions(&self) -> &[&str] {
        &["it"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        data.starts_with(b"IMPM")
    }

    fn load(&self, data: &[u8]) -> FormatResult<FormatData> {
        import_it(data).map_err(FormatError::from)
    }
}

// ─── Binary Helpers ──────────────────────────────────────────────────────────

fn read_u8(data: &[u8], offset: &mut usize) -> u8 {
    let v = data[*offset];
    *offset += 1;
    v
}

fn read_i8(data: &[u8], offset: &mut usize) -> i8 {
    let v = data[*offset] as i8;
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

// ─── BitReader for IT compression ───────────────────────────────────────────

struct BitReader<'a> {
    data: &'a [u8],
    data_index: usize,
    databit: u32,
    databit_index: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            databit: 0,
            databit_index: 0,
            data,
            data_index: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.data_index >= self.data.len() && self.databit_index == 0
    }

    fn read_bits(&mut self, n: u8) -> Option<u32> {
        if n == 0 || n > 32 || self.is_empty() {
            return None;
        }

        let mut retval: u32 = 0;
        for _ in 0..n {
            if self.databit_index == 0 {
                if self.data_index >= self.data.len() {
                    return None;
                }
                self.databit = self.data[self.data_index] as u32;
                self.data_index += 1;
                self.databit_index = 8;
            }
            retval = (retval >> 1) | ((self.databit & 1) << 31);
            self.databit >>= 1;
            self.databit_index -= 1;
        }
        Some(retval >> (32 - n))
    }
}

// ─── IT Header ───────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct ItHeader {
    name: String,
    order_count: u16,
    instrument_count: u16,
    sample_count: u16,
    pattern_count: u16,
    flags: u16,
    initial_speed: u8,
    initial_bpm: u8,
    global_volume: u8,
    mix_volume: u8,
    pan_separation: u8,
    initial_channel_pan: [u8; 64],
    initial_channel_volume: [u8; 64],
    orders: Vec<u8>,
    instrument_offsets: Vec<u32>,
    sample_offsets: Vec<u32>,
    pattern_offsets: Vec<u32>,
    use_instruments: bool,
    linear_slides: bool,
}

fn parse_it_header(data: &[u8]) -> Result<ItHeader, String> {
    if data.len() < 192 + 64 + 64 {
        return Err("IT file too short for header".into());
    }

    let mut off = 0;
    let id = read_string(data, &mut off, 4);
    if id != "IMPM" {
        return Err(format!("Not an IT file (got '{}')", id));
    }

    let name = read_string(data, &mut off, 26);
    let _rows_per_beat = read_u8(data, &mut off);
    let _rows_per_measure = read_u8(data, &mut off);
    let order_count = read_u16_le(data, &mut off);
    let instrument_count = read_u16_le(data, &mut off);
    let sample_count = read_u16_le(data, &mut off);
    let pattern_count = read_u16_le(data, &mut off);
    let _created_with = read_u16_le(data, &mut off);
    let _compatible_with = read_u16_le(data, &mut off);
    let flags = read_u16_le(data, &mut off);
    let _special_flags = read_u16_le(data, &mut off);
    let global_volume = read_u8(data, &mut off);
    let mix_volume = read_u8(data, &mut off);
    let initial_speed = read_u8(data, &mut off);
    let initial_bpm = read_u8(data, &mut off);
    let pan_separation = read_u8(data, &mut off);
    let _pitch_wheel_depth = read_u8(data, &mut off);
    let _message_length = read_u16_le(data, &mut off);
    let _message_offset = read_u32_le(data, &mut off);
    let _reserved = read_u32_le(data, &mut off);

    // Channel pan (64 bytes)
    let mut initial_channel_pan = [0u8; 64];
    initial_channel_pan.copy_from_slice(&data[off..off + 64]);
    off += 64;

    // Channel volume (64 bytes)
    let mut initial_channel_volume = [0u8; 64];
    initial_channel_volume.copy_from_slice(&data[off..off + 64]);
    off += 64;

    // Orders
    let orders = data[off..off + order_count as usize].to_vec();
    off += order_count as usize;

    // Instrument offsets
    let mut instrument_offsets = Vec::with_capacity(instrument_count as usize);
    for _ in 0..instrument_count {
        instrument_offsets.push(read_u32_le(data, &mut off));
    }

    // Sample offsets
    let mut sample_offsets = Vec::with_capacity(sample_count as usize);
    for _ in 0..sample_count {
        sample_offsets.push(read_u32_le(data, &mut off));
    }

    // Pattern offsets
    let mut pattern_offsets = Vec::with_capacity(pattern_count as usize);
    for _ in 0..pattern_count {
        pattern_offsets.push(read_u32_le(data, &mut off));
    }

    let use_instruments = flags & (1 << 2) != 0;
    let linear_slides = flags & (1 << 3) != 0;

    Ok(ItHeader {
        name,
        order_count,
        instrument_count,
        sample_count,
        pattern_count,
        flags,
        initial_speed,
        initial_bpm,
        global_volume,
        mix_volume,
        pan_separation,
        initial_channel_pan,
        initial_channel_volume,
        orders,
        instrument_offsets,
        sample_offsets,
        pattern_offsets,
        use_instruments,
        linear_slides,
    })
}

// ─── IT Pattern (compressed) ─────────────────────────────────────────────────

/// Raw IT pattern slot.
#[derive(Clone, Copy, Default)]
struct ItSlot {
    note: u8,         // 0=none, 1-120=note, 254=notecut, 255=noteoff
    instrument: u8,   // 0=none, 1-99=instrument
    volume: u8,       // volume column value
    has_volume: bool, // whether volume column was explicitly present
    effect: u8,       // effect command (1=A, 4=D, etc.)
    effect_param: u8,
}

struct ItPattern {
    num_rows: u16,
    slots: Vec<Vec<ItSlot>>, // rows × channels (64 max)
}

fn parse_it_pattern(data: &[u8], offset: u32) -> Result<ItPattern, String> {
    if offset == 0 {
        // Empty pattern
        return Ok(ItPattern {
            num_rows: 64,
            slots: vec![vec![ItSlot::default(); 64]; 64],
        });
    }

    let off = offset as usize;
    if off + 8 > data.len() {
        return Err("IT pattern header truncated".into());
    }

    let packed_length = u16::from_le_bytes([data[off], data[off + 1]]);
    let row_count = u16::from_le_bytes([data[off + 2], data[off + 3]]);
    // 4 bytes reserved
    let packed_data = &data[off + 8..off + 8 + packed_length as usize];

    let mut result = vec![
        vec![
            ItSlot {
                note: 253,
                ..ItSlot::default()
            };
            64
        ];
        row_count as usize
    ];
    let mut last_mask_vars = [0u8; 64];
    let mut last_note = [253u8; 64]; // Initialize to "no note"
    let mut last_instrument = [0u8; 64];
    let mut last_volume = [0u8; 64];
    let mut last_effect = [0u8; 64];
    let mut last_effect_param = [0u8; 64];

    let mut iter = packed_data.iter();

    for row_slots in result.iter_mut().take(row_count as usize) {
        while let Some(&channel_mask) = iter.next() {
            if channel_mask == 0 {
                break; // End of row
            }

            let channel = ((channel_mask - 1) & 63) as usize;

            let mask_variable = if channel_mask & 0x80 != 0 {
                let var = *iter.next().ok_or("IT pattern truncated")?;
                last_mask_vars[channel] = var;
                var
            } else {
                last_mask_vars[channel]
            };

            let mut slot = ItSlot {
                note: 253,
                ..ItSlot::default()
            };

            if mask_variable & 0x01 != 0 {
                let n = *iter.next().ok_or("IT pattern truncated")?;
                last_note[channel] = n;
                slot.note = n;
            } else if mask_variable & 0x10 != 0 {
                slot.note = last_note[channel];
            }

            if mask_variable & 0x02 != 0 {
                let inst = *iter.next().ok_or("IT pattern truncated")?;
                last_instrument[channel] = inst;
                slot.instrument = inst;
            } else if mask_variable & 0x20 != 0 {
                slot.instrument = last_instrument[channel];
            }

            if mask_variable & 0x04 != 0 {
                let vol = *iter.next().ok_or("IT pattern truncated")?;
                last_volume[channel] = vol;
                slot.volume = vol;
                slot.has_volume = true;
            } else if mask_variable & 0x40 != 0 {
                slot.volume = last_volume[channel];
                slot.has_volume = true;
            }

            if mask_variable & 0x08 != 0 {
                let eff = *iter.next().ok_or("IT pattern truncated")?;
                let param = *iter.next().ok_or("IT pattern truncated")?;
                last_effect[channel] = eff;
                last_effect_param[channel] = param;
                slot.effect = eff;
                slot.effect_param = param;
            } else if mask_variable & 0x80 != 0 {
                slot.effect = last_effect[channel];
                slot.effect_param = last_effect_param[channel];
            }

            row_slots[channel] = slot;
        }
    }

    Ok(ItPattern {
        num_rows: row_count,
        slots: result,
    })
}

// ─── IT Sample ───────────────────────────────────────────────────────────────

#[allow(dead_code)]
struct ItSampleHeader {
    name: String,
    dos_filename: String,
    global_volume: u8,
    flags: u8,
    default_volume: u8,
    convert_flags: u8,
    default_pan: u8,
    sample_length: u32,
    loop_start: u32,
    loop_end: u32,
    c5_speed: u32,
    sustain_loop_start: u32,
    sustain_loop_end: u32,
    sample_pointer: u32,
    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_sweep: u8,
    vibrato_waveform: u8,
}

#[allow(dead_code)]
impl ItSampleHeader {
    fn is_associated(&self) -> bool {
        self.flags & 0x01 != 0
    }
    fn is_16bit(&self) -> bool {
        self.flags & 0x02 != 0
    }
    fn is_stereo(&self) -> bool {
        self.flags & 0x04 != 0
    }
    fn is_compressed(&self) -> bool {
        self.flags & 0x08 != 0
    }
    fn use_loop(&self) -> bool {
        self.flags & 0x10 != 0
    }
    fn use_sustain_loop(&self) -> bool {
        self.flags & 0x20 != 0
    }
    fn is_pingpong(&self) -> bool {
        self.use_loop() && self.flags & 0x40 != 0
    }
    fn is_signed(&self) -> bool {
        self.convert_flags & 0x01 != 0
    }
    fn is_double_delta(&self) -> bool {
        self.convert_flags & 0x04 != 0
    }
}

fn parse_it_sample_header(data: &[u8], offset: u32) -> Result<ItSampleHeader, String> {
    let off = offset as usize;
    if off + 80 > data.len() {
        return Err("IT sample header truncated".into());
    }

    let mut o = off;
    let _id = read_string(data, &mut o, 4); // "IMPS"
    let dos_filename = read_string(data, &mut o, 12);
    let _reserved = read_u8(data, &mut o);
    let global_volume = read_u8(data, &mut o);
    let flags = read_u8(data, &mut o);
    let default_volume = read_u8(data, &mut o);
    let name = read_string(data, &mut o, 26);
    let convert_flags = read_u8(data, &mut o);
    let default_pan = read_u8(data, &mut o);
    let sample_length = read_u32_le(data, &mut o);
    let loop_start = read_u32_le(data, &mut o);
    let loop_end = read_u32_le(data, &mut o);
    let c5_speed = read_u32_le(data, &mut o);
    let sustain_loop_start = read_u32_le(data, &mut o);
    let sustain_loop_end = read_u32_le(data, &mut o);
    let sample_pointer = read_u32_le(data, &mut o);
    let vibrato_speed = read_u8(data, &mut o);
    let vibrato_depth = read_u8(data, &mut o);
    let vibrato_sweep = read_u8(data, &mut o);
    let vibrato_waveform = read_u8(data, &mut o);

    Ok(ItSampleHeader {
        name,
        dos_filename,
        global_volume,
        flags,
        default_volume,
        convert_flags,
        default_pan,
        sample_length,
        loop_start,
        loop_end,
        c5_speed,
        sustain_loop_start,
        sustain_loop_end,
        sample_pointer,
        vibrato_speed,
        vibrato_depth,
        vibrato_sweep,
        vibrato_waveform,
    })
}

/// IT 2.14 8-bit compressed sample decompression.
fn it_unpack_8bit(input: &[u8], output_len: usize, double_delta: bool) -> Vec<i8> {
    let mut output = Vec::new();
    let mut p_src = input;

    while output.len() < output_len {
        if p_src.len() < 2 {
            break;
        }
        let block_len = u16::from_le_bytes([p_src[0], p_src[1]]) as usize;
        p_src = &p_src[2..];
        if p_src.len() < block_len {
            break;
        }

        let mut bit_reader = BitReader::new(&p_src[..block_len]);
        p_src = &p_src[block_len..];
        let mut left: u8 = 9;
        let mut temp: u8 = 0;
        let mut temp2: u8 = 0;
        let mut block_output_len = 0u32;

        loop {
            if bit_reader.is_empty() || block_output_len == 0x8000 || output.len() >= output_len {
                break;
            }

            let mut bits = match bit_reader.read_bits(left) {
                Some(b) => b as u16,
                None => break,
            };

            if left < 7 {
                if (1u16) << (left - 1) == bits {
                    let nb = match bit_reader.read_bits(3) {
                        Some(b) => b as u8,
                        None => break,
                    };
                    left = if nb + 1 < left { nb + 1 } else { nb + 1 + 1 };
                    continue;
                }
            } else if left < 9 {
                let i: u16 = (0xFF >> (9 - left)) + 4;
                let j: u16 = i - 8;
                if bits > j && bits <= i {
                    bits -= j;
                    left = if (bits as u8) < left {
                        bits as u8
                    } else {
                        (bits + 1) as u8
                    };
                    continue;
                }
            } else if left >= 10 {
                output.push(0);
                block_output_len += 1;
                continue;
            } else if bits >= 256 {
                left = (bits + 1) as u8;
                continue;
            }

            // Sign extension
            if left < 8 {
                let shift = 8 - left;
                let mut c = (bits << shift) as i8;
                c >>= shift;
                bits = c as u16;
            }
            bits = bits.wrapping_add(temp as u16);
            temp = bits as u8;
            temp2 = temp2.wrapping_add(temp);

            let value = if double_delta { temp2 } else { temp };
            output.push(value as i8);
            block_output_len += 1;
        }
    }
    output
}

/// IT 2.14 16-bit compressed sample decompression.
fn it_unpack_16bit(input: &[u8], output_len: usize, double_delta: bool) -> Vec<i16> {
    let mut output = Vec::new();
    let mut p_src = input;

    while output.len() < output_len {
        if p_src.len() < 2 {
            break;
        }
        let block_len = u16::from_le_bytes([p_src[0], p_src[1]]) as usize;
        p_src = &p_src[2..];
        if p_src.len() < block_len {
            break;
        }

        let mut bit_reader = BitReader::new(&p_src[..block_len]);
        p_src = &p_src[block_len..];
        let mut left: u8 = 17;
        let mut temp: i16 = 0;
        let mut temp2: i16 = 0;
        let mut block_output_len = 0u32;

        loop {
            if bit_reader.is_empty() || block_output_len == 0x4000 || output.len() >= output_len {
                break;
            }

            let mut bits = match bit_reader.read_bits(left) {
                Some(b) => b,
                None => break,
            };

            if left < 7 {
                if (1u32) << (left - 1) == bits {
                    let nb = match bit_reader.read_bits(4) {
                        Some(b) => b as u8,
                        None => break,
                    };
                    left = if nb + 1 < left { nb + 1 } else { nb + 1 + 1 };
                    continue;
                }
            } else if left < 17 {
                let i: u32 = (0xFFFF >> (17 - left)) + 8;
                let j: u32 = (i - 16) & 0xFFFF;
                if bits > j && bits <= (i & 0xFFFF) {
                    bits -= j;
                    left = if (bits as u8) < left {
                        bits as u8
                    } else {
                        (bits + 1) as u8
                    };
                    continue;
                }
            } else if left >= 18 {
                output.push(0);
                block_output_len += 1;
                continue;
            } else if bits >= 0x10000 {
                left = (bits + 1) as u8;
                continue;
            }

            // Sign extension
            if left < 16 {
                let shift = 16 - left;
                let mut c = (bits << shift) as i16;
                c >>= shift;
                bits = c as u32;
            }
            bits = bits.wrapping_add(temp as u32);
            temp = bits as i16;
            temp2 = temp2.wrapping_add(temp);

            let value = if double_delta { temp2 } else { temp };
            output.push(value);
            block_output_len += 1;
        }
    }
    output
}

fn load_it_sample_data(file_data: &[u8], sh: &ItSampleHeader) -> Option<Vec<f32>> {
    if !sh.is_associated() || sh.sample_length == 0 {
        return None;
    }

    let start = sh.sample_pointer as usize;
    if start >= file_data.len() {
        return None;
    }
    let sample_data = &file_data[start..];

    let num_channels = if sh.is_stereo() { 2 } else { 1 };
    let num_frames = sh.sample_length as usize;
    let is_16bit = sh.is_16bit();
    let is_delta = sh.is_double_delta(); // bit 2 of convert_flags
    let is_signed = sh.is_signed();

    let mut channels_pcm = Vec::with_capacity(num_channels);

    // IT stores stereo samples as L-block then R-block
    let mut current_sample_data = sample_data;
    for _ch in 0..num_channels {
        let pcm_float: Vec<f32> = if sh.is_compressed() {
            if is_16bit {
                let raw = it_unpack_16bit(current_sample_data, num_frames, is_delta);
                // Advance current_sample_data based on how many bytes were consumed by blocks
                let mut consumed = 0;
                let mut temp_raw_len = 0;
                while temp_raw_len < num_frames && consumed + 2 <= current_sample_data.len() {
                    let block_len = u16::from_le_bytes([
                        current_sample_data[consumed],
                        current_sample_data[consumed + 1],
                    ]) as usize;
                    consumed += 2 + block_len;
                    temp_raw_len += 0x4000; // Each block is up to 0x4000 samples
                }
                current_sample_data =
                    &current_sample_data[consumed.min(current_sample_data.len())..];

                let xor_val = if !is_signed { -32768i16 } else { 0 };
                raw.into_iter()
                    .map(|s| (s ^ xor_val) as f32 / 32768.0)
                    .collect()
            } else {
                let raw = it_unpack_8bit(current_sample_data, num_frames, is_delta);
                let mut consumed = 0;
                let mut temp_raw_len = 0;
                while temp_raw_len < num_frames && consumed + 2 <= current_sample_data.len() {
                    let block_len = u16::from_le_bytes([
                        current_sample_data[consumed],
                        current_sample_data[consumed + 1],
                    ]) as usize;
                    consumed += 2 + block_len;
                    temp_raw_len += 0x8000; // Each block is up to 0x8000 samples for 8-bit
                }
                current_sample_data =
                    &current_sample_data[consumed.min(current_sample_data.len())..];

                let xor_val = if !is_signed { -128i8 } else { 0 };
                raw.into_iter()
                    .map(|s| (s ^ xor_val) as f32 / 128.0)
                    .collect()
            }
        } else {
            // Uncompressed
            let block_data = current_sample_data;

            if is_16bit {
                let mut raw = Vec::with_capacity(num_frames);
                let mut sum = 0i16;
                let xor_val = if !is_signed { -32768i16 } else { 0 };
                for i in 0..num_frames {
                    let b = i * 2;
                    if b + 1 < block_data.len() {
                        let mut sample = i16::from_le_bytes([block_data[b], block_data[b + 1]]);
                        if is_delta {
                            sum = sum.wrapping_add(sample);
                            sample = sum;
                        }
                        raw.push((sample ^ xor_val) as f32 / 32768.0);
                    } else {
                        raw.push(0.0);
                    }
                }
                current_sample_data =
                    &current_sample_data[(num_frames * 2).min(current_sample_data.len())..];
                raw
            } else {
                let mut raw = Vec::with_capacity(num_frames);
                let mut sum = 0i8;
                let xor_val = if !is_signed { -128i8 } else { 0 };
                for i in 0..num_frames {
                    if i < block_data.len() {
                        let mut sample = block_data[i] as i8;
                        if is_delta {
                            sum = sum.wrapping_add(sample);
                            sample = sum;
                        }
                        raw.push((sample ^ xor_val) as f32 / 128.0);
                    } else {
                        raw.push(0.0);
                    }
                }
                current_sample_data =
                    &current_sample_data[num_frames.min(current_sample_data.len())..];
                raw
            }
        };
        channels_pcm.push(pcm_float);
    }

    if channels_pcm.is_empty() {
        return None;
    }

    // Interleave
    if num_channels == 1 {
        Some(channels_pcm.remove(0))
    } else if channels_pcm.len() == 2 {
        let mut interleaved = Vec::with_capacity(num_frames * 2);
        let left = &channels_pcm[0];
        let right = &channels_pcm[1];
        for i in 0..num_frames {
            interleaved.push(*left.get(i).unwrap_or(&0.0));
            interleaved.push(*right.get(i).unwrap_or(&0.0));
        }
        Some(interleaved)
    } else {
        Some(channels_pcm.remove(0))
    }
}

// ─── IT Instrument ──────────────────────────────────────────────────────────

#[allow(dead_code)]
struct ItInstrData {
    name: String,
    note_sample_table: [(u8, u8); 120], // (note, sample) pairs
    volume_envelope: Option<Envelope>,
    panning_envelope: Option<Envelope>,
    fadeout: u16,
    global_volume: u8,
    /// Raw default_pan byte from the IT instrument header.
    /// Bit 7 set means "use this panning value"; bit 7 clear = use channel pan.
    /// Lower 7 bits are the pan value (0-64, 32 = centre).
    default_pan: u8,
}

/// Panning and pitch envelope values are 0-64 where 32 is center.
/// Volume envelope values are 0-64.
/// Both types are normalized to a bipolar scale for spatial/frequency logic if `bipolar` is true.
fn parse_it_envelope(data: &[u8], offset: &mut usize, bipolar: bool) -> Envelope {
    let flags = read_u8(data, offset);
    let node_count = read_u8(data, offset);
    let loop_start = read_u8(data, offset);
    let loop_end = read_u8(data, offset);
    let sustain_loop_start = read_u8(data, offset);
    let sustain_loop_end = read_u8(data, offset);

    let mut points = Vec::new();
    for _ in 0..25 {
        let raw = data[*offset];
        *offset += 1;
        let frame = read_u16_le(data, offset);

        let value = if bipolar {
            // Panning/Pitch: 0..64 mapped to -1.0..1.0 (32 = center)
            (raw as f32 - 32.0) / 32.0
        } else {
            // Volume: 0..64 mapped to 0.0..1.0
            raw as f32 / 64.0
        };
        points.push(EnvelopePoint {
            frame,
            value: value.clamp(if bipolar { -1.0 } else { 0.0 }, 1.0),
        });
    }
    points.truncate(node_count.min(25) as usize);

    // Skip trailing byte
    *offset += 1;

    Envelope {
        enabled: flags & 0x01 != 0,
        points,
        sustain_enabled: flags & 0x04 != 0,
        sustain_start_point: sustain_loop_start as usize,
        sustain_end_point: sustain_loop_end as usize,
        loop_enabled: flags & 0x02 != 0,
        loop_start_point: loop_start as usize,
        loop_end_point: loop_end as usize,
    }
}

fn parse_it_instrument_post2(data: &[u8], offset: u32) -> Result<ItInstrData, String> {
    let mut off = offset as usize;
    if off + 4 > data.len() {
        return Err("IT instrument header truncated".into());
    }

    let id = read_string(data, &mut off, 4);
    if id != "IMPI" {
        return Err(format!("Bad IT instrument header: '{}'", id));
    }

    let _dos_filename = read_string(data, &mut off, 12);
    let _reserved1 = read_u8(data, &mut off);
    let _nna = read_u8(data, &mut off);
    let _dct = read_u8(data, &mut off);
    let _dca = read_u8(data, &mut off);
    let fadeout = read_u16_le(data, &mut off);
    let _pps = read_i8(data, &mut off);
    let _ppc = read_u8(data, &mut off);
    let global_volume = read_u8(data, &mut off);
    let default_pan = read_u8(data, &mut off);
    let _rvv = read_u8(data, &mut off);
    let _rpv = read_u8(data, &mut off);
    let _tracker_version = read_u16_le(data, &mut off);
    let _num_samples = read_u8(data, &mut off);
    let _reserved2 = read_u8(data, &mut off);
    let name = read_string(data, &mut off, 26);
    let _cutoff = read_u8(data, &mut off);
    let _resonance = read_u8(data, &mut off);
    let _midi_channel = read_u8(data, &mut off);
    let _midi_program = read_u8(data, &mut off);
    let _midi_bank = read_u16_le(data, &mut off);

    // Note-sample keyboard table: 120 × (note, sample)
    let mut note_sample_table = [(0u8, 0u8); 120];
    for entry in &mut note_sample_table {
        let note = read_u8(data, &mut off);
        let sample = read_u8(data, &mut off);
        *entry = (note, sample);
    }

    // Envelopes
    // Volume: unsigned 0..64
    // Panning: signed -32..+32
    // Pitch: signed -32..+32
    let volume_envelope = parse_it_envelope(data, &mut off, false);
    let panning_envelope = parse_it_envelope(data, &mut off, true);
    let _pitch_envelope = parse_it_envelope(data, &mut off, true);

    let vol_env = if volume_envelope.enabled {
        Some(volume_envelope)
    } else {
        None
    };
    let pan_env = if panning_envelope.enabled {
        Some(panning_envelope)
    } else {
        None
    };

    Ok(ItInstrData {
        name,
        note_sample_table,
        volume_envelope: vol_env,
        panning_envelope: pan_env,
        fadeout,
        global_volume,
        default_pan,
    })
}

// ─── IT Effect Conversion ────────────────────────────────────────────────────

/// Convert IT volume column byte to an optional Effect and/or volume value.
fn convert_it_volume_column(vol: u8) -> (Option<u8>, Option<Effect>) {
    match vol {
        0..=64 => (Some(vol), None),
        // 65-74: Fine volume up
        65..=74 => {
            let param = vol - 65;
            (None, Some(Effect::new(0x0E, 0xA0 | param)))
        }
        // 75-84: Fine volume down
        75..=84 => {
            let param = vol - 75;
            (None, Some(Effect::new(0x0E, 0xB0 | param)))
        }
        // 85-94: Volume slide up
        85..=94 => {
            let param = vol - 85;
            (None, Some(Effect::new(0x0A, param << 4)))
        }
        // 95-104: Volume slide down
        95..=104 => {
            let param = vol - 95;
            (None, Some(Effect::new(0x0A, param)))
        }
        // 105-114: Pan slide left (Lx), x = vol - 105
        105..=114 => {
            let x = vol - 105;
            (None, Some(Effect::new(0x12, x))) // PanningSlide: low nibble = left speed
        }
        // 115-124: Pan slide right (Rx), x = vol - 115
        115..=124 => {
            let x = vol - 115;
            (None, Some(Effect::new(0x12, x << 4))) // PanningSlide: high nibble = right speed
        }
        // 128-192: Set panning
        128..=192 => {
            let param = ((vol - 128) as u32 * 255 / 64) as u8;
            (None, Some(Effect::new(0x08, param)))
        }
        // 193-202: Tone portamento
        193..=202 => {
            let param = vol - 193;
            (None, Some(Effect::new(0x03, param << 4)))
        }
        // 203-212: Vibrato depth
        203..=212 => {
            let param = vol - 203;
            (None, Some(Effect::new(0x04, param)))
        }
        // 213-222: Pitch slide up (f)
        213..=222 => {
            let param = vol - 213;
            (None, Some(Effect::new(0x01, param)))
        }
        // 223..=232: Pitch slide down (e)
        223..=232 => {
            let param = vol - 223;
            (None, Some(Effect::new(0x02, param)))
        }
        _ => (None, None),
    }
}

/// Convert IT effect command + param to our Effect — direct byte mapping.
/// IT uses letter-based commands: A=1, B=2, C=3, D=4, etc.
/// This is where the xmrs bug was: `nibble_high = fx & 0xF0 >> 4` (wrong precedence).
/// We fix it with `(param >> 4) & 0x0F` and `param & 0x0F`.
fn convert_it_effect(cmd: u8, param: u8) -> Option<Effect> {
    if cmd == 0 && param == 0 {
        return None;
    }

    let hi = (param >> 4) & 0x0F; // CORRECT: shift first, then mask
    let lo = param & 0x0F;

    match cmd {
        // A: Set speed
        0x01 => Some(Effect::new(0x0F, param.min(31))),

        // B: Position jump
        0x02 => Some(Effect::new(0x0B, param)),

        // C: Pattern break (stored as decimal in IT: high nibble × 10 + low)
        0x03 => Some(Effect::new(0x0D, param)),

        // D: Volume slide — THE BUG FIX IS HERE
        0x04 => {
            if hi == 0x0F && lo != 0 {
                // DFy: Fine volume slide down by y
                Some(Effect::new(0x0E, 0xB0 | lo))
            } else if lo == 0x0F && hi != 0 {
                // DxF: Fine volume slide up by x
                Some(Effect::new(0x0E, 0xA0 | hi))
            } else if hi == 0 {
                // D0y: Volume slide down by y
                Some(Effect::new(0x0A, lo))
            } else if lo == 0 {
                // Dx0: Volume slide up by x
                Some(Effect::new(0x0A, hi << 4))
            } else {
                // Both non-zero: slide up takes priority
                Some(Effect::new(0x0A, hi << 4))
            }
        }

        // E: Pitch slide down
        0x05 => {
            if hi == 0x0F {
                // EFy: Fine pitch slide down
                Some(Effect::new(0x0E, 0x20 | lo))
            } else if hi == 0x0E {
                // EEy: Extra fine pitch slide down
                Some(Effect::new(0x22, lo))
            } else {
                Some(Effect::new(0x02, param))
            }
        }

        // F: Pitch slide up
        0x06 => {
            if hi == 0x0F {
                // FFy: Fine pitch slide up
                Some(Effect::new(0x0E, 0x10 | lo))
            } else if hi == 0x0E {
                // FEy: Extra fine pitch slide up
                Some(Effect::new(0x21, lo))
            } else {
                Some(Effect::new(0x01, param))
            }
        }

        // G: Tone portamento
        0x07 => Some(Effect::new(0x03, param)),

        // H: Vibrato
        0x08 => Some(Effect::new(0x04, param)),

        // I: Tremor
        0x09 => Some(Effect::new(0x15, param)),

        // J: Arpeggio
        0x0A => {
            if param != 0 {
                Some(Effect::new(0x00, param))
            } else {
                None
            }
        }

        // K: Vibrato + volume slide
        0x0B => {
            // Same volume slide decoding as D but combined with vibrato
            if hi == 0x0F && lo != 0 {
                Some(Effect::new(0x06, 0x0F | (lo & 0x0F)))
            } else if lo == 0x0F && hi != 0 {
                Some(Effect::new(0x06, (hi << 4) | 0x0F))
            } else {
                Some(Effect::new(0x06, param))
            }
        }

        // L: Tone portamento + volume slide
        0x0C => {
            if hi == 0x0F && lo != 0 {
                Some(Effect::new(0x05, 0x0F | (lo & 0x0F)))
            } else if lo == 0x0F && hi != 0 {
                Some(Effect::new(0x05, (hi << 4) | 0x0F))
            } else {
                Some(Effect::new(0x05, param))
            }
        }

        // M: Set channel volume
        0x0D => Some(Effect::new(0x13, param.min(64))),

        // N: Channel volume slide
        0x0E => {
            if hi == 0x0F && lo != 0 {
                // NFy: Fine channel volume slide down
                Some(Effect::new(0x14, 0xB0 | lo))
            } else if lo == 0x0F && hi != 0 {
                // NxF: Fine channel volume slide up
                Some(Effect::new(0x14, 0xA0 | hi))
            } else if hi == 0 {
                // N0y: Channel volume slide down
                Some(Effect::new(0x14, lo))
            } else {
                // Nx0: Channel volume slide up
                Some(Effect::new(0x14, hi << 4))
            }
        }

        // O: Sample offset
        0x0F => Some(Effect::new(0x09, param)),

        // P: Panning slide
        0x10 => Some(Effect::new(0x12, param)),

        // Q: Retrigger note
        0x11 => Some(Effect::new(0x16, param)),

        // R: Tremolo
        0x12 => Some(Effect::new(0x07, param)),

        // S: Extended effects (Sxy)
        0x13 => {
            match hi {
                0x1 => Some(Effect::new(0x0E, 0x30 | lo)), // S1x: Glissando
                0x3 => Some(Effect::new(0x0E, 0x40 | lo)), // S3x: Vibrato waveform
                0x4 => Some(Effect::new(0x0E, 0x70 | lo)), // S4x: Tremolo waveform
                0x5 => Some(Effect::new(0x0E, 0x50 | lo)), // S5x: Set finetune
                0x6 => Some(Effect::new(0x0E, 0x60 | lo)), // S6x: Pattern loop
                0x8 => Some(Effect::new(0x08, lo * 17)),   // S8x: Set panning position
                0x9 => Some(Effect::new(0x0E, 0x90 | lo)), // S9x: Retrigger note (every x ticks)
                0xB => Some(Effect::new(0x0E, 0x60 | lo)), // SBx: Pattern loop (alt)
                0xC => Some(Effect::new(0x0E, 0xC0 | lo)), // SCx: Note cut
                0xD => Some(Effect::new(0x0E, 0xD0 | lo)), // SDx: Note delay
                0xE => Some(Effect::new(0x0E, 0xE0 | lo)), // SEx: Pattern delay
                _ => None,
            }
        }

        // T: Set tempo
        0x14 => Some(Effect::new(0x0F, param.max(32))),

        // U: Fine vibrato
        0x15 => Some(Effect::new(0x04, param)), // map to regular vibrato for now

        // V: Set global volume
        0x16 => Some(Effect::new(0x10, param.min(128))),

        // W: Global volume slide
        0x17 => Some(Effect::new(0x11, param)),

        // X: Set panning
        0x18 => {
            if param == 0x80 {
                // Surround: map to center for now
                Some(Effect::new(0x08, 128))
            } else if param <= 0x40 {
                // IT panning is 0..64 (0x40). Internal engine uses 0..255.
                let scaled = (param as u32 * 255 / 64) as u8;
                Some(Effect::new(0x08, scaled))
            } else {
                None
            }
        }

        // Y: Panbrello
        0x19 => Some(Effect::new(0x18, param)),

        // Z: MIDI macro
        0x1A => None, // Not supported

        _ => None,
    }
}

// ─── Main Import ─────────────────────────────────────────────────────────────

/// Import an IT file from raw bytes, producing our internal FormatData.
pub fn import_it(data: &[u8]) -> Result<FormatData, String> {
    // ── Parse header ──
    let header = parse_it_header(data)?;

    // ── Parse sample headers and data ──
    let mut sample_headers = Vec::new();
    let mut sample_data_vec = Vec::new();
    for &offset in &header.sample_offsets {
        let sh = parse_it_sample_header(data, offset)?;
        let sdata = load_it_sample_data(data, &sh);
        sample_headers.push(sh);
        sample_data_vec.push(sdata);
    }

    // ── Parse instruments (if used) ──
    let mut it_instruments = Vec::new();
    if header.use_instruments {
        for &offset in &header.instrument_offsets {
            match parse_it_instrument_post2(data, offset) {
                Ok(inst) => it_instruments.push(inst),
                Err(_) => {
                    // Fallback: create empty instrument
                    it_instruments.push(ItInstrData {
                        name: String::new(),
                        note_sample_table: [(0, 0); 120],
                        volume_envelope: None,
                        panning_envelope: None,
                        fadeout: 0,
                        global_volume: 128,
                        default_pan: 0, // no override
                    });
                }
            }
        }
    }

    // ── Build samples and instruments ──
    let mut out_samples: Vec<Sample> = Vec::new();
    let mut out_instruments: Vec<Instrument> = Vec::new();

    // sample_idx → tracker_instrument_idx mapping
    let mut sample_to_tracker_inst: Vec<Option<usize>> = vec![None; sample_headers.len()];

    for (s_idx, sh) in sample_headers.iter().enumerate() {
        if let Some(ref float_data) = sample_data_vec[s_idx] {
            if float_data.is_empty() {
                continue;
            }

            let channels = if sh.is_stereo() { 2u16 } else { 1u16 };
            let c5_speed = if sh.c5_speed > 0 { sh.c5_speed } else { 16726 };

            let mut sample = Sample::new(
                float_data.clone(),
                c5_speed,
                channels,
                Some(if sh.name.is_empty() {
                    sh.dos_filename.clone()
                } else {
                    sh.name.clone()
                }),
            );
            // Sample volume stays at 1.0 (default).
            // All IT volume factors are consolidated on the Instrument to avoid
            // double-multiplication (the mixer multiplies inst_vol * sample.volume).

            // Loop
            if sh.use_loop() && sh.loop_end > sh.loop_start {
                let loop_start = sh.loop_start as usize;
                let loop_end = (sh.loop_end as usize).saturating_sub(1);
                if loop_end > loop_start {
                    let mode = if sh.is_pingpong() {
                        LoopMode::PingPong
                    } else {
                        LoopMode::Forward
                    };
                    sample = sample.with_loop(mode, loop_start, loop_end);
                }
            }

            // Sustain loop (active while key is held, plays through on release)
            if sh.use_sustain_loop() && sh.sustain_loop_end > sh.sustain_loop_start {
                let sus_start = sh.sustain_loop_start as usize;
                let sus_end = (sh.sustain_loop_end as usize).saturating_sub(1);
                if sus_end > sus_start {
                    let mode = if sh.flags & 0x40 != 0 && !sh.use_loop() {
                        // Bit 6 is ping-pong for sustain loop when no regular loop
                        LoopMode::PingPong
                    } else {
                        LoopMode::Forward
                    };
                    sample = sample.with_sustain_loop(mode, sus_start, sus_end);
                }
            }

            // IT c5_speed is the playback rate for note C-5 (MIDI 60).
            sample = sample.with_base_note(60);

            sample_to_tracker_inst[s_idx] = Some(out_instruments.len());

            let sname = if sh.name.is_empty() {
                sh.dos_filename.clone()
            } else {
                sh.name.clone()
            };

            let mut inst = Instrument::new(sname);
            inst.sample_index = Some(out_samples.len());
            // Combine sample default volume with sample global volume.
            // This is the sample-level static multiplier.
            sample.volume = (sh.default_volume as f32 / 64.0) * (sh.global_volume as f32 / 64.0);
            inst.volume = 1.0; // instrument global volume will be multiplied in later

            // Bit 7 set means "use this panning value"; bit 7 clear means "use channel pan".
            if sh.default_pan & 0x80 != 0 {
                let pan_val = sh.default_pan & 0x7F; // 0..64, 32 = centre
                inst.panning = Some((pan_val as f32 - 32.0) / 32.0);
            }

            out_samples.push(sample);
            out_instruments.push(inst);
        }
    }

    // Build IT instrument → sample resolution table
    // inst_to_sample[it_inst_idx][note] = (tracker_instrument_idx, target_note, instr_global_volume)
    let mut inst_to_sample: Vec<Vec<Option<(usize, u8, f32)>>> = Vec::new();

    // If instruments are used, enrich per-sample instruments with envelope data
    // and build the inst_to_sample resolution table
    if header.use_instruments {
        for it_inst in &it_instruments {
            let mut note_map: Vec<Option<(usize, u8, f32)>> = Vec::with_capacity(120);
            let inst_vol = it_inst.global_volume as f32 / 128.0;

            for &(target_note, sample) in &it_inst.note_sample_table {
                if sample > 0 {
                    let si = (sample as usize).saturating_sub(1);
                    let tracker_idx = sample_to_tracker_inst.get(si).copied().flatten();
                    if let Some(ti) = tracker_idx {
                        note_map.push(Some((ti, target_note, inst_vol)));
                    } else {
                        note_map.push(None);
                    }
                } else {
                    note_map.push(None);
                }
            }

            // Push envelope data to each referenced sample's instrument entry.
            // Track which instruments we've already updated to avoid repeated
            // multiplication of the IT instrument global volume.
            let mut updated_instruments = std::collections::HashSet::new();
            for (tracker_idx, _, vol) in note_map.iter().flatten() {
                let tracker_idx = *tracker_idx;
                if tracker_idx < out_instruments.len() && updated_instruments.insert(tracker_idx) {
                    // Combine IT instrument global volume with existing sample-level volume.
                    out_instruments[tracker_idx].volume *= *vol;
                    out_instruments[tracker_idx].fadeout = it_inst.fadeout;

                    // Apply instrument default panning if bit 7 is set (overrides channel/sample pan).
                    if it_inst.default_pan & 0x80 != 0 {
                        let pan_val = it_inst.default_pan & 0x7F; // 0..64, 32 = centre
                        out_instruments[tracker_idx].panning = Some((pan_val as f32 - 32.0) / 32.0);
                    }

                    if out_instruments[tracker_idx].volume_envelope.is_none() {
                        out_instruments[tracker_idx].volume_envelope =
                            it_inst.volume_envelope.clone();
                    }
                    if out_instruments[tracker_idx].panning_envelope.is_none() {
                        out_instruments[tracker_idx].panning_envelope =
                            it_inst.panning_envelope.clone();
                    }
                }
            }

            inst_to_sample.push(note_map);
        }
    }

    // ── Build song ──
    let mut song = Song::new(header.name.clone(), header.initial_bpm as f64);
    song.tpl = header.initial_speed as u32;
    song.instruments = out_instruments;

    // ── Determine active channels ──
    let num_channels = {
        let mut max_ch = 0;
        for &offset in &header.pattern_offsets {
            if let Ok(pat) = parse_it_pattern(data, offset) {
                for row in &pat.slots {
                    for (ch, slot) in row.iter().enumerate() {
                        if slot.note <= 119
                            || slot.instrument > 0
                            || slot.has_volume
                            || slot.effect > 0
                            || slot.effect_param > 0
                        {
                            max_ch = max_ch.max(ch + 1);
                        }
                    }
                }
            }
        }
        max_ch.max(1)
    };

    // Initialize all 64 tracks from IT header
    let mut tracks = Vec::with_capacity(64);
    for i in 0..64 {
        let it_vol = header.initial_channel_volume[i];
        let it_pan = header.initial_channel_pan[i];
        let mut t = Track::with_number(i + 1);

        // IT channel volume is 0-64.
        t.volume = it_vol as f32 / 64.0;

        // IT channel pan is 0(L) to 32(C) to 64(R). 100/128+ are surround/muted.
        let pan_val = if (it_pan & 127) <= 64 {
            it_pan & 127
        } else {
            32
        };
        t.pan = (pan_val as f32 - 32.0) / 32.0;

        if (it_pan & 128) != 0 {
            t.muted = true;
        }
        tracks.push(t);
    }
    song.tracks = tracks;
    let num_channels = 64; // IT always has 64 channels in the header environment

    song.global_volume = (header.global_volume as f32 / 128.0) * (header.mix_volume as f32 / 128.0);
    // If Global Volume is 128 and Mix Volume is 128, it will be 1.0.
    // If Global Vol is 128 and Mix Vol is 48 (standard), it will be ~0.375.

    song.pan_separation = header.pan_separation;
    song.panning_law = PanningLaw::Linear; // IT uses linear panning (amplitude proportional to pan position)

    // ── Convert patterns ──
    song.patterns.clear();
    let mut last_instrument: Vec<Option<usize>> = vec![None; num_channels];
    let mut last_sample: Vec<Option<u8>> = vec![None; num_channels];

    for &offset in &header.pattern_offsets {
        let it_pat = parse_it_pattern(data, offset)?;
        let num_rows = it_pat.num_rows as usize;
        let mut pat = Pattern::new(num_rows.max(1), num_channels);

        // Copy IT channel panning/volume/mute from song.tracks into the pattern tracks,
        // so that mixer.tick(pattern) sees the correct initial panning every row.
        for (pat_track, song_track) in pat.tracks_mut().iter_mut().zip(song.tracks.iter()) {
            pat_track.pan = song_track.pan;
            pat_track.volume = song_track.volume;
            pat_track.muted = song_track.muted;
        }

        for (r_idx, row) in it_pat.slots.iter().enumerate() {
            for c_idx in 0..num_channels {
                let slot = row[c_idx];
                let mut cell = Cell::empty();

                // ── Note ──
                let mut note_event = None;
                if slot.note == 255 {
                    note_event = Some(NoteEvent::Off);
                } else if slot.note == 254 {
                    note_event = Some(NoteEvent::Cut);
                } else if slot.note <= 119 {
                    let note_val = slot.note; // IT notes in packed data are 0-119
                    let octave = note_val / 12;
                    let semitone = note_val % 12;
                    if let Some(pitch) = Pitch::from_semitone(semitone) {
                        let vel = 127;
                        note_event = Some(NoteEvent::On(Note::new(pitch, octave, vel, 0)));
                    }
                }

                // ── Instrument and sample resolution ──
                let resolved_inst_idx = if slot.instrument > 0 {
                    let i = (slot.instrument as usize).saturating_sub(1);
                    last_instrument[c_idx] = Some(i);
                    Some(i)
                } else {
                    last_instrument[c_idx]
                };

                let mut mapped_sample_idx = None;
                let mut target_pitch = None;

                if let Some(i) = resolved_inst_idx {
                    if header.use_instruments {
                        // Resolve through IT instrument's note-sample table
                        if i < inst_to_sample.len() {
                            // Use the note to look up the sample.
                            // If it's a note-only event, use the note from the pattern.
                            // If it's an instrument-only event, we can't really "play" it yet,
                            // but we can at least resolve a sample identifier.
                            let lookup_note = if slot.note <= 119 {
                                slot.note as usize
                            } else {
                                // Default for instrument-only event
                                60 // C-5 is the anchor
                            };

                            if let Some((tracker_idx, note_pitch, _instr_vol)) =
                                inst_to_sample[i].get(lookup_note).copied().flatten()
                            {
                                mapped_sample_idx = Some(tracker_idx as u8);
                                last_sample[c_idx] = mapped_sample_idx;
                                target_pitch = Some(note_pitch);

                                if slot.instrument > 0 {
                                    cell.instrument = mapped_sample_idx;
                                }
                            }
                        }
                    } else {
                        // Direct sample mode
                        if let Some(tracker_idx) = sample_to_tracker_inst.get(i).copied().flatten()
                        {
                            mapped_sample_idx = Some(tracker_idx as u8);
                            last_sample[c_idx] = mapped_sample_idx;
                            if slot.instrument > 0 {
                                cell.instrument = mapped_sample_idx;
                            }
                        }
                    }
                }

                // ── Finalize note event ──
                if let Some(NoteEvent::On(mut n)) = note_event {
                    let inst_to_use = mapped_sample_idx.or(last_sample[c_idx]);

                    if let Some(idx) = inst_to_use {
                        n.instrument = idx;

                        // Apply keyboard table transposition if available
                        if let Some(pitch) = target_pitch {
                            n.pitch = Pitch::from_semitone(pitch % 12).unwrap_or(Pitch::C);
                            n.octave = pitch / 12;
                        }

                        cell.note = Some(NoteEvent::On(n));
                    } else {
                        // No instrument/sample known yet, can't play
                        cell.note = None;
                    }
                } else {
                    cell.note = note_event;
                }

                // ── Volume column ──
                if slot.has_volume {
                    let (vol_set, vol_effect) = convert_it_volume_column(slot.volume);
                    if let Some(v) = vol_set {
                        cell.volume = Some(v);
                    }
                    if let Some(eff) = vol_effect {
                        if cell.effects.len() < crate::pattern::effect::MAX_EFFECTS_PER_CELL {
                            cell.effects.push(eff);
                        }
                    }
                }

                // ── Effect column ──
                if let Some(eff) = convert_it_effect(slot.effect, slot.effect_param) {
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
    for &order in &header.orders {
        if order == 255 {
            break; // End of song
        }
        if order == 254 {
            continue; // Skip marker
        }
        let idx = order as usize;
        if idx < song.patterns.len() {
            song.arrangement.push(idx);
        }
    }
    if song.arrangement.is_empty() {
        song.arrangement.push(0);
    }

    song.effect_mode = EffectMode::Compatible;
    song.format_is_it = true;
    if header.linear_slides {
        song.slide_mode = crate::audio::pitch::SlideMode::Linear;
        song.format_is_s3m = false;
    } else {
        song.slide_mode = crate::audio::pitch::SlideMode::AmigaPeriod;
        song.format_is_s3m = true;
    };

    Ok(FormatData {
        song,
        samples: out_samples,
    })
}
