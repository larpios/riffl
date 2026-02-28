/// UI rendering and components
///
/// This module contains all UI-related code including layout management,
/// theming, and modal dialogs.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

// Submodules
pub mod layout;

// Submodules to be added in later phases:
// pub mod modal;
// pub mod theme;

/// Render the application UI
///
/// This is the main rendering function that draws the entire UI.
/// It uses a three-part layout with header, content, and footer areas
/// that adapts responsively to terminal size changes.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - The application state to render
pub fn render(frame: &mut Frame, app: &App) {
    // Create main layout with header (3 lines), content (flexible), and footer (1 line)
    let (header_area, content_area, footer_area) = layout::create_main_layout(frame.area(), 3, 1);

    // Render header
    render_header(frame, header_area);

    // Render main content
    render_content(frame, content_area, app);

    // Render footer
    render_footer(frame, footer_area, app);
}

/// Render the header area
///
/// The header displays the application title and branding.
/// It uses a bordered block with centered text.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The rectangular area to render the header in
fn render_header(frame: &mut Frame, area: ratatui::layout::Rect) {
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Tracker RS - TUI Music Tracker ")
        .title_alignment(Alignment::Center);

    let header_text = Paragraph::new("A terminal-based music tracker built with Rust")
        .block(header_block)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(header_text, area);
}

/// Render the main content area
///
/// The content area displays the primary application content.
/// For now, this shows a welcome message. In later phases, this will
/// display the pattern editor, instrument list, and other tracker components.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The rectangular area to render the content in
/// * `app` - The application state to render
fn render_content(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Main Content ")
        .title_alignment(Alignment::Left);

    // Create welcome text
    let welcome_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Tracker RS!",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("This is a terminal-based music tracker with vim-style keybindings."),
        Line::from(""),
        Line::from("The application uses a responsive layout that adapts to terminal size."),
        Line::from("Try resizing your terminal window to see the layout adapt gracefully."),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            if app.running { "● Status: Running" } else { "○ Status: Stopped" },
            Style::default().fg(if app.running { Color::Green } else { Color::Red }),
        )),
    ];

    let paragraph = Paragraph::new(welcome_text)
        .block(content_block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

/// Render the footer area
///
/// The footer displays status information and keyboard shortcuts.
/// It provides contextual help to the user.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The rectangular area to render the footer in
/// * `app` - The application state (for future context-sensitive help)
fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, _app: &App) {
    let footer_text = vec![
        Span::raw(" "),
        Span::styled("q", Style::default().fg(Color::Green)),
        Span::raw(": Quit "),
        Span::raw(" | "),
        Span::raw("TUI Framework: Ratatui + Crossterm"),
    ];

    let footer = Paragraph::new(Line::from(footer_text))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(footer, area);
}
