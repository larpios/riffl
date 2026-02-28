/// Theme and color scheme management
///
/// This module provides theme support for the TUI, including color definitions
/// that work across both 256-color and truecolor terminals. The theme system
/// allows for consistent styling throughout the application.

use ratatui::style::{Color, Modifier, Style};

/// Color palette for the application theme
///
/// The Theme struct contains all color definitions used throughout the UI.
/// Colors are defined to work well in both 256-color and truecolor terminals.
///
/// # Example
/// ```
/// let theme = Theme::default();
/// let header_style = theme.header_style();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // Primary UI colors - to be defined in next subtask
    // Borders and UI chrome
    // Text colors
    // Status colors
    // Modal colors
}

impl Theme {
    /// Create a new theme with default colors
    ///
    /// This creates the default theme with a carefully chosen color palette
    /// that works well in both dark and light terminals and supports both
    /// 256-color and truecolor modes.
    ///
    /// # Returns
    /// A new Theme instance with default color definitions
    pub fn new() -> Self {
        Self {}
    }

    /// Get the style for header elements
    ///
    /// Returns the default style for header areas including title bars
    /// and section headers.
    ///
    /// # Returns
    /// Style configured for header elements
    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    }

    /// Get the style for footer elements
    ///
    /// Returns the default style for footer areas including status bars
    /// and keyboard hint displays.
    ///
    /// # Returns
    /// Style configured for footer elements
    pub fn footer_style(&self) -> Style {
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
    }

    /// Get the border color for normal UI elements
    ///
    /// # Returns
    /// Color for standard UI borders
    pub fn border_color(&self) -> Color {
        Color::Cyan
    }

    /// Get the border style for UI elements
    ///
    /// # Returns
    /// Style configured for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border_color())
    }

    /// Get the style for highlighted/selected elements
    ///
    /// Returns the style used for cursor position, selected items,
    /// and other highlighted UI elements.
    ///
    /// # Returns
    /// Style configured for highlighted elements
    pub fn highlight_style(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
    }

    /// Get the style for normal text content
    ///
    /// # Returns
    /// Style configured for regular text
    pub fn text_style(&self) -> Style {
        Style::default().fg(Color::White)
    }

    /// Get the style for dimmed/secondary text
    ///
    /// Used for less important information or disabled elements.
    ///
    /// # Returns
    /// Style configured for dimmed text
    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }

    /// Get color for success/positive status
    ///
    /// # Returns
    /// Color for success states
    pub fn success_color(&self) -> Color {
        Color::Green
    }

    /// Get color for warning status
    ///
    /// # Returns
    /// Color for warning states
    pub fn warning_color(&self) -> Color {
        Color::Yellow
    }

    /// Get color for error/danger status
    ///
    /// # Returns
    /// Color for error states
    pub fn error_color(&self) -> Color {
        Color::Red
    }

    /// Get color for info status
    ///
    /// # Returns
    /// Color for informational elements
    pub fn info_color(&self) -> Color {
        Color::Cyan
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_new() {
        let theme = Theme::new();
        assert_eq!(theme, Theme::default());
    }

    #[test]
    fn test_theme_default() {
        let theme1 = Theme::default();
        let theme2 = Theme::default();
        assert_eq!(theme1, theme2);
    }

    #[test]
    fn test_header_style() {
        let theme = Theme::default();
        let style = theme.header_style();
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn test_footer_style() {
        let theme = Theme::default();
        let style = theme.footer_style();
        assert_eq!(style.fg, Some(Color::White));
        assert_eq!(style.bg, Some(Color::DarkGray));
    }

    #[test]
    fn test_border_color() {
        let theme = Theme::default();
        assert_eq!(theme.border_color(), Color::Cyan);
    }

    #[test]
    fn test_border_style() {
        let theme = Theme::default();
        let style = theme.border_style();
        assert_eq!(style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_highlight_style() {
        let theme = Theme::default();
        let style = theme.highlight_style();
        assert_eq!(style.fg, Some(Color::Black));
        assert_eq!(style.bg, Some(Color::Yellow));
    }

    #[test]
    fn test_text_style() {
        let theme = Theme::default();
        let style = theme.text_style();
        assert_eq!(style.fg, Some(Color::White));
    }

    #[test]
    fn test_dimmed_style() {
        let theme = Theme::default();
        let style = theme.dimmed_style();
        assert_eq!(style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_status_colors() {
        let theme = Theme::default();
        assert_eq!(theme.success_color(), Color::Green);
        assert_eq!(theme.warning_color(), Color::Yellow);
        assert_eq!(theme.error_color(), Color::Red);
        assert_eq!(theme.info_color(), Color::Cyan);
    }

    #[test]
    fn test_theme_clone() {
        let theme1 = Theme::default();
        let theme2 = theme1;
        assert_eq!(theme1, theme2);
    }
}
