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
            match riffl_core::project::save_project(&path, &self.song) {
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
            match riffl_core::project::save_project(&path, &self.song) {
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

        // :transpose N — transpose selection by N semitones
        if parts[0] == "transpose" || parts[0] == "tr" {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<i32>().ok()) {
                self.editor.transpose_selection(val);
                self.mark_dirty();
            } else {
                self.open_modal(Modal::error(
                    "Invalid transpose".to_string(),
                    "Usage: :transpose <semitones>  (e.g. :transpose -12)".to_string(),
                ));
            }
            return;
        }

        // :quantize — quantize selection to step grid
        if parts[0] == "quantize" {
            self.editor.quantize();
            self.mark_dirty();
            return;
        }

        // :interpolate / :interp — interpolate volume across visual selection
        if parts[0] == "interpolate" || parts[0] == "interp" {
            self.editor.interpolate();
            self.mark_dirty();
            return;
        }

        // :clear — clear all cells in the current pattern
        if parts[0] == "clear" {
            let rows = self.editor.pattern().num_rows();
            let channels = self.editor.pattern().num_channels();
            let pat = self.editor.pattern_mut();
            for r in 0..rows {
                for c in 0..channels {
                    pat.clear_cell(r, c);
                }
            }
            let pos = self.transport.arrangement_position();
            self.flush_editor_pattern(pos);
            self.mark_dirty();
            return;
        }

        // :len N — resize current pattern to N rows
        if parts[0] == "len" || parts[0] == "length" {
            if let Some(n) = parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                use riffl_core::pattern::pattern::{MAX_ROW_COUNT, MIN_ROW_COUNT};
                let clamped = n.clamp(MIN_ROW_COUNT, MAX_ROW_COUNT);
                self.editor.pattern_mut().set_row_count(clamped);
                self.transport.set_num_rows(clamped);
                let cursor = self.editor.cursor_row();
                if cursor >= clamped {
                    self.editor.go_to_row(clamped.saturating_sub(1));
                }
                let pos = self.transport.arrangement_position();
                self.flush_editor_pattern(pos);
                self.mark_dirty();
            } else {
                let current = self.editor.pattern().num_rows();
                self.open_modal(Modal::info(
                    "Pattern Length".to_string(),
                    format!("Current: {} rows. Usage: :len <16-512>", current),
                ));
            }
            return;
        }

        // :speed N — set ticks per line (1-31)
        if parts[0] == "speed" || parts[0] == "tpl" {
            if let Some(val) = parts.get(1).and_then(|s| s.trim().parse::<u32>().ok()) {
                let clamped = val.clamp(1, 31);
                self.transport.set_tpl(clamped);
                if let Ok(mut mixer) = self.mixer.lock() {
                    mixer.set_tpl(clamped);
                }
                self.song.tpl = clamped;
                self.mark_dirty();
            } else {
                let current = self.transport.tpl();
                self.open_modal(Modal::info(
                    "Speed (TPL)".to_string(),
                    format!("Current: {} ticks/line. Usage: :speed <1-31>", current),
                ));
            }
            return;
        }

        // :loop N M — set loop region to rows N..=M and activate it
        if parts[0] == "loop" {
            let nums: Vec<usize> = parts
                .get(1)
                .map(|s| {
                    s.split_whitespace()
                        .filter_map(|t| t.parse::<usize>().ok())
                        .collect()
                })
                .unwrap_or_default();
            if nums.len() == 2 {
                let (start, end) = (nums[0].min(nums[1]), nums[0].max(nums[1]));
                self.transport.set_loop_region(start, end);
                self.transport.set_loop_region_active(true);
            } else {
                self.open_modal(Modal::info(
                    "Loop Region".to_string(),
                    "Usage: :loop <start_row> <end_row>".to_string(),
                ));
            }
            return;
        }

        // :fill <note> [<step>] — fill current channel with a note every <step> rows
        // Note format: "C-4", "C#4", "A-5" etc.  Step defaults to 1.
        if parts[0] == "fill" {
            let args = parts.get(1).copied().unwrap_or("").to_string();
            let mut arg_parts = args.split_whitespace();
            let note_str = arg_parts.next().unwrap_or("");
            let step: usize = arg_parts
                .next()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 64);

            use riffl_core::pattern::note::Note;
            if let Some(note) = Note::from_tracker_str(note_str) {
                let ch = self.editor.cursor_channel();
                let num_rows = self.editor.pattern().num_rows();
                // Determine which rows to fill: visual selection or whole pattern
                use crate::editor::EditorMode;
                let (r0, r1) = if self.editor.mode() == EditorMode::Visual {
                    self.editor
                        .visual_selection()
                        .map(|((r0, _), (r1, _))| (r0, r1))
                        .unwrap_or((0, num_rows.saturating_sub(1)))
                } else {
                    (0, num_rows.saturating_sub(1))
                };
                for row in (r0..=r1).step_by(step) {
                    self.editor.pattern_mut().set_note(row, ch, note);
                }
                self.mark_dirty();
            } else {
                self.open_modal(Modal::error(
                    "Invalid note".to_string(),
                    "Usage: :fill <note> [<step>]   e.g.  :fill C-4 4".to_string(),
                ));
            }
            return;
        }

        // :adsr A D S R — set ADSR volume envelope on current instrument
        // Values: A/D/R in milliseconds (≥0), S is sustain level 0-100 (%).
        if parts[0] == "adsr" {
            // Re-split the args portion since `parts` uses splitn(2)
            let adsr_args: Vec<&str> = parts
                .get(1)
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            if adsr_args.len() >= 4 {
                let parse = |s: &str| s.trim().parse::<f32>().ok();
                if let (Some(a), Some(d), Some(s_pct), Some(r)) = (
                    parse(adsr_args[0]),
                    parse(adsr_args[1]),
                    parse(adsr_args[2]),
                    parse(adsr_args[3]),
                ) {
                    use riffl_core::song::Adsr;
                    let adsr = Adsr::new(
                        a.max(0.0),
                        d.max(0.0),
                        (s_pct / 100.0).clamp(0.0, 1.0),
                        r.max(0.0),
                    );
                    let idx = self.instrument_selection().unwrap_or(0);
                    if let Some(inst) = self.song.instruments.get_mut(idx) {
                        inst.volume_adsr = Some(adsr);
                        self.sync_mixer_instruments();
                        self.mark_dirty();
                    }
                } else {
                    self.open_modal(Modal::error(
                        "Invalid ADSR".to_string(),
                        "Usage: :adsr <attack_ms> <decay_ms> <sustain%> <release_ms>".to_string(),
                    ));
                }
            } else {
                // Show current ADSR for this instrument (or usage if args are bad)
                let idx = self.instrument_selection().unwrap_or(0);
                let info = if let Some(inst) = self.song.instruments.get(idx) {
                    if let Some(ref a) = inst.volume_adsr {
                        format!(
                            "A:{} D:{} S:{:.0}% R:{}",
                            a.attack as i32,
                            a.decay as i32,
                            a.sustain * 100.0,
                            a.release as i32
                        )
                    } else {
                        "(none — using point envelope)".to_string()
                    }
                } else {
                    "(no instrument)".to_string()
                };
                self.open_modal(Modal::info(
                    "ADSR Envelope".to_string(),
                    format!(
                        "{}\nUsage: :adsr <attack_ms> <decay_ms> <sustain%> <release_ms>",
                        info
                    ),
                ));
            }
            return;
        }

        // :mode <native|compat|amiga> — switch effect interpretation mode
        if parts[0] == "mode" {
            use riffl_core::pattern::effect::EffectMode;
            match parts.get(1).map(|s| s.trim()) {
                Some("native") | Some("riffl") => {
                    self.song.effect_mode = EffectMode::RifflNative;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.set_effect_mode(EffectMode::RifflNative);
                    }
                    self.mark_dirty();
                }
                Some("compat") | Some("compatible") | Some("it") | Some("xm") => {
                    self.song.effect_mode = EffectMode::Compatible;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.set_effect_mode(EffectMode::Compatible);
                    }
                    self.mark_dirty();
                }
                Some("amiga") | Some("pt") | Some("mod") => {
                    self.song.effect_mode = EffectMode::Amiga;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.set_effect_mode(EffectMode::Amiga);
                    }
                    self.mark_dirty();
                }
                _ => {
                    let current = match self.song.effect_mode {
                        EffectMode::RifflNative => "native",
                        EffectMode::Compatible => "compat",
                        EffectMode::Amiga => "amiga",
                    };
                    self.open_modal(Modal::info(
                        "Effect Mode".to_string(),
                        format!("Current: {}\nUsage: :mode <native|compat|amiga>", current),
                    ));
                }
            }
            return;
        }

        // :track add / :track del — add or remove a channel in the pattern editor
        if parts[0] == "track" {
            match parts.get(1).map(|s| s.trim()) {
                Some("add") | Some("new") => {
                    self.editor.add_track();
                    self.sync_mixer_channels();
                    self.mark_dirty();
                }
                Some("del") | Some("delete") | Some("remove") | Some("rm") => {
                    self.editor.delete_track();
                    self.sync_mixer_channels();
                    self.mark_dirty();
                }
                _ => {
                    let channels = self.editor.pattern().num_channels();
                    self.open_modal(Modal::info(
                        "Track".to_string(),
                        format!("Channels: {}\nUsage: :track <add|del>", channels),
                    ));
                }
            }
            return;
        }

        // :pname <name> — rename the selected or currently-edited pattern
        if parts[0] == "pname" {
            let arr_pos = self.transport.arrangement_position();
            let pat_idx = self
                .pattern_selection()
                .unwrap_or_else(|| self.song.arrangement.get(arr_pos).copied().unwrap_or(0));
            let name = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                let current = self
                    .song
                    .patterns
                    .get(pat_idx)
                    .map(|p| {
                        if p.name.is_empty() {
                            "(unnamed)".to_string()
                        } else {
                            p.name.clone()
                        }
                    })
                    .unwrap_or_else(|| "(none)".to_string());
                self.open_modal(Modal::info(
                    "Pattern Name".to_string(),
                    format!("Current: {}\nUsage: :pname <name>", current),
                ));
            } else {
                if let Some(pat) = self.song.patterns.get_mut(pat_idx) {
                    pat.name = name.clone();
                }
                // Sync to editor if it holds this pattern
                let editor_pat_idx = self.song.arrangement.get(arr_pos).copied().unwrap_or(0);
                if editor_pat_idx == pat_idx {
                    self.editor.pattern_mut().name = name;
                }
                self.mark_dirty();
            }
            return;
        }

        // :title <name> — set song title
        if parts[0] == "title" {
            let name = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                let current = if self.song.name.is_empty() {
                    "(untitled)".to_string()
                } else {
                    self.song.name.clone()
                };
                self.open_modal(Modal::info(
                    "Song Title".to_string(),
                    format!("Current: {}\nUsage: :title <name>", current),
                ));
            } else {
                self.song.name = name;
                self.mark_dirty();
            }
            return;
        }

        // :artist <name> — set song artist
        if parts[0] == "artist" {
            let name = parts
                .get(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                let current = if self.song.artist.is_empty() {
                    "(none)".to_string()
                } else {
                    self.song.artist.clone()
                };
                self.open_modal(Modal::info(
                    "Artist".to_string(),
                    format!("Current: {}\nUsage: :artist <name>", current),
                ));
            } else {
                self.song.artist = name;
                self.mark_dirty();
            }
            return;
        }

        // :dup — duplicate current/selected pattern into a new pattern slot
        if parts[0] == "dup" {
            let arr_pos = self.transport.arrangement_position();
            let pat_idx = self
                .pattern_selection()
                .unwrap_or_else(|| self.song.arrangement.get(arr_pos).copied().unwrap_or(0));
            // Flush editor changes back to song before cloning
            self.flush_editor_pattern(arr_pos);
            if let Some(new_idx) = self.song.duplicate_pattern(pat_idx) {
                // Name the clone after the original
                let src_name = self
                    .song
                    .patterns
                    .get(pat_idx)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                let clone_name = if src_name.is_empty() {
                    format!("Pattern {:02} copy", pat_idx + 1)
                } else {
                    format!("{} copy", src_name)
                };
                if let Some(p) = self.song.patterns.get_mut(new_idx) {
                    p.name = clone_name.clone();
                }
                self.mark_dirty();
                self.open_modal(Modal::info(
                    "Pattern Duplicated".to_string(),
                    format!("Created pattern {:02}: {}", new_idx + 1, clone_name),
                ));
            } else {
                self.open_modal(Modal::error(
                    "Duplicate Failed".to_string(),
                    "Pattern pool is full (max 256 patterns).".to_string(),
                ));
            }
            return;
        }

        // :rename <name> — rename current track (channel) in the pattern editor
        if parts[0] == "rename" {
            let name = parts[1..].join(" ").trim().to_string();
            if name.is_empty() {
                self.open_modal(Modal::error(
                    "Invalid name".to_string(),
                    "Usage: :rename <track name>".to_string(),
                ));
            } else {
                let ch = self.editor.cursor_channel();
                if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
                    track.name = name.clone();
                }
                if let Some(track) = self.song.tracks.get_mut(ch) {
                    track.name = name;
                }
                self.mark_dirty();
            }
            return;
        }

        // :marker <label> — add/update a section marker at the current arrangement cursor
        // :marker — without label, removes the marker at current arrangement cursor
        if parts[0] == "marker" || parts[0] == "mark" {
            if let Some(label) = parts.get(1).map(|s| s.trim().to_string()) {
                if label.is_empty() {
                    self.arrangement_remove_marker();
                    self.open_modal(Modal::info(
                        "Marker Removed".to_string(),
                        format!("Removed marker at position {}", self.arrangement_view.cursor()),
                    ));
                } else {
                    let pos = self.arrangement_view.cursor();
                    self.arrangement_add_marker(label.clone());
                    self.open_modal(Modal::info(
                        "Marker Added".to_string(),
                        format!("Added marker \"{}\" at position {}", label, pos),
                    ));
                }
            } else {
                // No arg: show current marker or usage
                let pos = self.arrangement_view.cursor();
                let info = if let Some(m) = self.song.section_marker_at(pos) {
                    format!("Current marker: \"{}\"\nUse :marker <label> to rename, :marker (empty) to remove.", m.label)
                } else {
                    format!("No marker at position {}.\nUsage: :marker <label>", pos)
                };
                self.open_modal(Modal::info("Section Marker".to_string(), info));
            }
            return;
        }

        // :keyzone list — list keyzones for the current instrument
        // :keyzone add <note_min> <note_max> <vel_min> <vel_max> <sample_idx> — add a keyzone
        // :keyzone del <index> — delete a keyzone by index
        // :keyzone clear — clear all keyzones for current instrument
        if parts[0] == "keyzone" || parts[0] == "kz" {
            let idx = self.instrument_selection().unwrap_or(0);
            // Re-split the full args from parts[1] if present
            let sub_parts: Vec<&str> = parts
                .get(1)
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            let sub = sub_parts.first().copied().unwrap_or("list");
            match sub {
                "list" | "ls" => {
                    if let Some(inst) = self.song.instruments.get(idx) {
                        if inst.keyzones.is_empty() {
                            self.open_modal(Modal::info(
                                format!("Keyzones: {}", inst.name),
                                format!("No keyzones (using sample_index: {:?})\nAdd with: :keyzone add <note_min> <note_max> <vel_min> <vel_max> <sample_idx>", inst.sample_index),
                            ));
                        } else {
                            let lines: Vec<String> = inst.keyzones.iter().enumerate().map(|(i, kz)| {
                                format!("  [{}] notes {:3}-{:3}  vel {:3}-{:3}  → sample {}", i, kz.note_min, kz.note_max, kz.velocity_min, kz.velocity_max, kz.sample_index)
                            }).collect();
                            self.open_modal(Modal::info(
                                format!("Keyzones: {} ({} zones)", inst.name, inst.keyzones.len()),
                                lines.join("\n"),
                            ));
                        }
                    }
                }
                "add" => {
                    let rest = parts.get(1).map(|s| s.trim()).unwrap_or("");
                    // Skip the "add" subcommand word to get the numeric arguments
                    let rest_after_add = rest.splitn(2, ' ').nth(1).unwrap_or("").trim();
                    let nums: Vec<u64> = rest_after_add
                        .split_whitespace()
                        .filter_map(|t| t.parse::<u64>().ok())
                        .collect();
                    if nums.len() >= 5 {
                        use riffl_core::song::Keyzone;
                        let kz = Keyzone {
                            note_min: (nums[0] as u8).min(119),
                            note_max: (nums[1] as u8).min(119),
                            velocity_min: (nums[2] as u8).min(127),
                            velocity_max: (nums[3] as u8).min(127),
                            sample_index: nums[4] as usize,
                            base_note_override: None,
                        };
                        if let Some(inst) = self.song.instruments.get_mut(idx) {
                            inst.keyzones.push(kz);
                            inst.keyzones.sort_by_key(|k| k.note_min);
                            self.sync_mixer_instruments();
                            self.mark_dirty();
                            self.open_modal(Modal::info(
                                "Keyzone Added".to_string(),
                                format!("notes {}-{}  vel {}-{}  → sample {}", nums[0], nums[1], nums[2], nums[3], nums[4]),
                            ));
                        }
                    } else {
                        self.open_modal(Modal::error(
                            "Invalid keyzone".to_string(),
                            "Usage: :keyzone add <note_min> <note_max> <vel_min> <vel_max> <sample_idx>\nMIDI notes 0-119, velocities 0-127".to_string(),
                        ));
                    }
                }
                "del" | "delete" | "remove" | "rm" => {
                    let zone_idx: Option<usize> = sub_parts.get(1).and_then(|s| s.trim().parse().ok());
                    if let Some(zi) = zone_idx {
                        if let Some(inst) = self.song.instruments.get_mut(idx) {
                            if zi < inst.keyzones.len() {
                                inst.keyzones.remove(zi);
                                self.sync_mixer_instruments();
                                self.mark_dirty();
                            }
                        }
                    } else {
                        self.open_modal(Modal::error(
                            "Invalid index".to_string(),
                            "Usage: :keyzone del <index>  (use :keyzone list to see indices)".to_string(),
                        ));
                    }
                }
                "clear" => {
                    if let Some(inst) = self.song.instruments.get_mut(idx) {
                        inst.keyzones.clear();
                        self.sync_mixer_instruments();
                        self.mark_dirty();
                    }
                }
                _ => {
                    self.open_modal(Modal::error(
                        "Unknown keyzone subcommand".to_string(),
                        "Usage: :keyzone <list|add|del|clear>".to_string(),
                    ));
                }
            }
            return;
        }

        // :automate vol <start_vol> <end_vol> — fill channel with volume slide effects
        // :automate pan <start_pan> <end_pan> — fill channel with pan effects
        // Generates a linear ramp of Cxx/8xx effect commands across the visual selection
        // or entire current channel if not in visual mode.
        if parts[0] == "automate" || parts[0] == "auto" {
            let sub_parts: Vec<&str> = parts
                .get(1)
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            match sub_parts.first().copied() {
                Some("vol") | Some("volume") => {
                    let start = sub_parts.get(1).and_then(|s| s.parse::<f64>().ok());
                    let end = sub_parts.get(2).and_then(|s| s.parse::<f64>().ok());
                    if let (Some(sv), Some(ev)) = (start, end) {
                        use riffl_core::pattern::effect::Effect;
                        use crate::editor::EditorMode;
                        let ch = self.editor.cursor_channel();
                        let num_rows = self.editor.pattern().num_rows();
                        let (r0, r1) = if self.editor.mode() == EditorMode::Visual {
                            self.editor.visual_selection()
                                .map(|((r0, _), (r1, _))| (r0, r1))
                                .unwrap_or((0, num_rows.saturating_sub(1)))
                        } else {
                            (0, num_rows.saturating_sub(1))
                        };
                        let span = (r1 - r0) as f64;
                        for row in r0..=r1 {
                            let t = if span > 0.0 { (row - r0) as f64 / span } else { 0.0 };
                            let vol = (sv + t * (ev - sv)).clamp(0.0, 255.0).round() as u8;
                            if let Some(cell) = self.editor.pattern_mut().get_cell_mut(row, ch) {
                                cell.set_effect(Effect::new(0xC, vol));
                            }
                        }
                        self.mark_dirty();
                        self.open_modal(Modal::info(
                            "Volume Automation".to_string(),
                            format!("Applied vol ramp {:.0}→{:.0} over rows {}-{} on channel {}", sv, ev, r0, r1, ch + 1),
                        ));
                    } else {
                        self.open_modal(Modal::error(
                            "Invalid automate".to_string(),
                            "Usage: :automate vol <start_vol> <end_vol>  (0-255)\nApplies Cxx volume commands across the selection or current channel.".to_string(),
                        ));
                    }
                }
                Some("pan") | Some("panning") => {
                    let start = sub_parts.get(1).and_then(|s| s.parse::<f64>().ok());
                    let end = sub_parts.get(2).and_then(|s| s.parse::<f64>().ok());
                    if let (Some(sp), Some(ep)) = (start, end) {
                        use riffl_core::pattern::effect::Effect;
                        use crate::editor::EditorMode;
                        let ch = self.editor.cursor_channel();
                        let num_rows = self.editor.pattern().num_rows();
                        let (r0, r1) = if self.editor.mode() == EditorMode::Visual {
                            self.editor.visual_selection()
                                .map(|((r0, _), (r1, _))| (r0, r1))
                                .unwrap_or((0, num_rows.saturating_sub(1)))
                        } else {
                            (0, num_rows.saturating_sub(1))
                        };
                        let span = (r1 - r0) as f64;
                        for row in r0..=r1 {
                            let t = if span > 0.0 { (row - r0) as f64 / span } else { 0.0 };
                            let pan = (sp + t * (ep - sp)).clamp(0.0, 255.0).round() as u8;
                            if let Some(cell) = self.editor.pattern_mut().get_cell_mut(row, ch) {
                                cell.set_effect(Effect::new(0x8, pan));
                            }
                        }
                        self.mark_dirty();
                        self.open_modal(Modal::info(
                            "Pan Automation".to_string(),
                            format!("Applied pan ramp {:.0}→{:.0} over rows {}-{} on channel {}", sp, ep, r0, r1, ch + 1),
                        ));
                    } else {
                        self.open_modal(Modal::error(
                            "Invalid automate".to_string(),
                            "Usage: :automate pan <start_pan> <end_pan>  (0-255)\nApplies 8xx pan commands across the selection or current channel.".to_string(),
                        ));
                    }
                }
                _ => {
                    self.open_modal(Modal::error(
                        "Unknown automate subcommand".to_string(),
                        "Usage: :automate <vol|pan> <start> <end>".to_string(),
                    ));
                }
            }
            return;
        }

        // :instruments / :insts — list all instruments with their sample assignments
        if parts[0] == "instruments" || parts[0] == "insts" || parts[0] == "inst" {
            if self.song.instruments.is_empty() {
                self.open_modal(Modal::info("Instruments".to_string(), "(none loaded)".to_string()));
            } else {
                let lines: Vec<String> = self.song.instruments.iter().enumerate().map(|(i, inst)| {
                    let sample_info = if inst.keyzones.is_empty() {
                        inst.sample_index.map(|si| format!("→ sample {}", si))
                            .unwrap_or_else(|| "(no sample)".to_string())
                    } else {
                        format!("{} keyzones", inst.keyzones.len())
                    };
                    let vol_info = if (inst.volume - 1.0).abs() > 0.01 {
                        format!(" vol:{:.0}%", inst.volume * 100.0)
                    } else {
                        String::new()
                    };
                    format!("  [{:02X}] {:<20} {}{}", i, inst.name, sample_info, vol_info)
                }).collect();
                self.open_modal(Modal::info(
                    format!("Instruments ({} total)", self.song.instruments.len()),
                    lines.join("\n"),
                ));
            }
            return;
        }

        // :countin N — set count-in bars (0 to disable, N bars before playback)
        if parts[0] == "countin" || parts[0] == "count-in" {
            if let Some(bars) = parts.get(1).and_then(|s| s.trim().parse::<u8>().ok()) {
                self.transport.count_in_bars = bars;
                if bars == 0 {
                    self.open_modal(Modal::info(
                        "Count-in".to_string(),
                        "Count-in disabled.".to_string(),
                    ));
                } else {
                    self.open_modal(Modal::info(
                        "Count-in".to_string(),
                        format!(
                            "Count-in set to {} bar{}. Start playback to activate.",
                            bars,
                            if bars == 1 { "" } else { "s" }
                        ),
                    ));
                }
            } else {
                let current = self.transport.count_in_bars;
                self.open_modal(Modal::info(
                    "Count-in".to_string(),
                    format!(
                        "Current: {} bar{}\nUsage: :countin <0-8>",
                        current,
                        if current == 1 { "" } else { "s" }
                    ),
                ));
            }
            return;
        }

        // :metronome on|off|toggle|vol <n> — control metronome
        if parts[0] == "metronome" || parts[0] == "metro" {
            let sub = parts.get(1).map(|s| s.trim()).unwrap_or("toggle");
            match sub {
                "on" => self.set_metronome_enabled(true),
                "off" => self.set_metronome_enabled(false),
                "toggle" => self.toggle_metronome(),
                s if s.starts_with("vol")
                    || s.chars().next().map_or(false, |c| c.is_ascii_digit()) =>
                {
                    let vol_str = if s.starts_with("vol") {
                        parts.get(1).and_then(|t| t.split_whitespace().nth(1)).unwrap_or("")
                    } else {
                        s
                    };
                    if let Ok(vol) = vol_str.trim().parse::<f32>() {
                        let clamped = (vol / 100.0).clamp(0.0, 1.0);
                        self.set_metronome_volume(clamped);
                    }
                }
                _ => {
                    let enabled = self.metronome_enabled();
                    let vol = self.metronome_volume();
                    self.open_modal(Modal::info(
                        "Metronome".to_string(),
                        format!(
                            "Status: {}\nVolume: {:.0}%\nUsage: :metronome <on|off|toggle|vol <0-100>>",
                            if enabled { "ON" } else { "OFF" },
                            vol * 100.0,
                        ),
                    ));
                }
            }
            return;
        }

        // :filter [ch] <lpf|hpf|off> [cutoff_hz] — apply per-channel filter
        if parts[0] == "filter" || parts[0] == "flt" {
            let rest = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();
            let tokens: Vec<&str> = rest.split_whitespace().collect();

            // Parse optional channel number (1-based), filter type, cutoff
            let (ch, type_token, cutoff_token) =
                if tokens.first().and_then(|t| t.parse::<usize>().ok()).is_some() {
                    let ch = tokens[0].parse::<usize>().unwrap_or(1).saturating_sub(1);
                    (ch, tokens.get(1).copied().unwrap_or("off"), tokens.get(2).copied())
                } else {
                    (
                        self.editor.cursor_channel(),
                        tokens.first().copied().unwrap_or("off"),
                        tokens.get(1).copied(),
                    )
                };

            match type_token {
                "off" | "bypass" | "none" => {
                    self.set_channel_filter(ch, None, riffl_core::pattern::track::FilterType::LowPass);
                    self.open_modal(Modal::info(
                        "Filter".to_string(),
                        format!("Channel {} filter disabled.", ch + 1),
                    ));
                }
                t @ ("lpf" | "lowpass" | "hpf" | "highpass") => {
                    let is_hpf = matches!(t, "hpf" | "highpass");
                    let cutoff = cutoff_token
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(1000.0)
                        .clamp(20.0, 20000.0);
                    let ftype = if is_hpf {
                        riffl_core::pattern::track::FilterType::HighPass
                    } else {
                        riffl_core::pattern::track::FilterType::LowPass
                    };
                    self.set_channel_filter(ch, Some(cutoff), ftype);
                    self.open_modal(Modal::info(
                        "Filter".to_string(),
                        format!(
                            "Channel {} {} @ {:.0}Hz",
                            ch + 1,
                            if is_hpf { "HPF" } else { "LPF" },
                            cutoff
                        ),
                    ));
                }
                _ => {
                    self.open_modal(Modal::error(
                        "Filter".to_string(),
                        "Usage: :filter [channel] <lpf|hpf|off> [cutoff_hz]\nExample: :filter lpf 800  :filter 3 hpf 200  :filter off".to_string(),
                    ));
                }
            }
            return;
        }

        // :<number> — jump to specific row
        if let Ok(row) = cmd.parse::<usize>() {
            self.editor.go_to_row(row.saturating_sub(1));
            return;
        }

        // :goto N / :g N — jump to specific row (1-based)
        if parts[0] == "goto" || (parts[0] == "g" && parts.len() == 2) {
            if let Some(row) = parts.get(1).and_then(|s| s.trim().parse::<usize>().ok()) {
                self.editor.go_to_row(row.saturating_sub(1));
            } else {
                let current = self.editor.cursor_row() + 1;
                self.open_modal(Modal::info(
                    "Go to Row".to_string(),
                    format!("Current: row {}. Usage: :goto <row>", current),
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
