use riffl_core::audio::{load_sample, ChipRenderData};
use riffl_core::pattern::note::Pitch;
use std::path::Path;
use std::sync::Arc;

impl super::App {
    pub fn load_selected_sample(&mut self) -> Result<usize, String> {
        let path = self
            .file_browser
            .selected_path()
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        self.load_sample_from_path(&path)
    }

    /// Load the currently selected file from the dedicated sample browser view.
    /// Returns Ok(instrument_index) on success, or an error message.
    pub fn load_sample_from_browser(&mut self) -> Result<usize, String> {
        let path = self
            .sample_browser
            .selected_path()
            .filter(|_| self.sample_browser.selected_is_file())
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(&path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let name = sample.name().unwrap_or("unknown").to_string();
        let chip_render = ChipRenderData::from_sample(&sample);

        let idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        use riffl_core::song::Instrument;
        let mut instrument = Instrument::new(&name);
        instrument.sample_index = Some(idx);
        instrument.sample_path = Some(path.display().to_string());
        instrument.chip_render = Some(chip_render);
        self.song.instruments.push(instrument);
        self.sync_mixer_instruments();

        self.instrument_names.push(name);
        self.mark_dirty();
        Ok(idx)
    }

    /// Load a sample from an explicit path and add it as a new instrument.
    ///
    /// Used by the sample browser action menu ("Load as new instrument").
    pub fn load_sample_from_path(&mut self, path: &Path) -> Result<usize, String> {
        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let name = sample.name().unwrap_or("unknown").to_string();
        let chip_render = ChipRenderData::from_sample(&sample);

        let idx = if let Ok(mut mixer) = self.mixer.lock() {
            mixer.add_sample(Arc::new(sample))
        } else {
            return Err("Failed to lock mixer".to_string());
        };

        use riffl_core::song::Instrument;
        let mut instrument = Instrument::new(&name);
        instrument.sample_index = Some(idx);
        instrument.sample_path = Some(path.display().to_string());
        instrument.chip_render = Some(chip_render);
        self.song.instruments.push(instrument);
        self.sync_mixer_instruments();
        self.instrument_names.push(name);
        self.mark_dirty();
        Ok(idx)
    }

    /// Assign a sample file to an existing instrument slot, replacing its current sample.
    ///
    /// Loads the audio from `path` into the mixer and updates the instrument's
    /// `sample_index` and `sample_path`. The instrument name is preserved.
    /// Apply the waveform editor pencil value to the selected instrument sample.
    pub fn draw_waveform_sample(&mut self) -> Result<(), String> {
        let inst_idx = self
            .instrument_selection()
            .ok_or_else(|| "No instrument selected".to_string())?;
        let sample_idx = self.song.instruments[inst_idx]
            .sample_index
            .ok_or_else(|| "Selected instrument has no sample".to_string())?;
        let sample = self
            .loaded_samples()
            .get(sample_idx)
            .map(|sample| sample.as_ref().clone())
            .ok_or_else(|| "Selected sample is not loaded".to_string())?;

        let mut updated = sample;
        self.waveform_editor.draw_at_cursor(&mut updated);
        self.replace_instrument_sample(inst_idx, sample_idx, updated)
    }
    /// Preview a note pitch through the current pattern editor instrument's sample.
    /// Called when the user enters a note in Insert mode.
    pub fn preview_note_pitch(&mut self, pitch: Pitch, octave: u8) {
        let inst_idx = self.editor.current_instrument() as usize;
        self.preview_instrument_note_pitch(inst_idx, pitch, octave);
    }
    /// Preview the currently selected sample in the sample browser.
    /// Loads and plays it at natural pitch without adding it to the instrument list.
    /// Starts from `self.browser_preview_offset_frames` so scrubbing is preserved.
    pub fn preview_selected_sample(&mut self) -> Result<(), String> {
        let path = self
            .sample_browser
            .selected_path()
            .filter(|_| self.sample_browser.selected_is_file())
            .ok_or_else(|| "No file selected".to_string())?
            .to_path_buf();

        let output_sample_rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);

        let sample =
            load_sample(&path, output_sample_rate).map_err(|e| format!("Failed to load: {e}"))?;

        let rate = sample.sample_rate() as f64 / output_sample_rate as f64;
        let sample = Arc::new(sample);

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.trigger_preview_at(
                Arc::clone(&sample),
                rate,
                self.browser_preview_offset_frames,
            );
        } else {
            return Err("Failed to lock mixer".to_string());
        }

        // Keep sample + rate for scrubbing re-triggers
        self.browser_preview_sample = Some(sample);
        self.browser_preview_rate = rate;
        self.browser_preview_active = true;

        // Ensure audio engine is running so the preview is audible
        if let Some(ref mut engine) = self.audio_engine {
            if !engine.is_playing() {
                let _ = engine.start();
            }
        }

        Ok(())
    }
}
