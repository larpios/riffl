use crate::ui::modal::Modal;
use std::path::PathBuf;

impl super::App {
    /// Execute the current command-line input and exit command mode.
    pub fn execute_command(&mut self) {
        let cmd = self.command_input.trim().to_string();
        if !cmd.is_empty() {
            self.command_history.push(cmd.clone());
            self.command_history_index = None;
        }
        self.command_mode = false;
        self.command_input.clear();

        // Parse "bpm N" or "t N" or "tempo N"
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let is_bpm_cmd = matches!(parts[0], "bpm" | "t" | "tempo");

        if is_bpm_cmd {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<f64>().ok()) {
                let clamped = val.clamp(20.0, 999.0);
                self.transport.set_bpm(clamped);
                self.song.bpm = clamped;
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.update_tempo(clamped);
                }
                self.mark_dirty();
            } else {
                self.open_modal(Modal::error(
                    "Invalid BPM".to_string(),
                    format!("Usage: :bpm <value>  (got: {:?})", parts.get(1)),
                ));
            }
            return;
        }

        // :step N — set row advance step size
        if parts[0] == "step" {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                self.editor.set_step_size(val);
            } else {
                self.open_modal(Modal::error(
                    "Invalid step".to_string(),
                    "Usage: :step <0-8>".to_string(),
                ));
            }
            return;
        }

        // :w filename — save as a new/specific file
        if parts[0] == "w" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            let current_pos = self.transport.arrangement_position();
            self.flush_editor_pattern(current_pos);
            match tracker_core::project::save_project(&path, &self.song) {
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
            return;
        }

        // :e filename — open/load a project file
        if parts[0] == "e" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            self.load_project(&path);
            return;
        }

        // :load filename — open/load a project file (alias for :e)
        if parts[0] == "load" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            self.load_project(&path);
            return;
        }

        // :save filename — save project (alias for :w)
        if parts[0] == "save" && parts.len() == 2 {
            let path = PathBuf::from(parts[1].trim());
            let current_pos = self.transport.arrangement_position();
            self.flush_editor_pattern(current_pos);
            match tracker_core::project::save_project(&path, &self.song) {
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
            return;
        }

        // :volume N — set global volume (0-100)
        if parts[0] == "volume" {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<f64>().ok()) {
                let clamped = (val / 100.0).clamp(0.0, 1.0) as f32;
                self.song.global_volume = clamped;
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.set_global_volume(clamped);
                }
                self.mark_dirty();
            } else {
                let current_vol = (self.song.global_volume * 100.0).round() as i32;
                self.open_modal(Modal::info(
                    "Volume".to_string(),
                    format!("Current: {}%. Usage: :volume <0-100>", current_vol),
                ));
            }
            return;
        }

        match cmd.as_str() {
            "w" => self.save_project(),
            "wq" | "x" => {
                self.save_project();
                if !self.is_dirty {
                    self.force_quit();
                }
            }
            "q" => self.quit(),
            "q!" => self.force_quit(),
            "tutor" => {
                self.show_tutor = true;
                self.tutor_scroll = 0;
            }
            _ => {
                self.open_modal(Modal::error(
                    "Unknown command".to_string(),
                    format!(":{}", cmd),
                ));
            }
        }
    }
}
