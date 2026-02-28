/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.

use anyhow::Result;

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
    pub fn move_down(&mut self) {
        self.cursor_y = self.cursor_y.saturating_add(1);
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
    pub fn move_right(&mut self) {
        self.cursor_x = self.cursor_x.saturating_add(1);
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
