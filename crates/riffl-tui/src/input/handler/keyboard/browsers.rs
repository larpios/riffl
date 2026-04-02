use crate::app::App;
use crate::ui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_sample_browser_key(app: &mut App, key: KeyEvent) -> bool {
    // Only consume plain (no-modifier) navigation keys.
    // Everything else falls through so view switching, command mode, help,
    // transport shortcuts, etc. all keep working.
    if key.modifiers != KeyModifiers::NONE {
        return false;
    }

    let consumed = match key.code {
        // Navigation — also clears any active preview so offset resets on item change
        KeyCode::Char('j') | KeyCode::Down => {
            app.reset_browser_preview();
            app.sample_browser.move_down();
            true
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.reset_browser_preview();
            app.sample_browser.move_up();
            true
        }

        // Enter directory (l always navigates; Right scrubs when previewing)
        KeyCode::Char('l') => {
            app.sample_browser.enter_dir();
            true
        }
        KeyCode::Right => {
            if app.browser_preview_active {
                app.scrub_browser_preview(true);
            } else {
                app.sample_browser.enter_dir();
            }
            true
        }

        // Go up a directory (h / Backspace always navigate; Left scrubs when previewing)
        KeyCode::Char('h') | KeyCode::Backspace => {
            app.sample_browser.go_up();
            true
        }
        KeyCode::Left => {
            if app.browser_preview_active {
                app.scrub_browser_preview(false);
            } else {
                app.sample_browser.go_up();
            }
            true
        }

        // Jump back to the roots list from anywhere in the filesystem
        KeyCode::Char('~') => {
            app.sample_browser.reset_to_roots();
            true
        }

        // Load file or enter directory
        KeyCode::Enter => {
            if app.sample_browser.selected_is_file() {
                let path = app.sample_browser.selected_path().map(|p| p.to_path_buf());
                if let Some(path) = path {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();

                    if matches!(ext.as_str(), "mod" | "xm" | "it" | "s3m") {
                        // Module files always import the whole song — no choice needed.
                        match app.import_file(&path) {
                            Ok(()) => {
                                let name = path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("file")
                                    .to_string();
                                app.open_modal(ui::modal::Modal::info(
                                    "Module Imported".to_string(),
                                    format!("Loaded '{name}'"),
                                ));
                            }
                            Err(e) => {
                                app.open_modal(ui::modal::Modal::error(
                                    "Import Failed".to_string(),
                                    e,
                                ));
                            }
                        }
                    } else {
                        // Show an action menu so the user can choose what to do.
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("sample")
                            .to_string();

                        let assign_line = app
                            .instrument_selection()
                            .and_then(|i| {
                                app.song.instruments.get(i).map(|inst| {
                                    format!("\n  [a]  Assign to '{}' (slot {:02X})", inst.name, i)
                                })
                            })
                            .unwrap_or_default();

                        let message =
                            format!("'{filename}'\n\n  [l]  Load as new instrument{assign_line}");

                        app.pending_sample_path = Some(path);
                        app.open_modal(ui::modal::Modal::menu("Load Sample".to_string(), message));
                    }
                }
            } else {
                app.sample_browser.enter_dir();
            }
            true
        }

        // Preview selected file — Space toggles play/stop; does not restart if already playing
        KeyCode::Char(' ') => {
            if app.sample_browser.selected_is_file() {
                app.toggle_browser_preview();
            }
            true
        }

        // Bookmark selected directory — b toggles bookmark, persists to config
        KeyCode::Char('b') => {
            app.toggle_browser_bookmark();
            true
        }

        _ => false,
    };

    if consumed {
        maybe_update_waveform(app);
    }
    consumed
}

fn maybe_update_waveform(app: &mut App) {
    use crate::ui::sample_browser::compute_waveform_peaks;

    let path = match app
        .sample_browser
        .selected_path()
        .filter(|_| app.sample_browser.selected_is_file())
        .map(|p| p.to_path_buf())
    {
        Some(p) => p,
        None => {
            app.sample_browser.clear_waveform();
            return;
        }
    };

    let is_wav = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    if !is_wav {
        app.sample_browser.clear_waveform();
        return;
    }

    // Only reload when the selection has changed.
    if app.sample_browser.waveform_path() == Some(path.as_path()) {
        return;
    }

    let peaks = compute_waveform_peaks(&path, 128);
    app.sample_browser.set_waveform_peaks(path, peaks);
}

pub fn handle_file_browser_key(app: &mut App, key: KeyEvent) {
    if key.modifiers != KeyModifiers::NONE {
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.close_file_browser();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.file_browser.move_down();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.file_browser.move_up();
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.file_browser.go_up();
        }
        KeyCode::Char('l') | KeyCode::Right => {
            if app.file_browser.selected_is_dir() {
                app.file_browser.enter_selected_dir();
            }
        }
        KeyCode::Enter => {
            if app.file_browser.selected_is_dir() {
                app.file_browser.enter_selected_dir();
                return;
            }

            let is_module = app
                .file_browser
                .selected_path()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .map(|e| {
                    let e = e.to_ascii_lowercase();
                    e == "mod" || e == "xm" || e == "it" || e == "s3m"
                })
                .unwrap_or(false);

            if is_module {
                let path = app.file_browser.selected_path().map(|p| p.to_path_buf());
                if let Some(path) = path {
                    match app.import_file(&path) {
                        Ok(()) => {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            app.close_file_browser();
                            app.open_modal(ui::modal::Modal::info(
                                "Module Imported".to_string(),
                                format!("Imported '{}'", name),
                            ));
                        }
                        Err(msg) => {
                            app.close_file_browser();
                            app.open_modal(ui::modal::Modal::error(
                                "Import Failed".to_string(),
                                msg,
                            ));
                        }
                    }
                }
            } else {
                match app.load_selected_sample() {
                    Ok(idx) => {
                        let name = app
                            .song
                            .instruments
                            .get(idx)
                            .map(|i| i.name.clone())
                            .unwrap_or_else(|| "unknown".to_string());
                        app.close_file_browser();
                        app.open_modal(ui::modal::Modal::info(
                            "Sample Loaded".to_string(),
                            format!("Loaded '{}' as instrument {:02X}", name, idx),
                        ));
                    }
                    Err(msg) => {
                        app.close_file_browser();
                        app.open_modal(ui::modal::Modal::error("Load Failed".to_string(), msg));
                    }
                }
            }
        }
        _ => {}
    }
}
