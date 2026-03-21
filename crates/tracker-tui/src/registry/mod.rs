/// Action Category for grouping in WhichKey/Help
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionCategory {
    Navigation,
    Editing,
    Transport,
    Track,
    Instrument,
    Pattern,
    View,
    Clipboard,
    Project,
    Application,
    None,
}

impl ActionCategory {
    pub fn name(&self) -> &str {
        match self {
            Self::Navigation => "Navigation",
            Self::Editing => "Editing",
            Self::Transport => "Transport",
            Self::Track => "Track",
            Self::Instrument => "Instrument",
            Self::Pattern => "Pattern",
            Self::View => "View",
            Self::Clipboard => "Clipboard",
            Self::Project => "Project",
            Self::Application => "Application",
            Self::None => "None",
        }
    }
}

pub trait ActionMetadata {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ActionCategory;
}

pub trait CommandMetadata {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Bpm,
    Step,
    Write,
    Edit,
    Quit,
    ForceQuit,
    SaveAndQuit,
    Tutor,
}

impl CommandMetadata for Command {
    fn name(&self) -> &str {
        match self {
            Self::Bpm => "bpm",
            Self::Step => "step",
            Self::Write => "w",
            Self::Edit => "e",
            Self::Quit => "q",
            Self::ForceQuit => "q!",
            Self::SaveAndQuit => "wq",
            Self::Tutor => "tutor",
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Bpm => "Set song tempo",
            Self::Step => "Set edit step size",
            Self::Write => "Save project",
            Self::Edit => "Load project",
            Self::Quit => "Quit application",
            Self::ForceQuit => "Quit without saving",
            Self::SaveAndQuit => "Save and quit",
            Self::Tutor => "Open tutor screen",
        }
    }

    fn usage(&self) -> &str {
        match self {
            Self::Bpm => ":bpm <value>",
            Self::Step => ":step <0-8>",
            Self::Write => ":w [filename]",
            Self::Edit => ":e <filename>",
            Self::Quit => ":q",
            Self::ForceQuit => ":q!",
            Self::SaveAndQuit => ":wq",
            Self::Tutor => ":tutor",
        }
    }

    fn aliases(&self) -> Vec<&str> {
        match self {
            Self::Bpm => vec!["t", "tempo"],
            Self::SaveAndQuit => vec!["x"],
            _ => vec![],
        }
    }
}

pub struct CommandRegistry;

impl CommandRegistry {
    pub fn all_commands() -> Vec<Command> {
        vec![
            Command::Bpm,
            Command::Step,
            Command::Write,
            Command::Edit,
            Command::Quit,
            Command::ForceQuit,
            Command::SaveAndQuit,
            Command::Tutor,
        ]
    }

    pub fn find_command(input: &str) -> Option<Command> {
        Self::all_commands()
            .into_iter()
            .find(|cmd| cmd.name() == input || cmd.aliases().contains(&input))
    }
}

/// A simplified keybinding representation for discovery
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keybinding {
    pub key: String,
    pub action: String,
    pub description: String,
    pub category: ActionCategory,
}
