use crate::app::PickerOutcome;
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

    /// Open the sample file picker (Ctrl-F). Loads the selection as a sample.
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

        if self.config.file_picker == "builtin" {
            self.file_browser = crate::ui::file_browser::FileBrowser::new(&start_dir);
            self.file_browser.open();
            return;
        }

        self.launch_external_picker(start_dir, false);
    }

    /// Open the module file picker (Ctrl-I). Imports the selection as a module.
    pub fn open_module_browser(&mut self) {
        let start_dir = crate::config::Config::default_modules_dir();

        if self.config.file_picker == "builtin" {
            // file_browser is already rooted at the modules dir (see refresh_browser_roots)
            self.file_browser.open();
            return;
        }

        self.launch_external_picker(start_dir, true);
    }

    /// Spawn the external picker on a background thread and yield the terminal to it.
    /// Does nothing if a picker is already running.
    fn launch_external_picker(&mut self, start_dir: std::path::PathBuf, is_module: bool) {
        if self.picker_rx.is_some() {
            return;
        }

        let picker = self.config.file_picker.clone();
        let cmd = if picker == "auto" || picker == "yazi" {
            "yazi".to_string()
        } else {
            picker.clone()
        };

        use crossterm::{
            execute,
            terminal::{disable_raw_mode, LeaveAlternateScreen},
        };

        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let _ = disable_raw_mode();

        let temp_file = std::env::temp_dir().join("riffl_picker_selection");
        let _ = std::fs::remove_file(&temp_file);

        let (tx, rx) = std::sync::mpsc::channel::<PickerOutcome>();
        self.picker_rx = Some(rx);
        self.picker_is_module = is_module;

        let is_auto = picker == "auto";
        std::thread::spawn(move || {
            let result = std::process::Command::new(&cmd)
                .arg("--chooser-file")
                .arg(&temp_file)
                .arg(&start_dir)
                .status();

            let outcome = match result {
                Ok(s) if s.success() => {
                    let selected = std::fs::read_to_string(&temp_file)
                        .ok()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .map(std::path::PathBuf::from);
                    match selected {
                        Some(path) => PickerOutcome::File(path),
                        None => PickerOutcome::Cancelled,
                    }
                }
                Ok(_) => PickerOutcome::Cancelled,
                Err(_) => {
                    if is_auto {
                        PickerOutcome::Fallback(start_dir)
                    } else {
                        PickerOutcome::Cancelled
                    }
                }
            };
            let _ = tx.send(outcome);
        });
    }

    /// Poll the background picker thread. Called every tick while the picker is running.
    /// Restores the terminal and handles the result when the picker exits.
    pub fn poll_picker(&mut self) {
        use std::sync::mpsc::TryRecvError;

        let outcome = match self.picker_rx.as_ref() {
            None => return,
            Some(rx) => match rx.try_recv() {
                Ok(o) => {
                    self.picker_rx = None;
                    o
                }
                Err(TryRecvError::Disconnected) => {
                    self.picker_rx = None;
                    PickerOutcome::Cancelled
                }
                Err(TryRecvError::Empty) => return,
            },
        };

        use crossterm::{
            execute,
            terminal::{enable_raw_mode, EnterAlternateScreen},
        };
        let mut stdout = std::io::stdout();
        let _ = enable_raw_mode();
        let _ = execute!(stdout, EnterAlternateScreen);
        self.needs_full_redraw = true;

        let is_module = self.picker_is_module;
        match outcome {
            PickerOutcome::Cancelled => {}
            PickerOutcome::Fallback(start_dir) => {
                self.file_browser = crate::ui::file_browser::FileBrowser::new(&start_dir);
                self.file_browser.open();
            }
            PickerOutcome::File(path) => {
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

    /// Returns true while a background external file-picker is running.
    pub fn has_external_picker_running(&self) -> bool {
        self.picker_rx.is_some()
    }

    /// Close the file browser overlay
    pub fn close_file_browser(&mut self) {
        self.file_browser.close();
        self.is_project_browser = false;
    }

    /// Open the project file browser (Ctrl-O). Shows only .rtm files.
    pub fn open_project_browser(&mut self) {
        let start_dir = self
            .project_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        self.file_browser = crate::ui::file_browser::FileBrowser::new_project(&start_dir);
        self.file_browser.open();
        self.is_project_browser = true;
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
