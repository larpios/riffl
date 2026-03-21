use crate::audio::sample::{LoopMode, Sample};
use crate::pattern::{Cell, Note, NoteEvent};
use crate::song::{Instrument, Song};

/// Result of a successful format import.
pub struct FormatData {
    /// Song structure: patterns, arrangement, instrument definitions.
    pub song: Song,
    /// Raw audio data for each instrument slot.
    pub samples: Vec<Sample>,
}

pub mod it;
pub mod protracker;
pub mod s3m;
pub mod xm;

/// Encode an xmrs Waveform variant into the 2-bit tracker waveform index.
/// Bit 2 (value 4) signals "no retrigger on new note" when set.
fn encode_waveform(waveform: &xmrs::waveform::Waveform, retrig: bool) -> u8 {
    use xmrs::waveform::Waveform;
    let w = match waveform {
        Waveform::Sine => 0,
        Waveform::RampDown => 1,
        Waveform::Square => 2,
        _ => 0,
    };
    // retrig=true means reset on new note (bit 2 = 0); retrig=false means don't reset (bit 2 = 1)
    w | if retrig { 0 } else { 4 }
}

pub fn convert_xmrs_module(module: xmrs::module::Module) -> Result<FormatData, String> {
    use crate::song::{Envelope, EnvelopePoint};
    use xmrs::instrument::InstrumentType;
    use xmrs::sample::{LoopType, SampleDataType};

    let mut song = Song::new(module.name.clone(), module.default_bpm as f64);
    song.tpl = module.default_tempo as u32;

    let mut out_samples = vec![];
    let mut out_instruments = vec![];
    let mut inst_to_tracker_inst: Vec<Vec<Option<usize>>> = vec![];

    for xm_inst in module.instrument.iter() {
        let mut sample_map = vec![];

        if let InstrumentType::Default(def) = &xm_inst.instr_type {
            sample_map.resize(def.sample.len(), None);
            for (s_idx, s_opt) in def.sample.iter().enumerate() {
                if let Some(xm_samp) = s_opt {
                    if let Some(ref data) = xm_samp.data {
                        let float_data = match data {
                            SampleDataType::Mono8(v) => {
                                v.iter().map(|&s| s as f32 / 128.0).collect()
                            }
                            SampleDataType::Mono16(v) => {
                                v.iter().map(|&s| s as f32 / 32768.0).collect()
                            }
                            SampleDataType::Stereo8(v) => {
                                v.iter().map(|&s| s as f32 / 128.0).collect()
                            }
                            SampleDataType::Stereo16(v) => {
                                v.iter().map(|&s| s as f32 / 32768.0).collect()
                            }
                            SampleDataType::StereoFloat(v) => v.clone(),
                        };

                        let channels = match data {
                            SampleDataType::Mono8(_) | SampleDataType::Mono16(_) => 1,
                            _ => 2,
                        };

                        let mut sample =
                            Sample::new(float_data, 8363, channels, Some(xm_samp.name.clone()));
                        sample.volume = xm_samp.volume;
                        sample.finetune = (xm_samp.finetune * 100.0) as i32;

                        match xm_samp.loop_flag {
                            LoopType::No => {}
                            LoopType::Forward => {
                                let loop_start = xm_samp.loop_start as usize;
                                let loop_end =
                                    loop_start + xm_samp.loop_length.saturating_sub(1) as usize;
                                if loop_end > loop_start {
                                    sample =
                                        sample.with_loop(LoopMode::Forward, loop_start, loop_end);
                                }
                            }
                            LoopType::PingPong => {
                                let loop_start = xm_samp.loop_start as usize;
                                let loop_end =
                                    loop_start + xm_samp.loop_length.saturating_sub(1) as usize;
                                if loop_end > loop_start {
                                    sample =
                                        sample.with_loop(LoopMode::PingPong, loop_start, loop_end);
                                }
                            }
                        }

                        let base_note_midi =
                            (48_i32 - xm_samp.relative_pitch as i32).clamp(0, 119) as u8;
                        sample = sample.with_base_note(base_note_midi);

                        let inst_name = if xm_samp.name.is_empty() {
                            xm_inst.name.clone()
                        } else if xm_inst.name.is_empty() {
                            xm_samp.name.clone()
                        } else {
                            format!("{} - {}", xm_inst.name, xm_samp.name)
                        };

                        let mut inst = Instrument::new(inst_name);
                        inst.sample_index = Some(out_samples.len());
                        inst.volume = xm_samp.volume;

                        let convert_env = |env: &xmrs::envelope::Envelope| -> Envelope {
                            Envelope {
                                enabled: env.enabled,
                                points: env
                                    .point
                                    .iter()
                                    .map(|p| EnvelopePoint {
                                        frame: p.frame as u16,
                                        value: p.value,
                                    })
                                    .collect(),
                                sustain_enabled: env.sustain_enabled,
                                sustain_start_point: env.sustain_start_point,
                                sustain_end_point: env.sustain_end_point,
                                loop_enabled: env.loop_enabled,
                                loop_start_point: env.loop_start_point,
                                loop_end_point: env.loop_end_point,
                            }
                        };

                        if let InstrumentType::Default(def) = &xm_inst.instr_type {
                            if def.volume_envelope.enabled {
                                inst.volume_envelope = Some(convert_env(&def.volume_envelope));
                            }
                            if def.pan_envelope.enabled {
                                inst.panning_envelope = Some(convert_env(&def.pan_envelope));
                            }
                        }

                        sample_map[s_idx] = Some(out_instruments.len());
                        out_samples.push(sample);
                        out_instruments.push(inst);
                    }
                }
            }
        }
        inst_to_tracker_inst.push(sample_map);
    }
    song.instruments = out_instruments;

    song.patterns.clear();
    let num_channels = module.get_num_channels().max(1);
    let mut last_instrument: Vec<Option<usize>> = vec![None; num_channels];
    let mut last_sample: Vec<Option<u8>> = vec![None; num_channels];

    for xm_pat in &module.pattern {
        let mut pat = crate::pattern::Pattern::new(xm_pat.len().max(1), num_channels);

        for (r_idx, xm_row) in xm_pat.iter().enumerate() {
            for (c_idx, xm_tu) in xm_row.iter().enumerate() {
                if c_idx >= num_channels {
                    continue;
                }

                let mut cell = Cell::empty();

                let mut note_event = None;
                if xm_tu.note.is_keyoff() || xm_tu.note.value() == 97 {
                    note_event = Some(NoteEvent::Off);
                } else if xm_tu.note == xmrs::pitch::Pitch::Cut {
                    note_event = Some(NoteEvent::Cut);
                } else if xm_tu.note.is_valid() && !xm_tu.note.is_none() {
                    let val = xm_tu.note.value();
                    let octave = val / 12;
                    if let Some(pitch) = crate::pattern::note::Pitch::from_semitone(val % 12) {
                        // XM format relies on instrument volume and volume columns, not per-note velocity.
                        // Force max velocity here since `xmrs` may provide 0.0 if instrument is omitted.
                        let vel = 127;
                        note_event = Some(NoteEvent::On(Note::new(pitch, octave, vel, 0)));
                    }
                }

                let mut explicit_inst = false;
                let resolved_inst = if let Some(inst_idx_1based) = xm_tu.instrument {
                    explicit_inst = true;
                    let i = inst_idx_1based.saturating_sub(1);
                    last_instrument[c_idx] = Some(i);
                    Some(i)
                } else if note_event.is_some() {
                    last_instrument[c_idx]
                } else {
                    None
                };

                let mut mapped_sample_idx = None;

                if let Some(i) = resolved_inst {
                    if i < module.instrument.len() {
                        if let InstrumentType::Default(def) = &module.instrument[i].instr_type {
                            let mut sample_idx = 0;
                            if let Some(NoteEvent::On(n)) = &note_event {
                                let midi_pitch =
                                    n.octave as usize * 12 + n.pitch.semitone() as usize;
                                if midi_pitch < 120 {
                                    if let Some(s) = def.sample_for_pitch[midi_pitch] {
                                        sample_idx = s;
                                    }
                                }
                            }
                            if i < inst_to_tracker_inst.len()
                                && sample_idx < inst_to_tracker_inst[i].len()
                            {
                                if let Some(mapped_idx) = inst_to_tracker_inst[i][sample_idx] {
                                    mapped_sample_idx = Some(mapped_idx as u8);
                                    last_sample[c_idx] = mapped_sample_idx;
                                    if explicit_inst {
                                        cell.instrument = mapped_sample_idx;
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(NoteEvent::On(mut n)) = note_event {
                    if let Some(idx) = mapped_sample_idx {
                        n.instrument = idx;
                        cell.note = Some(NoteEvent::On(n));
                    } else if let Some(fallback_idx) = last_sample[c_idx] {
                        // Ghost note: no instrument specified and sample lookup failed,
                        // but we have a previously-used sample on this channel — re-use it.
                        n.instrument = fallback_idx;
                        cell.note = Some(NoteEvent::On(n));
                    } else {
                        // No sample ever played on this channel — drop the note
                        cell.note = None;
                    }
                } else {
                    cell.note = note_event;
                }

                // Convert xmrs TrackEffect variants to our internal Effect byte encoding.
                // xmrs normalises raw bytes to floats; we reverse the normalisation here.
                for ef in &xm_tu.effects {
                    use crate::pattern::effect::Effect;
                    use xmrs::effect::TrackEffect;
                    let maybe_effect: Option<Effect> = match ef {
                        TrackEffect::Arpeggio { half1, half2 } => {
                            let p = ((*half1 as u8 & 0xF) << 4) | (*half2 as u8 & 0xF);
                            if p != 0 {
                                Some(Effect::new(0x0, p))
                            } else {
                                None
                            }
                        }
                        // Portamento: negative speed = pitch UP, positive = pitch DOWN
                        TrackEffect::Portamento(speed) => {
                            if *speed < 0.0 {
                                Some(Effect::new(0x1, ((-speed) / 4.0).clamp(0.0, 255.0) as u8))
                            } else if *speed > 0.0 {
                                Some(Effect::new(0x2, (speed / 4.0).clamp(0.0, 255.0) as u8))
                            } else {
                                None
                            }
                        }
                        TrackEffect::TonePortamento(speed) => {
                            let p = (speed / 4.0).clamp(0.0, 255.0) as u8;
                            if p > 0 {
                                Some(Effect::new(0x3, p))
                            } else {
                                None
                            }
                        }
                        TrackEffect::Vibrato { speed, depth } => {
                            let s = (speed * 64.0).clamp(0.0, 15.0) as u8;
                            let d = (depth * 16.0).clamp(0.0, 15.0) as u8;
                            if s > 0 || d > 0 {
                                Some(Effect::new(0x4, (s << 4) | d))
                            } else {
                                None
                            }
                        }
                        TrackEffect::Tremolo { speed, depth } => {
                            let s = (speed * 64.0).clamp(0.0, 15.0) as u8;
                            let d = (depth * 16.0).clamp(0.0, 15.0) as u8;
                            Some(Effect::new(0x7, (s << 4) | d))
                        }
                        TrackEffect::Panning(pos) => {
                            Some(Effect::new(0x8, (pos * 255.0).clamp(0.0, 255.0) as u8))
                        }
                        TrackEffect::InstrumentSampleOffset(offset) => {
                            Some(Effect::new(0x9, (offset / 256).min(255) as u8))
                        }
                        TrackEffect::PanningSlide { speed, fine } => {
                            if *speed > 0.0 {
                                let nibble = (speed * 15.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x12, 0xA0 | nibble)) // Or handle fine differently
                                } else {
                                    Some(Effect::new(0x12, nibble << 4))
                                }
                            } else if *speed < 0.0 {
                                let nibble = ((-speed) * 15.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x12, 0xB0 | nibble))
                                } else {
                                    Some(Effect::new(0x12, nibble))
                                }
                            } else {
                                None
                            }
                        }
                        // VolumeSlide: signed speed (pos=up, neg=down); fine flag → EAx/EBx
                        TrackEffect::VolumeSlide { speed, fine } => {
                            if *speed > 0.0 {
                                let nibble = (speed * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0xE, 0xA0 | nibble))
                                } else {
                                    Some(Effect::new(0xA, nibble << 4))
                                }
                            } else if *speed < 0.0 {
                                let nibble = ((-speed) * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0xE, 0xB0 | nibble))
                                } else {
                                    Some(Effect::new(0xA, nibble))
                                }
                            } else {
                                None
                            }
                        }
                        // Volume set at tick 0 → Cxx
                        TrackEffect::Volume { value, tick } if *tick == 0 => {
                            Some(Effect::new(0xC, (value * 64.0).clamp(0.0, 64.0) as u8))
                        }
                        TrackEffect::ChannelVolume(value) => {
                            Some(Effect::new(0x13, (value * 64.0).clamp(0.0, 64.0) as u8))
                        }
                        TrackEffect::ChannelVolumeSlide { speed, fine } => {
                            if *speed > 0.0 {
                                let nibble = (speed * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x14, 0xA0 | nibble))
                                } else {
                                    Some(Effect::new(0x14, nibble << 4))
                                }
                            } else if *speed < 0.0 {
                                let nibble = ((-speed) * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x14, 0xB0 | nibble))
                                } else {
                                    Some(Effect::new(0x14, nibble))
                                }
                            } else {
                                None
                            }
                        }
                        TrackEffect::Glissando(on) => Some(Effect::new(0xE, 0x30 | (*on as u8))),
                        TrackEffect::VibratoWaveform { waveform, retrig } => {
                            Some(Effect::new(0xE, 0x40 | encode_waveform(waveform, *retrig)))
                        }
                        TrackEffect::InstrumentFineTune(ft) => {
                            let enc = ((ft + 1.0) * 8.0).clamp(0.0, 15.0) as u8;
                            Some(Effect::new(0xE, 0x50 | enc))
                        }
                        TrackEffect::Tremor { on_time, off_time } => {
                            let on = (*on_time as u8).min(15);
                            let off = (*off_time as u8).min(15);
                            Some(Effect::new(0x15, (on << 4) | off))
                        }
                        TrackEffect::TremoloWaveform { waveform, retrig } => {
                            Some(Effect::new(0xE, 0x70 | encode_waveform(waveform, *retrig)))
                        }
                        TrackEffect::NoteRetrig {
                            speed,
                            volume_modifier,
                        } => {
                            use xmrs::effect::NoteRetrigOperator;
                            let mut vol_nibble = 0;
                            match volume_modifier {
                                NoteRetrigOperator::Mul(f) if *f < 1.0 => {
                                    vol_nibble = 1; // Example: approximate decrement
                                }
                                NoteRetrigOperator::Mul(f) if *f > 1.0 => {
                                    vol_nibble = 9; // Example: approximate increment
                                }
                                NoteRetrigOperator::Sum(f) if *f < 0.0 => {
                                    vol_nibble = 6;
                                }
                                NoteRetrigOperator::Sum(f) if *f > 0.0 => {
                                    vol_nibble = 0xE;
                                }
                                _ => {}
                            }
                            Some(Effect::new(0x16, (vol_nibble << 4) | (*speed as u8 & 0xF)))
                        }
                        TrackEffect::InstrumentVolumeEnvelopePosition(tick) => {
                            Some(Effect::new(0x17, *tick as u8))
                        }
                        TrackEffect::Panbrello { speed, depth } => {
                            let s = (speed * 15.0).clamp(0.0, 15.0) as u8;
                            let d = (depth * 15.0).clamp(0.0, 15.0) as u8;
                            Some(Effect::new(0x18, (s << 4) | d))
                        }
                        TrackEffect::NoteCut { tick, .. } => {
                            Some(Effect::new(0xE, 0xC0 | (*tick as u8 & 0xF)))
                        }
                        TrackEffect::NoteDelay(tick) => {
                            Some(Effect::new(0xE, 0xD0 | (*tick as u8 & 0xF)))
                        }
                        _ => None,
                    };
                    if let Some(eff) = maybe_effect {
                        if cell.effects.len() < crate::pattern::effect::MAX_EFFECTS_PER_CELL {
                            cell.effects.push(eff);
                        }
                    }
                }

                // Convert xmrs GlobalEffect variants (speed, BPM, jumps, loops).
                for gef in &xm_tu.global_effects {
                    use crate::pattern::effect::Effect;
                    use xmrs::effect::GlobalEffect;
                    let maybe_effect: Option<Effect> = match gef {
                        GlobalEffect::Speed(speed) => {
                            // < 32 → ticks per line (SetSpeed handles this range)
                            Some(Effect::new(0xF, (*speed as u8).min(31)))
                        }
                        GlobalEffect::Bpm(bpm) => {
                            // >= 32 → BPM (SetSpeed handler checks param range)
                            Some(Effect::new(0xF, (*bpm as u8).max(32)))
                        }
                        GlobalEffect::PositionJump(pos) => Some(Effect::new(0xB, *pos as u8)),
                        GlobalEffect::PatternBreak(row) => Some(Effect::new(0xD, *row as u8)),
                        GlobalEffect::PatternLoop(count) => {
                            Some(Effect::new(0xE, 0x60 | (*count as u8 & 0xF)))
                        }
                        GlobalEffect::PatternDelay { quantity, .. } => {
                            Some(Effect::new(0xE, 0xE0 | (*quantity as u8 & 0xF)))
                        }
                        GlobalEffect::Volume(vol) => {
                            Some(Effect::new(0x10, (vol * 64.0).clamp(0.0, 64.0) as u8))
                        }
                        GlobalEffect::VolumeSlide { speed, fine } => {
                            if *speed > 0.0 {
                                let nibble = (speed * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x11, 0xA0 | nibble))
                                } else {
                                    Some(Effect::new(0x11, nibble << 4))
                                }
                            } else if *speed < 0.0 {
                                let nibble = ((-speed) * 64.0).clamp(0.0, 15.0) as u8;
                                if *fine {
                                    Some(Effect::new(0x11, 0xB0 | nibble))
                                } else {
                                    Some(Effect::new(0x11, nibble))
                                }
                            } else {
                                None
                            }
                        }
                        GlobalEffect::MidiMacro(_) => Some(Effect::new(0x19, 0x00)),
                        _ => None,
                    };
                    if let Some(eff) = maybe_effect {
                        if cell.effects.len() < crate::pattern::effect::MAX_EFFECTS_PER_CELL {
                            cell.effects.push(eff);
                        }
                    }
                }

                pat.set_cell(r_idx, c_idx, cell);
            }
        }
        song.add_pattern(pat);
    }

    song.arrangement.clear();
    if let Some(order) = module.pattern_order.first() {
        for &pat_idx in order {
            if pat_idx < song.patterns.len() {
                song.arrangement.push(pat_idx);
            }
        }
    }

    if song.arrangement.is_empty() {
        song.arrangement.push(0);
    }

    Ok(FormatData {
        song,
        samples: out_samples,
    })
}
