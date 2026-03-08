use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::layout;
use super::theme::Theme;

/// Render help/cheatsheet overlay
pub fn render_help(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
    let help_area = layout::create_centered_rect(area, 80, 70);
    frame.render_widget(Clear, help_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(" KEYBOARD SHORTCUTS ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let lines = vec![
        Line::from(Span::styled(
            "NAVIGATION",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h j k l / arrows     Navigate"),
        Line::from("  PageUp/PageDown      Page navigation"),
        Line::from("  Tab                  Next track"),
        Line::from("  ( )                  Octave down/up (insert mode)"),
        Line::from(""),
        Line::from(Span::styled(
            "MODES",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  i                    Insert mode"),
        Line::from("  v                    Visual mode"),
        Line::from("  Esc                  Normal mode"),
        Line::from(""),
        Line::from(Span::styled(
            "EDITING",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  x / Delete           Delete cell"),
        Line::from("  u                    Undo"),
        Line::from("  Ctrl+R               Redo"),
        Line::from("  y                    Copy (visual)"),
        Line::from("  p                    Paste"),
        Line::from(""),
        Line::from(Span::styled(
            "VIEW SWITCHING",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  1-5 / F1-F4          Switch views"),
        Line::from("  ?                    Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "TRANSPORT",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Space                 Play/Stop"),
        Line::from("  = / -                 BPM up/down"),
        Line::from("  [ ]                   Prev/Next pattern"),
        Line::from(""),
        Line::from(Span::styled(
            "PATTERN EDITOR",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Shift+T               Add track"),
        Line::from("  Shift+D               Delete track"),
        Line::from("  Shift+C               Clone track"),
        Line::from("  Shift+Q               Quantize selection"),
        Line::from("  Shift+G               Go to row 0"),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to close",
            Style::default().fg(theme.text_secondary),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(theme.text_style());

    frame.render_widget(paragraph, inner);
}
