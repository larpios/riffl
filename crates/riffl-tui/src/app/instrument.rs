use super::App;
use riffl_core::pattern::note::{NoteEvent, Pitch};
use riffl_core::pattern::{Note, Pattern};
use riffl_core::song::{Instrument, Song};

impl App {
    pub fn instrument_selection(&self) -> Option<usize> {
        self.instrument_selection
    }

    /// Set the selected instrument index.
    pub fn set_instrument_selection(&mut self, index: Option<usize>) {
        self.instrument_selection = index;
    }

    /// Move instrument selection up.
    pub fn instrument_selection_up(&mut self) {
        let count = self.song.instruments.len();
        if count == 0 {
            self.instrument_selection = None;
            return;
        }
        match self.instrument_selection {
            None => self.instrument_selection = Some(count - 1),
            Some(0) => self.instrument_selection = Some(count - 1),
            Some(i) => self.instrument_selection = Some(i - 1),
        }
    }

    /// Move instrument selection down.
    pub fn instrument_selection_down(&mut self) {
        let count = self.song.instruments.len();
        if count == 0 {
            self.instrument_selection = None;
            return;
        }
        match self.instrument_selection {
            None => self.instrument_selection = Some(0),
            Some(i) if i >= count - 1 => self.instrument_selection = Some(0),
            Some(i) => self.instrument_selection = Some(i + 1),
        }
    }

    /// Add a new empty instrument.
    pub fn add_instrument(&mut self) {
        let idx = self.song.instruments.len();
        let name = format!("Inst{:02X}", idx);
        let inst = Instrument::new(&name);
        self.song.instruments.push(inst);
        self.sync_mixer_instruments();
        self.instrument_selection = Some(idx);
    }

    /// Delete the selected instrument.
    pub fn delete_instrument(&mut self) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments.remove(idx);
                self.sync_mixer_instruments();
                // Adjust selection
                if self.song.instruments.is_empty() {
                    self.instrument_selection = None;
                } else if idx >= self.song.instruments.len() {
                    self.instrument_selection = Some(self.song.instruments.len() - 1);
                }
                return true;
            }
        }
        false
    }

    /// Rename the selected instrument.
    pub fn rename_instrument(&mut self, new_name: String) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments[idx].name = new_name;
                return true;
            }
        }
        false
    }

    /// Update instrument properties (volume, base_note as MIDI value).
    pub fn update_instrument(&mut self, volume: f32, base_note_midi: u8) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments[idx].volume = volume;
                if let Some(pitch) = Pitch::from_semitone(base_note_midi % 12) {
                    let octave = base_note_midi / 12;
                    self.song.instruments[idx].base_note = Note::simple(pitch, octave);
                }
                self.sync_mixer_instruments();
                self.mark_dirty();
                return true;
            }
        }
        false
    }

    /// Set the name of the selected instrument.
    pub fn set_instrument_name(&mut self, name: String) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() && !name.is_empty() {
                self.song.instruments[idx].name = name;
                self.mark_dirty();
            }
        }
    }

    /// Set loop settings for the sample of the specified instrument.
    #[allow(dead_code)]
    pub fn set_sample_loop_settings(
        &mut self,
        _inst_idx: usize,
        sample_idx: usize,
        mode: riffl_core::audio::sample::LoopMode,
        loop_start: usize,
        loop_end: usize,
    ) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_sample_loop(sample_idx, mode, loop_start, loop_end);
        }
        self.mark_dirty();
    }

    /// Adjust volume of the selected instrument by `delta` percentage points (clamped 0..=100).
    pub fn adjust_instrument_volume(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current_pct = (self.song.instruments[idx].volume * 100.0).round() as i32;
                let new_pct = (current_pct + delta).clamp(0, 100);
                self.song.instruments[idx].volume = new_pct as f32 / 100.0;
                self.sync_mixer_instruments();
                self.mark_dirty();
            }
        }
    }

    /// Adjust the base note of the selected instrument by `semitones`.
    pub fn adjust_instrument_base_note(&mut self, semitones: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current_midi = self.song.instruments[idx].base_note.midi_note() as i32;
                let new_midi = (current_midi + semitones).clamp(0, 127) as u8;
                if let Some(pitch) = Pitch::from_semitone(new_midi % 12) {
                    let octave = new_midi / 12;
                    self.song.instruments[idx].base_note = Note::simple(pitch, octave);
                    self.sync_mixer_instruments();
                    self.mark_dirty();
                }
            }
        }
    }

    /// Adjust the finetune of the selected instrument by `delta` (clamped -8..=7).
    pub fn adjust_instrument_finetune(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let current = self.song.instruments[idx].finetune as i32;
                let new_val = (current + delta).clamp(-8, 7) as i8;
                self.song.instruments[idx].finetune = new_val;
                self.sync_mixer_instruments();
                self.mark_dirty();
            }
        }
    }

    /// Cycle the loop mode of the selected instrument's sample (Off -> Forward -> PingPong -> Off).
    pub fn cycle_instrument_loop_mode(&mut self) {
        if let Some(idx) = self.instrument_selection {
            if let Some(sample_idx) = self.song.instruments[idx].sample_index {
                let (current, frame_count) = {
                    let mixer = match self.mixer.lock() {
                        Ok(m) => m,
                        Err(_) => return,
                    };
                    if let Some(sample) = mixer.samples().get(sample_idx) {
                        (sample.loop_mode, sample.frame_count())
                    } else {
                        return;
                    }
                };
                let next = match current {
                    riffl_core::audio::sample::LoopMode::NoLoop => {
                        riffl_core::audio::sample::LoopMode::Forward
                    }
                    riffl_core::audio::sample::LoopMode::Forward => {
                        riffl_core::audio::sample::LoopMode::PingPong
                    }
                    riffl_core::audio::sample::LoopMode::PingPong => {
                        riffl_core::audio::sample::LoopMode::NoLoop
                    }
                };
                if let Ok(mut m) = self.mixer.lock() {
                    m.set_sample_loop(sample_idx, next, 0, frame_count.saturating_sub(1));
                }
                self.mark_dirty();
            }
        }
    }

    /// Adjust loop start position of the selected instrument's sample.
    pub fn adjust_instrument_loop_start(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(sample_idx) = self.song.instruments[idx].sample_index {
                let (loop_mode, loop_start, loop_end) = {
                    let mixer = match self.mixer.lock() {
                        Ok(m) => m,
                        Err(_) => return,
                    };
                    if let Some(sample) = mixer.samples().get(sample_idx) {
                        (sample.loop_mode, sample.loop_start, sample.loop_end)
                    } else {
                        return;
                    }
                };
                let new_val = (loop_start as i32 + delta).clamp(0, loop_end as i32);
                if let Ok(mut m) = self.mixer.lock() {
                    m.set_sample_loop(sample_idx, loop_mode, new_val as usize, loop_end);
                }
                self.mark_dirty();
            }
        }
    }

    /// Adjust loop end position of the selected instrument's sample.
    pub fn adjust_instrument_loop_end(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(sample_idx) = self.song.instruments[idx].sample_index {
                let (loop_mode, loop_start, loop_end, frame_count) = {
                    let mixer = match self.mixer.lock() {
                        Ok(m) => m,
                        Err(_) => return,
                    };
                    if let Some(sample) = mixer.samples().get(sample_idx) {
                        (
                            sample.loop_mode,
                            sample.loop_start,
                            sample.loop_end,
                            sample.frame_count() as i32,
                        )
                    } else {
                        return;
                    }
                };
                let new_val = (loop_end as i32 + delta)
                    .clamp(loop_start as i32, frame_count.saturating_sub(1));
                if let Ok(mut m) = self.mixer.lock() {
                    m.set_sample_loop(sample_idx, loop_mode, loop_start, new_val as usize);
                }
                self.mark_dirty();
            }
        }
    }

    /// Adjust the minimum note of the selected keyzone.
    pub fn adjust_keyzone_note_min(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].note_min as i32;
                        let new_val = (current + delta).clamp(0, 119) as u8;
                        inst.keyzones[kz_idx].note_min = new_val;
                        if inst.keyzones[kz_idx].note_max < new_val {
                            inst.keyzones[kz_idx].note_max = new_val;
                        }
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Adjust the maximum note of the selected keyzone.
    pub fn adjust_keyzone_note_max(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].note_max as i32;
                        let new_val = (current + delta).clamp(0, 119) as u8;
                        inst.keyzones[kz_idx].note_max = new_val;
                        if inst.keyzones[kz_idx].note_min > new_val {
                            inst.keyzones[kz_idx].note_min = new_val;
                        }
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Adjust the minimum velocity of the selected keyzone.
    pub fn adjust_keyzone_velocity_min(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].velocity_min as i32;
                        let new_val = (current + delta).clamp(0, 127) as u8;
                        inst.keyzones[kz_idx].velocity_min = new_val;
                        if inst.keyzones[kz_idx].velocity_max < new_val {
                            inst.keyzones[kz_idx].velocity_max = new_val;
                        }
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Adjust the maximum velocity of the selected keyzone.
    pub fn adjust_keyzone_velocity_max(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].velocity_max as i32;
                        let new_val = (current + delta).clamp(0, 127) as u8;
                        inst.keyzones[kz_idx].velocity_max = new_val;
                        if inst.keyzones[kz_idx].velocity_min > new_val {
                            inst.keyzones[kz_idx].velocity_min = new_val;
                        }
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Cycle the selected keyzone by delta (+1 next, -1 prev).
    pub fn adjust_keyzone_selection(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                let count = self.song.instruments[idx].keyzones.len();
                if count == 0 {
                    self.inst_editor.selected_keyzone = None;
                    return;
                }
                let current = self.inst_editor.selected_keyzone.unwrap_or(0) as i32;
                let new_idx = (current + delta).rem_euclid(count as i32) as usize;
                self.inst_editor.selected_keyzone = Some(new_idx);
            }
        }
    }

    /// Adjust the sample index of the selected keyzone.
    pub fn adjust_keyzone_sample(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let sample_count = self.mixer.lock().map(|m| m.sample_count()).unwrap_or(0);
                    let max_sample = sample_count.saturating_sub(1) as i32;
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].sample_index as i32;
                        let new_val = (current + delta).clamp(0, max_sample.max(0)) as usize;
                        inst.keyzones[kz_idx].sample_index = new_val;
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Adjust the base note override of the selected keyzone by delta semitones.
    pub fn adjust_keyzone_base_note(&mut self, delta: i32) {
        if let Some(idx) = self.instrument_selection {
            if let Some(kz_idx) = self.inst_editor.selected_keyzone_index() {
                if idx < self.song.instruments.len() {
                    let inst = &mut self.song.instruments[idx];
                    if kz_idx < inst.keyzones.len() {
                        let current = inst.keyzones[kz_idx].base_note_override.unwrap_or(48) as i32;
                        let new_val = (current + delta).clamp(0, 119) as u8;
                        inst.keyzones[kz_idx].base_note_override = Some(new_val);
                        self.mark_dirty();
                    }
                }
            }
        }
    }

    /// Select instrument for use in pattern editor.
    pub fn select_instrument(&mut self) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.editor.set_instrument(idx);
            }
        }
    }

    /// Get loaded instrument count.
    pub fn instrument_count(&self) -> usize {
        self.song.instruments.len()
    }
}

use riffl_core::audio::{load_sample, ChipRenderData, Sample};
use std::path::Path;
use std::sync::Arc;

impl App {
    /// Loads the audio from `path` into the mixer and updates the instrument's
    /// `sample_index` and `sample_path`. The instrument name is preserved.
    pub fn assign_sample_to_instrument(
        &mut self,
        path: &Path,
        inst_idx: usize,
    ) -> Result<(), String> {
        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;
        let chip_render = ChipRenderData::from_sample(&sample);

        let sample_idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        let inst = self
            .song
            .instruments
            .get_mut(inst_idx)
            .ok_or_else(|| format!("Instrument slot {inst_idx:02X} does not exist"))?;

        inst.sample_index = Some(sample_idx);
        inst.sample_path = Some(path.display().to_string());
        inst.chip_render = Some(chip_render);
        self.sync_mixer_instruments();
        self.mark_dirty();
        Ok(())
    }

    /// Replace an assigned sample and refresh the instrument's derived chip data.
    pub fn replace_instrument_sample(
        &mut self,
        inst_idx: usize,
        sample_idx: usize,
        sample: Sample,
    ) -> Result<(), String> {
        let chip_render = ChipRenderData::from_sample(&sample);

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.replace_sample(sample_idx, Arc::new(sample));
        } else {
            return Err("Failed to lock mixer".to_string());
        }

        let inst = self
            .song
            .instruments
            .get_mut(inst_idx)
            .ok_or_else(|| format!("Instrument slot {inst_idx:02X} does not exist"))?;
        inst.chip_render = Some(chip_render);

        self.sync_mixer_instruments();
        self.mark_dirty();
        Ok(())
    }

    /// Preview a note pitch through a specific instrument's sample.
    pub fn preview_instrument_note_pitch(&mut self, inst_idx: usize, pitch: Pitch, octave: u8) {
        let note = Note::simple(pitch, octave);
        let target_freq = note.frequency();

        // Resolve the instrument's sample_index; fall back to inst_idx for
        // instruments loaded from formats that map 1:1 (e.g. MOD/XM).
        let sample_idx = self
            .song
            .instruments
            .get(inst_idx)
            .and_then(|inst| inst.sample_index)
            .unwrap_or(inst_idx);

        let sample = {
            let mixer = match self.mixer.lock() {
                Ok(m) => m,
                Err(_) => return,
            };
            mixer.samples().get(sample_idx).cloned()
        };

        let sample = match sample {
            Some(s) => s,
            None => return,
        };

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let base_freq = sample.base_frequency();
        let rate =
            (target_freq / base_freq) * (sample.sample_rate() as f64 / output_sample_rate as f64);

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.trigger_preview(sample, rate);
        }

        if let Some(ref mut engine) = self.audio_engine {
            if !engine.is_playing() {
                let _ = engine.start();
            }
        }
    }
}
