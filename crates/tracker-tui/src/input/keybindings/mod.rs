/// Keybinding handling with vim-style navigation and mode-aware dispatch
///
/// This module provides keybinding infrastructure for the application,
/// with support for vim-style navigation keys (h/j/k/l) and modal editing
/// (Normal, Insert, Visual modes).
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::AppView;
use crate::editor::EditorMode;
use crate::registry::{ActionCategory, ActionMetadata, Keybinding};


mod action;
mod dispatch;
mod mappings;
#[cfg(test)]
mod tests;

pub use action::Action;
pub use dispatch::{is_modal_dismiss_action, is_navigation_action, map_key_to_action};

use mappings::{ChordMapping, KeyMapping, CHORD_MAPPINGS, KEY_MAPPINGS};

pub struct KeybindingRegistry;

impl KeybindingRegistry {
    /// Get all available keybindings for a given editor mode
    pub fn get_bindings_for_mode(mode: EditorMode) -> Vec<Keybinding> {
        KEY_MAPPINGS
            .iter()
            .filter(|m| m.mode == mode)
            .map(|m| Keybinding {
                key: m.key.to_string(),
                action: m.action.name().to_string(),
                description: m.action.description().to_string(),
                category: m.action.category(),
            })
            .collect()
    }

    /// Get which-key entries for a given chord prefix
    pub fn get_which_key_entries(prefix: char) -> Vec<(String, String)> {
        CHORD_MAPPINGS
            .iter()
            .filter(|m| m.prefix == prefix)
            .map(|m| {
                let key = format!("{}{}", prefix, m.completion);
                let desc = m.action.description().to_string();
                (key, desc)
            })
            .collect()
    }

    /// Get all which-key entries (for the full menu display)
    pub fn get_all_which_key_entries() -> Vec<(String, String)> {
        CHORD_MAPPINGS
            .iter()
            .map(|m| {
                let key = format!("{}{}", m.prefix, m.completion);
                let desc = m.action.description().to_string();
                (key, desc)
            })
            .collect()
    }
}

