use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppView};
use crate::editor::{EditorMode, SubColumn};
use crate::input::keybindings::KeybindingRegistry;
use crate::registry::{CommandMetadata, CommandRegistry};
use riffl_core::pattern::effect::EffectMode;
use riffl_core::pattern::note::NoteEvent;
use riffl_core::transport::{PlaybackMode, TransportState};

use super::{CHANNEL_COL_WIDTH, ROW_NUM_WIDTH};

use super::{oscilloscope, vu_meters};

pub(super) fn render_pattern_with_area(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();
    let is_playing_or_paused = app.transport.is_playing() || app.transport.is_paused();
    let playback_row = app.transport.current_row();

    let pat_idx = app.transport.arrangement_position();
    let pat_name = app.editor.pattern().name.as_str();
    let title = if pat_name.is_empty() {
        format!(" Pattern {:02} ", pat_idx)
    } else {
        format!(" Pattern {:02}: {} ", pat_idx, pat_name)
    };
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title)
        .title_alignment(Alignment::Left);

    let inner = content_block.inner(area);
    let mut visible_rows = inner.height as usize;

    // Use persistent horizontal channel scroll offset (reset with Ctrl+Left)
    let num_channels = pattern.num_channels();
    let ch_scroll = app.channel_scroll.min(num_channels.saturating_sub(1));
    let channel_space = inner.width.saturating_sub(ROW_NUM_WIDTH);
    let visible_channels = ((channel_space / CHANNEL_COL_WIDTH) as usize).min(num_channels);
    let ch_end = (ch_scroll + visible_channels).min(num_channels);

    // Reserve 2 rows at top for VU meters and oscilloscopes if there's enough space
    let show_meters = visible_rows >= 4;
    let meter_rows = if show_meters { 2 } else { 0 };
    visible_rows = visible_rows.saturating_sub(meter_rows);

    // Pre-compute track audibility for muted/solo display
    let any_soloed = pattern.any_track_soloed();

    // During playback, auto-scroll to keep the playback row visible if follow mode is active.
    // When stopped or not in follow mode, follow the editor cursor instead.
    let scroll_target = if app.transport.is_playing() && app.follow_mode {
        playback_row
    } else {
        cursor_row
    };

    let scroll_offset = calculate_scroll_offset(
        scroll_target,
        visible_rows.saturating_sub(1), // reserve 1 row for channel header
        pattern.num_rows(),
    );

    let mut lines: Vec<Line> = Vec::new();

    // Render VU meters and oscilloscopes if we have space
    if show_meters && visible_channels > 0 {
        // Fetch levels and waveforms for all channels up to ch_end, then slice visible range
        let all_levels = app.channel_levels(ch_end);
        let all_waveforms = app.oscilloscope_data(ch_end);
        let levels: Vec<(f32, f32)> = all_levels
            .into_iter()
            .skip(ch_scroll)
            .take(visible_channels)
            .collect();
        let waveforms: Vec<Vec<f32>> = all_waveforms
            .into_iter()
            .skip(ch_scroll)
            .take(visible_channels)
            .collect();

        // Content width per channel (excluding separator "│ ")
        let content_width = (CHANNEL_COL_WIDTH - 2) as usize;
        let left_width = content_width / 2;
        let right_width = content_width - left_width;

        // VU meters row
        let mut vu_spans = Vec::new();
        vu_spans.push(Span::styled(
            "VU    ",
            Style::default().fg(theme.text_secondary),
        ));
        for &(l, r) in levels.iter() {
            let l_bar = vu_meters::level_to_bar(l, left_width as u16);
            let r_bar = vu_meters::level_to_bar(r, right_width as u16);
            let l_color = vu_meters::level_to_color(l, theme);
            let r_color = vu_meters::level_to_color(r, theme);
            vu_spans.push(Span::styled("│ ", Style::default().fg(theme.text_dimmed)));
            vu_spans.push(Span::styled(l_bar, Style::default().fg(l_color)));
            vu_spans.push(Span::styled(r_bar, Style::default().fg(r_color)));
        }
        lines.push(Line::from(vu_spans));

        // Oscilloscope row
        let mut osc_spans = Vec::new();
        osc_spans.push(Span::styled(
            "\u{223F}     ",
            Style::default().fg(theme.text_secondary),
        ));
        for waveform in waveforms.iter() {
            let line = oscilloscope::render_waveform_line(waveform, content_width, theme);
            osc_spans.push(Span::styled("│ ", Style::default().fg(theme.text_dimmed)));
            for span in line.spans {
                osc_spans.push(span);
            }
        }
        lines.push(Line::from(osc_spans));
    }

    // Channel header row with track names, mute/solo indicators
    let mut header_spans = Vec::new();
    header_spans.push(Span::styled(
        "  ROW ",
        Style::default().fg(theme.text_secondary),
    ));
    for ch in ch_scroll..ch_end {
        let track = pattern.get_track(ch);
        let is_muted = track.is_some_and(|t| t.muted);
        let is_soloed = track.is_some_and(|t| t.solo);

        // Build header label: name (6) + badge (3) + vol (3) + pan (3) = 15 chars
        let track_name = track.map_or_else(
            || format!("CH{:02}", ch + 1),
            |t| {
                if t.name.is_empty() {
                    format!("CH{:02}", ch + 1)
                } else if t.name.len() <= 6 {
                    t.name.clone()
                } else {
                    // Try to preserve a trailing number so "Track 12" → "Trk12" etc.
                    let trimmed = t.name.trim_end_matches(|c: char| c.is_ascii_digit());
                    let num_part = &t.name[trimmed.len()..];
                    if !num_part.is_empty() && num_part.len() <= 5 {
                        format!("#{:<5}", num_part)
                    } else {
                        t.name[..6].to_string()
                    }
                }
            },
        );

        let badge = if is_muted {
            "[M]"
        } else if is_soloed {
            "[S]"
        } else {
            "   "
        };

        let (vol, pan) = track
            .map(|t| (t.volume, t.pan))
            .unwrap_or((1.0, 0.0));

        let vol_str = format!("{:3}", (vol * 100.0).round() as i32);
        let pan_pct = (pan * 100.0).round() as i32;
        let pan_str = if pan_pct == 0 {
            "CTR".to_string()
        } else if pan_pct > 0 {
            format!("R{:02}", pan_pct.min(99))
        } else {
            format!("L{:02}", (-pan_pct).min(99))
        };

        // Total: 6 + 3 + 3 + 3 = 15 chars
        let label = format!("{:<6}{}{}{}", track_name, badge, vol_str, pan_str);
        // Pad/truncate to fit column width (15 chars content + separator)
        let display = format!("│ {:<15}", label);

        let header_style = if is_soloed {
            Style::default()
                .fg(Color::Black)
                .bg(theme.warning_color())
                .add_modifier(Modifier::BOLD)
        } else if is_muted {
            Style::default().fg(theme.text_dimmed)
        } else {
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD)
        };
        header_spans.push(Span::styled(display, header_style));
    }
    lines.push(Line::from(header_spans));

    // Pattern rows
    let rows_to_show = visible_rows.saturating_sub(1); // subtract header
    for display_idx in 0..rows_to_show {
        let row_idx = scroll_offset + display_idx;
        if row_idx >= pattern.num_rows() {
            break;
        }

        let mut row_spans = Vec::new();

        // Row number in hex (tracker convention)
        // Playback row is highlighted when playing OR paused (to show where playback is)
        let is_playback_row = is_playing_or_paused && row_idx == playback_row;
        let loop_region = app.transport.loop_region();
        let loop_active = app.transport.loop_region_active();
        let is_loop_start = loop_region.is_some_and(|(s, _)| s == row_idx);
        let is_loop_end = loop_region.is_some_and(|(_, e)| e == row_idx);
        let is_in_loop =
            loop_active && loop_region.is_some_and(|(s, e)| row_idx > s && row_idx < e);
        let (row_prefix, row_num_style) = if is_playback_row {
            (
                "▶ ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_loop_start && is_loop_end {
            // Single-row loop region
            let c = if loop_active {
                theme.warning_color()
            } else {
                theme.text_secondary
            };
            ("◈ ", Style::default().fg(c).add_modifier(Modifier::BOLD))
        } else if is_loop_start {
            let c = if loop_active {
                theme.warning_color()
            } else {
                theme.text_secondary
            };
            ("[ ", Style::default().fg(c).add_modifier(Modifier::BOLD))
        } else if is_loop_end {
            let c = if loop_active {
                theme.warning_color()
            } else {
                theme.text_secondary
            };
            ("] ", Style::default().fg(c).add_modifier(Modifier::BOLD))
        } else if is_in_loop {
            ("¦ ", Style::default().fg(theme.warning_color()))
        } else if row_idx.is_multiple_of(16) {
            (
                "│ ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            )
        } else if row_idx.is_multiple_of(4) {
            ("· ", Style::default().fg(theme.primary))
        } else {
            ("  ", Style::default().fg(theme.text_secondary))
        };

        row_spans.push(Span::styled(
            format!("{}{:02X}  ", row_prefix, row_idx),
            row_num_style,
        ));

        // Cells for each visible channel
        let mode = app.editor.mode();
        let sub_column = app.editor.sub_column();
        let visual_sel = app.editor.visual_selection();

        for ch in ch_scroll..ch_end {
            let is_track_muted = pattern.get_track(ch).is_some_and(|t| t.muted);
            let is_track_inaudible = !pattern.is_channel_audible(ch);

            let separator_style = if is_playback_row {
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(theme.success_color())
            } else {
                Style::default().fg(theme.text_dimmed)
            };
            row_spans.push(Span::styled("│ ", separator_style));

            let cell = pattern.get_cell(row_idx, ch);
            let is_cursor = cursor_row == row_idx && cursor_channel == ch;

            // Check if this cell is inside a visual selection
            let is_visual_selected = if mode == EditorMode::Visual {
                visual_sel.is_some_and(|((r0, c0), (r1, c1))| {
                    row_idx >= r0 && row_idx <= r1 && ch >= c0 && ch <= c1
                })
            } else {
                false
            };

            // Format cell parts
            let (note_str, inst_str, vol_str, eff_str) = format_cell_parts(cell);

            if is_cursor
                && matches!(
                    mode,
                    EditorMode::Insert | EditorMode::Normal | EditorMode::Replace
                )
                && !is_playback_row
            {
                // Insert/Normal/Replace mode: highlight only the active sub-column
                let (active, inactive) = if matches!(mode, EditorMode::Insert | EditorMode::Replace)
                {
                    (theme.insert_cursor_style(), theme.insert_inactive_style())
                } else {
                    (theme.highlight_style(), theme.normal_inactive_style())
                };
                let (ns, is, vs, es) = match sub_column {
                    SubColumn::Note => (active, inactive, inactive, inactive),
                    SubColumn::Instrument => (inactive, active, inactive, inactive),
                    SubColumn::Volume => (inactive, inactive, active, inactive),
                    SubColumn::Effect => (inactive, inactive, inactive, active),
                };
                row_spans.push(Span::styled(note_str, ns));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(inst_str, is));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(vol_str, vs));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(eff_str, es));
            } else if is_cursor && mode == EditorMode::Visual && !is_playback_row {
                // Visual mode cursor: active sub-column gets visual_cursor_style,
                // inactive sub-columns get visual_selection_style — same granularity as Normal mode
                let cur = theme.visual_cursor_style();
                let sel = theme.visual_selection_style();
                let (ns, is, vs, es) = match sub_column {
                    SubColumn::Note => (cur, sel, sel, sel),
                    SubColumn::Instrument => (sel, cur, sel, sel),
                    SubColumn::Volume => (sel, sel, cur, sel),
                    SubColumn::Effect => (sel, sel, sel, cur),
                };
                row_spans.push(Span::styled(note_str, ns));
                row_spans.push(Span::styled(" ", sel));
                row_spans.push(Span::styled(inst_str, is));
                row_spans.push(Span::styled(" ", sel));
                row_spans.push(Span::styled(vol_str, vs));
                row_spans.push(Span::styled(" ", sel));
                row_spans.push(Span::styled(eff_str, es));
            } else {
                // Determine the base style for override situations (playback, cursor+playback, visual, muted)
                let override_style = if is_cursor && is_playback_row {
                    Some(
                        Style::default()
                            .fg(theme.cursor_fg)
                            .bg(theme.success_color())
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_visual_selected {
                    Some(theme.visual_selection_style())
                } else if is_playback_row {
                    Some(
                        Style::default()
                            .fg(theme.cursor_fg)
                            .bg(theme.success_color())
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_track_muted || (any_soloed && is_track_inaudible) {
                    Some(Style::default().fg(theme.text_dimmed))
                } else {
                    None
                };

                if let Some(cell_style) = override_style {
                    // Uniform style for special states (playback, visual, muted)
                    let cell_text = format!("{} {} {} {}", note_str, inst_str, vol_str, eff_str);
                    row_spans.push(Span::styled(cell_text, cell_style));
                } else {
                    // Normal state: color-code each sub-column distinctly
                    let dimmed = Style::default().fg(theme.text_dimmed);
                    let has_note = note_str != "---" && note_str != "===" && note_str != "^^^";
                    let is_note_off = note_str == "===";
                    let is_note_cut = note_str == "^^^";
                    let has_inst = inst_str != "..";
                    let has_vol = vol_str != "..";
                    let has_effect = eff_str != "....";

                    let note_style = if is_note_cut {
                        Style::default()
                            .fg(theme.warning_color())
                            .add_modifier(Modifier::BOLD)
                    } else if is_note_off {
                        Style::default()
                            .fg(theme.error_color())
                            .add_modifier(Modifier::BOLD)
                    } else if has_note {
                        Style::default().fg(theme.primary)
                    } else {
                        dimmed
                    };
                    let inst_style = if has_inst {
                        Style::default().fg(theme.inst_color)
                    } else {
                        dimmed
                    };
                    let vol_style = if has_vol {
                        Style::default().fg(theme.vol_color)
                    } else {
                        dimmed
                    };
                    let eff_style = if has_effect {
                        Style::default().fg(theme.eff_color)
                    } else {
                        dimmed
                    };
                    row_spans.push(Span::styled(note_str, note_style));
                    row_spans.push(Span::styled(" ", dimmed));
                    row_spans.push(Span::styled(inst_str, inst_style));
                    row_spans.push(Span::styled(" ", dimmed));
                    row_spans.push(Span::styled(vol_str, vol_style));
                    row_spans.push(Span::styled(" ", dimmed));
                    row_spans.push(Span::styled(eff_str, eff_style));
                }
            }
            let trailing_style = if is_playback_row {
                Style::default().bg(theme.success_color())
            } else {
                Style::default()
            };
            row_spans.push(Span::styled(" ", trailing_style));
        }

        lines.push(Line::from(row_spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(content_block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Format a cell into its four sub-column parts: (note, instrument, volume, effect).
pub(super) fn format_cell_parts(
    cell: Option<&riffl_core::pattern::row::Cell>,
) -> (String, String, String, String) {
    match cell {
        Some(cell) => {
            let note_str = match &cell.note {
                Some(NoteEvent::On(note)) => note.display_str(),
                Some(NoteEvent::Off) => "===".to_string(),
                Some(NoteEvent::Cut) => "^^^".to_string(),
                None => "---".to_string(),
            };
            let inst_str = match cell.instrument {
                Some(inst) => format!("{:02X}", inst),
                None => "..".to_string(),
            };
            let vol_str = match cell.volume {
                Some(vol) => format!("{:02X}", vol),
                None => "..".to_string(),
            };
            let eff_str = match cell.first_effect() {
                Some(eff) => format!("{}", eff),
                None => "....".to_string(),
            };
            (note_str, inst_str, vol_str, eff_str)
        }
        None => (
            "---".to_string(),
            "..".to_string(),
            "..".to_string(),
            "....".to_string(),
        ),
    }
}

/// Format a cell for display in the tracker grid
pub(super) fn format_cell_display(cell: &riffl_core::pattern::row::Cell) -> String {
    let note_str = match &cell.note {
        Some(NoteEvent::On(note)) => note.display_str(),
        Some(NoteEvent::Off) => "===".to_string(),
        Some(NoteEvent::Cut) => "^^^".to_string(),
        None => "---".to_string(),
    };

    let inst_str = match cell.instrument {
        Some(inst) => format!("{:02X}", inst),
        None => "..".to_string(),
    };

    let vol_str = match cell.volume {
        Some(vol) => format!("{:02X}", vol),
        None => "..".to_string(),
    };

    let eff_str = match cell.first_effect() {
        Some(eff) => format!("{}", eff),
        None => "....".to_string(),
    };

    format!("{} {} {} {}", note_str, inst_str, vol_str, eff_str)
}

/// Calculate scroll offset to keep a target row visible
pub(super) fn calculate_scroll_offset(cursor_row: usize, visible_rows: usize, total_rows: usize) -> usize {
    if visible_rows >= total_rows {
        return 0;
    }
    if cursor_row < visible_rows / 2 {
        0
    } else if cursor_row + visible_rows / 2 >= total_rows {
        total_rows.saturating_sub(visible_rows)
    } else {
        cursor_row.saturating_sub(visible_rows / 2)
    }
}
