use super::App;
use crate::editor::Editor;
use crate::ui::arrangement::ArrangementView;
use crate::ui::modal::Modal;
use riffl_core::audio::load_sample;
use riffl_core::audio::ChipRenderData;
use riffl_core::audio::Sample;
use riffl_core::pattern::Pattern;
use riffl_core::{export, project};
use std::path::{Path, PathBuf};
use std::sync::Arc;

impl App {
    /// Import a module file (.mod, .xm, .it), replacing the current song.
    /// Returns Ok(()) on success, or an error message.
    pub fn import_file(&mut self, path: &Path) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("Read error: {e}"))?;

        let result = riffl_core::format::load(&data).map_err(|e| format!("Import error: {e}"))?;

        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.clear_samples();
            self.instrument_names.clear();
            for (sample, inst) in result.samples.iter().zip(result.song.instruments.iter()) {
                mixer.add_sample(Arc::new(sample.clone()));
                self.instrument_names.push(inst.name.clone());
            }
        } else {
            return Err("Failed to lock mixer".to_string());
        }

        self.song = result.song;
        self.initial_bpm = self.song.bpm;
        self.initial_tpl = self.song.tpl;
        self.transport.set_bpm(self.song.bpm);
        self.transport.set_tpl(self.song.tpl);
        self.transport.set_lpb(self.song.lpb);
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.set_num_channels(self.song.tracks.len());
            mixer.update_tempo(self.song.bpm);
            mixer.set_tpl(self.song.tpl);
            mixer.set_global_volume(self.song.global_volume);
            mixer.set_effect_mode(self.song.effect_mode);
            mixer.set_format_is_s3m(self.song.format_is_s3m);
            mixer.set_global_volume_range(if self.song.format_is_it { 128.0 } else { 64.0 });
            mixer.set_slide_mode(self.song.slide_mode);
            mixer.set_pan_separation(self.song.pan_separation);
            mixer.set_panning_law(self.song.panning_law);
        }
        self.sync_mixer_instruments();
        self.sync_mixer_tracks();
        // Snap strip pans immediately so the first rendered sample uses the
        // correct IT channel panning, bypassing the normal smoothing ramp.
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.snap_channel_pans(&self.song.tracks);
        }
        self.transport.stop();

        let pattern_idx = self.song.arrangement.first().copied().unwrap_or(0);
        let pattern = if pattern_idx < self.song.patterns.len() {
            self.song.patterns[pattern_idx].clone()
        } else {
            Pattern::default()
        };

        self.editor = Editor::new(pattern);
        self.sync_mixer_channels();

        self.arrangement_view = ArrangementView::new();
        self.is_dirty = false;

        Ok(())
    }

    /// Open the export dialog.
    pub fn open_export_dialog(&mut self) {
        let name = self
            .project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .unwrap_or("untitled");
        let default_path = format!("{}.wav", name);
        self.export_dialog.open(&default_path);
    }

    /// Execute the WAV export using current dialog settings.
    pub fn execute_export(&mut self) {
        let config = self.export_dialog.to_config();
        let path = PathBuf::from(&self.export_dialog.output_path);

        self.export_dialog.start_export();

        // Clone sample references from the mixer for offline rendering
        let samples: Vec<Arc<Sample>> = if let Ok(mixer) = self.mixer.lock() {
            mixer.samples().to_vec()
        } else {
            self.export_dialog
                .finish_error("Failed to lock mixer".to_string());
            return;
        };

        // Run export synchronously (offline rendering)
        let duration = export::song_duration(&self.song);
        match export::export_wav(&path, &self.song, &samples, &config, |progress| {
            // Progress is 0.0-1.0, but we can't update the dialog in a closure
            // because &mut self is already borrowed. Progress is best-effort here.
            let _ = progress;
        }) {
            Ok(()) => {
                let message = format!(
                    "Exported successfully!\n\nFile: {}\nDuration: {:.1}s\nSample rate: {} Hz\nBit depth: {}-bit",
                    path.display(),
                    duration,
                    config.sample_rate,
                    config.bit_depth.bits_per_sample(),
                );
                self.export_dialog.finish_success(message);
            }
            Err(e) => {
                self.export_dialog.finish_error(format!("{}", e));
            }
        }
    }

    /// Save the current project to disk.
    pub fn save_project(&mut self) {
        let current_pos = self.transport.arrangement_position();
        self.flush_editor_pattern(current_pos);

        let path = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("untitled.rtm"));

        match project::save_project(&path, &self.song) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.is_dirty = false;
                self.open_modal(Modal::info(
                    "Project Saved".to_string(),
                    format!("Saved to: {}", path.display()),
                ));
            }
            Err(e) => {
                self.open_modal(Modal::error("Save Failed".to_string(), format!("{}", e)));
            }
        }
    }

    /// Load a project from disk.
    pub fn load_project(&mut self, path: &Path) {
        match project::load_project(path) {
            Ok(mut song) => {
                let pattern = if !song.patterns.is_empty() {
                    song.patterns[0].clone()
                } else {
                    Pattern::default()
                };
                self.editor = Editor::new(pattern);

                let output_sample_rate = self
                    .audio_engine
                    .as_ref()
                    .map(|e| e.sample_rate())
                    .unwrap_or(44100);

                let mut missing_samples = Vec::new();
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.clear_samples();
                    self.instrument_names.clear();

                    for (idx, inst) in song.instruments.iter_mut().enumerate() {
                        let sample_name = if let Some(sample_path) = &inst.sample_path {
                            let sp_path = PathBuf::from(sample_path);
                            match load_sample(&sp_path, output_sample_rate) {
                                Ok(sample) => {
                                    inst.sample_index = Some(idx);
                                    inst.chip_render = Some(ChipRenderData::from_sample(&sample));
                                    mixer.add_sample(Arc::new(sample));
                                    inst.name.clone()
                                }
                                Err(_) => {
                                    inst.sample_index = Some(idx);
                                    inst.chip_render = None;
                                    mixer.add_sample(Arc::new(Sample::default()));
                                    missing_samples.push(sample_path.clone());
                                    format!("{} (MISSING)", inst.name)
                                }
                            }
                        } else {
                            inst.sample_index = Some(idx);
                            inst.chip_render = None;
                            mixer.add_sample(Arc::new(Sample::default()));
                            inst.name.clone()
                        };
                        self.instrument_names.push(sample_name);
                    }
                }

                self.song = song;
                self.initial_bpm = self.song.bpm;
                self.initial_tpl = self.song.tpl;
                self.transport.set_bpm(self.song.bpm);
                self.transport.set_tpl(self.song.tpl);
                self.transport.set_lpb(self.song.lpb);
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.update_tempo(self.song.bpm);
                    mixer.set_tpl(self.song.tpl);
                    mixer.set_global_volume(self.song.global_volume);
                    mixer.set_effect_mode(self.song.effect_mode);
                    mixer.set_format_is_s3m(self.song.format_is_s3m);
                    mixer.set_slide_mode(self.song.slide_mode);
                    mixer.set_pan_separation(self.song.pan_separation);
                    mixer.set_panning_law(self.song.panning_law);
                }
                self.sync_mixer_instruments();
                self.project_path = Some(path.to_path_buf());
                self.is_dirty = false;
                self.arrangement_view = ArrangementView::new();
                self.transport.stop();
                // Auto-detect project-relative samples dir
                self.refresh_browser_roots();

                if missing_samples.is_empty() {
                    self.open_modal(Modal::info(
                        "Project Loaded".to_string(),
                        format!("Loaded: {}", path.display()),
                    ));
                } else {
                    self.open_modal(Modal::error(
                        "Project Loaded with Missing Samples".to_string(),
                        format!(
                            "Loaded: {}\n\nMissing samples:\n{}",
                            path.display(),
                            missing_samples.join("\n")
                        ),
                    ));
                }
            }
            Err(e) => {
                self.open_modal(Modal::error("Load Failed".to_string(), format!("{}", e)));
            }
        }
    }

    /// Get the loaded samples from the mixer.
    pub fn loaded_samples(&self) -> Vec<Arc<Sample>> {
        if let Ok(mixer) = self.mixer.lock() {
            mixer.samples().to_vec()
        } else {
            Vec::new()
        }
    }
}
