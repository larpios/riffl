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

pub fn convert_xmrs_module(mut module: xmrs::module::Module) -> Result<FormatData, String> {
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
                    if xm_samp.data.is_some() {
                        let float_data = match xm_samp.data.as_ref().unwrap() {
                            SampleDataType::Mono8(v) => v.iter().map(|&s| s as f32 / 128.0).collect(),
                            SampleDataType::Mono16(v) => v.iter().map(|&s| s as f32 / 32768.0).collect(),
                            SampleDataType::Stereo8(v) => v.iter().map(|&s| s as f32 / 128.0).collect(),
                            SampleDataType::Stereo16(v) => v.iter().map(|&s| s as f32 / 32768.0).collect(),
                            SampleDataType::StereoFloat(v) => v.clone(),
                        };

                        let channels = match xm_samp.data.as_ref().unwrap() {
                            SampleDataType::Mono8(_) | SampleDataType::Mono16(_) => 1,
                            _ => 2,
                        };

                        let mut sample = Sample::new(float_data, 8363, channels, Some(xm_samp.name.clone()));
                        sample.volume = xm_samp.volume;
                        sample.finetune = (xm_samp.finetune * 100.0) as i32;

                        match xm_samp.loop_flag {
                            LoopType::No => {}
                            LoopType::Forward => {
                                sample = sample.with_loop(
                                    LoopMode::Forward,
                                    xm_samp.loop_start as usize,
                                    xm_samp.loop_start as usize
                                        + xm_samp.loop_length.saturating_sub(1) as usize,
                                );
                            }
                            LoopType::PingPong => {
                                sample = sample.with_loop(
                                    LoopMode::PingPong,
                                    xm_samp.loop_start as usize,
                                    xm_samp.loop_start as usize
                                        + xm_samp.loop_length.saturating_sub(1) as usize,
                                );
                            }
                        }

                        let base_note_midi = (48_i32 - xm_samp.relative_pitch as i32).clamp(0, 119) as u8;
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

    for xm_pat in &module.pattern {
        let mut pat = crate::pattern::Pattern::new(xm_pat.len().max(1), num_channels);

        for (r_idx, xm_row) in xm_pat.iter().enumerate() {
            for (c_idx, xm_tu) in xm_row.iter().enumerate() {
                if c_idx >= num_channels {
                    continue;
                }

                let mut cell = Cell::empty();

                if xm_tu.note.is_keyoff() {
                    cell.note = Some(NoteEvent::Off);
                } else if xm_tu.note == xmrs::pitch::Pitch::Cut {
                    cell.note = Some(NoteEvent::Cut);
                } else if xm_tu.note.is_valid() && !xm_tu.note.is_none() {
                    let val = xm_tu.note.value();
                    let octave = val / 12;
                    if let Some(pitch) = crate::pattern::note::Pitch::from_semitone(val % 12) {
                        let vel = (xm_tu.velocity * 127.0).clamp(0.0, 127.0) as u8;
                        cell.note = Some(NoteEvent::On(Note::new(pitch, octave, vel, 0)));
                    }
                }

                if let Some(inst_idx_1based) = xm_tu.instrument {
                    let i = inst_idx_1based.saturating_sub(1) as usize;
                    if i < module.instrument.len() {
                        if let InstrumentType::Default(def) = &module.instrument[i].instr_type {
                            let mut sample_idx = 0;
                            if let Some(NoteEvent::On(n)) = &cell.note {
                                let midi_pitch = n.octave as usize * 12 + n.pitch.semitone() as usize;
                                if midi_pitch < 120 {
                                    if let Some(s) = def.sample_for_pitch[midi_pitch] {
                                        sample_idx = s;
                                    }
                                }
                            }
                            if i < inst_to_tracker_inst.len() && sample_idx < inst_to_tracker_inst[i].len() {
                                if let Some(mapped_idx) = inst_to_tracker_inst[i][sample_idx] {
                                    cell.instrument = Some(mapped_idx as u8);
                                }
                            }
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
