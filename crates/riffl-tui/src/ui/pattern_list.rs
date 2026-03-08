/// Pattern list view UI.
///
/// Displays all patterns in the pattern pool with their properties.
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;
use riffl_core::song::Song;

/// Render the pattern list view.
pub fn render_pattern_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    song: &Song,
    theme: &Theme,
    selection: Option<usize>,
    current_pattern_in_editor: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Patterns ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {:>3}  {:>6}  {:>6}  {}",
            "Idx", "Rows", "Chans", "Description"
        ),
        Style::default().fg(theme.text_secondary),
    )]));

    // List patterns from the song model
    for (idx, pattern) in song.patterns.iter().enumerate() {
        let is_current = idx == current_pattern_in_editor;
        let is_selected = selection == Some(idx);

        let marker = if is_current { "*" } else { " " };
        let name = if pattern.name.is_empty() {
            if is_current { "Currently editing".to_string() } else { String::new() }
        } else {
            pattern.name.clone()
        };

        let line_text = format!(
            " {} {:02X}  {:6}  {:6}  {}",
            marker,
            idx,
            pattern.num_rows(),
            pattern.num_channels(),
            name
        );

        let style = if is_selected {
            Style::default().fg(theme.text).bg(theme.bg_highlight)
        } else if is_current {
            Style::default().fg(theme.success_color())
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(Span::styled(line_text, style)));
    }

    // Footer with keybinding hints
    let visible_rows = inner.height as usize;
    let content_lines = lines.len();
    if content_lines < visible_rows {
        for _ in content_lines..visible_rows.saturating_sub(2) {
            lines.push(Line::from(""));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("1-5", Style::default().fg(theme.success_color())),
        Span::raw(":view  "),
        Span::styled("n", Style::default().fg(theme.success_color())),
        Span::raw(":new  "),
        Span::styled("d", Style::default().fg(theme.success_color())),
        Span::raw(":del  "),
        Span::styled("c", Style::default().fg(theme.success_color())),
        Span::raw(":dup  "),
        Span::styled("Enter", Style::default().fg(theme.success_color())),
        Span::raw(":load"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffl_core::song::Song;

    #[test]
    fn test_render_pattern_list_no_panic_empty() {
        let song = Song::new("Test", 120.0);
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_pattern_list(frame, area, &song, &theme, None, 0);
            })
            .unwrap();
    }

    #[test]
    fn test_render_pattern_list_no_panic_with_patterns() {
        let mut song = Song::new("Test", 120.0);
        song.add_pattern(riffl_core::pattern::Pattern::new(32, 4));
        song.add_pattern(riffl_core::pattern::Pattern::new(16, 8));
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_pattern_list(frame, area, &song, &theme, Some(1), 0);
            })
            .unwrap();
    }
}
