mod app;
mod input;
mod ui;

use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use app::App;
use input::keybindings::{map_key_to_action, Action};

/// Tick rate for the event loop (250ms = 4 FPS)
///
/// This controls how often we update the application state and redraw the UI.
/// 250ms provides a good balance between responsiveness and CPU usage.
const TICK_RATE: Duration = Duration::from_millis(250);

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

/// Main application loop with tick rate control
///
/// This implements a standard TUI event loop pattern:
/// - Poll for events with a timeout (tick rate)
/// - Handle keyboard input events
/// - Update application state on each tick
/// - Render the UI
///
/// The loop continues while app.should_run() returns true.
/// Pressing 'q' will trigger app.quit() and exit the loop.
///
/// # Arguments
/// * `terminal` - The terminal instance to render to
/// * `app` - The application state
///
/// # Returns
/// Ok(()) if the app runs successfully
///
/// # Errors
/// Returns an error if rendering, event polling, or app logic fails
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    // Main event loop - runs until app signals it should quit
    while app.should_run() {
        // Render the UI
        terminal.draw(|frame| ui::render(frame, app))?;

        // Poll for events with a timeout equal to our tick rate
        // This ensures we update the UI regularly even if there are no events
        if event::poll(TICK_RATE)? {
            // An event is available, read it
            match event::read()? {
                Event::Key(key) => {
                    // Only handle KeyEventKind::Press to avoid duplicate events
                    // on some platforms that emit both Press and Release
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key);
                    }
                }
                Event::Resize(_width, _height) => {
                    // Terminal was resized
                    // The terminal backend will automatically handle the resize
                    // on the next draw() call, so we just need to acknowledge
                    // the event and continue to trigger a redraw
                }
                // Note: We ignore other event types (Mouse, etc.) for now
                // They will be handled in later phases
                _ => {}
            }
        }

        // Update application state (for animations, background tasks, etc.)
        app.update()?;
    }

    Ok(())
}

/// Handle keyboard input events
///
/// This processes keyboard input and updates the application state accordingly.
/// Uses the keybinding handler to map keys to actions, supporting vim-style
/// navigation (h/j/k/l) and standard keys (q for quit, etc.).
///
/// # Arguments
/// * `app` - The application state to update
/// * `key` - The keyboard event to process
fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Map the key event to an action using the keybinding handler
    let action = map_key_to_action(key);

    // Handle the action
    match action {
        // Vim-style navigation
        Action::MoveLeft => app.move_left(),
        Action::MoveDown => app.move_down(),
        Action::MoveUp => app.move_up(),
        Action::MoveRight => app.move_right(),

        // Application controls
        Action::Quit => app.quit(),

        // Modal controls
        Action::OpenModal => {
            // Open a test modal to demonstrate modal system
            app.open_test_modal();
        }
        Action::Cancel => {
            // Close the current modal if one is open
            // Otherwise, this is a no-op
            app.close_modal();
        }

        // Actions to be handled in future phases
        Action::Confirm => {
            // Will be used for confirming dialogs, selections, etc.
        }

        // Unmapped key - do nothing
        Action::None => {}
    }
}
