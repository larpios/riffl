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
/// For now, it renders a simple welcome screen with instructions.
/// In later phases, this will be expanded to render the full layout,
/// modals, and other UI components.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - The application state to render
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create a simple centered welcome screen
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Tracker RS - TUI Music Tracker")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));

    // Create welcome text
    let welcome_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Tracker RS!",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(""),
        Line::from("A terminal-based music tracker built with Rust"),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'q' to quit",
            Style::default().fg(Color::Green),
        )),
        Line::from(""),
        Line::from(Span::styled(
            if app.running { "Status: Running" } else { "Status: Stopped" },
            Style::default().fg(if app.running { Color::Green } else { Color::Red }),
        )),
    ];

    let paragraph = Paragraph::new(welcome_text)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
