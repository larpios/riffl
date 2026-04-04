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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandCategory {
    Project,
    Pattern,
    Transport,
    Editing,
    Track,
    Navigation,
    Instrument,
    View,
    Misc,
}

impl CommandCategory {
    pub fn name(&self) -> &str {
        match self {
            Self::Project => "Project",
            Self::Pattern => "Pattern",
            Self::Transport => "Transport",
            Self::Editing => "Editing",
            Self::Track => "Track",
            Self::Navigation => "Navigation",
            Self::Instrument => "Instrument",
            Self::View => "View",
            Self::Misc => "Misc",
        }
    }
}

pub trait CommandMetadata {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn usage(&self) -> &str;
    fn category(&self) -> CommandCategory;
    fn aliases(&self) -> Vec<&str> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    // Project
    Bpm,
    Step,
    Write,
    Edit,
    Load,
    Save,
    Quit,
    ForceQuit,
    SaveAndQuit,
    Tutor,
    // Song metadata
    Title,
    Artist,
    // Pattern
    Len,
    Clear,
    Dup,
    Pname,
    Alias,
    Plen,
    // Transport / timing
    Speed,
    Lpb,
    Loop,
    CountIn,
    Metronome,
    // Editing
    Transpose,
    Quantize,
    Interpolate,
    Fill,
    Reverse,
    Expand,
    Compress,
    // Track / channel
    Track,
    Rename,
    Volume,
    Filter,
    Automate,
    // Navigation
    Goto,
    // Instrument
    Adsr,
    Instruments,
    InstCopy,
    LoadSample,
    Keyzone,
    Wave,
    // Effects / misc
    Mode,
    Marker,
    Samples,
    Effects,
    DumpSamples,
    // Transport control
    Play,
    Stop,
    Follow,
    // Track targeting
    Mute,
    Solo,
    TrackVol,
    TrackPan,
    // Pattern management
    NewPat,
    DelPat,
    // Editing transforms
    Octave,
    Humanize,
    Randomize,
    // View switching
    View,
}

impl CommandMetadata for Command {
    fn name(&self) -> &str {
        match self {
            Self::Bpm => "bpm",
            Self::Step => "step",
            Self::Write => "w",
            Self::Edit => "e",
            Self::Load => "load",
            Self::Save => "save",
            Self::Quit => "q",
            Self::ForceQuit => "q!",
            Self::SaveAndQuit => "wq",
            Self::Tutor => "tutor",
            Self::Title => "title",
            Self::Artist => "artist",
            Self::Len => "len",
            Self::Clear => "clear",
            Self::Dup => "dup",
            Self::Pname => "pname",
            Self::Alias => "alias",
            Self::Plen => "plen",
            Self::Speed => "speed",
            Self::Lpb => "lpb",
            Self::Loop => "loop",
            Self::CountIn => "countin",
            Self::Metronome => "metronome",
            Self::Transpose => "transpose",
            Self::Quantize => "quantize",
            Self::Interpolate => "interpolate",
            Self::Fill => "fill",
            Self::Reverse => "reverse",
            Self::Expand => "expand",
            Self::Compress => "compress",
            Self::Track => "track",
            Self::Rename => "rename",
            Self::Volume => "volume",
            Self::Filter => "filter",
            Self::Automate => "automate",
            Self::Goto => "goto",
            Self::Adsr => "adsr",
            Self::Instruments => "instruments",
            Self::InstCopy => "instcopy",
            Self::LoadSample => "loadsample",
            Self::Keyzone => "keyzone",
            Self::Wave => "wave",
            Self::Mode => "mode",
            Self::Marker => "marker",
            Self::Samples => "samples",
            Self::Effects => "effects",
            Self::DumpSamples => "dump-samples",
            Self::Play => "play",
            Self::Stop => "stop",
            Self::Follow => "follow",
            Self::Mute => "mute",
            Self::Solo => "solo",
            Self::TrackVol => "tvol",
            Self::TrackPan => "tpan",
            Self::NewPat => "newpat",
            Self::DelPat => "delpat",
            Self::Octave => "octave",
            Self::Humanize => "humanize",
            Self::Randomize => "randomize",
            Self::View => "view",
        }
    }

    fn description(&self) -> &str {
        match self {
            Self::Bpm => "Set song tempo",
            Self::Step => "Set edit step size",
            Self::Write => "Save project to file",
            Self::Edit => "Load project from file",
            Self::Load => "Load project from file (alias :e)",
            Self::Save => "Save project to file (alias :w)",
            Self::Quit => "Quit application",
            Self::ForceQuit => "Quit without saving",
            Self::SaveAndQuit => "Save and quit",
            Self::Tutor => "Open tutor screen",
            Self::Title => "Set song title",
            Self::Artist => "Set song artist",
            Self::Len => "Resize current pattern",
            Self::Clear => "Clear all cells in current pattern",
            Self::Dup => "Duplicate current pattern",
            Self::Pname => "Rename selected pattern",
            Self::Alias => "Insert an existing pattern as an alias in the arrangement",
            Self::Plen => "Set the row count of a pattern (for polyrhythm)",
            Self::Speed => "Set ticks per line (1-31)",
            Self::Lpb => "Set lines per beat",
            Self::Loop => "Set loop region rows",
            Self::CountIn => "Set count-in bars before playback",
            Self::Metronome => "Control metronome",
            Self::Transpose => "Transpose selection by semitones",
            Self::Quantize => "Quantize selection to step grid",
            Self::Interpolate => "Interpolate values across selection",
            Self::Fill => "Fill channel with a note at interval",
            Self::Reverse => "Reverse notes in selection",
            Self::Expand => "Insert empty rows between each existing row",
            Self::Compress => "Keep every Nth row, delete others",
            Self::Track => "Add or remove a channel",
            Self::Rename => "Rename current track/channel",
            Self::Volume => "Set global volume (0-100)",
            Self::Filter => "Apply per-channel filter",
            Self::Automate => "Fill channel with volume/pan automation",
            Self::Goto => "Jump to a specific row",
            Self::Adsr => "Set ADSR envelope on current instrument",
            Self::Instruments => "List all instruments",
            Self::InstCopy => "Copy instrument settings to another slot",
            Self::LoadSample => "Load audio file into selected instrument",
            Self::Keyzone => "Manage instrument keyzones",
            Self::Wave => "Generate a built-in waveform for instrument",
            Self::Mode => "Switch effect interpretation mode",
            Self::Marker => "Add/remove a section marker in arrangement",
            Self::Samples => "List all loaded samples",
            Self::Effects => "Show effect command reference",
            Self::DumpSamples => "Export all loaded samples to WAV files",
            Self::Play => "Start or resume playback",
            Self::Stop => "Stop playback",
            Self::Follow => "Toggle or set follow mode",
            Self::Mute => "Toggle mute on a track",
            Self::Solo => "Toggle solo on a track",
            Self::TrackVol => "Set track volume (0-100)",
            Self::TrackPan => "Set track pan (-100 to 100)",
            Self::NewPat => "Add a new empty pattern",
            Self::DelPat => "Delete a pattern",
            Self::Octave => "Set current octave (0-9)",
            Self::Humanize => "Add timing jitter to notes in selection",
            Self::Randomize => "Randomize pitches of notes in selection",
            Self::View => "Switch to a named view",
        }
    }

    fn usage(&self) -> &str {
        match self {
            Self::Bpm => ":bpm <value>",
            Self::Step => ":step <0-8>",
            Self::Write => ":w <filename>",
            Self::Edit => ":e <filename>",
            Self::Load => ":load <filename>",
            Self::Save => ":save <filename>",
            Self::Quit => ":q",
            Self::ForceQuit => ":q!",
            Self::SaveAndQuit => ":wq",
            Self::Tutor => ":tutor",
            Self::Title => ":title <name>",
            Self::Artist => ":artist <name>",
            Self::Len => ":len <rows>",
            Self::Clear => ":clear",
            Self::Dup => ":dup",
            Self::Pname => ":pname <name>",
            Self::Alias => ":alias <pattern-index>",
            Self::Plen => ":plen <rows> [pattern-index]",
            Self::Speed => ":speed <1-31>",
            Self::Lpb => ":lpb <value>",
            Self::Loop => ":loop <start> <end>",
            Self::CountIn => ":countin <bars>",
            Self::Metronome => ":metronome on|off|vol <n>",
            Self::Transpose => ":transpose <semitones>",
            Self::Quantize => ":quantize",
            Self::Interpolate => ":interpolate",
            Self::Fill => ":fill <note> [step]",
            Self::Reverse => ":reverse",
            Self::Expand => ":expand <n>",
            Self::Compress => ":compress <n>",
            Self::Track => ":track add|del",
            Self::Rename => ":rename <name>",
            Self::Volume => ":volume <0-100>",
            Self::Filter => ":filter <lpf|hpf|off> [hz]",
            Self::Automate => ":automate vol|pan <start> <end>",
            Self::Goto => ":g <row>",
            Self::Adsr => ":adsr <A> <D> <S%> <R>",
            Self::Instruments => ":instruments",
            Self::InstCopy => ":instcopy <src> <dst>",
            Self::LoadSample => ":loadsample <path>",
            Self::Keyzone => ":keyzone add|del|list|clear",
            Self::Wave => ":wave <type> [ms] [hz]",
            Self::Mode => ":mode native|compat|amiga",
            Self::Marker => ":marker [label]",
            Self::Samples => ":samples",
            Self::Effects => ":effects [cmd]",
            Self::DumpSamples => ":dump-samples <dir>",
            Self::Play => ":play",
            Self::Stop => ":stop",
            Self::Follow => ":follow [on|off]",
            Self::Mute => ":mute [track]",
            Self::Solo => ":solo [track]",
            Self::TrackVol => ":tvol [track] <0-100>",
            Self::TrackPan => ":tpan [track] <-100..100>",
            Self::NewPat => ":newpat [name]",
            Self::DelPat => ":delpat [idx]",
            Self::Octave => ":octave <0-9>",
            Self::Humanize => ":humanize [amount]",
            Self::Randomize => ":randomize",
            Self::View => ":view <pat|arr|inst|code|plist|samples>",
        }
    }

    fn category(&self) -> CommandCategory {
        match self {
            Self::Bpm
            | Self::Step
            | Self::Write
            | Self::Edit
            | Self::Load
            | Self::Save
            | Self::Quit
            | Self::ForceQuit
            | Self::SaveAndQuit
            | Self::Tutor
            | Self::Title
            | Self::Artist => CommandCategory::Project,
            Self::Len | Self::Clear | Self::Dup | Self::Pname | Self::Alias | Self::Plen => {
                CommandCategory::Pattern
            }
            Self::Speed | Self::Lpb | Self::Loop | Self::CountIn | Self::Metronome => {
                CommandCategory::Transport
            }
            Self::Transpose
            | Self::Quantize
            | Self::Interpolate
            | Self::Fill
            | Self::Reverse
            | Self::Expand
            | Self::Compress => CommandCategory::Editing,
            Self::Track | Self::Rename | Self::Volume | Self::Filter | Self::Automate => {
                CommandCategory::Track
            }
            Self::Goto => CommandCategory::Navigation,
            Self::Adsr
            | Self::Instruments
            | Self::InstCopy
            | Self::LoadSample
            | Self::Keyzone
            | Self::Wave => CommandCategory::Instrument,
            Self::Mode | Self::Marker | Self::Samples | Self::Effects | Self::DumpSamples => {
                CommandCategory::Misc
            }
            Self::Play | Self::Stop | Self::Follow => CommandCategory::Transport,
            Self::Mute | Self::Solo | Self::TrackVol | Self::TrackPan => CommandCategory::Track,
            Self::NewPat | Self::DelPat => CommandCategory::Pattern,
            Self::Octave | Self::Humanize | Self::Randomize => CommandCategory::Editing,
            Self::View => CommandCategory::View,
        }
    }

    fn aliases(&self) -> Vec<&str> {
        match self {
            Self::Bpm => vec!["t", "tempo"],
            Self::SaveAndQuit => vec!["x"],
            Self::Load => vec!["e"],
            Self::Save => vec!["w"],
            Self::Len => vec!["length"],
            Self::Speed => vec!["tpl"],
            Self::Transpose => vec!["tr"],
            Self::Interpolate => vec!["interp"],
            Self::Reverse => vec!["rev"],
            Self::Compress => vec!["shrink"],
            Self::CountIn => vec!["count-in"],
            Self::Metronome => vec!["metro"],
            Self::Filter => vec!["flt"],
            Self::Automate => vec!["auto"],
            Self::Goto => vec!["g"],
            Self::Instruments => vec!["insts", "inst"],
            Self::InstCopy => vec!["icopy"],
            Self::LoadSample => vec!["ls"],
            Self::Keyzone => vec!["kz"],
            Self::Effects => vec!["fx", "efx"],
            Self::Marker => vec!["mark"],
            Self::Samples => vec!["samps"],
            Self::Play => vec!["start"],
            Self::TrackVol => vec!["trackvol"],
            Self::TrackPan => vec!["trackpan"],
            Self::NewPat => vec!["np"],
            Self::DelPat => vec!["dp"],
            Self::Octave => vec!["oct"],
            Self::Randomize => vec!["rand"],
            _ => vec![],
        }
    }
}

pub struct CommandRegistry;

impl CommandRegistry {
    pub fn all_commands() -> Vec<Command> {
        vec![
            // Project
            Command::Bpm,
            Command::Step,
            Command::Write,
            Command::Edit,
            Command::Load,
            Command::Save,
            Command::Quit,
            Command::ForceQuit,
            Command::SaveAndQuit,
            Command::Tutor,
            // Song metadata
            Command::Title,
            Command::Artist,
            // Pattern
            Command::Len,
            Command::Clear,
            Command::Dup,
            Command::Pname,
            // Transport / timing
            Command::Speed,
            Command::Lpb,
            Command::Loop,
            Command::CountIn,
            Command::Metronome,
            // Editing
            Command::Transpose,
            Command::Quantize,
            Command::Interpolate,
            Command::Fill,
            Command::Reverse,
            Command::Expand,
            Command::Compress,
            // Track / channel
            Command::Track,
            Command::Rename,
            Command::Volume,
            Command::Filter,
            Command::Automate,
            // Navigation
            Command::Goto,
            // Instrument
            Command::Adsr,
            Command::Instruments,
            Command::InstCopy,
            Command::LoadSample,
            Command::Keyzone,
            Command::Wave,
            // Effects / misc
            Command::Mode,
            Command::Marker,
            Command::Samples,
            Command::Effects,
            Command::DumpSamples,
            // Transport control
            Command::Play,
            Command::Stop,
            Command::Follow,
            // Track targeting
            Command::Mute,
            Command::Solo,
            Command::TrackVol,
            Command::TrackPan,
            // Pattern management
            Command::NewPat,
            Command::DelPat,
            // Editing transforms
            Command::Octave,
            Command::Humanize,
            Command::Randomize,
            // View switching
            Command::View,
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
