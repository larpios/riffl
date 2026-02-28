/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.

use anyhow::Result;

use crate::ui::modal::Modal;
use crate::ui::theme::Theme;

/// Application state
///
/// The App struct is the central state manager for the TUI application.
/// It coordinates between different subsystems (UI, input, etc.) and
/// maintains the application's runtime state.
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,

    /// Whether the application is running (for state management)
    pub running: bool,

    /// Cursor X position (for vim-style navigation)
    pub cursor_x: u16,

    /// Cursor Y position (for vim-style navigation)
    pub cursor_y: u16,

    /// Stack of active modal dialogs (top modal is last in Vec)
    ///
    /// Modals are stacked so that multiple modals can be opened on top of
    /// each other. The last modal in the Vec is the currently active one.
    /// When a modal is closed, it's popped from the stack.
    modal_stack: Vec<Modal>,

    /// The application's color theme
    ///
    /// This theme is used throughout the UI to maintain consistent styling
    /// and color choices. It supports both 256-color and truecolor terminals.
    pub theme: Theme,
}

impl App {
    /// Create a new App instance with default state
    ///
    /// # Returns
    /// A new App instance ready to be initialized
    pub fn new() -> Self {
        Self {
            should_quit: false,
            running: false,
            cursor_x: 0,
            cursor_y: 0,
            modal_stack: Vec::new(),
            theme: Theme::default(),
        }
    }

    /// Initialize the application
    ///
    /// This method sets up the initial application state and prepares
    /// it for the event loop. Call this after creating a new App instance
    /// but before starting the event loop.
    ///
    /// # Returns
    /// Ok(()) if initialization succeeds
    ///
    /// # Errors
    /// Returns an error if initialization fails
    pub fn init(&mut self) -> Result<()> {
        self.running = true;
        Ok(())
    }

    /// Update application state
    ///
    /// This method is called each tick of the event loop to update
    /// application state. Can be used for time-based updates, animations,
    /// or other periodic tasks.
    ///
    /// # Returns
    /// Ok(()) if update succeeds
    ///
    /// # Errors
    /// Returns an error if update fails
    pub fn update(&mut self) -> Result<()> {
        // Update logic will be expanded in future phases
        Ok(())
    }

    /// Check if the application should continue running
    ///
    /// # Returns
    /// true if the app should keep running, false if it should exit
    pub fn should_run(&self) -> bool {
        self.running && !self.should_quit
    }

    /// Handle application quit
    ///
    /// This method initiates a graceful shutdown of the application.
    /// It sets the quit flag which will cause the event loop to exit
    /// and trigger cleanup procedures.
    pub fn quit(&mut self) {
        self.should_quit = true;
        self.running = false;
    }

    /// Move cursor left (vim: h)
    ///
    /// Decrements the cursor X position unless already at the leftmost position.
    /// This implements vim-style h key navigation.
    pub fn move_left(&mut self) {
        self.cursor_x = self.cursor_x.saturating_sub(1);
    }

    /// Move cursor down (vim: j)
    ///
    /// Increments the cursor Y position unless already at the maximum.
    /// This implements vim-style j key navigation.
    /// Bounded to a maximum of 9 for the demo grid.
    pub fn move_down(&mut self) {
        if self.cursor_y < 9 {
            self.cursor_y = self.cursor_y.saturating_add(1);
        }
    }

    /// Move cursor up (vim: k)
    ///
    /// Decrements the cursor Y position unless already at the topmost position.
    /// This implements vim-style k key navigation.
    pub fn move_up(&mut self) {
        self.cursor_y = self.cursor_y.saturating_sub(1);
    }

    /// Move cursor right (vim: l)
    ///
    /// Increments the cursor X position unless already at the maximum.
    /// This implements vim-style l key navigation.
    /// Bounded to a maximum of 9 for the demo grid.
    pub fn move_right(&mut self) {
        if self.cursor_x < 9 {
            self.cursor_x = self.cursor_x.saturating_add(1);
        }
    }

    /// Open a modal dialog by adding it to the modal stack
    ///
    /// This pushes a new modal onto the stack, making it the currently
    /// active modal. The modal will be rendered on top of the current UI.
    ///
    /// # Arguments
    /// * `modal` - The modal dialog to open
    ///
    /// # Example
    /// ```no_run
    /// app.open_modal(Modal::info(
    ///     "Welcome".to_string(),
    ///     "Welcome to Tracker RS!".to_string()
    /// ));
    /// ```
    pub fn open_modal(&mut self, modal: Modal) {
        self.modal_stack.push(modal);
    }

    /// Close the current modal dialog by removing it from the modal stack
    ///
    /// This pops the top modal from the stack. If there are multiple modals
    /// stacked, the next one becomes active. If there are no modals, this
    /// is a no-op.
    ///
    /// # Returns
    /// The closed modal, or None if there were no modals open
    pub fn close_modal(&mut self) -> Option<Modal> {
        self.modal_stack.pop()
    }

    /// Get the currently active modal dialog, if any
    ///
    /// This returns a reference to the top modal on the stack without
    /// removing it. Returns None if there are no modals open.
    ///
    /// # Returns
    /// A reference to the current modal, or None if no modal is open
    pub fn current_modal(&self) -> Option<&Modal> {
        self.modal_stack.last()
    }

    /// Check if any modal is currently open
    ///
    /// # Returns
    /// true if at least one modal is open, false otherwise
    pub fn has_modal(&self) -> bool {
        !self.modal_stack.is_empty()
    }

    /// Open a test modal (for demonstration purposes)
    ///
    /// This is a convenience method for testing modal functionality.
    /// It opens a welcome modal with sample content.
    pub fn open_test_modal(&mut self) {
        let modal = Modal::info(
            "Welcome to Tracker RS".to_string(),
            "This is a test modal dialog!\n\nYou can:\n• Press 'm' to open this modal\n• Press ESC to close it\n• Stack multiple modals\n\nModal dialogs are perfect for showing messages,\nconfirmations, and help text.".to_string()
        ).with_size(70, 50);

        self.open_modal(modal);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
