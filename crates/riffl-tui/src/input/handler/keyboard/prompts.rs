use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Outcome returned by [`TextPrompt::handle_key`].
pub enum PromptAction {
    /// User pressed Enter — caller should execute the prompt.
    Confirm,
    /// User pressed Esc — prompt was already closed by `handle_key`.
    Cancel,
    /// Key was consumed (Backspace, accepted char).
    Consumed,
}

/// A simple single-line text prompt with an accept filter.
///
/// Handles Enter / Esc / Backspace and filtered character input.
/// The caller owns the execute logic (so it can access `App` fields).
pub struct TextPrompt {
    pub active: bool,
    pub input: String,
    /// Returns `true` for characters that should be accepted into the buffer.
    filter: fn(char) -> bool,
}

impl TextPrompt {
    pub fn new(filter: fn(char) -> bool) -> Self {
        Self {
            active: false,
            input: String::new(),
            filter,
        }
    }

    pub fn open(&mut self, initial: String) {
        self.active = true;
        self.input = initial;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.input.clear();
    }

    /// Handle a key event. Returns `Some(PromptAction)` if the key was consumed,
    /// `None` if ignored (caller may handle it differently).
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<PromptAction> {
        match key.code {
            KeyCode::Enter => Some(PromptAction::Confirm),
            KeyCode::Esc => {
                self.close();
                Some(PromptAction::Cancel)
            }
            KeyCode::Backspace => {
                self.input.pop();
                Some(PromptAction::Consumed)
            }
            KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE && (self.filter)(c) => {
                self.input.push(c);
                Some(PromptAction::Consumed)
            }
            _ => None,
        }
    }
}
