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
/// Uses a mix of named colors and indexed colors (256-color palette) which
/// automatically work in truecolor terminals as well.
///
/// # Example
/// ```
/// let theme = Theme::default();
/// let header_style = theme.header_style();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // Primary UI colors
    /// Primary accent color for interactive elements
    pub primary: Color,
    /// Secondary accent color for less prominent elements
    pub secondary: Color,

    // Borders and UI chrome
    /// Border color for normal UI elements
    pub border: Color,
    /// Border color for focused/active elements
    pub border_focused: Color,

    // Text colors
    /// Primary text color for main content
    pub text: Color,
    /// Secondary text color for less important content
    pub text_secondary: Color,
    /// Dimmed text color for disabled/inactive elements
    pub text_dimmed: Color,

    // Background colors
    /// Background color for highlighted/selected elements
    pub bg_highlight: Color,
    /// Background color for the header area
    pub bg_header: Color,
    /// Background color for the footer area
    pub bg_footer: Color,

    // Status colors
    /// Color for success/positive status
    pub status_success: Color,
    /// Color for warning status
    pub status_warning: Color,
    /// Color for error/danger status
    pub status_error: Color,
    /// Color for informational status
    pub status_info: Color,
}

impl Theme {
    /// Create a new theme with default colors
    ///
    /// This creates the default theme with a carefully chosen color palette
    /// that works well in both dark and light terminals and supports both
    /// 256-color and truecolor modes.
    ///
    /// The color palette uses:
    /// - Named colors (Color::White, Color::Cyan, etc.) for basic colors
    /// - Indexed colors (Color::Indexed) for 256-color terminal support
    /// - All colors automatically work in truecolor terminals
    ///
    /// # Returns
    /// A new Theme instance with default color definitions
    pub fn new() -> Self {
        Self {
            // Primary UI colors - using indexed colors for better 256-color support
            primary: Color::Cyan,          // Cyan for primary accent
            secondary: Color::Blue,        // Blue for secondary accent

            // Borders and UI chrome
            border: Color::Cyan,           // Cyan for normal borders
            border_focused: Color::Yellow, // Yellow for focused borders

            // Text colors
            text: Color::White,                 // White for main text
            text_secondary: Color::Gray,        // Gray for secondary text
            text_dimmed: Color::DarkGray,       // Dark gray for dimmed text

            // Background colors
            bg_highlight: Color::Yellow,   // Yellow background for highlights
            bg_header: Color::Reset,       // Default background for header
            bg_footer: Color::DarkGray,    // Dark gray background for footer

            // Status colors
            status_success: Color::Green,  // Green for success
            status_warning: Color::Yellow, // Yellow for warning
            status_error: Color::Red,      // Red for errors
            status_info: Color::Cyan,      // Cyan for info
        }
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
            .fg(self.text)
            .bg(self.bg_header)
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
            .fg(self.text)
            .bg(self.bg_footer)
    }

    /// Get the border color for normal UI elements
    ///
    /// # Returns
    /// Color for standard UI borders
    pub fn border_color(&self) -> Color {
        self.border
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
            .bg(self.bg_highlight)
    }

    /// Get the style for normal text content
    ///
    /// # Returns
    /// Style configured for regular text
    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text)
    }

    /// Get the style for dimmed/secondary text
    ///
    /// Used for less important information or disabled elements.
    ///
    /// # Returns
    /// Style configured for dimmed text
    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(self.text_dimmed)
    }

    /// Get color for success/positive status
    ///
    /// # Returns
    /// Color for success states
    pub fn success_color(&self) -> Color {
        self.status_success
    }

    /// Get color for warning status
    ///
    /// # Returns
    /// Color for warning states
    pub fn warning_color(&self) -> Color {
        self.status_warning
    }

    /// Get color for error/danger status
    ///
    /// # Returns
    /// Color for error states
    pub fn error_color(&self) -> Color {
        self.status_error
    }

    /// Get color for info status
    ///
    /// # Returns
    /// Color for informational elements
    pub fn info_color(&self) -> Color {
        self.status_info
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

    #[test]
    fn test_color_palette_fields() {
        let theme = Theme::default();

        // Test primary UI colors
        assert_eq!(theme.primary, Color::Cyan);
        assert_eq!(theme.secondary, Color::Blue);

        // Test border colors
        assert_eq!(theme.border, Color::Cyan);
        assert_eq!(theme.border_focused, Color::Yellow);

        // Test text colors
        assert_eq!(theme.text, Color::White);
        assert_eq!(theme.text_secondary, Color::Gray);
        assert_eq!(theme.text_dimmed, Color::DarkGray);

        // Test background colors
        assert_eq!(theme.bg_highlight, Color::Yellow);
        assert_eq!(theme.bg_header, Color::Reset);
        assert_eq!(theme.bg_footer, Color::DarkGray);

        // Test status colors
        assert_eq!(theme.status_success, Color::Green);
        assert_eq!(theme.status_warning, Color::Yellow);
        assert_eq!(theme.status_error, Color::Red);
        assert_eq!(theme.status_info, Color::Cyan);
    }

    #[test]
    fn test_256_color_compatibility() {
        let theme = Theme::default();

        // All colors should work in 256-color terminals
        // Named colors (White, Cyan, etc.) are supported in all terminals
        // This test verifies the color assignments are compatible
        assert!(matches!(
            theme.text,
            Color::White | Color::Indexed(_) | Color::Rgb(_, _, _)
        ));
        assert!(matches!(
            theme.border,
            Color::Cyan | Color::Indexed(_) | Color::Rgb(_, _, _)
        ));
        assert!(matches!(
            theme.status_success,
            Color::Green | Color::Indexed(_) | Color::Rgb(_, _, _)
        ));
    }

    #[test]
    fn test_style_methods_use_theme_colors() {
        let theme = Theme::default();

        // Verify that style methods use theme color fields
        assert_eq!(theme.header_style().fg, Some(theme.text));
        assert_eq!(theme.footer_style().bg, Some(theme.bg_footer));
        assert_eq!(theme.border_style().fg, Some(theme.border));
        assert_eq!(theme.highlight_style().bg, Some(theme.bg_highlight));
        assert_eq!(theme.text_style().fg, Some(theme.text));
        assert_eq!(theme.dimmed_style().fg, Some(theme.text_dimmed));
    }
}
