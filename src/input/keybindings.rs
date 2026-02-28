/// Keybinding handling with vim-style navigation
///
/// This module provides keybinding infrastructure for the application,
/// with support for vim-style navigation keys (h/j/k/l) and extensible
/// action mapping for future features.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Actions that can be triggered by keybindings
///
/// This enum represents all possible user actions that can be triggered
/// via keyboard input. It provides a layer of abstraction between raw
/// key events and application logic, making it easy to rebind keys or
/// add new actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move cursor/selection left (vim: h)
    MoveLeft,

    /// Move cursor/selection down (vim: j)
    MoveDown,

    /// Move cursor/selection up (vim: k)
    MoveUp,

    /// Move cursor/selection right (vim: l)
    MoveRight,

    /// Quit the application
    Quit,

    /// Confirm/Accept current selection (Enter)
    Confirm,

    /// Cancel/Escape from current context (Esc)
    Cancel,

    /// Open a test modal (for testing modal system)
    OpenModal,

    /// No action (unmapped key)
    None,
}

/// Map a keyboard event to an action
///
/// This function implements the keybinding logic, translating raw keyboard
/// events into application actions. It supports vim-style navigation with
/// h/j/k/l keys and standard keys like q for quit, Enter for confirm, etc.
///
/// # Vim-style navigation:
/// - h: Move left
/// - j: Move down
/// - k: Move up
/// - l: Move right
///
/// # Standard keys:
/// - q: Quit (when not in a modal or text input)
/// - Enter: Confirm/Accept
/// - Esc: Cancel/Escape
///
/// # Arguments
/// * `key` - The keyboard event to map
///
/// # Returns
/// The corresponding Action, or Action::None if the key is not mapped
///
/// # Example
/// ```no_run
/// use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
/// use tracker_rs::input::keybindings::{Action, map_key_to_action};
///
/// let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
/// let action = map_key_to_action(key);
/// assert_eq!(action, Action::MoveDown);
/// ```
pub fn map_key_to_action(key: KeyEvent) -> Action {
    // For now, we only handle unmodified keys (no Ctrl, Alt, Shift combinations)
    // Modifier support will be added in future phases for advanced keybindings
    if key.modifiers != KeyModifiers::NONE {
        return Action::None;
    }

    match key.code {
        // Vim-style navigation
        KeyCode::Char('h') => Action::MoveLeft,
        KeyCode::Char('j') => Action::MoveDown,
        KeyCode::Char('k') => Action::MoveUp,
        KeyCode::Char('l') => Action::MoveRight,

        // Arrow keys (alternative to vim keys)
        KeyCode::Left => Action::MoveLeft,
        KeyCode::Down => Action::MoveDown,
        KeyCode::Up => Action::MoveUp,
        KeyCode::Right => Action::MoveRight,

        // Standard application keys
        KeyCode::Char('q') => Action::Quit,
        KeyCode::Enter => Action::Confirm,
        KeyCode::Esc => Action::Cancel,

        // Modal test key
        KeyCode::Char('m') => Action::OpenModal,

        // Unmapped key
        _ => Action::None,
    }
}

/// Check if an action represents a navigation movement
///
/// This is a utility function to determine if an action is a navigation
/// action (move left/right/up/down). Useful for context-sensitive behavior.
///
/// # Arguments
/// * `action` - The action to check
///
/// # Returns
/// true if the action is a navigation movement, false otherwise
///
/// # Example
/// ```no_run
/// use tracker_rs::input::keybindings::{Action, is_navigation_action};
///
/// assert!(is_navigation_action(Action::MoveLeft));
/// assert!(is_navigation_action(Action::MoveDown));
/// assert!(!is_navigation_action(Action::Quit));
/// ```
pub fn is_navigation_action(action: Action) -> bool {
    matches!(
        action,
        Action::MoveLeft | Action::MoveDown | Action::MoveUp | Action::MoveRight
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vim_navigation_keys() {
        let h_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
        let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let k_key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let l_key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);

        assert_eq!(map_key_to_action(h_key), Action::MoveLeft);
        assert_eq!(map_key_to_action(j_key), Action::MoveDown);
        assert_eq!(map_key_to_action(k_key), Action::MoveUp);
        assert_eq!(map_key_to_action(l_key), Action::MoveRight);
    }

    #[test]
    fn test_arrow_keys() {
        let left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

        assert_eq!(map_key_to_action(left), Action::MoveLeft);
        assert_eq!(map_key_to_action(down), Action::MoveDown);
        assert_eq!(map_key_to_action(up), Action::MoveUp);
        assert_eq!(map_key_to_action(right), Action::MoveRight);
    }

    #[test]
    fn test_standard_keys() {
        let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        assert_eq!(map_key_to_action(q_key), Action::Quit);
        assert_eq!(map_key_to_action(enter_key), Action::Confirm);
        assert_eq!(map_key_to_action(esc_key), Action::Cancel);
    }

    #[test]
    fn test_unmapped_key() {
        let x_key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(map_key_to_action(x_key), Action::None);
    }

    #[test]
    fn test_modified_keys_ignored() {
        let ctrl_h = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL);
        let alt_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT);

        assert_eq!(map_key_to_action(ctrl_h), Action::None);
        assert_eq!(map_key_to_action(alt_j), Action::None);
    }

    #[test]
    fn test_is_navigation_action() {
        assert!(is_navigation_action(Action::MoveLeft));
        assert!(is_navigation_action(Action::MoveDown));
        assert!(is_navigation_action(Action::MoveUp));
        assert!(is_navigation_action(Action::MoveRight));

        assert!(!is_navigation_action(Action::Quit));
        assert!(!is_navigation_action(Action::Confirm));
        assert!(!is_navigation_action(Action::Cancel));
        assert!(!is_navigation_action(Action::None));
    }
}
