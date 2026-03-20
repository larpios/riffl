/// Modal dialog system for popups and confirmations
///
/// This module provides a modal dialog system for displaying centered popups,
/// confirmation dialogs, and other modal interactions. Modals are rendered
/// on top of the main content and can be dismissed by user input.
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::layout::create_centered_rect;
use super::theme::Theme;

/// Type of modal dialog
///
/// Different modal types can have different visual styles and behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalType {
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Confirmation dialog: Enter = confirm, Esc = cancel
    Confirmation,
    /// Action menu: options are shown in the message body as `[key]  label`.
    /// The footer shows "Esc: cancel". Key routing is handled by the caller
    /// via `pending_*` state in App.
    Menu,
}

/// Modal dialog state
///
/// Represents a modal dialog with title, message, and visual styling.
/// Modals are rendered as centered popups on top of the main content.
#[derive(Debug, Clone)]
pub struct Modal {
    /// The title of the modal dialog
    pub title: String,
    /// The message content to display
    pub message: String,
    /// The type of modal (affects visual styling)
    pub modal_type: ModalType,
    /// Width as percentage of screen (0-100)
    pub width_percent: u16,
    /// Height as percentage of screen (0-100)
    pub height_percent: u16,
}

impl Modal {
    /// Create a new modal dialog
    ///
    /// # Arguments
    /// * `title` - The title text for the modal
    /// * `message` - The message content to display
    /// * `modal_type` - The type of modal (affects styling)
    ///
    /// # Returns
    /// A new Modal instance with default size (60% width, 40% height)
    ///
    /// # Example
    /// ```
    /// let modal = Modal::new(
    ///     "Welcome".to_string(),
    ///     "Welcome to Tracker RS!".to_string(),
    ///     ModalType::Info
    /// );
    /// ```
    pub fn new(title: String, message: String, modal_type: ModalType) -> Self {
        Self {
            title,
            message,
            modal_type,
            width_percent: 60,
            height_percent: 40,
        }
    }

    /// Create an info modal with custom message
    ///
    /// # Arguments
    /// * `title` - The title text
    /// * `message` - The message content
    ///
    /// # Returns
    /// A new Modal of type Info
    pub fn info(title: String, message: String) -> Self {
        Self::new(title, message, ModalType::Info)
    }

    /// Create a warning modal with custom message
    ///
    /// # Arguments
    /// * `title` - The title text
    /// * `message` - The message content
    ///
    /// # Returns
    /// A new Modal of type Warning
    pub fn warning(title: String, message: String) -> Self {
        Self::new(title, message, ModalType::Warning)
    }

    /// Create an error modal with custom message
    ///
    /// # Arguments
    /// * `title` - The title text
    /// * `message` - The message content
    ///
    /// # Returns
    /// A new Modal of type Error
    pub fn error(title: String, message: String) -> Self {
        Self::new(title, message, ModalType::Error)
    }

    /// Create a confirmation modal with custom message
    ///
    /// # Arguments
    /// * `title` - The title text
    /// * `message` - The message content
    ///
    /// # Returns
    /// A new Modal of type Confirmation
    pub fn confirmation(title: String, message: String) -> Self {
        Self::new(title, message, ModalType::Confirmation)
    }

    /// Set custom size for the modal
    ///
    /// # Arguments
    /// * `width_percent` - Width as percentage (0-100)
    /// * `height_percent` - Height as percentage (0-100)
    ///
    /// # Returns
    /// Self for method chaining
    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.width_percent = width_percent.min(100);
        self.height_percent = height_percent.min(100);
        self
    }

    /// Get the border color based on modal type and theme
    ///
    /// # Arguments
    /// * `theme` - The application theme to use for color selection
    ///
    /// # Returns
    /// The Color to use for the modal border
    fn border_color(&self, theme: &Theme) -> Color {
        match self.modal_type {
            ModalType::Info => theme.info_color(),
            ModalType::Warning => theme.warning_color(),
            ModalType::Error => theme.error_color(),
            ModalType::Confirmation => theme.warning_color(),
            ModalType::Menu => theme.primary,
        }
    }

    /// Get the title style based on modal type and theme
    ///
    /// # Arguments
    /// * `theme` - The application theme to use for styling
    ///
    /// # Returns
    /// The Style to use for the modal title
    fn title_style(&self, theme: &Theme) -> Style {
        Style::default()
            .fg(self.border_color(theme))
            .add_modifier(Modifier::BOLD)
    }

    /// Create a menu modal. Options are rendered in the message body as `[key]  label` lines.
    pub fn menu(title: String, message: String) -> Self {
        Self::new(title, message, ModalType::Menu)
    }
}

/// Render a modal dialog on top of the current UI
///
/// This function renders a modal dialog as a centered popup. It first clears
/// the area where the modal will appear, then renders the modal with its
/// border, title, message, and footer instructions.
///
/// # Arguments
/// * `frame` - The ratatui frame to render to
/// * `area` - The full screen area (modal will be centered within this)
/// * `modal` - The modal dialog to render
/// * `theme` - The application theme to use for styling
///
/// # Example
/// ```no_run
/// if let Some(modal) = &app.active_modal {
///     render_modal(frame, frame.area(), modal, &app.theme);
/// }
/// ```
pub fn render_modal(frame: &mut Frame, area: Rect, modal: &Modal, theme: &Theme) {
    // Create centered area for the modal
    let modal_area = create_centered_rect(area, modal.width_percent, modal.height_percent);

    // Clear the area where the modal will appear (creates the "overlay" effect)
    frame.render_widget(Clear, modal_area);

    // Create the modal border and title
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(modal.border_color(theme)))
        .title(format!(" {} ", modal.title))
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    // Calculate inner area for content
    let inner_area = block.inner(modal_area);

    // Render the border first
    frame.render_widget(block, modal_area);

    // Create the message content with footer instructions
    let mut lines = Vec::new();

    // Add the main message
    lines.push(Line::from(""));
    for line in modal.message.lines() {
        lines.push(Line::from(line.to_string()));
    }
    lines.push(Line::from(""));

    // Add footer with instructions based on modal type
    lines.push(Line::from(""));
    let footer_text = match modal.modal_type {
        ModalType::Confirmation => {
            vec![
                Span::styled("Enter", Style::default().fg(theme.success_color())),
                Span::raw(": confirm   "),
                Span::styled("Esc", Style::default().fg(theme.error_color())),
                Span::raw(": cancel"),
            ]
        }
        ModalType::Menu => {
            vec![
                Span::styled("Esc", Style::default().fg(theme.error_color())),
                Span::raw(": cancel"),
            ]
        }
        _ => {
            vec![
                Span::styled("Enter", Style::default().fg(theme.success_color())),
                Span::raw(": ok   "),
                Span::styled("Esc", Style::default().fg(theme.error_color())),
                Span::raw(": close"),
            ]
        }
    };
    lines.push(Line::from(footer_text));

    // Create and render the paragraph
    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(theme.text_style());

    frame.render_widget(paragraph, inner_area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_new() {
        let modal = Modal::new(
            "Test".to_string(),
            "Test message".to_string(),
            ModalType::Info,
        );

        assert_eq!(modal.title, "Test");
        assert_eq!(modal.message, "Test message");
        assert_eq!(modal.modal_type, ModalType::Info);
        assert_eq!(modal.width_percent, 60);
        assert_eq!(modal.height_percent, 40);
    }

    #[test]
    fn test_modal_info() {
        let modal = Modal::info("Info".to_string(), "Info message".to_string());
        assert_eq!(modal.modal_type, ModalType::Info);
    }

    #[test]
    fn test_modal_warning() {
        let modal = Modal::warning("Warning".to_string(), "Warning message".to_string());
        assert_eq!(modal.modal_type, ModalType::Warning);
    }

    #[test]
    fn test_modal_error() {
        let modal = Modal::error("Error".to_string(), "Error message".to_string());
        assert_eq!(modal.modal_type, ModalType::Error);
    }

    #[test]
    fn test_modal_confirmation() {
        let modal = Modal::confirmation("Confirm".to_string(), "Confirm?".to_string());
        assert_eq!(modal.modal_type, ModalType::Confirmation);
    }

    #[test]
    fn test_modal_with_size() {
        let modal = Modal::info("Test".to_string(), "Test".to_string()).with_size(80, 60);

        assert_eq!(modal.width_percent, 80);
        assert_eq!(modal.height_percent, 60);
    }

    #[test]
    fn test_modal_with_size_clamps() {
        let modal = Modal::info("Test".to_string(), "Test".to_string()).with_size(150, 150);

        assert_eq!(modal.width_percent, 100);
        assert_eq!(modal.height_percent, 100);
    }

    #[test]
    fn test_border_colors() {
        let theme = Theme::default();
        assert_eq!(
            Modal::info("".to_string(), "".to_string()).border_color(&theme),
            Color::Cyan
        );
        assert_eq!(
            Modal::warning("".to_string(), "".to_string()).border_color(&theme),
            Color::Yellow
        );
        assert_eq!(
            Modal::error("".to_string(), "".to_string()).border_color(&theme),
            Color::Red
        );
        assert_eq!(
            Modal::confirmation("".to_string(), "".to_string()).border_color(&theme),
            Color::Yellow
        );
    }

    #[test]
    fn test_modal_type_variants() {
        assert_ne!(ModalType::Info, ModalType::Warning);
        assert_ne!(ModalType::Warning, ModalType::Error);
        assert_ne!(ModalType::Error, ModalType::Confirmation);
        assert_eq!(ModalType::Info, ModalType::Info);
    }
}
