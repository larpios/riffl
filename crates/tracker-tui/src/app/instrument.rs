use super::App;
use tracker_core::pattern::note::{NoteEvent, Pitch};
use tracker_core::pattern::{Note, Pattern};
use tracker_core::song::{Instrument, Song};

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
        self.instrument_names.push(name);
        self.instrument_selection = Some(idx);
    }

    /// Delete the selected instrument.
    pub fn delete_instrument(&mut self) -> bool {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.song.instruments.remove(idx);
                self.sync_mixer_instruments();
                self.instrument_names.remove(idx);
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
                self.song.instruments[idx].name = new_name.clone();
                self.instrument_names[idx] = new_name;
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
                    self.sync_mixer_instruments();
                    return true;
                }
            }
        }
        false
    }

    /// Set the name of the selected instrument.
    pub fn set_instrument_name(&mut self, name: String) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() && !name.is_empty() {
                self.song.instruments[idx].name = name.clone();
                if idx < self.instrument_names.len() {
                    self.instrument_names[idx] = name;
                }
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
        mode: tracker_core::audio::sample::LoopMode,
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
                    tracker_core::audio::sample::LoopMode::NoLoop => {
                        tracker_core::audio::sample::LoopMode::Forward
                    }
                    tracker_core::audio::sample::LoopMode::Forward => {
                        tracker_core::audio::sample::LoopMode::PingPong
                    }
                    tracker_core::audio::sample::LoopMode::PingPong => {
                        tracker_core::audio::sample::LoopMode::NoLoop
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
    /// Select instrument for use in pattern editor.
    pub fn select_instrument(&mut self) {
        if let Some(idx) = self.instrument_selection {
            if idx < self.song.instruments.len() {
                self.editor.set_instrument(idx);
            }
        }
    }

    /// Get the list of loaded instrument names.
    pub fn instrument_names(&self) -> &[String] {
        &self.instrument_names
    }

    /// Get loaded instrument count.
    pub fn instrument_count(&self) -> usize {
        self.instrument_names.len()
    }
}
