/// Instrument list view UI.
///
/// Displays the instruments defined in the song alongside loaded sample names.
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;
use tracker_core::song::Song;

/// Render the instrument list view.
pub fn render_instrument_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    song: &Song,
    sample_names: &[String],
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Instruments ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {:>3}  {:<20}  {:<16}  {:>6}  {}",
            "Idx", "Name", "Sample", "Vol", "Base Note"
        ),
        Style::default().fg(theme.text_secondary),
    )]));

    // Show song instruments
    if song.instruments.is_empty() && sample_names.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No instruments defined. Press 'o' or F5 to load a sample.",
            Style::default().fg(theme.text_dimmed),
        )));
    } else {
        // List instruments from the song model
        for (idx, inst) in song.instruments.iter().enumerate() {
            let sample_name = inst
                .sample_index
                .and_then(|si| sample_names.get(si))
                .map(|s| {
                    if s.len() > 16 {
                        s[..16].to_string()
                    } else {
                        s.clone()
                    }
                })
                .unwrap_or_else(|| "---".to_string());

            let name_display = if inst.name.len() > 20 {
                inst.name[..20].to_string()
            } else {
                inst.name.clone()
            };

            let base_note = inst.base_note.display_str();

            let line_text = format!(
                "  {:3}  {:<20}  {:<16}  {:5.0}%  {}",
                format!("{:02X}", idx),
                name_display,
                sample_name,
                inst.volume * 100.0,
                base_note,
            );

            lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(theme.text),
            )));
        }

        // Also list loaded samples that don't have instrument entries yet
        let inst_count = song.instruments.len();
        for (idx, name) in sample_names.iter().enumerate() {
            // Skip samples already covered by instrument entries
            if song.instruments.iter().any(|i| i.sample_index == Some(idx)) {
                continue;
            }
            let display_name = if name.len() > 16 { &name[..16] } else { name };
            let line_text = format!(
                "  {:3}  {:<20}  {:<16}  {:>6}  {}",
                format!("{:02X}", inst_count + idx),
                display_name,
                "(loaded)",
                "100%",
                "C-4",
            );
            lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(theme.text_dimmed),
            )));
        }
    }

    // Footer with keybinding hints
    let visible_rows = inner.height as usize;
    let content_lines = lines.len();
    if content_lines < visible_rows {
        // Pad to push instructions to the bottom area
        for _ in content_lines..visible_rows.saturating_sub(2) {
            lines.push(Line::from(""));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("F1", Style::default().fg(theme.success_color())),
        Span::raw(":pattern  "),
        Span::styled("F2", Style::default().fg(theme.success_color())),
        Span::raw(":arrange  "),
        Span::styled("o/F5", Style::default().fg(theme.success_color())),
        Span::raw(":load sample"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracker_core::song::Song;

    #[test]
    fn test_render_instrument_list_no_panic_empty() {
        // Verify rendering doesn't panic with empty data
        let song = Song::new("Test", 120.0);
        let names: Vec<String> = vec![];
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_instrument_list(frame, area, &song, &names, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_render_instrument_list_no_panic_with_samples() {
        let song = Song::new("Test", 120.0);
        let names = vec!["sine440".to_string(), "kick.wav".to_string()];
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_instrument_list(frame, area, &song, &names, &theme);
            })
            .unwrap();
    }
}
