use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::Theme;

/// Total line count of the taller help column. Used to cap scroll offset.
pub fn content_line_count() -> u16 {
    left_column(&Theme::default())
        .len()
        .max(right_column(&Theme::default()).len()) as u16
}

/// Render help/cheatsheet overlay — two-column scrollable layout.
/// `scroll` is the vertical scroll offset in lines.
pub fn render_help(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme, scroll: u16) {
    let help_area = super::layout::create_centered_rect(area, 84, 85);
    frame.render_widget(Clear, help_area);

    let title = " KEYBOARD SHORTCUTS  (j/k scroll · ?/Esc close) ";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(title)
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    // Split inner area into two equal columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let col_style = Style::default().fg(theme.text).bg(theme.bg_surface);
    frame.render_widget(
        Paragraph::new(left_column(theme))
            .scroll((scroll, 0))
            .style(col_style),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(right_column(theme))
            .scroll((scroll, 0))
            .style(col_style),
        cols[1],
    );
}

fn section(label: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key(keys: &str, desc: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<18}", keys),
            Style::default().fg(theme.success_color()),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.text)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn left_column(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        section("NAVIGATION", theme),
        key("hjkl / arrows", "Move cursor", theme),
        key("Tab", "Next track", theme),
        key("PageUp / PageDown", "Page up / down", theme),
        key("( )", "Octave down / up", theme),
        blank(),
        section("MODES", theme),
        key("i", "Enter Insert mode", theme),
        key("v", "Enter Visual mode", theme),
        key("Esc", "Return to Normal mode", theme),
        blank(),
        section("EDITING  (Normal)", theme),
        key("x / Delete", "Delete cell", theme),
        key("Insert", "Insert row", theme),
        key("Ctrl+Delete", "Delete row", theme),
        key("u  /  Ctrl+R", "Undo / Redo", theme),
        key("y / p", "Copy / Paste", theme),
        key("Ctrl+C / Ctrl+V", "Copy / Paste (alt)", theme),
        key("Ctrl+X", "Cut", theme),
        blank(),
        section("TRACKS", theme),
        key("T", "Add track", theme),
        key("D", "Delete track", theme),
        key("C", "Clone track", theme),
        key("M", "Mute track", theme),
        key("S", "Solo track", theme),
        key("Q", "Quantize selection", theme),
        key("G", "Go to row 0", theme),
        blank(),
        section("INSERT MODE", theme),
        key("a–g", "Enter natural note (C D E F G A B)", theme),
        key("A–G (shift)", "Enter sharp (C# D# F# G# A#)", theme),
        key("` (backtick)", "Enter note-off (===)", theme),
        key("0–9", "Set octave", theme),
        key("0–F  (inst/vol/eff)", "Enter hex digit", theme),
        blank(),
        section("STEP SIZE", theme),
        key("{ / }", "Step -1 / +1", theme),
        key(":step N", "Set step to N (0–8)", theme),
    ]
}

fn right_column(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        section("TRANSPORT", theme),
        key("Space", "Play / Pause", theme),
        key("Enter  (stopped)", "Play from cursor row", theme),
        key("= / -", "BPM up / down", theme),
        key(":bpm <n>", "Set BPM directly", theme),
        key("[ / ]", "Prev / Next pattern", theme),
        key("Shift+P", "Toggle Pattern / Song", theme),
        key("Shift+L", "Toggle loop", theme),
        key("Shift+Up/Down", "Transpose +/- semitone", theme),
        key("Ctrl+Shift+Up/Down", "Transpose +/- octave", theme),
        blank(),
        section("VIEWS", theme),
        key("1", "Pattern editor", theme),
        key("2", "Arrangement", theme),
        key("3", "Instrument list", theme),
        key("4", "Code editor", theme),
        key("5", "Pattern list", theme),
        key("6", "Sample browser", theme),
        key("Ctrl+\\", "Toggle split view", theme),
        blank(),
        section("PROJECT", theme),
        key("Ctrl+S", "Save project", theme),
        key("Ctrl+O", "Load project", theme),
        key("Ctrl+F", "Load sample / MOD", theme),
        key("Ctrl+E", "Export audio", theme),
        blank(),
        section("LIVE / SCRIPT", theme),
        key("Ctrl+L", "Toggle Live mode", theme),
        key("Ctrl+Enter", "Execute script", theme),
        key("Ctrl+T", "Open templates", theme),
        blank(),
        section("VISUAL MODE", theme),
        key("hjkl", "Extend selection", theme),
        key("x / d", "Delete / Cut selection", theme),
        key("y / p", "Copy / Paste selection", theme),
        key("I", "Interpolate selection", theme),
        blank(),
        section("SAMPLE BROWSER (view 6)", theme),
        key("j / k", "Navigate", theme),
        key("l / Enter", "Enter dir / load", theme),
        key("h", "Go up a directory", theme),
        blank(),
        key("?", "Toggle this help", theme),
    ]
}
