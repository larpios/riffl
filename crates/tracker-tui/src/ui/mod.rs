/// UI rendering and components
///
/// This module contains all UI-related code including layout management,
/// theming, and modal dialogs.
use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppView};
use crate::editor::{EditorMode, SubColumn};
use crate::input::keybindings::KeybindingRegistry;
use crate::registry::{CommandMetadata, CommandRegistry};
use tracker_core::pattern::note::NoteEvent;
use tracker_core::transport::{PlaybackMode, TransportState};

// Submodules
pub mod arrangement;
pub mod code_editor;
pub mod export_dialog;
pub mod file_browser;
pub mod help;
pub mod instrument_editor;
pub mod instrument_list;
pub mod layout;
pub mod modal;
pub mod pattern_list;
pub mod sample_browser;
pub mod theme;
pub mod tutor;

use help::render_help;
use tutor::render_tutor;

/// Render the application UI
pub fn render(frame: &mut Frame, app: &App) {
    let full_area = frame.area();

    // Fill entire frame with theme background so Catppuccin/Nord bg colors are visible
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        full_area,
    );

    // Create main layout with header (3 lines), content (flexible), and footer (1 line)
    let (header_area, content_area, footer_area) = layout::create_main_layout(full_area, 3, 1);

    render_header(frame, header_area, app);

    // Handle split view: pattern left, code editor right
    if app.split_view && app.current_view == AppView::PatternEditor {
        let (left, right) =
            layout::create_split_layout(content_area, ratatui::layout::Direction::Horizontal, 50);
        render_content(frame, left, app);
        code_editor::render_code_editor(frame, right, &app.code_editor, &app.theme);
    } else {
        // Dispatch to the correct view renderer based on the active view
        match app.current_view {
            AppView::PatternEditor => render_content(frame, content_area, app),
            AppView::Arrangement => {
                let playback_pos = if app.transport.is_playing()
                    && app.transport.playback_mode() == PlaybackMode::Song
                {
                    Some(app.transport.arrangement_position())
                } else {
                    None
                };
                arrangement::render_arrangement(
                    frame,
                    content_area,
                    &app.song,
                    &app.arrangement_view,
                    playback_pos,
                    &app.theme,
                );
            }
            AppView::InstrumentList => {
                // If an instrument is selected, split: list on top, editor below.
                if let Some(idx) = app.instrument_selection() {
                    if idx < app.song.instruments.len() {
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([
                                ratatui::layout::Constraint::Percentage(55),
                                ratatui::layout::Constraint::Percentage(45),
                            ])
                            .split(content_area);
                        instrument_list::render_instrument_list(
                            frame,
                            chunks[0],
                            &app.song,
                            &app.loaded_samples(),
                            &app.theme,
                            Some(idx),
                        );
                        let sample = {
                            let samples = app.loaded_samples();
                            app.song.instruments[idx]
                                .sample_index
                                .and_then(|si| samples.get(si).cloned())
                        };
                        instrument_editor::render_instrument_editor(
                            frame,
                            chunks[1],
                            &app.song.instruments[idx],
                            &app.inst_editor,
                            &app.theme,
                            sample.as_deref(),
                        );
                    } else {
                        instrument_list::render_instrument_list(
                            frame,
                            content_area,
                            &app.song,
                            &app.loaded_samples(),
                            &app.theme,
                            app.instrument_selection(),
                        );
                    }
                } else {
                    instrument_list::render_instrument_list(
                        frame,
                        content_area,
                        &app.song,
                        &app.loaded_samples(),
                        &app.theme,
                        app.instrument_selection(),
                    );
                }
            }
            AppView::CodeEditor => {
                code_editor::render_code_editor(frame, content_area, &app.code_editor, &app.theme);
            }
            AppView::PatternList => {
                pattern_list::render_pattern_list(
                    frame,
                    content_area,
                    &app.song,
                    &app.theme,
                    app.pattern_selection(),
                    0,
                );
            }
            AppView::SampleBrowser => {
                let (preview_pos, total_frames, sample_rate) = app.preview_cursor_state();
                sample_browser::render_sample_browser(
                    frame,
                    content_area,
                    &app.sample_browser,
                    &app.theme,
                    preview_pos,
                    total_frames,
                    sample_rate,
                );
            }
        }
    }

    render_footer(frame, footer_area, app);

    // Render file browser on top if active
    if app.has_file_browser() {
        render_file_browser(frame, full_area, app);
    }

    // Render export dialog on top if active
    if app.export_dialog.active {
        export_dialog::render_export_dialog(frame, full_area, &app.export_dialog, &app.theme);
    }

    // Render modal on top if one is active
    if let Some(active_modal) = app.current_modal() {
        modal::render_modal(frame, full_area, active_modal, &app.theme);
    }

    // Render help overlay on top if active
    if app.show_help {
        render_help(frame, full_area, &app.theme, app.help_scroll);
    }

    // Render tutor view on top if active
    if app.show_tutor {
        render_tutor(frame, full_area, &app.theme, app.tutor_scroll);
    }

    // Render which-key popup when a chord is pending
    if app.pending_key.is_some() {
        render_which_key(frame, full_area, app);
    }

    // Render command autocomplete above the footer when in command mode
    if app.command_mode && !app.command_input.is_empty() {
        render_command_completions(frame, footer_area, app);
    }
}

/// Render the header with title, BPM, and play/stop status
fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();

    let (play_icon, play_status) = match app.transport.state() {
        TransportState::Playing => ("\u{25B6}", "PLAYING"),
        TransportState::Paused => ("\u{23F8}", "PAUSED"),
        TransportState::Stopped => ("\u{23F9}", "STOPPED"),
    };
    let play_color = match app.transport.state() {
        TransportState::Playing => theme.success_color(),
        TransportState::Paused => theme.warning_color(),
        TransportState::Stopped => theme.text_dimmed,
    };

    let mode_indicator = match app.transport.playback_mode() {
        PlaybackMode::Pattern => "PAT",
        PlaybackMode::Song => "SONG",
    };
    let loop_indicator = if app.transport.loop_enabled() {
        " L"
    } else {
        ""
    };
    let dirty_marker = if app.is_dirty { " *" } else { "" };
    let title = format!(
        " riffl{} | BPM: {:.0} | {} {}{} [{}] ",
        dirty_marker,
        app.transport.bpm(),
        play_icon,
        play_status,
        loop_indicator,
        mode_indicator
    );

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title)
        .title_alignment(Alignment::Center);

    let mut status_spans = vec![
        Span::styled(
            "riffl",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("BPM: {:.0}", app.transport.bpm()),
            Style::default().fg(theme.text),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} [{}]", play_icon, play_status),
            Style::default().fg(play_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

    // Show arrangement position + row in Song mode, just row in Pattern mode
    match app.transport.playback_mode() {
        PlaybackMode::Song => {
            status_spans.push(Span::styled(
                format!(
                    "Arr: {:02X}/{:02X}  Row: {:02X}/{:02X}",
                    app.transport.arrangement_position(),
                    app.song.arrangement.len(),
                    app.transport.current_row(),
                    pattern.num_rows(),
                ),
                Style::default().fg(theme.text_secondary),
            ));
        }
        PlaybackMode::Pattern => {
            status_spans.push(Span::styled(
                format!(
                    "Row: {:02X}/{:02X}",
                    app.transport.current_row(),
                    pattern.num_rows()
                ),
                Style::default().fg(theme.text_secondary),
            ));
        }
    }

    // Playback mode indicator
    let mode_color = match app.transport.playback_mode() {
        PlaybackMode::Pattern => theme.text_secondary,
        PlaybackMode::Song => theme.warning_color(),
    };
    status_spans.push(Span::raw("  "));
    status_spans.push(Span::styled(
        format!("[{}]", mode_indicator),
        Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
    ));

    if app.transport.loop_enabled() {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            "[LOOP]",
            Style::default()
                .fg(theme.info_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.live_mode {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            "[LIVE]",
            Style::default()
                .fg(theme.error_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    let header_text = Paragraph::new(Line::from(status_spans))
        .block(header_block)
        .alignment(Alignment::Center)
        .style(theme.header_style());

    frame.render_widget(header_text, area);
}

/// Width of a single channel column (including separator): "│ C#4 01 40 C20 " = 2 + 14 + 1 = 17
const CHANNEL_COL_WIDTH: u16 = 17;

/// Width of the row number column: "  XX  " = 6
const ROW_NUM_WIDTH: u16 = 6;

/// Calculate the horizontal channel scroll offset to keep the cursor channel visible.
fn calculate_channel_scroll(
    cursor_channel: usize,
    available_width: u16,
    num_channels: usize,
) -> usize {
    let channel_space = available_width.saturating_sub(ROW_NUM_WIDTH);
    let visible_channels = (channel_space / CHANNEL_COL_WIDTH) as usize;
    if visible_channels == 0 {
        return 0;
    }
    if visible_channels >= num_channels {
        return 0;
    }
    if cursor_channel < visible_channels / 2 {
        0
    } else if cursor_channel + visible_channels / 2 >= num_channels {
        num_channels.saturating_sub(visible_channels)
    } else {
        cursor_channel.saturating_sub(visible_channels / 2)
    }
}

/// Render the main content area with the tracker pattern grid
fn render_content(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();
    let is_playing_or_paused = app.transport.is_playing() || app.transport.is_paused();
    let playback_row = app.transport.current_row();

    let pat_idx = app.transport.arrangement_position();
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(format!(" Pattern {:02} ", pat_idx))
        .title_alignment(Alignment::Left);

    let inner = content_block.inner(area);
    let visible_rows = inner.height as usize;

    // Calculate horizontal channel scrolling
    let ch_scroll = calculate_channel_scroll(cursor_channel, inner.width, pattern.num_channels());
    let channel_space = inner.width.saturating_sub(ROW_NUM_WIDTH);
    let visible_channels =
        ((channel_space / CHANNEL_COL_WIDTH) as usize).min(pattern.num_channels());
    let ch_end = (ch_scroll + visible_channels).min(pattern.num_channels());

    // Pre-compute track audibility for muted/solo display
    let any_soloed = pattern.any_track_soloed();

    // During playback, auto-scroll to keep the playback row visible.
    // When stopped, follow the editor cursor instead.
    let scroll_target = if app.transport.is_playing() {
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

        // Build header label: "CH0 Name [M][S]"
        let track_name = track.map_or_else(
            || format!("CH{}", ch),
            |t| {
                // Truncate name to fit
                if t.name.len() > 7 {
                    t.name[..7].to_string()
                } else {
                    t.name.clone()
                }
            },
        );

        let mut label = format!("{:<8}", track_name);
        if is_muted {
            label.push_str("[M]");
        } else if is_soloed {
            label.push_str("[S]");
        } else {
            label.push_str("   ");
        }
        // Pad/truncate to fit column width (14 chars content + separator)
        let display = format!("│ {:<14}", label);

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
                && (mode == EditorMode::Insert || mode == EditorMode::Normal)
                && !is_playback_row
            {
                // Insert/Normal mode: highlight only the active sub-column
                let (active, inactive) = if mode == EditorMode::Insert {
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
                    let has_effect = eff_str != "...";

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
fn format_cell_parts(
    cell: Option<&tracker_core::pattern::row::Cell>,
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
                None => "...".to_string(),
            };
            (note_str, inst_str, vol_str, eff_str)
        }
        None => (
            "---".to_string(),
            "..".to_string(),
            "..".to_string(),
            "...".to_string(),
        ),
    }
}

/// Format a cell for display in the tracker grid
fn format_cell_display(cell: &tracker_core::pattern::row::Cell) -> String {
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
        None => "...".to_string(),
    };

    format!("{} {} {} {}", note_str, inst_str, vol_str, eff_str)
}

/// Calculate scroll offset to keep a target row visible
fn calculate_scroll_offset(cursor_row: usize, visible_rows: usize, total_rows: usize) -> usize {
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

/// Render the file browser overlay for loading audio samples
fn render_file_browser(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let browser = &app.file_browser;

    // Create centered area (70% width, 60% height)
    let browser_area = layout::create_centered_rect(area, 70, 60);
    frame.render_widget(Clear, browser_area);

    let dir_display = browser.directory().display().to_string();
    let title = format!(" Load File  {}  ", dir_display);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(title)
        .title_alignment(Alignment::Left)
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(browser_area);
    frame.render_widget(block, browser_area);

    let mut lines: Vec<Line> = Vec::new();

    if browser.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No files found (.wav, .flac, .ogg, .mod)",
            Style::default().fg(theme.text_dimmed),
        )));
        lines.push(Line::from(""));
    } else {
        // Calculate scroll for the file list
        let visible_rows = inner_area.height.saturating_sub(3) as usize; // reserve header + footer
        let selected = browser.selected_index();
        let total = browser.entries().len();
        let scroll_offset = if visible_rows >= total || selected < visible_rows / 2 {
            0
        } else if selected + visible_rows / 2 >= total {
            total.saturating_sub(visible_rows)
        } else {
            selected.saturating_sub(visible_rows / 2)
        };

        // Header line: describe what Enter will do for the selected file
        let selected_ext = browser
            .selected_path()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let action_hint = match selected_ext.as_deref() {
            Some("mod") => "  Enter → import MOD as new song (replaces current)".to_string(),
            _ => format!(
                "  Enter → load as instrument {:02X} (adds to instrument list)",
                app.song.instruments.len()
            ),
        };
        lines.push(Line::from(Span::styled(
            format!("  {} file(s)", total),
            Style::default().fg(theme.text_secondary),
        )));
        lines.push(Line::from(Span::styled(
            action_hint,
            Style::default().fg(theme.info_color()),
        )));
        lines.push(Line::from(""));

        for idx in scroll_offset..(scroll_offset + visible_rows).min(total) {
            let entry = &browser.entries()[idx];
            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("???");

            let is_selected = idx == selected;
            let prefix = if is_selected { "▸ " } else { "  " };
            let text = format!("{}{}", prefix, name);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.info_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            lines.push(Line::from(Span::styled(text, style)));
        }
    }

    // Footer with instructions
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k ↑↓", Style::default().fg(theme.success_color())),
        Span::raw(":navigate  "),
        Span::styled("Enter", Style::default().fg(theme.success_color())),
        Span::raw(":load  "),
        Span::styled("Esc", Style::default().fg(theme.error_color())),
        Span::raw(":close  "),
        Span::styled(
            ".wav .flac .ogg .mod",
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(theme.text_style());

    frame.render_widget(paragraph, inner_area);
}

/// Render the footer with mode indicator and keybindings
fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;

    // Pattern length prompt mode: show inline length input
    if app.len_prompt_mode {
        let input_style = Style::default().fg(theme.text);
        let label_style = Style::default()
            .fg(Color::Black)
            .bg(theme.info_color())
            .add_modifier(Modifier::BOLD);
        let line = Line::from(vec![
            Span::styled(" LEN ", label_style),
            Span::raw(" "),
            Span::styled(app.len_prompt_input.clone(), input_style),
            Span::styled("█", Style::default().fg(theme.primary)),
            Span::styled(
                "  Enter:apply  Esc:cancel  (16–512)",
                Style::default().fg(theme.text_secondary),
            ),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    // BPM prompt mode: show inline BPM input
    if app.bpm_prompt_mode {
        let bpm_style = Style::default().fg(theme.text);
        let label_style = Style::default()
            .fg(Color::Black)
            .bg(theme.warning_color())
            .add_modifier(Modifier::BOLD);
        let line = Line::from(vec![
            Span::styled(" BPM ", label_style),
            Span::raw(" "),
            Span::styled(app.bpm_prompt_input.clone(), bpm_style),
            Span::styled("█", Style::default().fg(theme.primary)),
            Span::styled(
                "  Enter:apply  Esc:cancel",
                Style::default().fg(theme.text_secondary),
            ),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    // Command mode: show command line input instead of normal footer
    if app.command_mode {
        let cmd_style = Style::default().fg(theme.text);
        let line = Line::from(vec![
            Span::styled(":", cmd_style),
            Span::styled(app.command_input.clone(), cmd_style),
            Span::styled("█", Style::default().fg(theme.primary)),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    let mode = app.editor.mode();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();

    let key_style = Style::default().fg(theme.success_color());
    // When code editor is active, show its mode; otherwise show pattern editor mode.
    // pending_replace overrides to show REPLACE mode pill.
    let (mode_label, mode_bg) = if app.pending_replace {
        ("REPLACE", theme.cursor_normal_bg)
    } else if app.is_code_editor_active() {
        if app.code_editor.insert_mode {
            ("INSERT", theme.warning_color())
        } else {
            ("NORMAL", theme.primary)
        }
    } else {
        (mode.label(), theme.primary)
    };
    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(mode_bg)
        .add_modifier(Modifier::BOLD);

    let mut footer_spans = vec![
        Span::raw(" "),
        Span::styled(format!(" {} ", mode_label), mode_style),
        Span::raw(" "),
    ];

    // Show context-specific state indicators (not a cheatsheet — press ? for that)
    // Only for pattern editor — skip when code editor is active.
    if app.is_code_editor_active() {
        // No pattern-editor context in code editor mode
    } else {
        match mode {
            EditorMode::Normal => {
                let hints: &[(&str, &str)] = match app.current_view {
                    AppView::PatternEditor => &[
                        ("Space", "play"),
                        ("i", "insert"),
                        ("v", "select"),
                        ("x", "del"),
                        ("y/p", "copy"),
                        ("[/]", "pat"),
                        ("A-[/]", "loop"),
                        ("^⇧L", "loop on/off"),
                        ("f", "follow"),
                        ("t", "tap bpm"),
                        ("^B", "set bpm"),
                        ("^P", "set len"),
                    ],
                    AppView::InstrumentList => &[
                        ("j/k", "nav"),
                        ("Enter", "edit"),
                        ("n", "new"),
                        ("d", "del"),
                    ],
                    AppView::SampleBrowser => &[
                        ("j/k", "nav"),
                        ("Space", "preview"),
                        ("l", "enter"),
                        ("h", "up"),
                    ],
                    AppView::PatternList => &[("j/k", "nav"), ("Enter", "load"), ("c", "clone")],
                    _ => &[],
                };
                for (k, desc) in hints {
                    footer_spans.push(Span::styled(*k, key_style));
                    footer_spans.push(Span::raw(format!(":{} ", desc)));
                }
            }
            EditorMode::Insert => {
                footer_spans.push(Span::styled(
                    format!("Oct:{}", app.editor.current_octave()),
                    Style::default().fg(theme.warning_color()),
                ));
                footer_spans.push(Span::raw(" "));
                footer_spans.push(Span::styled(
                    format!("Ins:{:02X}", app.editor.current_instrument()),
                    Style::default().fg(Color::Yellow),
                ));
                footer_spans.push(Span::raw(" "));
                footer_spans.push(Span::styled(
                    format!("Stp:{}", app.editor.step_size()),
                    Style::default().fg(Color::Cyan),
                ));
                if app.editor.sub_column() == SubColumn::Effect {
                    let cell = app
                        .editor
                        .pattern()
                        .get_cell(app.editor.cursor_row(), app.editor.cursor_channel());
                    let mnemonic = cell
                        .and_then(|c| c.first_effect())
                        .map(|e| e.mnemonic())
                        .unwrap_or("---");
                    footer_spans.push(Span::raw(" "));
                    footer_spans.push(Span::styled(
                        format!("Eff:{}", mnemonic),
                        Style::default().fg(theme.warning_color()),
                    ));
                    let pos = app.editor.effect_digit_position();
                    let pos_label = match pos {
                        0 => "cmd",
                        1 => "hi",
                        _ => "lo",
                    };
                    footer_spans.push(Span::raw(" "));
                    footer_spans.push(Span::styled(
                        format!("[{}]", pos_label),
                        Style::default().fg(theme.info_color()),
                    ));
                }
            }
            EditorMode::Visual => {}
        }

        // Step size (always visible — affects note entry row advance)
        if mode == EditorMode::Normal {
            footer_spans.extend([
                Span::raw("  "),
                Span::styled(
                    format!("Stp:{}", app.editor.step_size()),
                    Style::default().fg(Color::Cyan),
                ),
            ]);
        }
    } // end else (not code editor)

    // Help hint
    footer_spans.extend([
        Span::raw("  "),
        Span::styled("?", key_style),
        Span::raw(" help"),
    ]);

    // Follow mode indicator
    if app.follow_mode {
        footer_spans.extend([
            Span::raw(" "),
            Span::styled(
                " FOL ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    // Draw mode indicator
    if app.draw_mode {
        footer_spans.extend([
            Span::raw(" "),
            Span::styled(
                " DRW ",
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    // Loop region indicator
    if let Some((loop_start, loop_end)) = app.transport.loop_region() {
        let (label, bg) = if app.transport.loop_region_active() {
            (
                format!(" LOP {:02X}-{:02X} ", loop_start, loop_end),
                theme.warning_color(),
            )
        } else {
            (
                format!(" lop {:02X}-{:02X} ", loop_start, loop_end),
                theme.text_secondary,
            )
        };
        footer_spans.extend([
            Span::raw(" "),
            Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
    }

    // Pattern length indicator (always visible in pattern-related views)
    if matches!(
        app.current_view,
        AppView::PatternEditor | AppView::PatternList
    ) {
        let row_count = app.editor.pattern().row_count();
        footer_spans.extend([
            Span::raw("  "),
            Span::styled(
                format!("Len:{}", row_count),
                Style::default().fg(theme.info_color()),
            ),
        ]);
    }

    // View indicator
    let view_label = if app.split_view {
        "SPLIT"
    } else {
        match app.current_view {
            AppView::PatternEditor => "1:PAT",
            AppView::Arrangement => "2:ARR",
            AppView::InstrumentList => "3:INS",
            AppView::CodeEditor => "4:CODE",
            AppView::PatternList => "5:PATLIST",
            AppView::SampleBrowser => "6:SAMPLES",
        }
    };
    footer_spans.extend([
        Span::raw(" | "),
        Span::styled(
            view_label,
            Style::default()
                .fg(theme.info_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("CH:{} ROW:{:02X}", cursor_channel, cursor_row),
            Style::default().fg(theme.primary),
        ),
        Span::raw(" "),
        Span::styled(
            format!("Inst:{}", app.instrument_count()),
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let footer = Paragraph::new(Line::from(footer_spans)).style(theme.footer_style());

    frame.render_widget(footer, area);
}

/// Render command-line autocomplete suggestions above the footer.
fn render_command_completions(frame: &mut Frame, footer_area: ratatui::layout::Rect, app: &App) {
    let input = app.command_input.trim();
    let input_word = input.split_whitespace().next().unwrap_or(input);

    let matches: Vec<(String, String)> = CommandRegistry::all_commands()
        .into_iter()
        .filter_map(|cmd| {
            let name = cmd.name();
            let aliases = cmd.aliases();
            let all_names: Vec<&str> = std::iter::once(name)
                .chain(aliases.iter().copied())
                .collect();

            let _ = all_names.iter().find(|&&n| n.starts_with(input_word))?;
            let usage = cmd.usage().strip_prefix(':').unwrap_or(cmd.usage());
            Some((usage.to_string(), cmd.description().to_string()))
        })
        .collect();

    if matches.is_empty() {
        return;
    }

    let theme = &app.theme;
    let width = 30u16;
    let height = matches.len() as u16 + 2;
    let x = 0;
    let y = footer_area.y.saturating_sub(height);
    let area = ratatui::layout::Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let lines: Vec<Line> = matches
        .iter()
        .map(|(cmd, desc)| {
            Line::from(vec![
                Span::raw(" :"),
                Span::styled(
                    cmd.clone(),
                    Style::default()
                        .fg(theme.success_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}", desc)),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.text_dimmed))
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

/// Render a which-key popup showing completions for the current pending key.
fn render_which_key(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pending = match app.pending_key {
        Some(c) => c,
        None => return,
    };

    let theme = &app.theme;
    let entries = KeybindingRegistry::get_which_key_entries(pending);

    if entries.is_empty() {
        return;
    }

    let width = 24u16;
    let height = entries.len() as u16 + 2;
    let x = 0;
    let y = area.height.saturating_sub(height + 1);
    let popup_area = ratatui::layout::Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let title = format!(" {}… ", pending);
    let lines: Vec<Line> = entries
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    key.clone(),
                    Style::default()
                        .fg(theme.success_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}", desc)),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title(title)
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, popup_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_offset_small_pattern() {
        // Pattern fits in view
        assert_eq!(calculate_scroll_offset(0, 20, 16), 0);
        assert_eq!(calculate_scroll_offset(15, 20, 16), 0);
    }

    #[test]
    fn test_scroll_offset_at_top() {
        assert_eq!(calculate_scroll_offset(0, 10, 64), 0);
        assert_eq!(calculate_scroll_offset(3, 10, 64), 0);
    }

    #[test]
    fn test_scroll_offset_middle() {
        assert_eq!(calculate_scroll_offset(30, 10, 64), 25);
    }

    #[test]
    fn test_scroll_offset_at_bottom() {
        assert_eq!(calculate_scroll_offset(63, 10, 64), 54);
    }

    #[test]
    fn test_format_cell_empty() {
        let cell = tracker_core::pattern::row::Cell::empty();
        assert_eq!(format_cell_display(&cell), "--- .. .. ...");
    }

    #[test]
    fn test_format_cell_with_note() {
        use tracker_core::pattern::note::{Note, Pitch};
        let cell =
            tracker_core::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
        assert_eq!(format_cell_display(&cell), "C-4 .. .. ...");
    }

    #[test]
    fn test_format_cell_note_off() {
        let cell = tracker_core::pattern::row::Cell::with_note(NoteEvent::Off);
        assert_eq!(format_cell_display(&cell), "=== .. .. ...");
    }

    #[test]
    fn test_format_cell_note_cut() {
        let cell = tracker_core::pattern::row::Cell::with_note(NoteEvent::Cut);
        assert_eq!(format_cell_display(&cell), "^^^ .. .. ...");
    }

    #[test]
    fn test_format_cell_full() {
        use tracker_core::pattern::effect::Effect;
        use tracker_core::pattern::note::{Note, Pitch};
        let cell = tracker_core::pattern::row::Cell {
            note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
            instrument: Some(1),
            volume: Some(0x40),
            effects: vec![Effect::new(0xC, 0x20)],
        };
        assert_eq!(format_cell_display(&cell), "C#4 01 40 C20");
    }

    // --- format_cell_parts tests ---

    #[test]
    fn test_format_cell_parts_none() {
        let (n, i, v, e) = format_cell_parts(None);
        assert_eq!(n, "---");
        assert_eq!(i, "..");
        assert_eq!(v, "..");
        assert_eq!(e, "...");
    }

    #[test]
    fn test_format_cell_parts_empty() {
        let cell = tracker_core::pattern::row::Cell::empty();
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "---");
        assert_eq!(i, "..");
        assert_eq!(v, "..");
        assert_eq!(e, "...");
    }

    #[test]
    fn test_format_cell_parts_with_note() {
        use tracker_core::pattern::note::{Note, Pitch};
        let cell =
            tracker_core::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "C-4");
        assert_eq!(i, "..");
        assert_eq!(v, "..");
        assert_eq!(e, "...");
    }

    #[test]
    fn test_format_cell_parts_full() {
        use tracker_core::pattern::effect::Effect;
        use tracker_core::pattern::note::{Note, Pitch};
        let cell = tracker_core::pattern::row::Cell {
            note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
            instrument: Some(1),
            volume: Some(0x40),
            effects: vec![Effect::new(0xC, 0x20)],
        };
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "C#4");
        assert_eq!(i, "01");
        assert_eq!(v, "40");
        assert_eq!(e, "C20");
    }

    // --- ProTracker effect rendering tests (Phase 2 effects) ---

    #[test]
    fn test_format_cell_parts_effect_5xy_tone_porta_vol_slide() {
        use tracker_core::pattern::effect::Effect;
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0x5, 0x34)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "534");
    }

    #[test]
    fn test_format_cell_parts_effect_6xy_vibrato_vol_slide() {
        use tracker_core::pattern::effect::Effect;
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0x6, 0x12)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "612");
    }

    #[test]
    fn test_format_cell_parts_effect_7xy_tremolo() {
        use tracker_core::pattern::effect::Effect;
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0x7, 0x44)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "744");
    }

    #[test]
    fn test_format_cell_parts_effect_9xx_sample_offset() {
        use tracker_core::pattern::effect::Effect;
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0x9, 0x80)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "980");
    }

    #[test]
    fn test_format_cell_parts_effect_exy_extended() {
        use tracker_core::pattern::effect::Effect;
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xE, 0x10)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "E10");
    }

    #[test]
    fn test_format_cell_parts_effect_zero_param() {
        use tracker_core::pattern::effect::Effect;
        // Effect with zero param renders as "X00"
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xA, 0x00)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "A00");
    }

    #[test]
    fn test_format_cell_parts_effect_ff_param() {
        use tracker_core::pattern::effect::Effect;
        // Full-range param renders correctly
        let cell = tracker_core::pattern::row::Cell {
            note: None,
            instrument: None,
            volume: None,
            effects: vec![Effect::new(0xF, 0xFF)],
        };
        let (_, _, _, e) = format_cell_parts(Some(&cell));
        assert_eq!(e, "FFF");
    }

    #[test]
    fn test_format_cell_parts_note_off() {
        let cell = tracker_core::pattern::row::Cell::with_note(NoteEvent::Off);
        let (n, _i, _v, _e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "===");
    }

    // --- Scroll target selection tests ---

    #[test]
    fn test_scroll_offset_follows_playback_position() {
        // When playing, scroll should follow the playback row, not the cursor
        // The scroll_target logic in render_content selects transport.current_row()
        // during playback. Here we verify the scroll offset calculation works
        // correctly for a playback row deep in a large pattern.
        let playback_row = 50;
        let visible = 20;
        let total = 64;
        let offset = calculate_scroll_offset(playback_row, visible, total);
        // Playback row should be centered: 50 - 10 = 40
        assert_eq!(offset, 40);
    }

    #[test]
    fn test_scroll_offset_playback_at_start() {
        // When playback is at the start, offset should be 0
        assert_eq!(calculate_scroll_offset(0, 20, 64), 0);
        assert_eq!(calculate_scroll_offset(5, 20, 64), 0);
    }

    // --- Channel scroll tests ---

    #[test]
    fn test_channel_scroll_all_fit() {
        // 4 channels, wide terminal — no scrolling needed
        assert_eq!(calculate_channel_scroll(0, 200, 4), 0);
        assert_eq!(calculate_channel_scroll(3, 200, 4), 0);
    }

    #[test]
    fn test_channel_scroll_narrow_terminal() {
        // 8 channels, only room for 4 (6 + 4*17 = 74 needed, width=74)
        let width = ROW_NUM_WIDTH + CHANNEL_COL_WIDTH * 4; // 74
        assert_eq!(calculate_channel_scroll(0, width, 8), 0);
        assert_eq!(calculate_channel_scroll(1, width, 8), 0);
        // Cursor at ch 6 should scroll
        assert_eq!(calculate_channel_scroll(6, width, 8), 4);
        // Cursor at ch 7 (last) should show last 4
        assert_eq!(calculate_channel_scroll(7, width, 8), 4);
    }

    #[test]
    fn test_channel_scroll_center_cursor() {
        // 8 channels, room for 4
        let width = ROW_NUM_WIDTH + CHANNEL_COL_WIDTH * 4;
        // Cursor at ch 4 should center: 4 - 2 = 2
        assert_eq!(calculate_channel_scroll(4, width, 8), 2);
    }

    #[test]
    fn test_scroll_offset_playback_at_end() {
        // When playback is near the end, should not scroll past the bottom
        let offset = calculate_scroll_offset(63, 20, 64);
        assert_eq!(offset, 44); // 64 - 20 = 44
    }
}
