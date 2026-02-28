/// Main application state and logic
///
/// This module contains the core App struct that manages the application state,
/// handles updates, and coordinates between different subsystems.

/// Application state
pub struct App {
    /// Whether the application should exit
    pub should_quit: bool,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            should_quit: false,
        }
    }

    /// Handle application quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
