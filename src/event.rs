use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use std::io;
use std::time::Duration;

/// Represents user input events in the pattern editor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Move cursor up (arrow up or 'k')
    MoveUp,
    /// Move cursor down (arrow down or 'j')
    MoveDown,
    /// Move cursor left (arrow left or 'h')
    MoveLeft,
    /// Move cursor right (arrow right or 'l')
    MoveRight,
    /// Delete current note (Delete or Backspace)
    Delete,
    /// Insert a new row (Insert)
    Insert,
    /// Quit the application ('q' or Ctrl+C)
    Quit,
    /// Resize terminal event
    Resize,
    /// Unknown or unhandled event
    None,
}

/// Read the next keyboard event with a timeout
///
/// This function polls for keyboard input and converts crossterm events
/// into our application-specific Event enum.
///
/// # Arguments
/// * `timeout` - Maximum time to wait for an event in milliseconds
///
/// # Returns
/// * `Ok(Event)` - The next event that occurred
/// * `Err(io::Error)` - If an I/O error occurred while reading events
pub fn read_event(timeout: Duration) -> io::Result<Event> {
    if event::poll(timeout)? {
        match event::read()? {
            CrosstermEvent::Key(key_event) => Ok(handle_key_event(key_event)),
            CrosstermEvent::Resize(_, _) => Ok(Event::Resize),
            _ => Ok(Event::None),
        }
    } else {
        Ok(Event::None)
    }
}

/// Convert a crossterm KeyEvent into our Event type
fn handle_key_event(key: KeyEvent) -> Event {
    // Handle Ctrl+C for quit
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key.code {
            return Event::Quit;
        }
    }

    match key.code {
        // Navigation - Arrow keys
        KeyCode::Up => Event::MoveUp,
        KeyCode::Down => Event::MoveDown,
        KeyCode::Left => Event::MoveLeft,
        KeyCode::Right => Event::MoveRight,

        // Navigation - vim-style hjkl
        KeyCode::Char('h') => Event::MoveLeft,
        KeyCode::Char('j') => Event::MoveDown,
        KeyCode::Char('k') => Event::MoveUp,
        KeyCode::Char('l') => Event::MoveRight,

        // Edit operations
        KeyCode::Delete | KeyCode::Backspace => Event::Delete,
        KeyCode::Insert => Event::Insert,

        // Quit
        KeyCode::Char('q') => Event::Quit,
        KeyCode::Esc => Event::Quit,

        // Note input: Check for pitch (A-G) followed by octave (0-9)
        // For now, we handle single character input and will build note strings
        // in the main loop
        KeyCode::Char(c) => {
            // Check if it's a pitch letter (A-G, case insensitive)
            if matches!(c.to_ascii_uppercase(), 'A'..='G') {
                // We'll need the octave digit next, but for now return a marker
                // The main loop will need to handle two-character note entry
                Event::None // Will be enhanced in integration phase
            } else if c.is_ascii_digit() {
                // Octave digit - will be handled in integration
                Event::None
            } else {
                Event::None
            }
        }

        _ => Event::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn test_arrow_key_navigation() {
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Up, KeyModifiers::NONE)),
            Event::MoveUp
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Down, KeyModifiers::NONE)),
            Event::MoveDown
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Left, KeyModifiers::NONE)),
            Event::MoveLeft
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Right, KeyModifiers::NONE)),
            Event::MoveRight
        );
    }

    #[test]
    fn test_vim_navigation() {
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('h'), KeyModifiers::NONE)),
            Event::MoveLeft
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('j'), KeyModifiers::NONE)),
            Event::MoveDown
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('k'), KeyModifiers::NONE)),
            Event::MoveUp
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('l'), KeyModifiers::NONE)),
            Event::MoveRight
        );
    }

    #[test]
    fn test_delete_keys() {
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Delete, KeyModifiers::NONE)),
            Event::Delete
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Backspace, KeyModifiers::NONE)),
            Event::Delete
        );
    }

    #[test]
    fn test_insert_key() {
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Insert, KeyModifiers::NONE)),
            Event::Insert
        );
    }

    #[test]
    fn test_quit_keys() {
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('q'), KeyModifiers::NONE)),
            Event::Quit
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Esc, KeyModifiers::NONE)),
            Event::Quit
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Event::Quit
        );
    }

    #[test]
    fn test_event_equality() {
        assert_eq!(Event::MoveUp, Event::MoveUp);
        assert_ne!(Event::MoveUp, Event::MoveDown);
        assert_eq!(Event::Delete, Event::Delete);
        assert_eq!(Event::Quit, Event::Quit);
    }

    #[test]
    fn test_unknown_keys() {
        // Keys that don't map to any action should return Event::None
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::Char('x'), KeyModifiers::NONE)),
            Event::None
        );
        assert_eq!(
            handle_key_event(create_key_event(KeyCode::F(1), KeyModifiers::NONE)),
            Event::None
        );
    }

    #[test]
    fn test_ctrl_other_keys() {
        // Ctrl+other keys (not Ctrl+C) should not trigger quit
        assert_ne!(
            handle_key_event(create_key_event(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Event::Quit
        );
    }
}
