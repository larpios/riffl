/// Layout management for responsive TUI layouts
///
/// This module provides utilities for creating constraint-based layouts that
/// adapt to terminal size changes. It uses ratatui's Layout system to create
/// flexible, responsive layouts for the application.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

/// Create a vertical layout with header, content, and footer areas
///
/// This is a common TUI layout pattern with a fixed-height header at the top,
/// a flexible content area in the middle, and a fixed-height footer at the bottom.
///
/// # Arguments
/// * `area` - The available screen area to split
/// * `header_height` - Height of the header in lines
/// * `footer_height` - Height of the footer in lines
///
/// # Returns
/// A tuple of three Rect areas: (header, content, footer)
///
/// # Example
/// ```no_run
/// let (header, content, footer) = create_main_layout(frame.area(), 3, 1);
/// ```
pub fn create_main_layout(area: Rect, header_height: u16, footer_height: u16) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(0),
            Constraint::Length(footer_height),
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

/// Create a horizontal layout with sidebar and main content
///
/// This layout provides a fixed-width sidebar on the left and a flexible
/// main content area on the right. Useful for navigation menus or tool palettes.
///
/// # Arguments
/// * `area` - The available screen area to split
/// * `sidebar_width` - Width of the sidebar in columns
///
/// # Returns
/// A tuple of two Rect areas: (sidebar, content)
///
/// # Example
/// ```no_run
/// let (sidebar, content) = create_sidebar_layout(area, 20);
/// ```
pub fn create_sidebar_layout(area: Rect, sidebar_width: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(sidebar_width),
            Constraint::Min(0),
        ])
        .split(area);

    (chunks[0], chunks[1])
}

/// Create a centered popup area
///
/// This function creates a centered rectangular area that's suitable for
/// modal dialogs and popups. The popup size is specified as a percentage
/// of the available area.
///
/// # Arguments
/// * `area` - The available screen area
/// * `percent_x` - Width as percentage of area (0-100)
/// * `percent_y` - Height as percentage of area (0-100)
///
/// # Returns
/// A Rect representing the centered popup area
///
/// # Example
/// ```no_run
/// let popup_area = create_centered_rect(frame.area(), 60, 40);
/// ```
pub fn create_centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Create a responsive grid layout
///
/// Creates a grid layout with the specified number of rows and columns.
/// Each cell in the grid gets an equal share of the available space.
///
/// # Arguments
/// * `area` - The available screen area
/// * `rows` - Number of rows in the grid
/// * `cols` - Number of columns in the grid
///
/// # Returns
/// A Vec of Rect areas representing each cell in the grid (row-major order)
///
/// # Example
/// ```no_run
/// let cells = create_grid_layout(area, 2, 3); // 2 rows, 3 columns
/// ```
pub fn create_grid_layout(area: Rect, rows: usize, cols: usize) -> Vec<Rect> {
    let mut cells = Vec::with_capacity(rows * cols);

    // Create row constraints (equal height for each row)
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Percentage(100 / rows as u16))
        .collect();

    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    // For each row, create column constraints
    for row_chunk in row_chunks {
        let col_constraints: Vec<Constraint> = (0..cols)
            .map(|_| Constraint::Percentage(100 / cols as u16))
            .collect();

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(row_chunk);

        cells.extend(col_chunks);
    }

    cells
}

/// Create a split layout with adjustable divider
///
/// Creates a layout split either horizontally or vertically with an adjustable
/// divider position. Useful for resizable panels.
///
/// # Arguments
/// * `area` - The available screen area
/// * `direction` - Direction::Horizontal for left/right split, Direction::Vertical for top/bottom
/// * `split_percent` - Position of the divider as percentage (0-100)
///
/// # Returns
/// A tuple of two Rect areas: (first_panel, second_panel)
///
/// # Example
/// ```no_run
/// // 70/30 vertical split
/// let (top, bottom) = create_split_layout(area, Direction::Vertical, 70);
/// ```
pub fn create_split_layout(area: Rect, direction: Direction, split_percent: u16) -> (Rect, Rect) {
    let split_percent = split_percent.min(100); // Ensure we don't exceed 100%

    let chunks = Layout::default()
        .direction(direction)
        .constraints([
            Constraint::Percentage(split_percent),
            Constraint::Percentage(100 - split_percent),
        ])
        .split(area);

    (chunks[0], chunks[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_main_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let (header, content, footer) = create_main_layout(area, 3, 1);

        assert_eq!(header.height, 3);
        assert_eq!(footer.height, 1);
        assert!(content.height > 0);
        assert_eq!(header.height + content.height + footer.height, area.height);
    }

    #[test]
    fn test_create_sidebar_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let (sidebar, content) = create_sidebar_layout(area, 20);

        assert_eq!(sidebar.width, 20);
        assert!(content.width > 0);
        assert_eq!(sidebar.width + content.width, area.width);
    }

    #[test]
    fn test_create_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let popup = create_centered_rect(area, 60, 40);

        // Should be roughly centered (within rounding errors)
        assert!(popup.width <= 60);
        assert!(popup.height <= 20);
        assert!(popup.x > 0);
        assert!(popup.y > 0);
    }

    #[test]
    fn test_create_grid_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let cells = create_grid_layout(area, 2, 3);

        assert_eq!(cells.len(), 6); // 2 rows * 3 cols
    }

    #[test]
    fn test_create_split_layout() {
        let area = Rect::new(0, 0, 80, 24);
        let (left, right) = create_split_layout(area, Direction::Horizontal, 70);

        assert!(left.width > right.width);
        assert_eq!(left.width + right.width, area.width);
    }

    #[test]
    fn test_split_layout_clamps_percent() {
        let area = Rect::new(0, 0, 80, 24);
        let (first, second) = create_split_layout(area, Direction::Horizontal, 150);

        // Should clamp to 100% max
        assert_eq!(first.width + second.width, area.width);
    }
}
