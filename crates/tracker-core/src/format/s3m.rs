//! S3M (ScreamTracker III) binary parser.
//!
//! Parses S3M files from raw bytes, supporting PCM samples and standard effects.
//! Maps S3M-specific features to the tracker-core internal format.

#[cfg(feature = "adlib")]
use crate::audio::adlib::AdlibSynthesizer;
use crate::audio::sample::{LoopMode, Sample};
use crate::pattern::effect::{Effect, EffectMode};
use crate::pattern::note::{Note, Pitch};
use crate::pattern::{Cell, NoteEvent, Pattern, Track};
use crate::song::{Instrument, Song};

use super::{FormatData, FormatError, FormatResult, ModuleLoader};

pub struct S3mLoader;

impl ModuleLoader for S3mLoader {
    fn name(&self) -> &'static str {
        "ScreamTracker III"
    }

    fn extensions(&self) -> &[&str] {
        &["s3m"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        if data.len() < 0x30 {
            return false;
        }
        &data[0x2C..0x30] == b"SCRM"
    }

    fn load(&self, data: &[u8]) -> FormatResult<FormatData> {
        import_s3m(data).map_err(FormatError::from)
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

// ─── S3M Header ──────────────────────────────────────────────────────────────

struct S3mHeader {
    name: String,
    #[allow(dead_code)]
    ord_num: u16,
    ins_num: u16,
    #[allow(dead_code)]
    pat_num: u16,
    #[allow(dead_code)]
    flags: u16,
    global_vol: u8,
    initial_speed: u8,
    initial_tempo: u8,
    #[allow(dead_code)]
    master_vol: u8,
    channel_settings: [u8; 32],
    orders: Vec<u8>,
    inst_pointers: Vec<u16>,
    pat_pointers: Vec<u16>,
}

fn parse_s3m_header(data: &[u8]) -> Result<S3mHeader, String> {
    if data.len() < 0x60 {
        return Err("File too short for S3M header".into());
    }

    let mut off = 0;
    let name = read_string(data, &mut off, 28);

    // 0x1C (28) -> 0x1A
    if data[0x1C] != 0x1A {
        // Warning: Some files might not have this, but standard says they should.
    }
    off += 1;

    let _type = read_u8(data, &mut off); // 0x1D
    let _unused = read_u16_le(data, &mut off); // 0x1E

    let ord_num = read_u16_le(data, &mut off); // 0x20
    let ins_num = read_u16_le(data, &mut off); // 0x22
    let pat_num = read_u16_le(data, &mut off); // 0x24
    let flags = read_u16_le(data, &mut off); // 0x26
    let _cwt_v = read_u16_le(data, &mut off); // 0x28
    let _ffi = read_u16_le(data, &mut off); // 0x2A

    let magic = &data[0x2C..0x30];
    if magic != b"SCRM" {
        return Err("Not a valid S3M file (missing SCRM tag)".into());
    }
    off += 4;

    let global_vol = read_u8(data, &mut off); // 0x30
    let initial_speed = read_u8(data, &mut off); // 0x31
    let initial_tempo = read_u8(data, &mut off); // 0x32
    let master_vol = read_u8(data, &mut off); // 0x33
    let _ultra_click = read_u8(data, &mut off); // 0x34
    let _default_pan_flag = read_u8(data, &mut off); // 0x35

    // Reserved
    off += 8; // 0x36..0x3E

    // Special custom pointer (for Panning)
    let _special = read_u16_le(data, &mut off); // 0x3E

    // Channel settings (32 bytes) at 0x40
    let mut channel_settings = [0u8; 32];
    for ch in &mut channel_settings {
        *ch = read_u8(data, &mut off);
    }

    // Orders at 0x60
    let orders_len = ord_num as usize;
    if off + orders_len > data.len() {
        return Err("File too short for orders".into());
    }
    let orders = data[off..off + orders_len].to_vec();
    off += orders_len;

    // Parapointers for Instruments
    let mut inst_pointers = Vec::with_capacity(ins_num as usize);
    for _ in 0..ins_num {
        if off + 2 > data.len() {
            return Err("File truncated in instrument pointers".into());
        }
        inst_pointers.push(read_u16_le(data, &mut off));
    }

    // Parapointers for Patterns
    let mut pat_pointers = Vec::with_capacity(pat_num as usize);
    for _ in 0..pat_num {
        if off + 2 > data.len() {
            return Err("File truncated in pattern pointers".into());
        }
        pat_pointers.push(read_u16_le(data, &mut off));
    }

    Ok(S3mHeader {
        name,
        ord_num,
        ins_num,
        pat_num,
        flags,
        global_vol,
        initial_speed,
        initial_tempo,
        master_vol,
        channel_settings,
        orders,
        inst_pointers,
        pat_pointers,
    })
}

// ─── Instruments ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AdlibData {
    pub registers: Vec<u8>,
    pub is_opl3: bool,
}

impl Default for AdlibData {
    fn default() -> Self {
        Self {
            registers: vec![0u8; 256],
            is_opl3: false,
        }
    }
}

struct S3mInstrument {
    name: String,
    sample_data: Option<Vec<f32>>,
    #[allow(dead_code)]
    sample_len: u32,
    loop_begin: u32,
    loop_end: u32,
    volume: u8,
    c2spd: u32,
    flags: u8,
    #[allow(dead_code)]
    adlib_data: Option<AdlibData>,
}

fn parse_s3m_instrument(data: &[u8], para_ptr: u16) -> Result<S3mInstrument, String> {
    let offset = (para_ptr as usize) * 16;
    if offset + 0x50 > data.len() {
        return Err("Instrument header out of bounds".into());
    }

    let mut off = offset;
    let type_byte = read_u8(data, &mut off);
    eprintln!(
        "S3M instrument para_ptr {} type {} at offset {}",
        para_ptr, type_byte, offset
    );
    let _dos_filename = read_string(data, &mut off, 12);

    // If not a PCM instrument (type 1), parse as Adlib
    // Types: 0=Empty, 1=Sample, 2=Adlib/OPL2, 3=ODetune/OPL3
    if type_byte != 1 {
        let name = read_string(data, &mut off, 28);
        let _id = read_string(data, &mut off, 4);

        let is_opl3 = type_byte == 3;
        let reg_size = if is_opl3 { 512 } else { 256 };
        let mut registers = vec![0u8; reg_size.min(data.len().saturating_sub(offset + 0x50))];

        let reg_start = offset + 0x50;
        let reg_end = (reg_start + registers.len()).min(data.len());
        registers[..reg_end.saturating_sub(reg_start)].copy_from_slice(&data[reg_start..reg_end]);

        return Ok(S3mInstrument {
            name,
            sample_data: None,
            sample_len: 0,
            loop_begin: 0,
            loop_end: 0,
            volume: 64,
            c2spd: 8363,
            flags: 0,
            adlib_data: Some(AdlibData { registers, is_opl3 }),
        });
    }

    // Sample data pointer (24-bit value)
    // Stored as: ptrDataH (upper 8 bits at 0x0D), ptrDataL (lower 16 bits at 0x0E-0x0F)
    // Combined as 24-bit: (ptrDataH << 16) | ptrDataL
    // This IS the file offset - no multiplication needed
    off = offset + 0x0D;
    let ptr_data_h = data[off] as u32;
    let ptr_data_l = u16::from_le_bytes([data[off + 1], data[off + 2]]);
    let sample_ptr = ((ptr_data_h as u32) << 16) | (ptr_data_l as u32);
    eprintln!(
        "S3M instrument ptr_data_h={}, ptr_data_l={}, sample_ptr={}, data len={}",
        ptr_data_h,
        ptr_data_l,
        sample_ptr,
        data.len()
    );

    off += 3; // Skip the 3-byte sample pointer
    let length = read_u32_le(data, &mut off); // 0x10
    let loop_begin = read_u32_le(data, &mut off); // 0x14
    let loop_end = read_u32_le(data, &mut off); // 0x18
    let volume = read_u8(data, &mut off); // 0x1C
    let _dsk = read_u8(data, &mut off); // 0x1D
    let _pack = read_u8(data, &mut off); // 0x1E
    let flags = read_u8(data, &mut off); // 0x1F
    let c2spd = read_u32_le(data, &mut off); // 0x20
    eprintln!(
        "S3M instrument length={}, volume={}, flags={}, c2spd={}",
        length, volume, flags, c2spd
    );

    off = offset + 0x30;
    let name = read_string(data, &mut off, 28);
    let _id = read_string(data, &mut off, 4); // "SCRS"

    // Check bounds
    if sample_ptr as usize + length as usize > data.len() {
        eprintln!(
            "S3M instrument sample_ptr {} + length {} > data len {}, skipping sample",
            sample_ptr,
            length,
            data.len()
        );
        // Truncated sample
        return Ok(S3mInstrument {
            name,
            sample_data: None, // Or partial?
            sample_len: 0,
            loop_begin: 0,
            loop_end: 0,
            volume,
            c2spd,
            flags,
            adlib_data: None,
        });
    }

    // Parse Sample Data
    // S3M samples are 8-bit unsigned by default.
    // Flags bit 1 (2) = Stereo (unsupported in old S3M usually, but checked)
    // Flags bit 2 (4) = 16-bit
    let is_16bit = flags & 4 != 0;

    // S3M 16-bit data is Little Endian
    let raw_slice = &data[sample_ptr as usize..(sample_ptr + length) as usize];
    let float_data = if is_16bit {
        // 16-bit unsigned (rare in standard S3M but supported by format)
        let num_samples = raw_slice.len() / 2;
        let mut fd = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            if i * 2 + 1 >= raw_slice.len() {
                break;
            }
            let s = u16::from_le_bytes([raw_slice[i * 2], raw_slice[i * 2 + 1]]);
            // Unsigned 16-bit to float: (s - 32768) / 32768.0
            fd.push((s as i32 - 32768) as f32 / 32768.0);
        }
        fd
    } else {
        // 8-bit unsigned
        raw_slice
            .iter()
            .map(|&b| (b as i16 - 128) as f32 / 128.0)
            .collect()
    };

    Ok(S3mInstrument {
        name,
        sample_data: Some(float_data),
        sample_len: length,
        loop_begin,
        loop_end,
        volume,
        c2spd,
        flags,
        adlib_data: None,
    })
}

// ─── Patterns ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct S3mCell {
    note: u8, // 0=Empty, 254=Cut, 255=Off (S3M uses 255 for empty, but we map)
    // S3M: note 0..=119. 254 = key off?
    instrument: u8, // 0=Empty
    volume: u8,     // 0..64, 255=Empty
    command: u8,    // 0=Empty
    info: u8,
}

struct S3mPattern {
    rows: Vec<Vec<S3mCell>>, // 32 channels max
}

fn parse_s3m_pattern(data: &[u8], para_ptr: u16) -> Result<S3mPattern, String> {
    if para_ptr == 0 {
        // Empty pattern
        return Ok(S3mPattern {
            rows: vec![
                vec![
                    S3mCell {
                        note: 255,
                        instrument: 0,
                        volume: 255,
                        command: 0,
                        info: 0
                    };
                    32
                ];
                64
            ],
        });
    }

    let offset = (para_ptr as usize) * 16;
    if offset + 2 > data.len() {
        return Err("Pattern header out of bounds".into());
    }

    let length = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
    if offset + 2 + length > data.len() {
        return Err("Pattern data out of bounds".into());
    }

    let pat_data = &data[offset + 2..offset + 2 + length];
    let mut off = 0;

    let mut rows = Vec::with_capacity(64);
    let mut current_row = Vec::with_capacity(32);
    // Initialize current row with empty cells for 32 channels
    for _ in 0..32 {
        current_row.push(S3mCell {
            note: 255,
            instrument: 0,
            volume: 255,
            command: 0,
            info: 0,
        });
    }

    // S3M patterns are packed. Loop until 64 rows are filled.
    // 32 channels per row.
    let mut row_idx = 0;

    while off < pat_data.len() && row_idx < 64 {
        let b = pat_data[off];
        off += 1;

        if b == 0 {
            // End of row
            rows.push(current_row);
            current_row = Vec::with_capacity(32);
            for _ in 0..32 {
                current_row.push(S3mCell {
                    note: 255,
                    instrument: 0,
                    volume: 255,
                    command: 0,
                    info: 0,
                });
            }
            row_idx += 1;
            continue;
        }

        let channel = (b & 31) as usize;
        let mut note = 255;
        let mut instrument = 0;
        let mut volume = 255;
        let mut command = 0;
        let mut info = 0;

        if b & 32 != 0 {
            // Note + Instrument
            if off + 2 > pat_data.len() {
                break;
            }
            note = pat_data[off];
            instrument = pat_data[off + 1];
            off += 2;
        }
        if b & 64 != 0 {
            // Volume
            if off + 1 > pat_data.len() {
                break;
            }
            volume = pat_data[off];
            off += 1;
        }
        if b & 128 != 0 {
            // Command + Info
            if off + 2 > pat_data.len() {
                break;
            }
            command = pat_data[off];
            info = pat_data[off + 1];
            off += 2;
        }

        if channel < 32 {
            current_row[channel] = S3mCell {
                note,
                instrument,
                volume,
                command,
                info,
            };
        }
    }

    // Fill remaining rows if file ended early (shouldn't happen in valid files)
    while rows.len() < 64 {
        let mut row = Vec::with_capacity(32);
        for _ in 0..32 {
            row.push(S3mCell {
                note: 255,
                instrument: 0,
                volume: 255,
                command: 0,
                info: 0,
            });
        }
        rows.push(row);
    }

    Ok(S3mPattern { rows })
}

// ─── Effect Conversion ───────────────────────────────────────────────────────

fn convert_s3m_effect(cmd: u8, info: u8) -> Option<Effect> {
    if cmd == 0 {
        return None;
    }

    // S3M commands are 1-based (A=1, B=2...)
    // Internal Effect uses specific bytes.
    let cmd_char = (cmd + 64) as char; // 1 -> 'A'

    // Note: S3M param is straight byte.
    let _x = (info >> 4) & 0x0F;
    let y = info & 0x0F;

    match cmd_char {
        'A' => Some(Effect::new(0x0F, info)), // Set Speed (if < 32 usually)
        'B' => Some(Effect::new(0x0B, info)), // Order Jump
        'C' => Some(Effect::new(0x0D, info)), // Pattern Break
        'D' => Some(Effect::new(0x0A, info)), // Volume Slide
        'E' => Some(Effect::new(0x02, info)), // Portamento Down
        'F' => Some(Effect::new(0x01, info)), // Portamento Up
        'G' => Some(Effect::new(0x03, info)), // Tone Portamento
        'H' => Some(Effect::new(0x04, info)), // Vibrato
        'I' => Some(Effect::new(0x15, info)), // Tremor
        'J' => Some(Effect::new(0x00, info)), // Arpeggio
        'K' => Some(Effect::new(0x06, info)), // Vibrato + Vol Slide
        'L' => Some(Effect::new(0x05, info)), // Porta + Vol Slide
        'O' => Some(Effect::new(0x09, info)), // Sample Offset
        'Q' => Some(Effect::new(0x16, info)), // Retrig + Vol Slide
        'R' => Some(Effect::new(0x07, info)), // Tremolo
        'S' => {
            // Special commands (Sxy where x is subcmd)
            // S3M uses High nibble for subcmd
            match (info >> 4) & 0x0F {
                0x0 => None,                              // Set Filter (ignored)
                0x1 => Some(Effect::new(0x0E, 0x30 | y)), // Glissando
                0x2 => Some(Effect::new(0x0E, 0x50 | y)), // Finetune
                0x3 => Some(Effect::new(0x0E, 0x40 | y)), // Vibrato Waveform
                0x4 => Some(Effect::new(0x0E, 0x70 | y)), // Tremolo Waveform
                0x8 => Some(Effect::new(0x08, y * 17)),   // Pan position (0-15 -> 0-255)
                0xB => Some(Effect::new(0x0E, 0x60 | y)), // Pattern Loop
                0xC => Some(Effect::new(0x0E, 0xC0 | y)), // Note Cut
                0xD => Some(Effect::new(0x0E, 0xD0 | y)), // Note Delay
                0xE => Some(Effect::new(0x0E, 0xE0 | y)), // Pattern Delay
                _ => None,
            }
        }
        'T' => Some(Effect::new(0x0F, info)), // Tempo/BPM
        'U' => Some(Effect::new(0x04, info)), // Fine Vibrato (map to Vibrato for now)
        'V' => Some(Effect::new(0x10, info)), // Global Volume
        _ => None,
    }
}

// ─── Main Import ─────────────────────────────────────────────────────────────

pub fn import_s3m(data: &[u8]) -> Result<FormatData, String> {
    let header = parse_s3m_header(data)?;

    // Parse Instruments
    let mut instruments = Vec::new();
    let mut samples = Vec::new();
    let mut inst_map = vec![None; header.ins_num as usize]; // Maps S3M inst index to internal index

    for (i, &ptr) in header.inst_pointers.iter().enumerate() {
        let s3m_inst = parse_s3m_instrument(data, ptr)?;

        if let Some(float_data) = s3m_inst.sample_data {
            let mut sample = Sample::new(
                float_data,
                s3m_inst.c2spd,
                1, // Mono
                Some(s3m_inst.name.clone()),
            );
            sample.volume = s3m_inst.volume as f32 / 64.0;

            // Loop
            if s3m_inst.flags & 1 != 0 && s3m_inst.loop_end > s3m_inst.loop_begin {
                sample = sample.with_loop(
                    LoopMode::Forward,
                    s3m_inst.loop_begin as usize,
                    s3m_inst.loop_end as usize,
                );
            }

            // S3M samples don't have relative pitch bytes like XM.
            // C2SPD defines the frequency at Middle C (C-4).
            // Our engine uses base_note to calculate frequency.
            // Frequency = C4_Freq * 2^((Note - BaseNote)/12)
            // We want Frequency(C4) = C2SPD.
            // 8363 * 2^((60 - Base)/12) = C2SPD
            // log2(C2SPD/8363) = (60 - Base)/12
            // Base = 60 - 12 * log2(C2SPD/8363)
            let base_note = 60.0 - 12.0 * (s3m_inst.c2spd as f64 / 8363.0).log2();
            sample = sample.with_base_note(base_note.round() as u8);

            let mut inst = Instrument::new(s3m_inst.name.clone());
            inst.sample_index = Some(samples.len());
            inst.volume = 1.0; // S3M applies volume at sample level mostly, but we can put it here too?
                               // Actually S3M default volume is per-sample.
                               // We applied it to the sample.volume already.

            inst_map[i] = Some(instruments.len());
            samples.push(sample);
            instruments.push(inst);
        } else {
            // Empty / Adlib instrument
            // Just add a dummy instrument? Or skip?
            // To maintain indices, we might want to map to a dummy, or just None.
            // S3M patterns reference instrument by index (1-based).
            // If we skip, we break references.
            let name = s3m_inst.name.clone();
            let mut inst = Instrument::new(name.clone());
            let sample = if let Some(adlib) = s3m_inst.adlib_data {
                #[cfg(feature = "adlib")]
                {
                    const SAMPLE_RATE: u32 = 48000;
                    const DURATION_SECS: f32 = 0.5;
                    let num_samples = (SAMPLE_RATE as f32 * DURATION_SECS) as usize;

                    let adlib_result =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let mut synth = AdlibSynthesizer::new(SAMPLE_RATE);
                            synth.init(&adlib.registers);
                            synth.note_on(0, 60, 64);
                            let data = synth.render_samples(num_samples).to_vec();
                            synth.note_off(0);
                            data
                        }));

                    match adlib_result {
                        Ok(data) => {
                            let mut sample = Sample::new(data, SAMPLE_RATE, 1, Some(name));
                            sample.volume = s3m_inst.volume as f32 / 64.0;
                            let base_note = 60.0 - 12.0 * (s3m_inst.c2spd as f64 / 8363.0).log2();
                            sample = sample.with_base_note(base_note.round() as u8);
                            sample
                        }
                        Err(_) => {
                            eprintln!("S3M loader: adlib instrument '{}' failed to render (unsupported OPL feature), using fallback", name);
                            let freq = 440.0;
                            let mut data = Vec::with_capacity(num_samples);
                            for i in 0..num_samples {
                                let t = i as f32 / SAMPLE_RATE as f32;
                                let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
                                data.push(value);
                            }
                            let mut sample = Sample::new(data, SAMPLE_RATE, 1, Some(name));
                            sample.volume = s3m_inst.volume as f32 / 64.0;
                            sample = sample.with_base_note(69);
                            sample
                        }
                    }
                }
                #[cfg(not(feature = "adlib"))]
                {
                    // Adlib synthesis disabled, treat as empty instrument
                    Sample::default()
                }
            } else {
                // Empty instrument - keep dummy sample (will be replaced by sine wave injection)
                Sample::default()
            };
            inst.sample_index = Some(samples.len());
            inst.volume = 1.0;
            inst_map[i] = Some(instruments.len());
            samples.push(sample);
            instruments.push(inst);
        }
    }

    // Ensure all samples have audio data (some S3M files may have missing samples)
    for sample in &mut samples {
        if sample.data().is_empty() {
            eprintln!("S3M loader: injecting sine wave for empty sample");
            let sample_rate = 48000;
            let duration_secs = 0.5;
            let num_samples = (sample_rate as f32 * duration_secs) as usize;
            let mut data = Vec::with_capacity(num_samples);
            let freq = 440.0; // A4
            for i in 0..num_samples {
                let t = i as f32 / sample_rate as f32;
                let value = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.5;
                data.push(value);
            }
            let mut new_sample = Sample::new(data, sample_rate, 1, None);
            new_sample.volume = 0.5;
            new_sample = new_sample.with_base_note(69); // A4
            *sample = new_sample;
        }
    }

    // Convert Patterns
    let mut patterns = Vec::new();
    for &ptr in &header.pat_pointers {
        let s3m_pat = parse_s3m_pattern(data, ptr)?;
        let mut pattern = Pattern::new(64, 32); // S3M patterns are always 64 rows? Yes usually.

        for (r, row) in s3m_pat.rows.iter().enumerate() {
            if r >= 64 {
                break;
            }
            for (c, cell) in row.iter().enumerate() {
                if c >= 32 {
                    break;
                }

                let mut p_cell = Cell::empty();

                // Note
                if cell.note < 254 {
                    // S3M notes: 0x00=C-0 ... 0xFE=Cut?
                    // High nibble octave, low nibble note?
                    // S3M: High nibble = octave (0-7+), Low nibble = note (0-11).
                    // Actually: value = octave * 16 + note.
                    // 255 = Empty. 254 = Key Off (^).
                    // C-4 is roughly middle.
                    let octave = cell.note >> 4;
                    let note_idx = cell.note & 0x0F;
                    if note_idx < 12 {
                        if let Some(pitch) = Pitch::from_semitone(note_idx) {
                            p_cell.note = Some(NoteEvent::On(Note::new(
                                pitch, octave,
                                64, // S3M doesn't have note velocity, default to mid/max? 64 is max vol in S3M.
                                0,  // Instrument set below
                            )));
                        }
                    }
                } else if cell.note == 254 {
                    p_cell.note = Some(NoteEvent::Cut); // or Off?
                }

                // Instrument
                if cell.instrument > 0 {
                    let s3m_idx = (cell.instrument - 1) as usize;
                    if s3m_idx < inst_map.len() {
                        if let Some(mapped_idx) = inst_map[s3m_idx] {
                            p_cell.instrument = Some(mapped_idx as u8);
                            if let Some(NoteEvent::On(ref mut n)) = p_cell.note {
                                n.instrument = mapped_idx as u8;
                            }
                        }
                    }
                }

                // Volume (0..64)
                if cell.volume <= 64 {
                    p_cell.volume = Some(cell.volume);
                }

                // Effect
                if let Some(eff) = convert_s3m_effect(cell.command, cell.info) {
                    p_cell.effects.push(eff);
                }

                pattern.set_cell(r, c, p_cell);
            }
        }
        patterns.push(pattern);
    }

    // Build Arrangement
    let mut arrangement = Vec::new();
    for &ord in &header.orders {
        if ord == 255 {
            break;
        } // End of song
        if ord < patterns.len() as u8 {
            arrangement.push(ord as usize);
        } else {
            // Empty pattern / +++
            // We should probably push a dummy pattern index or handle +++ (skip).
            // 254 = +++ (Marker/Skip).
            // Standard S3M behavior: 254 is skipped in playback order, but
            // some trackers might treat it differently. We'll skip it in the list.
        }
    }
    if arrangement.is_empty() {
        arrangement.push(0);
    }

    // Build Tracks (Channel settings)
    let mut tracks = Vec::new();
    let _num_channels = 0;

    for i in 0..32 {
        let setting = header.channel_settings[i];
        let _enabled = setting < 16;

        let mut track = Track::with_number(i + 1);
        if setting == 255 {
            track.muted = true; // Effectively unused
        } else {
            // Default pan: S3M uses 0-7 for Left, 8-15 for Right
            if i < 16 {
                if setting < 8 {
                    track.pan = -0.5;
                } else {
                    track.pan = 0.5;
                }
            } else {
                track.pan = 0.0;
            }
        }
        tracks.push(track);
    }

    let mut song = Song::new(header.name, header.initial_tempo as f64);
    song.tpl = header.initial_speed as u32;
    song.global_volume = header.global_vol as f32 / 64.0;
    song.instruments = instruments;
    song.patterns = patterns;
    song.arrangement = arrangement;
    song.tracks = tracks;

    song.effect_mode = EffectMode::Compatible;

    Ok(FormatData { song, samples })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_s3m() {
        let mut data = vec![0u8; 0x60]; // Header is 96 bytes

        // Magic "SCRM" at 0x2C
        data[0x2C] = b'S';
        data[0x2C + 1] = b'C';
        data[0x2C + 2] = b'R';
        data[0x2C + 3] = b'M';

        // OrdNum = 1 at 0x20
        data[0x20] = 1;
        // InsNum = 0 at 0x22
        data[0x22] = 0;
        // PatNum = 0 at 0x24
        data[0x24] = 0;

        // Channel settings at 0x40 (all 255 = unused)
        for i in 0..32 {
            data[0x40 + i] = 255;
        }

        // Orders at 0x60 (len = OrdNum = 1)
        data.push(255); // End of song marker

        // Parapointers (InsNum=0, PatNum=0) -> 0 bytes added

        let result = import_s3m(&data);
        assert!(
            result.is_ok(),
            "Failed to parse minimal S3M: {:?}",
            result.err()
        );

        let format_data = result.unwrap();
        assert_eq!(format_data.song.tracks.len(), 32);
        assert_eq!(format_data.song.arrangement.len(), 1);
    }
}
