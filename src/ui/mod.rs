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

use crate::app::App;
use crate::editor::{EditorMode, SubColumn};
use crate::pattern::note::NoteEvent;

// Submodules
pub mod file_browser;
pub mod layout;
pub mod modal;
pub mod theme;

/// Render the application UI
pub fn render(frame: &mut Frame, app: &App) {
    let full_area = frame.area();

    // Create main layout with header (3 lines), content (flexible), and footer (1 line)
    let (header_area, content_area, footer_area) = layout::create_main_layout(full_area, 3, 1);

    render_header(frame, header_area, app);
    render_content(frame, content_area, app);
    render_footer(frame, footer_area, app);

    // Render file browser on top if active
    if app.has_file_browser() {
        render_file_browser(frame, full_area, app);
    }

    // Render modal on top if one is active
    if let Some(active_modal) = app.current_modal() {
        modal::render_modal(frame, full_area, active_modal, &app.theme);
    }
}

/// Render the header with title, BPM, and play/stop status
fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();

    let play_status = if app.is_playing { "PLAYING" } else { "STOPPED" };
    let play_color = if app.is_playing { theme.success_color() } else { theme.text_dimmed };

    let title = format!(" tracker-rs | BPM: {:.0} | {} ", app.bpm, play_status);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title)
        .title_alignment(Alignment::Center);

    let status_spans = vec![
        Span::styled("tracker-rs", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(format!("BPM: {:.0}", app.bpm), Style::default().fg(theme.text)),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", play_status),
            Style::default().fg(play_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Row: {:02X}/{:02X}", app.current_row, pattern.num_rows()),
            Style::default().fg(theme.text_secondary),
        ),
    ];

    let header_text = Paragraph::new(Line::from(status_spans))
        .block(header_block)
        .alignment(Alignment::Center)
        .style(theme.header_style());

    frame.render_widget(header_text, area);
}

/// Render the main content area with the tracker pattern grid
fn render_content(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Pattern Editor ")
        .title_alignment(Alignment::Left);

    let inner = content_block.inner(area);
    let visible_rows = inner.height as usize;

    // Calculate scroll offset to keep cursor visible
    let scroll_offset = calculate_scroll_offset(
        cursor_row,
        visible_rows.saturating_sub(1), // reserve 1 row for channel header
        pattern.num_rows(),
    );

    let mut lines: Vec<Line> = Vec::new();

    // Channel header row
    let mut header_spans = Vec::new();
    header_spans.push(Span::styled("  ROW ", Style::default().fg(theme.text_secondary)));
    for ch in 0..pattern.num_channels() {
        header_spans.push(Span::styled(
            format!("│ CH{:<11}", ch),
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ));
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
        let is_playback_row = app.is_playing && row_idx == app.current_row;
        let row_num_style = if is_playback_row {
            Style::default().fg(Color::Black).bg(theme.success_color()).add_modifier(Modifier::BOLD)
        } else if row_idx % 4 == 0 {
            Style::default().fg(theme.primary)
        } else {
            Style::default().fg(theme.text_secondary)
        };

        row_spans.push(Span::styled(format!("  {:02X}  ", row_idx), row_num_style));

        // Cells for each channel
        let mode = app.editor.mode();
        let sub_column = app.editor.sub_column();
        let visual_sel = app.editor.visual_selection();

        for ch in 0..pattern.num_channels() {
            row_spans.push(Span::styled("│ ", Style::default().fg(theme.text_dimmed)));

            let cell = pattern.get_cell(row_idx, ch);
            let is_cursor = cursor_row == row_idx && cursor_channel == ch;

            // Check if this cell is inside a visual selection
            let is_visual_selected = if mode == EditorMode::Visual {
                visual_sel.map_or(false, |((r0, c0), (r1, c1))| {
                    row_idx >= r0 && row_idx <= r1 && ch >= c0 && ch <= c1
                })
            } else {
                false
            };

            // Format cell parts
            let (note_str, inst_str, vol_str, eff_str) = format_cell_parts(cell);

            if is_cursor && mode == EditorMode::Insert {
                // Insert mode: highlight the active sub-column distinctly
                let active = theme.insert_cursor_style();
                let inactive = theme.insert_inactive_style();
                let (ns, is, vs, es) = match sub_column {
                    SubColumn::Note       => (active, inactive, inactive, inactive),
                    SubColumn::Instrument => (inactive, active, inactive, inactive),
                    SubColumn::Volume     => (inactive, inactive, active, inactive),
                    SubColumn::Effect     => (inactive, inactive, inactive, active),
                };
                row_spans.push(Span::styled(note_str, ns));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(inst_str, is));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(vol_str, vs));
                row_spans.push(Span::styled(" ", inactive));
                row_spans.push(Span::styled(eff_str, es));
            } else {
                // Single style for the whole cell
                let cell_text = format!("{} {} {} {}", note_str, inst_str, vol_str, eff_str);
                let cell_style = if is_cursor {
                    theme.highlight_style()
                } else if is_visual_selected {
                    theme.visual_selection_style()
                } else if is_playback_row {
                    Style::default().fg(theme.success_color()).add_modifier(Modifier::BOLD)
                } else if cell.map_or(true, |c| c.is_empty()) {
                    Style::default().fg(theme.text_dimmed)
                } else {
                    Style::default().fg(theme.text)
                };
                row_spans.push(Span::styled(cell_text, cell_style));
            }
            row_spans.push(Span::raw(" "));
        }

        lines.push(Line::from(row_spans));
    }

    let paragraph = Paragraph::new(lines)
        .block(content_block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Format a cell into its four sub-column parts: (note, instrument, volume, effect).
fn format_cell_parts(cell: Option<&crate::pattern::row::Cell>) -> (String, String, String, String) {
    match cell {
        Some(cell) => {
            let note_str = match &cell.note {
                Some(NoteEvent::On(note)) => note.display_str(),
                Some(NoteEvent::Off) => "===".to_string(),
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
            let eff_str = match &cell.effect {
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
fn format_cell_display(cell: &crate::pattern::row::Cell) -> String {
    let note_str = match &cell.note {
        Some(NoteEvent::On(note)) => note.display_str(),
        Some(NoteEvent::Off) => "===".to_string(),
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

    let eff_str = match &cell.effect {
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
    let title = format!(" Load Sample - {} ", dir_display);

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
            "  No audio files found (.wav, .flac, .ogg)",
            Style::default().fg(theme.text_dimmed),
        )));
        lines.push(Line::from(""));
    } else {
        // Calculate scroll for the file list
        let visible_rows = inner_area.height.saturating_sub(3) as usize; // reserve header + footer
        let selected = browser.selected_index();
        let total = browser.entries().len();
        let scroll_offset = if visible_rows >= total {
            0
        } else if selected < visible_rows / 2 {
            0
        } else if selected + visible_rows / 2 >= total {
            total.saturating_sub(visible_rows)
        } else {
            selected.saturating_sub(visible_rows / 2)
        };

        // Header line
        lines.push(Line::from(Span::styled(
            format!("  {} file(s) found", total),
            Style::default().fg(theme.text_secondary),
        )));
        lines.push(Line::from(""));

        for idx in scroll_offset..(scroll_offset + visible_rows).min(total) {
            let entry = &browser.entries()[idx];
            let name = entry
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("???");

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
        Span::styled("j/k", Style::default().fg(theme.success_color())),
        Span::raw(":navigate  "),
        Span::styled("Enter", Style::default().fg(theme.success_color())),
        Span::raw(":load  "),
        Span::styled("Esc", Style::default().fg(theme.error_color())),
        Span::raw(":cancel"),
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
    let mode = app.editor.mode();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();

    let key_style = Style::default().fg(theme.success_color());
    let mode_style = Style::default()
        .fg(Color::Black)
        .bg(theme.primary)
        .add_modifier(Modifier::BOLD);

    let mut footer_spans = vec![
        Span::raw(" "),
        Span::styled(format!(" {} ", mode.label()), mode_style),
        Span::raw(" "),
    ];

    // Show mode-specific hints
    match mode {
        EditorMode::Normal => {
            footer_spans.extend([
                Span::styled("i", key_style),
                Span::raw(":insert "),
                Span::styled("v", key_style),
                Span::raw(":visual "),
                Span::styled("o", key_style),
                Span::raw(":load "),
                Span::styled("space", key_style),
                Span::raw(":play "),
                Span::styled("x", key_style),
                Span::raw(":delete "),
                Span::styled("u", key_style),
                Span::raw(":undo "),
                Span::styled("q", key_style),
                Span::raw(":quit"),
            ]);
        }
        EditorMode::Insert => {
            footer_spans.extend([
                Span::styled("A-G", key_style),
                Span::raw(":note "),
                Span::styled("0-9", key_style),
                Span::raw(":octave "),
                Span::styled("Esc", key_style),
                Span::raw(":normal "),
                Span::styled(
                    format!("Oct:{}", app.editor.current_octave()),
                    Style::default().fg(theme.warning_color()),
                ),
            ]);
        }
        EditorMode::Visual => {
            footer_spans.extend([
                Span::styled("hjkl", key_style),
                Span::raw(":select "),
                Span::styled("x", key_style),
                Span::raw(":delete "),
                Span::styled("Esc", key_style),
                Span::raw(":normal"),
            ]);
        }
    }

    footer_spans.extend([
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

    let footer = Paragraph::new(Line::from(footer_spans))
        .style(theme.footer_style());

    frame.render_widget(footer, area);
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
        let cell = crate::pattern::row::Cell::empty();
        assert_eq!(format_cell_display(&cell), "--- .. .. ...");
    }

    #[test]
    fn test_format_cell_with_note() {
        use crate::pattern::note::{Note, Pitch};
        let cell = crate::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
        assert_eq!(format_cell_display(&cell), "C-4 .. .. ...");
    }

    #[test]
    fn test_format_cell_note_off() {
        let cell = crate::pattern::row::Cell::with_note(NoteEvent::Off);
        assert_eq!(format_cell_display(&cell), "=== .. .. ...");
    }

    #[test]
    fn test_format_cell_full() {
        use crate::pattern::note::{Note, Pitch};
        use crate::pattern::row::Effect;
        let cell = crate::pattern::row::Cell {
            note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
            instrument: Some(1),
            volume: Some(0x40),
            effect: Some(Effect::new(0xC, 0x20)),
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
        let cell = crate::pattern::row::Cell::empty();
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "---");
        assert_eq!(i, "..");
        assert_eq!(v, "..");
        assert_eq!(e, "...");
    }

    #[test]
    fn test_format_cell_parts_with_note() {
        use crate::pattern::note::{Note, Pitch};
        let cell = crate::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "C-4");
        assert_eq!(i, "..");
        assert_eq!(v, "..");
        assert_eq!(e, "...");
    }

    #[test]
    fn test_format_cell_parts_full() {
        use crate::pattern::note::{Note, Pitch};
        use crate::pattern::row::Effect;
        let cell = crate::pattern::row::Cell {
            note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
            instrument: Some(1),
            volume: Some(0x40),
            effect: Some(Effect::new(0xC, 0x20)),
        };
        let (n, i, v, e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "C#4");
        assert_eq!(i, "01");
        assert_eq!(v, "40");
        assert_eq!(e, "C20");
    }

    #[test]
    fn test_format_cell_parts_note_off() {
        let cell = crate::pattern::row::Cell::with_note(NoteEvent::Off);
        let (n, _i, _v, _e) = format_cell_parts(Some(&cell));
        assert_eq!(n, "===");
    }
}
