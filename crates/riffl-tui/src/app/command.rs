use crate::registry::{Command, CommandRegistry};
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

        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        let args = parts.get(1).copied().unwrap_or("").trim();

        // :<number> — jump to a specific row (1-based)
        if let Ok(row) = cmd.parse::<usize>() {
            self.editor.go_to_row(row.saturating_sub(1));
            return;
        }

        match CommandRegistry::find_command(parts[0]) {
            None => {
                self.open_modal(Modal::error(
                    "Unknown command".to_string(),
                    format!(":{}", cmd),
                ));
            }

            Some(Command::Bpm) => {
                if let Some(val) = args.parse::<f64>().ok().filter(|_| !args.is_empty()) {
                    self.set_bpm(val.clamp(20.0, 999.0));
                    self.mark_dirty();
                } else {
                    self.open_modal(Modal::error(
                        "Invalid BPM".to_string(),
                        format!("Usage: :bpm <value>  (got: {:?})", parts.get(1)),
                    ));
                }
            }

            Some(Command::Step) => {
                if let Some(val) = args.parse::<usize>().ok().filter(|_| !args.is_empty()) {
                    self.editor.set_step_size(val);
                } else {
                    self.open_modal(Modal::error(
                        "Invalid step".to_string(),
                        "Usage: :step <0-8>".to_string(),
                    ));
                }
            }

            // :w [filename] / :save [filename]
            Some(Command::Write) | Some(Command::Save) => {
                if args.is_empty() {
                    self.save_project();
                } else {
                    let path = PathBuf::from(args);
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
                            self.open_modal(Modal::error(
                                "Save Failed".to_string(),
                                format!("{}", e),
                            ));
                        }
                    }
                }
            }

            // :e <filename> / :load <filename>
            Some(Command::Edit) | Some(Command::Load) => {
                if args.is_empty() {
                    self.open_modal(Modal::error(
                        "Load".to_string(),
                        "Usage: :e <filename>".to_string(),
                    ));
                } else {
                    self.load_project(&PathBuf::from(args));
                }
            }

            Some(Command::Quit) => self.quit(),

            Some(Command::ForceQuit) => self.force_quit(),

            Some(Command::SaveAndQuit) => {
                self.save_project();
                if !self.is_dirty {
                    self.force_quit();
                }
            }

            Some(Command::Tutor) => {
                self.show_tutor = true;
                self.tutor_scroll = 0;
            }

            Some(Command::Title) => {
                if args.is_empty() {
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
                    self.song.name = args.to_string();
                    self.mark_dirty();
                }
            }

            Some(Command::Artist) => {
                if args.is_empty() {
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
                    self.song.artist = args.to_string();
                    self.mark_dirty();
                }
            }

            Some(Command::Volume) => {
                if let Some(val) = args.parse::<f64>().ok().filter(|_| !args.is_empty()) {
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
            }

            Some(Command::Transpose) => {
                if let Some(val) = args.parse::<i32>().ok().filter(|_| !args.is_empty()) {
                    self.editor.transpose_selection(val);
                    self.mark_dirty();
                } else {
                    self.open_modal(Modal::error(
                        "Invalid transpose".to_string(),
                        "Usage: :transpose <semitones>  (e.g. :transpose -12)".to_string(),
                    ));
                }
            }

            Some(Command::Quantize) => {
                self.editor.quantize();
                self.mark_dirty();
            }

            Some(Command::Interpolate) => {
                self.editor.interpolate();
                self.mark_dirty();
            }

            Some(Command::Clear) => {
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
            }

            Some(Command::Len) => {
                if let Some(n) = args.parse::<usize>().ok().filter(|_| !args.is_empty()) {
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
            }

            Some(Command::Speed) => {
                if let Some(val) = args.parse::<u32>().ok().filter(|_| !args.is_empty()) {
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
            }

            Some(Command::Lpb) => {
                if let Some(val) = args.parse::<u32>().ok().filter(|_| !args.is_empty()) {
                    let clamped = val.clamp(1, 255);
                    self.transport.set_lpb(clamped);
                    self.song.lpb = clamped;
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.set_metronome_lpb(clamped);
                    }
                    self.mark_dirty();
                } else {
                    let current = self.song.lpb;
                    self.open_modal(Modal::info(
                        "Lines Per Beat".to_string(),
                        format!("Current LPB: {}\nUsage: :lpb <1-255>", current),
                    ));
                }
            }

            Some(Command::Loop) => {
                let nums: Vec<usize> = args
                    .split_whitespace()
                    .filter_map(|t| t.parse::<usize>().ok())
                    .collect();
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
            }

            Some(Command::CountIn) => {
                if let Some(bars) = args.parse::<u8>().ok().filter(|_| !args.is_empty()) {
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
            }

            Some(Command::Metronome) => {
                let sub = if args.is_empty() { "toggle" } else { args };
                match sub {
                    "on" => self.set_metronome_enabled(true),
                    "off" => self.set_metronome_enabled(false),
                    "toggle" => self.toggle_metronome(),
                    s if s.starts_with("vol")
                        || s.chars().next().is_some_and(|c| c.is_ascii_digit()) =>
                    {
                        let vol_str = if s.starts_with("vol") {
                            s.split_whitespace().nth(1).unwrap_or("")
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
            }

            Some(Command::Fill) => {
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
            }

            Some(Command::Adsr) => {
                let adsr_args: Vec<&str> = args.split_whitespace().collect();
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
                            "Usage: :adsr <attack_ms> <decay_ms> <sustain%> <release_ms>"
                                .to_string(),
                        ));
                    }
                } else {
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
            }

            Some(Command::Mode) => {
                use riffl_core::pattern::effect::EffectMode;
                match args {
                    "native" | "riffl" => {
                        self.song.effect_mode = EffectMode::RifflNative;
                        if let Ok(mut mixer) = self.mixer.lock() {
                            mixer.set_effect_mode(EffectMode::RifflNative);
                        }
                        self.mark_dirty();
                    }
                    "compat" | "compatible" | "it" | "xm" => {
                        self.song.effect_mode = EffectMode::Compatible;
                        if let Ok(mut mixer) = self.mixer.lock() {
                            mixer.set_effect_mode(EffectMode::Compatible);
                        }
                        self.mark_dirty();
                    }
                    "amiga" | "pt" | "mod" => {
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
            }

            Some(Command::Track) => match args {
                "add" | "new" => {
                    self.editor.add_track();
                    self.sync_mixer_channels();
                    self.mark_dirty();
                }
                "del" | "delete" | "remove" | "rm" => {
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
            },

            Some(Command::Pname) => {
                let arr_pos = self.transport.arrangement_position();
                let pat_idx = self
                    .pattern_selection()
                    .unwrap_or_else(|| self.song.arrangement.get(arr_pos).copied().unwrap_or(0));
                if args.is_empty() {
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
                        pat.name = args.to_string();
                    }
                    let editor_pat_idx = self.song.arrangement.get(arr_pos).copied().unwrap_or(0);
                    if editor_pat_idx == pat_idx {
                        self.editor.pattern_mut().name = args.to_string();
                    }
                    self.mark_dirty();
                }
            }

            Some(Command::Dup) => {
                let arr_pos = self.transport.arrangement_position();
                let pat_idx = self
                    .pattern_selection()
                    .unwrap_or_else(|| self.song.arrangement.get(arr_pos).copied().unwrap_or(0));
                self.flush_editor_pattern(arr_pos);
                if let Some(new_idx) = self.song.duplicate_pattern(pat_idx) {
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
            }

            Some(Command::Rename) => {
                if args.is_empty() {
                    self.open_modal(Modal::error(
                        "Invalid name".to_string(),
                        "Usage: :rename <track name>".to_string(),
                    ));
                } else {
                    let ch = self.editor.cursor_channel();
                    if let Some(track) = self.editor.pattern_mut().get_track_mut(ch) {
                        track.name = args.to_string();
                    }
                    if let Some(track) = self.song.tracks.get_mut(ch) {
                        track.name = args.to_string();
                    }
                    self.mark_dirty();
                }
            }

            Some(Command::Marker) => {
                if args.is_empty() {
                    let pos = self.arrangement_view.cursor();
                    let info = if let Some(m) = self.song.section_marker_at(pos) {
                        format!(
                            "Current marker: \"{}\"\nUse :marker <label> to rename, :marker (empty) to remove.",
                            m.label
                        )
                    } else {
                        format!("No marker at position {}.\nUsage: :marker <label>", pos)
                    };
                    self.open_modal(Modal::info("Section Marker".to_string(), info));
                } else if args.trim().is_empty() {
                    self.arrangement_remove_marker();
                    self.open_modal(Modal::info(
                        "Marker Removed".to_string(),
                        format!(
                            "Removed marker at position {}",
                            self.arrangement_view.cursor()
                        ),
                    ));
                } else {
                    let label = args.to_string();
                    let pos = self.arrangement_view.cursor();
                    self.arrangement_add_marker(label.clone());
                    self.open_modal(Modal::info(
                        "Marker Added".to_string(),
                        format!("Added marker \"{}\" at position {}", label, pos),
                    ));
                }
            }

            Some(Command::Keyzone) => {
                let idx = self.instrument_selection().unwrap_or(0);
                let sub_parts: Vec<&str> = args.split_whitespace().collect();
                let sub = sub_parts.first().copied().unwrap_or("list");
                match sub {
                    "list" | "ls" => {
                        if let Some(inst) = self.song.instruments.get(idx) {
                            if inst.keyzones.is_empty() {
                                self.open_modal(Modal::info(
                                    format!("Keyzones: {}", inst.name),
                                    format!(
                                        "No keyzones (using sample_index: {:?})\nAdd with: :keyzone add <note_min> <note_max> <vel_min> <vel_max> <sample_idx>",
                                        inst.sample_index
                                    ),
                                ));
                            } else {
                                let lines: Vec<String> = inst
                                    .keyzones
                                    .iter()
                                    .enumerate()
                                    .map(|(i, kz)| {
                                        format!(
                                            "  [{}] notes {:3}-{:3}  vel {:3}-{:3}  → sample {}",
                                            i,
                                            kz.note_min,
                                            kz.note_max,
                                            kz.velocity_min,
                                            kz.velocity_max,
                                            kz.sample_index
                                        )
                                    })
                                    .collect();
                                self.open_modal(Modal::info(
                                    format!(
                                        "Keyzones: {} ({} zones)",
                                        inst.name,
                                        inst.keyzones.len()
                                    ),
                                    lines.join("\n"),
                                ));
                            }
                        }
                    }
                    "add" => {
                        // sub_parts[1..] are the numeric arguments
                        let nums: Vec<u64> = sub_parts[1..]
                            .iter()
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
                                    format!(
                                        "notes {}-{}  vel {}-{}  → sample {}",
                                        nums[0], nums[1], nums[2], nums[3], nums[4]
                                    ),
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
                        let zone_idx: Option<usize> =
                            sub_parts.get(1).and_then(|s| s.trim().parse().ok());
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
                                "Usage: :keyzone del <index>  (use :keyzone list to see indices)"
                                    .to_string(),
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
            }

            Some(Command::Automate) => {
                let sub_parts: Vec<&str> = args.split_whitespace().collect();
                match sub_parts.first().copied() {
                    Some("vol") | Some("volume") => {
                        let start = sub_parts.get(1).and_then(|s| s.parse::<f64>().ok());
                        let end = sub_parts.get(2).and_then(|s| s.parse::<f64>().ok());
                        if let (Some(sv), Some(ev)) = (start, end) {
                            use crate::editor::EditorMode;
                            use riffl_core::pattern::effect::Effect;
                            let ch = self.editor.cursor_channel();
                            let num_rows = self.editor.pattern().num_rows();
                            let (r0, r1) = if self.editor.mode() == EditorMode::Visual {
                                self.editor
                                    .visual_selection()
                                    .map(|((r0, _), (r1, _))| (r0, r1))
                                    .unwrap_or((0, num_rows.saturating_sub(1)))
                            } else {
                                (0, num_rows.saturating_sub(1))
                            };
                            let span = (r1 - r0) as f64;
                            for row in r0..=r1 {
                                let t = if span > 0.0 {
                                    (row - r0) as f64 / span
                                } else {
                                    0.0
                                };
                                let vol = (sv + t * (ev - sv)).clamp(0.0, 255.0).round() as u8;
                                if let Some(cell) = self.editor.pattern_mut().get_cell_mut(row, ch)
                                {
                                    cell.set_effect(Effect::new(0xC, vol));
                                }
                            }
                            self.mark_dirty();
                            self.open_modal(Modal::info(
                                "Volume Automation".to_string(),
                                format!(
                                    "Applied vol ramp {:.0}→{:.0} over rows {}-{} on channel {}",
                                    sv,
                                    ev,
                                    r0,
                                    r1,
                                    ch + 1
                                ),
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
                            use crate::editor::EditorMode;
                            use riffl_core::pattern::effect::Effect;
                            let ch = self.editor.cursor_channel();
                            let num_rows = self.editor.pattern().num_rows();
                            let (r0, r1) = if self.editor.mode() == EditorMode::Visual {
                                self.editor
                                    .visual_selection()
                                    .map(|((r0, _), (r1, _))| (r0, r1))
                                    .unwrap_or((0, num_rows.saturating_sub(1)))
                            } else {
                                (0, num_rows.saturating_sub(1))
                            };
                            let span = (r1 - r0) as f64;
                            for row in r0..=r1 {
                                let t = if span > 0.0 {
                                    (row - r0) as f64 / span
                                } else {
                                    0.0
                                };
                                let pan = (sp + t * (ep - sp)).clamp(0.0, 255.0).round() as u8;
                                if let Some(cell) = self.editor.pattern_mut().get_cell_mut(row, ch)
                                {
                                    cell.set_effect(Effect::new(0x8, pan));
                                }
                            }
                            self.mark_dirty();
                            self.open_modal(Modal::info(
                                "Pan Automation".to_string(),
                                format!(
                                    "Applied pan ramp {:.0}→{:.0} over rows {}-{} on channel {}",
                                    sp,
                                    ep,
                                    r0,
                                    r1,
                                    ch + 1
                                ),
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
            }

            Some(Command::Instruments) => {
                if self.song.instruments.is_empty() {
                    self.open_modal(Modal::info(
                        "Instruments".to_string(),
                        "(none loaded)".to_string(),
                    ));
                } else {
                    let lines: Vec<String> = self
                        .song
                        .instruments
                        .iter()
                        .enumerate()
                        .map(|(i, inst)| {
                            let sample_info = if inst.keyzones.is_empty() {
                                inst.sample_index
                                    .map(|si| format!("→ sample {}", si))
                                    .unwrap_or_else(|| "(no sample)".to_string())
                            } else {
                                format!("{} keyzones", inst.keyzones.len())
                            };
                            let vol_info = if (inst.volume - 1.0).abs() > 0.01 {
                                format!(" vol:{:.0}%", inst.volume * 100.0)
                            } else {
                                String::new()
                            };
                            format!(
                                "  [{:02X}] {:<20} {}{}",
                                i, inst.name, sample_info, vol_info
                            )
                        })
                        .collect();
                    self.open_modal(Modal::info(
                        format!("Instruments ({} total)", self.song.instruments.len()),
                        lines.join("\n"),
                    ));
                }
            }

            Some(Command::Filter) => {
                let tokens: Vec<&str> = args.split_whitespace().collect();
                let (ch, type_token, cutoff_token) = if tokens
                    .first()
                    .and_then(|t| t.parse::<usize>().ok())
                    .is_some()
                {
                    let ch = tokens[0].parse::<usize>().unwrap_or(1).saturating_sub(1);
                    (
                        ch,
                        tokens.get(1).copied().unwrap_or("off"),
                        tokens.get(2).copied(),
                    )
                } else {
                    (
                        self.editor.cursor_channel(),
                        tokens.first().copied().unwrap_or("off"),
                        tokens.get(1).copied(),
                    )
                };

                match type_token {
                    "off" | "bypass" | "none" => {
                        self.set_channel_filter(
                            ch,
                            None,
                            riffl_core::pattern::track::FilterType::LowPass,
                        );
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
            }

            Some(Command::Goto) => {
                if let Some(row) = args.parse::<usize>().ok().filter(|_| !args.is_empty()) {
                    self.editor.go_to_row(row.saturating_sub(1));
                } else {
                    let current = self.editor.cursor_row() + 1;
                    self.open_modal(Modal::info(
                        "Go to Row".to_string(),
                        format!("Current: row {}. Usage: :goto <row>", current),
                    ));
                }
            }

            Some(Command::Samples) => {
                let sub = if args.is_empty() { "list" } else { args };
                match sub {
                    "list" | "ls" => {
                        let loaded = self.loaded_samples();
                        if loaded.is_empty() {
                            self.open_modal(Modal::info(
                                "Samples".to_string(),
                                "(no samples loaded)".to_string(),
                            ));
                        } else {
                            let lines: Vec<String> = loaded
                                .iter()
                                .enumerate()
                                .map(|(i, s)| {
                                    let name = s.name().unwrap_or("(unnamed)");
                                    let frames = s.frame_count();
                                    let sr = s.sample_rate();
                                    let channels = s.channels();
                                    let duration_ms = if sr > 0 {
                                        frames * 1000 / sr as usize
                                    } else {
                                        0
                                    };
                                    let loop_info = match s.loop_mode {
                                        riffl_core::audio::sample::LoopMode::NoLoop => "",
                                        riffl_core::audio::sample::LoopMode::Forward => " [loop]",
                                        riffl_core::audio::sample::LoopMode::PingPong => {
                                            " [ping-pong]"
                                        }
                                    };
                                    format!(
                                        "  [{:02X}] {:<22} {:5}ms  {:2}ch  {}Hz{}",
                                        i, name, duration_ms, channels, sr, loop_info
                                    )
                                })
                                .collect();
                            self.open_modal(Modal::info(
                                format!("Samples ({} loaded)", loaded.len()),
                                lines.join("\n"),
                            ));
                        }
                    }
                    "dedup" => {
                        let loaded = self.loaded_samples();
                        let mut seen: std::collections::HashMap<String, Vec<usize>> =
                            std::collections::HashMap::new();
                        for (i, s) in loaded.iter().enumerate() {
                            let name = s
                                .name()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| format!("sample_{}", i));
                            seen.entry(name).or_default().push(i);
                        }
                        let dups: Vec<String> = seen
                            .into_iter()
                            .filter(|(_, indices)| indices.len() > 1)
                            .map(|(name, indices)| {
                                let idxs: Vec<String> =
                                    indices.iter().map(|i| format!("{:02X}", i)).collect();
                                format!("  {} → [{}]", name, idxs.join(", "))
                            })
                            .collect();
                        if dups.is_empty() {
                            self.open_modal(Modal::info(
                                "Sample Dedup".to_string(),
                                "No duplicate samples found.".to_string(),
                            ));
                        } else {
                            self.open_modal(Modal::info(
                                format!("Duplicate Samples ({} groups)", dups.len()),
                                dups.join("\n"),
                            ));
                        }
                    }
                    _ => {
                        self.open_modal(Modal::error(
                            "Samples".to_string(),
                            "Usage: :samples <list|dedup>".to_string(),
                        ));
                    }
                }
            }

            Some(Command::Effects) => {
                let specific = if args.is_empty() {
                    None
                } else {
                    Some(args.to_uppercase())
                };
                let reference = vec![
                    (
                        "0xx",
                        "Arpeggio: cycle between base, +x, +y semitones per tick",
                    ),
                    ("1xx", "Portamento up: slide pitch up by xx units per tick"),
                    (
                        "2xx",
                        "Portamento down: slide pitch down by xx units per tick",
                    ),
                    ("3xx", "Tone portamento: slide to target note, speed xx"),
                    ("4xy", "Vibrato: x=speed, y=depth (sine LFO on pitch)"),
                    (
                        "5xy",
                        "Tone portamento + volume slide (x=vol up, y=vol down)",
                    ),
                    ("6xy", "Vibrato + volume slide"),
                    ("7xy", "Tremolo: x=speed, y=depth (sine LFO on volume)"),
                    ("8xx", "Set panning: 00=left, 80=center, FF=right"),
                    ("9xx", "Sample offset: start playback at frame xx*256"),
                    ("Axy", "Volume slide: x=up, y=down (per tick)"),
                    ("Bxx", "Position jump: jump to arrangement position xx"),
                    ("Cxx", "Set volume: 00-40 (raw) or 00-FF (native mode)"),
                    ("Dxx", "Pattern break: end pattern, start next at row xx"),
                    ("E0x", "Set filter (Amiga mode)"),
                    ("E1x", "Fine portamento up by x units"),
                    ("E2x", "Fine portamento down by x units"),
                    ("E3x", "Glissando: round portamento to semitones (on/off)"),
                    ("E4x", "Set vibrato waveform: 0=sine, 1=ramp, 2=square"),
                    ("E5x", "Set finetune: -8 to +7 semitone units"),
                    ("E6x", "Pattern loop: E60=set start, E6x=loop x times"),
                    ("E7x", "Set tremolo waveform: 0=sine, 1=ramp, 2=square"),
                    ("E8x", "Set panning (coarse): 0=left, 8=center, F=right"),
                    ("E9x", "Retrigger: retrigger note every x ticks"),
                    ("EAx", "Fine volume slide up by x"),
                    ("EBx", "Fine volume slide down by x"),
                    ("ECx", "Note cut after x ticks"),
                    ("EDx", "Note delay: trigger note x ticks late"),
                    ("EEx", "Pattern delay: pause pattern for x rows"),
                    ("Fxx", "Set speed: F01-F1F sets TPL, F20+ sets BPM"),
                    ("Gxx", "Set global volume: 00-40"),
                    ("Hxy", "Global volume slide: x=up, y=down"),
                    ("Kxx", "Key off after xx ticks"),
                    ("Lxx", "Set envelope position to tick xx"),
                    ("Pxy", "Panning slide: x=right, y=left"),
                    ("Rxy", "Multi-retrigger with volume action y every x ticks"),
                    ("Txx", "Set BPM (20-FF)"),
                    ("Xxx", "Extra fine portamento up"),
                    ("Yxx", "Extra fine portamento down"),
                    ("Zxx", "Script trigger: run Rhai script with param xx"),
                ];
                if let Some(search) = specific {
                    let matches: Vec<&(&str, &str)> = reference
                        .iter()
                        .filter(|(cmd, _)| cmd.to_uppercase().starts_with(&search))
                        .collect();
                    if matches.is_empty() {
                        self.open_modal(Modal::error(
                            "Effect Not Found".to_string(),
                            format!("No effect matching '{}'. Use :effects to list all.", search),
                        ));
                    } else {
                        let lines: Vec<String> = matches
                            .iter()
                            .map(|(cmd, desc)| format!("  {:4} — {}", cmd, desc))
                            .collect();
                        self.open_modal(Modal::info(
                            format!("Effect: {}", search),
                            lines.join("\n"),
                        ));
                    }
                } else {
                    let lines: Vec<String> = reference
                        .iter()
                        .map(|(cmd, desc)| format!("  {:4} — {}", cmd, desc))
                        .collect();
                    self.open_modal(Modal::info(
                        "Effect Command Reference".to_string(),
                        lines.join("\n"),
                    ));
                }
            }

            Some(Command::Reverse) => {
                use crate::editor::EditorMode;
                let ch = self.editor.cursor_channel();
                let num_rows = self.editor.pattern().num_rows();
                let ((r0, c0), (r1, c1)) = if self.editor.mode() == EditorMode::Visual {
                    self.editor
                        .visual_selection()
                        .unwrap_or(((0, ch), (num_rows.saturating_sub(1), ch)))
                } else {
                    ((0, ch), (num_rows.saturating_sub(1), ch))
                };
                let num = r1.saturating_sub(r0) + 1;
                if num < 2 {
                    return;
                }
                let pat = self.editor.pattern_mut();
                for col in c0..=c1 {
                    let mut cells: Vec<_> = (r0..=r1)
                        .map(|r| {
                            pat.get_cell(r, col)
                                .cloned()
                                .unwrap_or_else(riffl_core::pattern::row::Cell::empty)
                        })
                        .collect();
                    cells.reverse();
                    for (i, row) in (r0..=r1).enumerate() {
                        pat.set_cell(row, col, cells[i].clone());
                    }
                }
                self.mark_dirty();
            }

            Some(Command::Expand) => {
                if let Some(n) = args.parse::<usize>().ok().filter(|_| !args.is_empty()) {
                    let n = n.clamp(2, 8);
                    use riffl_core::pattern::pattern::MAX_ROW_COUNT;
                    let cur_rows = self.editor.pattern().num_rows();
                    let new_rows = (cur_rows * n).min(MAX_ROW_COUNT);
                    let channels = self.editor.pattern().num_channels();
                    let old_cells: Vec<Vec<riffl_core::pattern::row::Cell>> = (0..cur_rows)
                        .map(|r| {
                            (0..channels)
                                .map(|c| {
                                    self.editor
                                        .pattern()
                                        .get_cell(r, c)
                                        .cloned()
                                        .unwrap_or_else(riffl_core::pattern::row::Cell::empty)
                                })
                                .collect()
                        })
                        .collect();
                    self.editor.pattern_mut().set_row_count(new_rows);
                    self.transport.set_num_rows(new_rows);
                    for r in 0..new_rows {
                        for c in 0..channels {
                            self.editor.pattern_mut().clear_cell(r, c);
                        }
                    }
                    for (orig_r, row_cells) in old_cells.iter().enumerate() {
                        let new_r = orig_r * n;
                        if new_r >= new_rows {
                            break;
                        }
                        for (c, cell) in row_cells.iter().enumerate() {
                            self.editor.pattern_mut().set_cell(new_r, c, cell.clone());
                        }
                    }
                    let pos = self.transport.arrangement_position();
                    self.flush_editor_pattern(pos);
                    self.mark_dirty();
                    self.open_modal(Modal::info(
                        "Expand Pattern".to_string(),
                        format!("Expanded {}→{} rows (factor {})", cur_rows, new_rows, n),
                    ));
                } else {
                    self.open_modal(Modal::error(
                        "Expand".to_string(),
                        "Usage: :expand <2-8>  (multiplies pattern length by N, spacing notes out)"
                            .to_string(),
                    ));
                }
            }

            Some(Command::Compress) => {
                if let Some(n) = args.parse::<usize>().ok().filter(|_| !args.is_empty()) {
                    let n = n.clamp(2, 8);
                    use riffl_core::pattern::pattern::MIN_ROW_COUNT;
                    let cur_rows = self.editor.pattern().num_rows();
                    let new_rows = (cur_rows / n).max(MIN_ROW_COUNT);
                    let channels = self.editor.pattern().num_channels();
                    let kept_cells: Vec<Vec<riffl_core::pattern::row::Cell>> = (0..new_rows)
                        .map(|i| {
                            let src_r = i * n;
                            (0..channels)
                                .map(|c| {
                                    self.editor
                                        .pattern()
                                        .get_cell(src_r, c)
                                        .cloned()
                                        .unwrap_or_else(riffl_core::pattern::row::Cell::empty)
                                })
                                .collect()
                        })
                        .collect();
                    self.editor.pattern_mut().set_row_count(new_rows);
                    self.transport.set_num_rows(new_rows);
                    self.editor.clamp_cursor();
                    for (r, row_cells) in kept_cells.iter().enumerate() {
                        for (c, cell) in row_cells.iter().enumerate() {
                            self.editor.pattern_mut().set_cell(r, c, cell.clone());
                        }
                    }
                    let pos = self.transport.arrangement_position();
                    self.flush_editor_pattern(pos);
                    self.mark_dirty();
                    self.open_modal(Modal::info(
                        "Compress Pattern".to_string(),
                        format!(
                            "Compressed {}→{} rows (kept every {}th row)",
                            cur_rows, new_rows, n
                        ),
                    ));
                } else {
                    self.open_modal(Modal::error(
                        "Compress".to_string(),
                        "Usage: :compress <2-8>  (keeps every Nth row, reduces pattern length)"
                            .to_string(),
                    ));
                }
            }

            Some(Command::InstCopy) => {
                let nums: Vec<usize> = args
                    .split_whitespace()
                    .filter_map(|t| t.parse::<usize>().ok())
                    .collect();
                if nums.len() >= 2 {
                    let (src, dst) = (nums[0], nums[1]);
                    if src < self.song.instruments.len()
                        && dst < self.song.instruments.len()
                        && src != dst
                    {
                        let cloned = self.song.instruments[src].clone();
                        let dst_name = self.song.instruments[dst].name.clone();
                        let new_inst = riffl_core::song::Instrument {
                            name: dst_name,
                            ..cloned
                        };
                        self.song.instruments[dst] = new_inst;
                        self.sync_mixer_instruments();
                        self.mark_dirty();
                        self.open_modal(Modal::info(
                            "Instrument Copied".to_string(),
                            format!("Copied settings from instrument {:02X} to {:02X}", src, dst),
                        ));
                    } else {
                        self.open_modal(Modal::error(
                            "Invalid indices".to_string(),
                            format!(
                                "Usage: :instcopy <src> <dst>  (have {} instruments)",
                                self.song.instruments.len()
                            ),
                        ));
                    }
                } else {
                    self.open_modal(Modal::error(
                        "Instrument Copy".to_string(),
                        "Usage: :instcopy <src_index> <dst_index>".to_string(),
                    ));
                }
            }

            Some(Command::LoadSample) => {
                if args.is_empty() {
                    self.open_modal(Modal::error(
                        "Load Sample".to_string(),
                        "Usage: :loadsample <path>".to_string(),
                    ));
                } else {
                    let path = std::path::PathBuf::from(args);
                    match self.instrument_selection {
                        None => {
                            self.open_modal(Modal::error(
                                "Load Sample".to_string(),
                                "No instrument selected. Select an instrument first.".to_string(),
                            ));
                        }
                        Some(inst_idx) => match self.assign_sample_to_instrument(&path, inst_idx) {
                            Ok(()) => {
                                let name = self
                                    .song
                                    .instruments
                                    .get(inst_idx)
                                    .map(|i| i.name.clone())
                                    .unwrap_or_default();
                                self.open_modal(Modal::info(
                                    "Sample Loaded".to_string(),
                                    format!(
                                        "Assigned '{}' to instrument {:02X} ({})",
                                        args, inst_idx, name
                                    ),
                                ));
                            }
                            Err(e) => {
                                self.open_modal(Modal::error("Load Sample".to_string(), e));
                            }
                        },
                    }
                }
            }

            Some(Command::Wave) => {
                if args.is_empty() {
                    self.open_modal(Modal::error(
                        "Wave Generator".to_string(),
                        "Usage: :wave <sine|square|saw|triangle|noise|pulse> [length_ms] [freq_hz]"
                            .to_string(),
                    ));
                    return;
                }
                let wave_parts: Vec<&str> = args.split_whitespace().collect();
                let wave_type = wave_parts[0].to_lowercase();
                let length_ms: u32 = wave_parts
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(500);
                let freq: f32 = wave_parts
                    .get(2)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(440.0);
                let sample_rate = self
                    .audio_engine
                    .as_ref()
                    .map(|e| e.sample_rate())
                    .unwrap_or(44100);
                match Self::generate_wave_sample(&wave_type, freq, length_ms, sample_rate) {
                    Err(e) => {
                        self.open_modal(Modal::error("Wave Generator".to_string(), e));
                    }
                    Ok(sample) => {
                        let sample_name = sample.name().map(|n| n.to_string()).unwrap_or_default();
                        let add_result = self
                            .mixer
                            .lock()
                            .ok()
                            .map(|mut m| m.add_sample(std::sync::Arc::new(sample)));
                        let sample_idx = match add_result {
                            Some(idx) => idx,
                            None => {
                                self.open_modal(Modal::error(
                                    "Wave Generator".to_string(),
                                    "Failed to lock mixer".to_string(),
                                ));
                                return;
                            }
                        };
                        match self.instrument_selection {
                            None => {
                                let idx = self.song.instruments.len();
                                let name = format!("{:02X}:{}", idx, sample_name);
                                let mut inst = riffl_core::song::Instrument::new(&name);
                                inst.sample_index = Some(sample_idx);
                                self.song.instruments.push(inst);
                                self.sync_mixer_instruments();
                                self.instrument_selection = Some(idx);
                                self.mark_dirty();
                                self.open_modal(Modal::info(
                                    "Wave Generated".to_string(),
                                    format!(
                                        "Created instrument {:02X} with {} ({:.0}Hz, {}ms)",
                                        idx, wave_type, freq, length_ms
                                    ),
                                ));
                            }
                            Some(inst_idx) => {
                                if inst_idx < self.song.instruments.len() {
                                    let inst = &mut self.song.instruments[inst_idx];
                                    inst.sample_index = Some(sample_idx);
                                    inst.sample_path = None;
                                    self.sync_mixer_instruments();
                                    self.mark_dirty();
                                    let inst_name = self.song.instruments[inst_idx].name.clone();
                                    self.open_modal(Modal::info(
                                        "Wave Generated".to_string(),
                                        format!(
                                            "Assigned {} ({:.0}Hz, {}ms) to instrument {:02X} ({})",
                                            wave_type, freq, length_ms, inst_idx, inst_name
                                        ),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
