use crate::ui::file_browser::FileBrowser;
use std::path::PathBuf;
use std::sync::Arc;

impl super::App {
    /// Set the sample directories used by both the overlay file browser and the dedicated view.
    pub fn set_sample_dirs(&mut self, dirs: Vec<std::path::PathBuf>) {
        self.configured_sample_dirs = dirs;
        self.refresh_browser_roots();
    }

    /// Rebuild browser roots from configured dirs plus any project-relative samples dir.
    ///
    /// Call this after changing `project_path`, `configured_sample_dirs`, or bookmarks.
    pub(crate) fn refresh_browser_roots(&mut self) {
        // File browser uses the default modules directory
        let default_modules = crate::config::Config::default_modules_dir();
        self.file_browser = FileBrowser::new(&default_modules);
        self.sample_browser
            .set_roots(self.configured_sample_dirs.clone());

        // Apply persisted bookmarks
        let bookmarks: Vec<std::path::PathBuf> = self
            .config
            .bookmarked_dirs
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        self.sample_browser.set_bookmarks(bookmarks);

        // Auto-add <project_dir>/samples/ if it exists and isn't already a root
        if let Some(proj_dir) = self
            .project_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
        {
            let samples_dir = proj_dir.join("samples");
            if samples_dir.is_dir() {
                self.sample_browser.add_auto_root(samples_dir);
            }
        }
    }

    /// Toggle a bookmark on the currently selected directory in the sample browser.
    ///
    /// If the selection is a directory, it is added to (or removed from) the
    /// `config.bookmarked_dirs` list, the config is saved, and the browser is
    /// refreshed so bookmarked dirs appear at the top of the roots list.
    /// Has no effect if the selected entry is a file or if the list is empty.
    pub fn toggle_browser_bookmark(&mut self) {
        let path = match self
            .sample_browser
            .selected_path()
            .filter(|_| self.sample_browser.selected_is_dir())
        {
            Some(p) => p.to_path_buf(),
            None => return,
        };

        let path_str = path.display().to_string();
        if let Some(pos) = self
            .config
            .bookmarked_dirs
            .iter()
            .position(|d| d == &path_str)
        {
            self.config.bookmarked_dirs.remove(pos);
        } else {
            self.config.bookmarked_dirs.push(path_str);
        }

        let _ = self.config.save();
        let bookmarks: Vec<std::path::PathBuf> = self
            .config
            .bookmarked_dirs
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        self.sample_browser.set_bookmarks(bookmarks);
    }

    /// Open the file browser overlay, delegating to an external picker or the built-in browser.
    ///
    /// Behaviour is controlled by `config.file_picker`:
    ///   "auto"    — try yazi, fall back to built-in on failure (default)
    ///   "builtin" — always use the built-in overlay browser
    ///   "yazi"    — always use yazi (no fallback)
    ///   "<name>"  — run `<name> --chooser-file <tmpfile> <dir>` (yazi-compatible protocol)
    pub fn open_file_browser(&mut self) {
        let start_dir = self
            .configured_sample_dirs
            .first()
            .cloned()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let picker = self.config.file_picker.clone();

        if picker == "builtin" {
            self.file_browser = crate::ui::file_browser::FileBrowser::new(&start_dir);
            self.file_browser.open();
            return;
        }

        // Determine the external command to try (empty string = skip external)
        let cmd = if picker == "auto" || picker == "yazi" {
            "yazi".to_string()
        } else {
            picker.clone()
        };

        use crossterm::{
            execute,
            terminal::{
                disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
            },
        };

        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();

        let temp_file = std::env::temp_dir().join("riffl_picker_selection");
        let _ = std::fs::remove_file(&temp_file);

        let status = std::process::Command::new(&cmd)
            .arg("--chooser-file")
            .arg(&temp_file)
            .arg(&start_dir)
            .status();

        // Restore UI
        let _ = enable_raw_mode();
        let _ = execute!(stdout, EnterAlternateScreen);

        // Force ratatui to flush its full buffer on the next draw cycle —
        // without this the screen stays blank until the user presses a key.
        self.needs_full_redraw = true;

        match status {
            Ok(s) if s.success() => {
                if let Ok(path_str) = std::fs::read_to_string(&temp_file) {
                    let trimmed = path_str.trim();
                    if !trimmed.is_empty() {
                        let path = std::path::PathBuf::from(trimmed);
                        let is_module = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| {
                                let e = e.to_ascii_lowercase();
                                e == "mod" || e == "xm" || e == "it" || e == "s3m" || e == "rtm"
                            })
                            .unwrap_or(false);

                        if is_module {
                            match self.import_file(&path) {
                                Ok(()) => {
                                    let name = path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                    self.open_modal(crate::ui::modal::Modal::info(
                                        "Module Imported".to_string(),
                                        format!("Imported '{}'", name),
                                    ));
                                }
                                Err(msg) => {
                                    self.open_modal(crate::ui::modal::Modal::error(
                                        "Import Failed".to_string(),
                                        msg,
                                    ));
                                }
                            }
                        } else {
                            match self.load_sample_from_path(&path) {
                                Ok(idx) => {
                                    let name = self
                                        .song
                                        .instruments
                                        .get(idx)
                                        .map(|i| i.name.clone())
                                        .unwrap_or_default();
                                    self.open_modal(crate::ui::modal::Modal::info(
                                        "Sample Loaded".to_string(),
                                        format!("Loaded '{}' to instrument {:02X}", name, idx),
                                    ));
                                }
                                Err(msg) => {
                                    self.open_modal(crate::ui::modal::Modal::error(
                                        "Load Failed".to_string(),
                                        msg,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                // External picker closed without selecting a file — nothing to do.
            }
            Err(_) => {
                // Command not found or failed to spawn.
                if picker == "auto" {
                    // Fall back to built-in browser.
                    self.file_browser = crate::ui::file_browser::FileBrowser::new(&start_dir);
                    self.file_browser.open();
                }
                // For "yazi" or a named command we don't silently fall back,
                // so the user knows their configured picker isn't available.
            }
        }
    }

    /// Close the file browser overlay
    pub fn close_file_browser(&mut self) {
        self.file_browser.close();
    }

    /// Check if the export dialog is currently active.
    pub fn has_export_dialog(&self) -> bool {
        self.export_dialog.active
    }

    /// Check if the file browser is open
    pub fn has_file_browser(&self) -> bool {
        self.file_browser.active
    }

    /// Load the currently selected file from the file browser.
    pub fn stop_browser_preview(&mut self) {
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.stop_preview();
        }
        self.browser_preview_active = false;
    }

    /// Toggle browser preview: stop if playing, start from current offset if not.
    pub fn toggle_browser_preview(&mut self) {
        let playing = self
            .mixer
            .lock()
            .map(|m| m.is_preview_playing())
            .unwrap_or(false);
        if playing {
            self.stop_browser_preview();
        } else {
            let _ = self.preview_selected_sample();
        }
    }

    /// Adjust the preview scrub offset and re-trigger from the new position.
    /// `forward` moves later into the sample; `false` moves earlier.
    /// Step is 0.25 s in sample-native frames, clamped to [0, sample_length].
    pub fn scrub_browser_preview(&mut self, forward: bool) {
        let sample = match self.browser_preview_sample.clone() {
            Some(s) => s,
            None => return,
        };
        let step = (sample.sample_rate() / 4) as usize; // 0.25 s
        let max_frame = sample.frame_count().saturating_sub(1);

        if forward {
            self.browser_preview_offset_frames =
                (self.browser_preview_offset_frames + step).min(max_frame);
        } else {
            self.browser_preview_offset_frames =
                self.browser_preview_offset_frames.saturating_sub(step);
        }

        let rate = self.browser_preview_rate;
        let offset = self.browser_preview_offset_frames;
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.trigger_preview_at(Arc::clone(&sample), rate, offset);
        }
        self.browser_preview_active = true;

        if let Some(ref mut engine) = self.audio_engine {
            if !engine.is_playing() {
                let _ = engine.start();
            }
        }
    }

    /// Clear browser preview state (called on selection change to reset offset).
    pub fn reset_browser_preview(&mut self) {
        self.stop_browser_preview();
        self.browser_preview_offset_frames = 0;
        self.browser_preview_sample = None;
    }

    /// Returns `(current_frame_pos, total_frames, output_sample_rate)` for the active browser preview.
    ///
    /// Used by the waveform renderer to draw a live cursor and time display.
    /// Returns `(0, 0, 44100)` when the mixer is unavailable.
    pub fn preview_cursor_state(&self) -> (usize, usize, u32) {
        let rate = self
            .audio_engine
            .as_ref()
            .map(|e| e.sample_rate())
            .unwrap_or(44100);
        let (pos, total) = self
            .mixer
            .lock()
            .map(|m| m.preview_pos_and_total())
            .unwrap_or((0, 0));
        (pos, total, rate)
    }
}
