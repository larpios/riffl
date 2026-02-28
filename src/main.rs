mod app;
mod input;
mod ui;

use std::io;
use std::panic;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use app::App;

fn main() -> Result<()> {
    // Set up panic hook to restore terminal before panicking
    // This is critical - if we panic without restoring the terminal,
    // the user's terminal will be left in a broken state
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore terminal state
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);

        // Call the original panic hook
        original_hook(panic_info);
    }));

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create and initialize app
    let mut app = App::new();
    app.init()?;

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal()?;

    // Propagate any errors from the app
    result
}

/// Initialize the terminal with crossterm backend
///
/// This sets up:
/// - Raw mode (disables line buffering and echo)
/// - Alternate screen (preserves user's terminal contents)
/// - Crossterm backend for ratatui
///
/// # Returns
/// A configured Terminal instance ready for rendering
///
/// # Errors
/// Returns an error if terminal initialization fails
fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal state
///
/// This cleans up:
/// - Exits alternate screen (restores user's terminal contents)
/// - Disables raw mode (restores normal terminal behavior)
///
/// # Errors
/// Returns an error if terminal restoration fails
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Main application loop
///
/// This is a minimal event loop that will be expanded in the next subtask.
/// For now, it just renders and immediately exits to verify terminal
/// initialization and cleanup work correctly.
///
/// # Arguments
/// * `terminal` - The terminal instance to render to
/// * `app` - The application state
///
/// # Returns
/// Ok(()) if the app runs successfully
///
/// # Errors
/// Returns an error if rendering or app logic fails
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    // For now, just clear the terminal to verify initialization works
    terminal.clear()?;

    // The event loop will be implemented in the next subtask (subtask-2-3)
    // For now, we just verify that the terminal initializes and cleans up properly

    Ok(())
}
