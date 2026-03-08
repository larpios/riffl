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
                    let adsr = Adsr::new(a.max(0.0), d.max(0.0), (s_pct / 100.0).clamp(0.0, 1.0), r.max(0.0));
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
                        format!("A:{} D:{} S:{:.0}% R:{}", a.attack as i32, a.decay as i32, a.sustain * 100.0, a.release as i32)
                    } else {
                        "(none — using point envelope)".to_string()
                    }
                } else {
                    "(no instrument)".to_string()
                };
                self.open_modal(Modal::info(
                    "ADSR Envelope".to_string(),
                    format!("{}\nUsage: :adsr <attack_ms> <decay_ms> <sustain%> <release_ms>", info),
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
                        format!(
                            "Current: {}\nUsage: :mode <native|compat|amiga>",
                            current
                        ),
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
                        format!(
                            "Channels: {}\nUsage: :track <add|del>",
                            channels
                        ),
                    ));
                }
            }
            return;
        }

        // :pname <name> — rename the selected or currently-edited pattern
        if parts[0] == "pname" {
            let arr_pos = self.transport.arrangement_position();
            let pat_idx = self.pattern_selection().unwrap_or_else(|| {
                self.song.arrangement.get(arr_pos).copied().unwrap_or(0)
            });
            let name = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
            if name.is_empty() {
                let current = self.song.patterns.get(pat_idx)
                    .map(|p| if p.name.is_empty() { "(unnamed)".to_string() } else { p.name.clone() })
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
            let name = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
            if name.is_empty() {
                let current = if self.song.name.is_empty() { "(untitled)".to_string() } else { self.song.name.clone() };
                self.open_modal(Modal::info("Song Title".to_string(), format!("Current: {}\nUsage: :title <name>", current)));
            } else {
                self.song.name = name;
                self.mark_dirty();
            }
            return;
        }

        // :artist <name> — set song artist
        if parts[0] == "artist" {
            let name = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
            if name.is_empty() {
                let current = if self.song.artist.is_empty() { "(none)".to_string() } else { self.song.artist.clone() };
                self.open_modal(Modal::info("Artist".to_string(), format!("Current: {}\nUsage: :artist <name>", current)));
            } else {
                self.song.artist = name;
                self.mark_dirty();
            }
            return;
        }

        // :dup — duplicate current/selected pattern into a new pattern slot
        if parts[0] == "dup" {
            let arr_pos = self.transport.arrangement_position();
            let pat_idx = self.pattern_selection().unwrap_or_else(|| {
                self.song.arrangement.get(arr_pos).copied().unwrap_or(0)
            });
            // Flush editor changes back to song before cloning
            self.flush_editor_pattern(arr_pos);
            if let Some(new_idx) = self.song.duplicate_pattern(pat_idx) {
                // Name the clone after the original
                let src_name = self.song.patterns.get(pat_idx)
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
