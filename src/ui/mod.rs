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
pub mod modal;
pub mod theme;

/// Render the application UI
///
/// This is the main rendering function that draws the entire UI.
/// It uses a three-part layout with header, content, and footer areas
/// that adapts responsively to terminal size changes.
///
/// If a modal is active, it will be rendered on top of the main UI.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `app` - The application state to render
pub fn render(frame: &mut Frame, app: &App) {
    let full_area = frame.area();

    // Create main layout with header (3 lines), content (flexible), and footer (1 line)
    let (header_area, content_area, footer_area) = layout::create_main_layout(full_area, 3, 1);

    // Render header
    render_header(frame, header_area);

    // Render main content
    render_content(frame, content_area, app);

    // Render footer
    render_footer(frame, footer_area, app);

    // Render modal on top if one is active
    if let Some(active_modal) = app.current_modal() {
        modal::render_modal(frame, full_area, active_modal);
    }
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
/// For now, this shows a navigable grid with cursor highlighting to demonstrate
/// vim-style navigation. In later phases, this will display the pattern editor,
/// instrument list, and other tracker components.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The rectangular area to render the content in
/// * `app` - The application state to render
fn render_content(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Main Content - Use hjkl or arrows to navigate ")
        .title_alignment(Alignment::Left);

    // Calculate the inner area (excluding borders)
    let inner = content_block.inner(area);

    // Create a navigable grid to demonstrate cursor movement
    // Each cell shows its coordinates, and the cursor position is highlighted
    let mut lines = Vec::new();

    // Add a header
    lines.push(Line::from(vec![
        Span::styled(
            "Welcome to Tracker RS!",
            Style::default().fg(Color::Yellow),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from("Navigate this grid using vim keys (hjkl) or arrow keys:"));
    lines.push(Line::from(""));

    // Create a simple navigable grid (10x10)
    // The cursor position will be highlighted with a different background
    let grid_size = 10;
    for y in 0..grid_size {
        let mut row_spans = Vec::new();

        for x in 0..grid_size {
            // Check if this is the cursor position
            let is_cursor = app.cursor_x == x && app.cursor_y == y;

            // Create the cell content (coordinates)
            let cell_text = format!("{:02},{:02} ", x, y);

            // Style the cell - highlight if it's the cursor position
            let cell_style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow) // Bright highlight for cursor
            } else {
                Style::default().fg(Color::DarkGray)
            };

            row_spans.push(Span::styled(cell_text, cell_style));
        }

        lines.push(Line::from(row_spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        if app.running { "● Status: Running" } else { "○ Status: Stopped" },
        Style::default().fg(if app.running { Color::Green } else { Color::Red }),
    )));

    let paragraph = Paragraph::new(lines)
        .block(content_block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

/// Render the footer area
///
/// The footer displays status information and keyboard shortcuts.
/// It provides contextual help to the user and shows the current cursor position.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The rectangular area to render the footer in
/// * `app` - The application state (for displaying cursor position)
fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let footer_text = vec![
        Span::raw(" "),
        Span::styled("hjkl/arrows", Style::default().fg(Color::Green)),
        Span::raw(": Navigate "),
        Span::raw(" | "),
        Span::styled("m", Style::default().fg(Color::Green)),
        Span::raw(": Modal "),
        Span::raw(" | "),
        Span::styled("ESC", Style::default().fg(Color::Green)),
        Span::raw(": Close "),
        Span::raw(" | "),
        Span::styled("q", Style::default().fg(Color::Green)),
        Span::raw(": Quit "),
        Span::raw(" | "),
        Span::styled(
            format!("Cursor: ({}, {})", app.cursor_x, app.cursor_y),
            Style::default().fg(Color::Yellow),
        ),
    ];

    let footer = Paragraph::new(Line::from(footer_text))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));

    frame.render_widget(footer, area);
}
