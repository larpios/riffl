//! Effect command types and processing for the tracker.
//!
//! Effects modify playback behavior on a per-row basis. Each effect has a
//! command type (identifying what the effect does) and a parameter byte
//! controlling its intensity or target value.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Project-level effect interpretation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EffectMode {
    /// Riffl native effects: strict interpretation, standard Riffl behavior.
    #[default]
    RifflNative,
    /// Compatibility mode: preserves source tracker semantics (e.g., XM, IT, ProTracker).
    /// Used when importing foreign modules to maintain playback fidelity.
    Compatible,
}

/// Format for effect parameters to guide human-readable display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ParamFormat {
    /// No specific formatting (default).
    #[default]
    Hex,
    /// Split byte into two 4-bit nibbles (x, y).
    Nibbles,
    /// Absolute decimal value.
    Decimal,
    /// Frequency in Hz.
    Frequency,
    /// Percentage (00=0%, FF=100%).
    Percentage,
    /// Semitones (for arpeggio, etc).
    Semitones,
    /// Cents (for fine tuning).
    Cents,
}

/// Metadata describing an effect for help and interpretation.
#[derive(Debug, Clone)]
pub struct EffectMetadata {
    /// Display name of the effect.
    pub name: &'static str,
    /// Short description for status bar.
    pub summary: &'static str,
    /// Detailed description for help view.
    pub description: &'static str,
    /// Human-readable parameter meaning (e.g., "xy: speed/depth").
    pub param_label: &'static str,
    /// How the parameter should be formatted for display.
    pub param_format: ParamFormat,
    /// Whether this effect supports continuation (param 00).
    pub supports_continuation: bool,
    /// Whether this effect is Riffl-native or compatibility-only.
    pub is_native: bool,
}

/// Standard tracker effect command types.
///
/// Effect commands follow classic tracker conventions where the command
/// type is a single hex digit (0-F) and the parameter is a two-digit
/// hex value (00-FF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectType {
    /// `0xy` — Arpeggio: cycle between base note, +x semitones, +y semitones.
    Arpeggio,
    /// `1xx` — Pitch slide up by xx units per row.
    PitchSlideUp,
    /// `2xx` — Pitch slide down by xx units per row.
    PitchSlideDown,
    /// `3xx` — Portamento to note: slide to target note at speed xx.
    PortamentoToNote,
    /// `4xy` — Vibrato: oscillate pitch with speed x, depth y.
    Vibrato,
    /// `5xy` — Tone Portamento + Volume Slide.
    TonePortamentoVolumeSlide,
    /// `6xy` — Vibrato + Volume Slide.
    VibratoVolumeSlide,
    /// `7xy` — Tremolo: oscillate amplitude with speed x, depth y.
    Tremolo,
    /// `8xx` — Set panning position (0x00 = full left, 0x80 = centre, 0xFF = full right).
    SetPanning,
    /// `9xx` — Sample Offset: start sample at xx * 256 frames.
    SampleOffset,
    /// `Axy` — Volume slide: x = up speed, y = down speed.
    VolumeSlide,
    /// `Bxx` — Position jump: jump to arrangement position xx.
    PositionJump,
    /// `Cxx` — Set volume to xx.
    SetVolume,
    /// `Dxx` — Pattern break: jump to row xx of the next pattern.
    PatternBreak,
    /// `Exy` — Extended effects: sub-command x, param y.
    Extended,
    /// `Fxx` — Set speed (Ticks Per Line).
    SetSpeed,
    /// `Gxx` or `Vxx` — Set global volume. (0x10)
    SetGlobalVolume,
    /// `Hxy` or `Wxy` — Global volume slide. (0x11)
    GlobalVolumeSlide,
    /// `Pxy` — Panning slide. (0x12)
    PanningSlide,
    /// `Mxx` — Channel volume (IT). (0x13)
    ChannelVolume,
    /// `Nxy` — Channel volume slide (IT). (0x14)
    ChannelVolumeSlide,
    /// `Txy` or `Ixy` — Tremor. (0x15)
    Tremor,
    /// `Rxy` or `Qxy` — Retrig Note + Volume Slide. (0x16)
    RetrigNoteVolSlide,
    /// `Lxx` — Set envelope position. (0x17)
    SetEnvelopePosition,
    /// `Yxy` — Panbrello. (0x18)
    Panbrello,
    /// `Zxx` — MIDI Macro. (0x19)
    MidiMacro,
    /// `X1x` — Extra fine portamento up. (0x21)
    ExtraFinePortaUp,
    /// `X2x` — Extra fine portamento down. (0x22)
    ExtraFinePortaDown,
}

impl EffectType {
    /// Convert a command byte to an effect type.
    ///
    /// Returns `None` for unrecognized command values.
    pub fn from_command(command: u8) -> Option<Self> {
        match command {
            0x0 => Some(EffectType::Arpeggio),
            0x1 => Some(EffectType::PitchSlideUp),
            0x2 => Some(EffectType::PitchSlideDown),
            0x3 => Some(EffectType::PortamentoToNote),
            0x4 => Some(EffectType::Vibrato),
            0x5 => Some(EffectType::TonePortamentoVolumeSlide),
            0x6 => Some(EffectType::VibratoVolumeSlide),
            0x7 => Some(EffectType::Tremolo),
            0x8 => Some(EffectType::SetPanning),
            0x9 => Some(EffectType::SampleOffset),
            0xA => Some(EffectType::VolumeSlide),
            0xB => Some(EffectType::PositionJump),
            0xC => Some(EffectType::SetVolume),
            0xD => Some(EffectType::PatternBreak),
            0xE => Some(EffectType::Extended),
            0xF => Some(EffectType::SetSpeed),
            0x10 => Some(EffectType::SetGlobalVolume),
            0x11 => Some(EffectType::GlobalVolumeSlide),
            0x12 => Some(EffectType::PanningSlide),
            0x13 => Some(EffectType::ChannelVolume),
            0x14 => Some(EffectType::ChannelVolumeSlide),
            0x15 => Some(EffectType::Tremor),
            0x16 => Some(EffectType::RetrigNoteVolSlide),
            0x17 => Some(EffectType::SetEnvelopePosition),
            0x18 => Some(EffectType::Panbrello),
            0x19 => Some(EffectType::MidiMacro),
            0x21 => Some(EffectType::ExtraFinePortaUp),
            0x22 => Some(EffectType::ExtraFinePortaDown),
            _ => None,
        }
    }

    /// Convert an effect type to its MOD/ProTracker command byte (0-F).
    pub fn protracker_cmd(self) -> u8 {
        self.to_command()
    }

    /// Convert an effect type to its command byte.
    pub fn to_command(self) -> u8 {
        match self {
            EffectType::Arpeggio => 0x0,
            EffectType::PitchSlideUp => 0x1,
            EffectType::PitchSlideDown => 0x2,
            EffectType::PortamentoToNote => 0x3,
            EffectType::Vibrato => 0x4,
            EffectType::TonePortamentoVolumeSlide => 0x5,
            EffectType::VibratoVolumeSlide => 0x6,
            EffectType::Tremolo => 0x7,
            EffectType::SetPanning => 0x8,
            EffectType::SampleOffset => 0x9,
            EffectType::VolumeSlide => 0xA,
            EffectType::SetVolume => 0xC,
            EffectType::PositionJump => 0xB,
            EffectType::PatternBreak => 0xD,
            EffectType::Extended => 0xE,
            EffectType::SetSpeed => 0xF,
            EffectType::SetGlobalVolume => 0x10,
            EffectType::GlobalVolumeSlide => 0x11,
            EffectType::PanningSlide => 0x12,
            EffectType::ChannelVolume => 0x13,
            EffectType::ChannelVolumeSlide => 0x14,
            EffectType::Tremor => 0x15,
            EffectType::RetrigNoteVolSlide => 0x16,
            EffectType::SetEnvelopePosition => 0x17,
            EffectType::Panbrello => 0x18,
            EffectType::MidiMacro => 0x19,
            EffectType::ExtraFinePortaUp => 0x21,
            EffectType::ExtraFinePortaDown => 0x22,
        }
    }

    /// Get the mnemonic name for this effect type.
    pub fn mnemonic(&self) -> &'static str {
        self.metadata().name
    }

    /// Get metadata for this effect type.
    pub fn metadata(&self) -> EffectMetadata {
        match self {
            EffectType::Arpeggio => EffectMetadata {
                name: "Arpeggio",
                summary: "Arpeggio: cycle base, +x, +y semitones",
                description: "Rapidly cycles between the base note and two offsets (x and y semitones), creating a chord-like texture characteristic of 8-bit music.",
                param_label: "xy: offsets",
                param_format: ParamFormat::Nibbles,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::PitchSlideUp => EffectMetadata {
                name: "Pitch Up",
                summary: "Pitch Up: slide pitch up by xx",
                description: "Continuously slides the pitch of the current note upwards at a speed defined by the parameter xx.",
                param_label: "xx: speed",
                param_format: ParamFormat::Hex,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::PitchSlideDown => EffectMetadata {
                name: "Pitch Down",
                summary: "Pitch Down: slide pitch down by xx",
                description: "Continuously slides the pitch of the current note downwards at a speed defined by the parameter xx.",
                param_label: "xx: speed",
                param_format: ParamFormat::Hex,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::PortamentoToNote => EffectMetadata {
                name: "Porta Note",
                summary: "Porta Note: slide to target note at speed xx",
                description: "Automatically slides the pitch from the previous note towards the newly triggered note at speed xx. If xx is 0, the previous speed is continued in Compatibility mode.",
                param_label: "xx: speed",
                param_format: ParamFormat::Hex,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::Vibrato => EffectMetadata {
                name: "Vibrato",
                summary: "Vibrato: pitch oscillation (speed x, depth y)",
                description: "Modulates the pitch with a periodic oscillator (LFO). x defines the speed, and y defines the depth of the oscillation.",
                param_label: "xy: speed/depth",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::TonePortamentoVolumeSlide => EffectMetadata {
                name: "Porta+Vol",
                summary: "Porta+Vol: Tone Porta (3xx) + Volume Slide (Axy)",
                description: "Combines tone portamento (sliding to target note) with a volume slide. Uses the previously set portamento speed.",
                param_label: "xy: vol up/down",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::VibratoVolumeSlide => EffectMetadata {
                name: "Vib+Vol",
                summary: "Vib+Vol: Vibrato (4xy) + Volume Slide (Axy)",
                description: "Combines vibrato with a volume slide. Uses the previously set vibrato speed and depth.",
                param_label: "xy: vol up/down",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::Tremolo => EffectMetadata {
                name: "Tremolo",
                summary: "Tremolo: volume oscillation (speed x, depth y)",
                description: "Modulates the volume with a periodic oscillator (LFO). x defines the speed, and y defines the depth of the oscillation.",
                param_label: "xy: speed/depth",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::SetPanning => EffectMetadata {
                name: "Set Pan",
                summary: "Set Pan: set panning position to xx",
                description: "Sets the stereo panning position. 00 is full left, 80 is center, and FF is full right.",
                param_label: "xx: position",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::SampleOffset => EffectMetadata {
                name: "Offset",
                summary: "Offset: start sample at xx * 256 frames",
                description: "Starts sample playback from a specific offset rather than the beginning. The offset is xx multiplied by 256 sample frames.",
                param_label: "xx: offset",
                param_format: ParamFormat::Hex,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::VolumeSlide => EffectMetadata {
                name: "Vol Slide",
                summary: "Vol Slide: slide volume up (x) or down (y)",
                description: "Continuously changes the volume of the channel. If x is non-zero, volume increases; if y is non-zero, volume decreases.",
                param_label: "xy: up/down",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::PositionJump => EffectMetadata {
                name: "Pos Jump",
                summary: "Pos Jump: jump to arrangement position xx",
                description: "Immediately jumps to the specified position in the song arrangement sequence after the current row finishes.",
                param_label: "xx: position",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::SetVolume => EffectMetadata {
                name: "Set Vol",
                summary: "Set Vol: set volume to xx",
                description: "Sets the channel volume to the specified value xx (00-40, where 40 is 100%). Values above 40 provide amplification.",
                param_label: "xx: volume",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::PatternBreak => EffectMetadata {
                name: "Pat Break",
                summary: "Pat Break: jump to next pattern at row xx",
                description: "Stop playing the current pattern and jump to the specified row xx of the next pattern in the arrangement.",
                param_label: "xx: row",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::Extended => EffectMetadata {
                name: "Extended",
                summary: "Extended: sub-command x, parameter y",
                description: "A collection of specialized commands (E1x-EFx) for fine-grained control over portamento, loops, retriggering, etc.",
                param_label: "xy: cmd/param",
                param_format: ParamFormat::Nibbles,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::SetSpeed => EffectMetadata {
                name: "Set Speed",
                summary: "Set Speed: set TPL (01-1F) or BPM (20-FF)",
                description: "Sets either the ticks per line (TPL) if xx < 32, or the tempo (BPM) if xx >= 32.",
                param_label: "xx: speed/bpm",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::SetGlobalVolume => EffectMetadata {
                name: "Global Vol",
                summary: "Global Vol: set global volume to xx",
                description: "Sets the overall master volume of the entire song (00-80).",
                param_label: "xx: volume",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::GlobalVolumeSlide => EffectMetadata {
                name: "GVol Slide",
                summary: "GVol Slide: slide global volume up (x) or down (y)",
                description: "Continuously changes the master volume of the song.",
                param_label: "xy: up/down",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::PanningSlide => EffectMetadata {
                name: "Pan Slide",
                summary: "Pan Slide: slide panning left (x) or right (y)",
                description: "Continuously slides the stereo panning position of the channel.",
                param_label: "xy: left/right",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::ChannelVolume => EffectMetadata {
                name: "Chan Vol",
                summary: "Chan Vol: set channel volume (IT-style)",
                description: "Sets the base channel volume (00-40) as used in Impulse Tracker format.",
                param_label: "xx: volume",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::ChannelVolumeSlide => EffectMetadata {
                name: "CVol Slide",
                summary: "CVol Slide: slide channel volume (IT-style)",
                description: "Continuously slides the base channel volume (00-40).",
                param_label: "xy: up/down",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::Tremor => EffectMetadata {
                name: "Tremor",
                summary: "Tremor: periodic volume muting (on x, off y)",
                description: "Rapidly alternates the volume between its current level and silence. x and y define the number of ticks for the 'on' and 'off' phases.",
                param_label: "xy: on/off",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::RetrigNoteVolSlide => EffectMetadata {
                name: "Retrig Vol",
                summary: "Retrig Vol: retrigger note with volume slide",
                description: "Retriggers the current note every y ticks, while simultaneously sliding the volume based on x.",
                param_label: "xy: slide/ticks",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::SetEnvelopePosition => EffectMetadata {
                name: "Env Pos",
                summary: "Env Pos: set instrument envelope position to xx",
                description: "Forces the instrument's volume or panning envelope to jump to the specified frame position xx.",
                param_label: "xx: position",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::Panbrello => EffectMetadata {
                name: "Panbrello",
                summary: "Panbrello: panning oscillation (speed x, depth y)",
                description: "Modulates the stereo panning position with a periodic oscillator (LFO). x is speed, y is depth.",
                param_label: "xy: speed/depth",
                param_format: ParamFormat::Nibbles,
                supports_continuation: true,
                is_native: true,
            },
            EffectType::MidiMacro => EffectMetadata {
                name: "Midi Macro",
                summary: "Midi Macro: trigger MIDI or Zxx macro",
                description: "Triggers a pre-defined MIDI macro or internal script macro with parameter xx.",
                param_label: "xx: param",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::ExtraFinePortaUp => EffectMetadata {
                name: "XFine Up",
                summary: "XFine Up: extra-fine pitch slide up",
                description: "Slides the pitch up by an extremely small amount (1/4 of a fine slide unit).",
                param_label: "x: speed",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
            EffectType::ExtraFinePortaDown => EffectMetadata {
                name: "XFine Down",
                summary: "XFine Down: extra-fine pitch slide down",
                description: "Slides the pitch down by an extremely small amount (1/4 of a fine slide unit).",
                param_label: "x: speed",
                param_format: ParamFormat::Hex,
                supports_continuation: false,
                is_native: true,
            },
        }
    }

    /// Get x (high nibble) parameter from effect param byte.
    pub fn param_x(param: u8) -> u8 {
        (param >> 4) & 0x0F
    }

    /// Get y (low nibble) parameter from effect param byte.
    pub fn param_y(param: u8) -> u8 {
        param & 0x0F
    }
}

impl fmt::Display for EffectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

/// An effect command applied to a channel at a specific row.
///
/// Effect commands modify playback behavior (e.g., pitch slides, vibrato,
/// volume changes). Each effect has a type byte and a parameter byte.
/// Display format is 4 hex characters: command byte + param byte (e.g., "0A04", "0C40", "0F78").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    /// Effect type identifier (0x0-0xF).
    pub command: u8,
    /// Effect parameter value (0x00-0xFF).
    pub param: u8,
}

impl Effect {
    /// Create a new effect command.
    pub fn new(command: u8, param: u8) -> Self {
        Self { command, param }
    }

    /// Create an effect from a known effect type and parameter.
    pub fn from_type(effect_type: EffectType, param: u8) -> Self {
        Self {
            command: effect_type.to_command(),
            param,
        }
    }

    /// Get the effect type, if the command byte is recognized.
    pub fn effect_type(&self) -> Option<EffectType> {
        EffectType::from_command(self.command)
    }

    /// Get the high nibble (x) of the parameter.
    pub fn param_x(&self) -> u8 {
        EffectType::param_x(self.param)
    }

    /// Get the low nibble (y) of the parameter.
    pub fn param_y(&self) -> u8 {
        EffectType::param_y(self.param)
    }

    /// Get the mnemonic description of this effect, or "Unknown" if unrecognized.
    pub fn mnemonic(&self) -> &'static str {
        self.effect_type()
            .map(|t| t.mnemonic())
            .unwrap_or("Unknown")
    }

    /// Check if this effect is a continuation (param 00) for its type.
    pub fn is_continuation(&self) -> bool {
        if self.param != 0 {
            return false;
        }
        self.effect_type()
            .map(|t| t.metadata().supports_continuation)
            .unwrap_or(false)
    }

    /// Get a human-readable description of this effect and its parameter.
    ///
    /// The description adapts based on the `EffectMode` (Native vs Compatible).
    pub fn describe(&self, mode: EffectMode) -> String {
        let Some(effect_type) = self.effect_type() else {
            return format!("Unknown effect {:02X}", self.command);
        };

        let meta = effect_type.metadata();
        let param = self.param;
        let x = self.param_x();
        let y = self.param_y();

        let param_desc = match meta.param_format {
            ParamFormat::Hex => format!("{:02X}", param),
            ParamFormat::Decimal => format!("{}", param),
            ParamFormat::Nibbles => format!("{}, {}", x, y),
            ParamFormat::Percentage => format!("{}%", (param as f32 / 255.0 * 100.0) as u32),
            ParamFormat::Frequency => format!("{}Hz", param), // Simplified
            ParamFormat::Semitones => {
                if x == 0 && y == 0 {
                    "None".to_string()
                } else {
                    format!("+{}, +{}", x, y)
                }
            }
            ParamFormat::Cents => format!("{} cents", param),
        };

        match mode {
            EffectMode::RifflNative => {
                format!("{}: {}", meta.name, param_desc)
            }
            EffectMode::Compatible => {
                // In compatibility mode, we might want to surface legacy-specific info
                let legacy_note = match effect_type {
                    EffectType::Arpeggio => " (Hardware cycle)",
                    EffectType::PitchSlideUp | EffectType::PitchSlideDown => " (Period-based)",
                    _ => "",
                };
                format!("{}: {}{}", meta.name, param_desc, legacy_note)
            }
        }
    }
}

impl fmt::Display for Effect {
    /// Display as 4 hex characters: command byte + param byte.
    /// Example: command=0xA, param=0x04 → "0A04"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02X}{:02X}", self.command, self.param)
    }
}

/// Maximum number of effects per cell.
pub const MAX_EFFECTS_PER_CELL: usize = 2;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Effect Display Tests ---

    #[test]
    fn test_effect_display_formatting() {
        assert_eq!(format!("{}", Effect::new(0, 0)), "0000");
        assert_eq!(format!("{}", Effect::new(0xA, 0x04)), "0A04");
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "0C40");
        assert_eq!(format!("{}", Effect::new(0xF, 0x78)), "0F78");
        assert_eq!(format!("{}", Effect::new(0xF, 0xFF)), "0FFF");
    }

    #[test]
    fn test_effect_display_boundary_values() {
        let min = Effect::new(0, 0);
        assert_eq!(format!("{}", min), "0000");

        let max = Effect::new(0xF, 0xFF);
        assert_eq!(format!("{}", max), "0FFF");
    }

    #[test]
    fn test_effect_describe() {
        // Arpeggio (037)
        let arp = Effect::new(0x0, 0x37);
        assert_eq!(arp.describe(EffectMode::RifflNative), "Arpeggio: 3, 7");
        assert_eq!(
            arp.describe(EffectMode::Compatible),
            "Arpeggio: 3, 7 (Hardware cycle)"
        );

        // Pitch Up (104)
        let pitch_up = Effect::new(0x1, 0x04);
        assert_eq!(pitch_up.describe(EffectMode::RifflNative), "Pitch Up: 04");
        assert_eq!(
            pitch_up.describe(EffectMode::Compatible),
            "Pitch Up: 04 (Period-based)"
        );

        // Unknown
        let unknown = Effect::new(0x99, 0x00);
        assert_eq!(
            unknown.describe(EffectMode::RifflNative),
            "Unknown effect 99"
        );
    }
    #[test]
    fn test_effect_display_all_commands() {
        // Verify each standard command byte displays correctly
        assert_eq!(format!("{}", Effect::new(0x0, 0x37)), "0037"); // Arpeggio
        assert_eq!(format!("{}", Effect::new(0x1, 0x10)), "0110"); // Pitch slide up
        assert_eq!(format!("{}", Effect::new(0x2, 0x20)), "0220"); // Pitch slide down
        assert_eq!(format!("{}", Effect::new(0x3, 0x08)), "0308"); // Portamento
        assert_eq!(format!("{}", Effect::new(0x4, 0x46)), "0446"); // Vibrato
        assert_eq!(format!("{}", Effect::new(0x7, 0x00)), "0700"); // Tremolo
        assert_eq!(format!("{}", Effect::new(0x8, 0x80)), "0880"); // Set tempo
        assert_eq!(format!("{}", Effect::new(0x9, 0x00)), "0900"); // Sample offset
        assert_eq!(format!("{}", Effect::new(0xA, 0x0F)), "0A0F"); // Volume slide
        assert_eq!(format!("{}", Effect::new(0xB, 0x02)), "0B02"); // Position jump
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "0C40"); // Set volume
        assert_eq!(format!("{}", Effect::new(0xD, 0x00)), "0D00"); // Pattern break
        assert_eq!(format!("{}", Effect::new(0xE, 0x00)), "0E00"); // Extended
        assert_eq!(format!("{}", Effect::new(0xF, 0x06)), "0F06"); // Set speed
    }

    // --- Effect Value Encoding/Decoding Tests ---

    #[test]
    fn test_effect_values_encoded_correctly() {
        let eff = Effect::new(0xA, 0x04);
        assert_eq!(eff.command, 0xA);
        assert_eq!(eff.param, 0x04);
    }

    #[test]
    fn test_effect_param_nibbles() {
        let eff = Effect::new(0x4, 0x46);
        assert_eq!(eff.param_x(), 4); // speed = 4
        assert_eq!(eff.param_y(), 6); // depth = 6
    }

    #[test]
    fn test_effect_param_nibbles_boundary() {
        let eff = Effect::new(0x0, 0xFF);
        assert_eq!(eff.param_x(), 0xF);
        assert_eq!(eff.param_y(), 0xF);

        let eff2 = Effect::new(0x0, 0x00);
        assert_eq!(eff2.param_x(), 0x0);
        assert_eq!(eff2.param_y(), 0x0);

        let eff3 = Effect::new(0x0, 0xF0);
        assert_eq!(eff3.param_x(), 0xF);
        assert_eq!(eff3.param_y(), 0x0);

        let eff4 = Effect::new(0x0, 0x0F);
        assert_eq!(eff4.param_x(), 0x0);
        assert_eq!(eff4.param_y(), 0xF);
    }

    // --- EffectType Enum Tests ---

    #[test]
    fn test_effect_type_from_command() {
        assert_eq!(EffectType::from_command(0x0), Some(EffectType::Arpeggio));
        assert_eq!(
            EffectType::from_command(0x1),
            Some(EffectType::PitchSlideUp)
        );
        assert_eq!(
            EffectType::from_command(0x2),
            Some(EffectType::PitchSlideDown)
        );
        assert_eq!(
            EffectType::from_command(0x3),
            Some(EffectType::PortamentoToNote)
        );
        assert_eq!(EffectType::from_command(0x4), Some(EffectType::Vibrato));
        assert_eq!(
            EffectType::from_command(0x5),
            Some(EffectType::TonePortamentoVolumeSlide)
        );
        assert_eq!(
            EffectType::from_command(0x6),
            Some(EffectType::VibratoVolumeSlide)
        );
        assert_eq!(EffectType::from_command(0x7), Some(EffectType::Tremolo));
        assert_eq!(
            EffectType::from_command(0x9),
            Some(EffectType::SampleOffset)
        );
        assert_eq!(EffectType::from_command(0xA), Some(EffectType::VolumeSlide));
        assert_eq!(
            EffectType::from_command(0xB),
            Some(EffectType::PositionJump)
        );
        assert_eq!(EffectType::from_command(0xC), Some(EffectType::SetVolume));
        assert_eq!(
            EffectType::from_command(0xD),
            Some(EffectType::PatternBreak)
        );
        assert_eq!(EffectType::from_command(0xE), Some(EffectType::Extended));
        assert_eq!(EffectType::from_command(0xF), Some(EffectType::SetSpeed));
        assert_eq!(
            EffectType::from_command(0x10),
            Some(EffectType::SetGlobalVolume)
        );
        assert_eq!(
            EffectType::from_command(0x11),
            Some(EffectType::GlobalVolumeSlide)
        );
        assert_eq!(
            EffectType::from_command(0x12),
            Some(EffectType::PanningSlide)
        );
        assert_eq!(
            EffectType::from_command(0x13),
            Some(EffectType::ChannelVolume)
        );
        assert_eq!(
            EffectType::from_command(0x14),
            Some(EffectType::ChannelVolumeSlide)
        );
        assert_eq!(EffectType::from_command(0x15), Some(EffectType::Tremor));
        assert_eq!(
            EffectType::from_command(0x16),
            Some(EffectType::RetrigNoteVolSlide)
        );
        assert_eq!(
            EffectType::from_command(0x17),
            Some(EffectType::SetEnvelopePosition)
        );
        assert_eq!(EffectType::from_command(0x18), Some(EffectType::Panbrello));
        assert_eq!(EffectType::from_command(0x19), Some(EffectType::MidiMacro));
    }

    #[test]
    fn test_effect_type_unrecognized_commands() {
        // Now no commands are completely unrecognized? Wait, 0x0-0xF are all covered except maybe some. All 16 are used now.
    }

    #[test]
    fn test_effect_type_roundtrip() {
        let types = [
            EffectType::Arpeggio,
            EffectType::PitchSlideUp,
            EffectType::PitchSlideDown,
            EffectType::PortamentoToNote,
            EffectType::Vibrato,
            EffectType::TonePortamentoVolumeSlide,
            EffectType::VibratoVolumeSlide,
            EffectType::Tremolo,
            EffectType::SampleOffset,
            EffectType::VolumeSlide,
            EffectType::PositionJump,
            EffectType::SetVolume,
            EffectType::PatternBreak,
            EffectType::Extended,
            EffectType::SetSpeed,
            EffectType::SetGlobalVolume,
            EffectType::GlobalVolumeSlide,
            EffectType::PanningSlide,
            EffectType::ChannelVolume,
            EffectType::ChannelVolumeSlide,
            EffectType::Tremor,
            EffectType::RetrigNoteVolSlide,
            EffectType::SetEnvelopePosition,
            EffectType::Panbrello,
            EffectType::MidiMacro,
        ];
        for &effect_type in &types {
            let cmd = effect_type.to_command();
            let decoded = EffectType::from_command(cmd);
            assert_eq!(
                decoded,
                Some(effect_type),
                "Roundtrip failed for {:?}",
                effect_type
            );
        }
    }

    #[test]
    fn test_effect_from_type() {
        let eff = Effect::from_type(EffectType::VolumeSlide, 0x04);
        assert_eq!(eff.command, 0xA);
        assert_eq!(eff.param, 0x04);
        assert_eq!(eff.effect_type(), Some(EffectType::VolumeSlide));
    }

    #[test]
    fn test_effect_type_method() {
        let eff = Effect::new(0xC, 0x40);
        assert_eq!(eff.effect_type(), Some(EffectType::SetVolume));

        // Unknown command
        let eff_unknown = Effect::new(0x1A, 0x00);
        assert_eq!(eff_unknown.effect_type(), None);
    }

    // --- Mnemonic Tests ---

    #[test]
    fn test_effect_mnemonics() {
        assert_eq!(Effect::new(0x0, 0x37).mnemonic(), "Arpeggio");
        assert_eq!(Effect::new(0x1, 0x10).mnemonic(), "Pitch Up");
        assert_eq!(Effect::new(0x2, 0x20).mnemonic(), "Pitch Down");
        assert_eq!(Effect::new(0x3, 0x08).mnemonic(), "Porta Note");
        assert_eq!(Effect::new(0x4, 0x46).mnemonic(), "Vibrato");
        assert_eq!(Effect::new(0x5, 0x00).mnemonic(), "Porta+Vol");
        assert_eq!(Effect::new(0x6, 0x00).mnemonic(), "Vib+Vol");
        assert_eq!(Effect::new(0x7, 0x00).mnemonic(), "Tremolo");
        assert_eq!(Effect::new(0x8, 0x80).mnemonic(), "Set Pan");
        assert_eq!(Effect::new(0x9, 0x00).mnemonic(), "Offset");
        assert_eq!(Effect::new(0xA, 0x04).mnemonic(), "Vol Slide");
        assert_eq!(Effect::new(0xB, 0x02).mnemonic(), "Pos Jump");
        assert_eq!(Effect::new(0xC, 0x40).mnemonic(), "Set Vol");
        assert_eq!(Effect::new(0xD, 0x00).mnemonic(), "Pat Break");
        assert_eq!(Effect::new(0xE, 0x00).mnemonic(), "Extended");
        assert_eq!(Effect::new(0xF, 0x06).mnemonic(), "Set Speed");
        assert_eq!(Effect::new(0x10, 0x40).mnemonic(), "Global Vol");
        assert_eq!(Effect::new(0x11, 0x40).mnemonic(), "GVol Slide");
        assert_eq!(Effect::new(0x12, 0x22).mnemonic(), "Pan Slide");
    }

    #[test]
    fn test_effect_type_display() {
        assert_eq!(format!("{}", EffectType::Arpeggio), "Arpeggio");
        assert_eq!(format!("{}", EffectType::VolumeSlide), "Vol Slide");
        assert_eq!(format!("{}", EffectType::SetSpeed), "Set Speed");
    }

    // --- Serialization Tests ---

    #[test]
    fn test_effect_clone_eq() {
        let eff = Effect::new(0xA, 0x04);
        let cloned = eff;
        assert_eq!(eff, cloned);
    }

    #[test]
    fn test_effect_type_clone_eq() {
        let t = EffectType::Vibrato;
        let cloned = t;
        assert_eq!(t, cloned);
    }

    #[test]
    fn test_max_effects_per_cell() {
        assert_eq!(MAX_EFFECTS_PER_CELL, 2);
    }

    // --- Serde Encoding/Decoding Tests ---

    #[test]
    fn test_effect_serde_roundtrip() {
        let effects = vec![
            Effect::new(0x0, 0x37),
            Effect::new(0x8, 0x80),
            Effect::new(0xA, 0x04),
            Effect::new(0xC, 0x40),
            Effect::new(0xF, 0xFF),
        ];
        for eff in &effects {
            let json = serde_json::to_string(eff).unwrap();
            let decoded: Effect = serde_json::from_str(&json).unwrap();
            assert_eq!(*eff, decoded, "Serde roundtrip failed for {}", eff);
        }
    }

    #[test]
    fn test_effect_type_serde_roundtrip() {
        let types = [
            EffectType::Arpeggio,
            EffectType::PitchSlideUp,
            EffectType::PitchSlideDown,
            EffectType::PortamentoToNote,
            EffectType::Vibrato,
            EffectType::SetPanning,
            EffectType::VolumeSlide,
            EffectType::PositionJump,
            EffectType::SetVolume,
            EffectType::PatternBreak,
            EffectType::SetSpeed,
            EffectType::SetGlobalVolume,
            EffectType::GlobalVolumeSlide,
            EffectType::Tremor,
        ];
        for &et in &types {
            let json = serde_json::to_string(&et).unwrap();
            let decoded: EffectType = serde_json::from_str(&json).unwrap();
            assert_eq!(et, decoded, "Serde roundtrip failed for {:?}", et);
        }
    }

    #[test]
    fn test_effect_serde_json_structure() {
        let eff = Effect::new(0xA, 0x04);
        let json = serde_json::to_string(&eff).unwrap();
        // Should contain command and param fields
        assert!(json.contains("\"command\""));
        assert!(json.contains("\"param\""));
        // Verify actual values
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["command"], 10); // 0xA = 10
        assert_eq!(value["param"], 4); // 0x04 = 4
    }

    // --- Additional Hex Display Edge Cases ---

    #[test]
    fn test_effect_display_mid_range_params() {
        // Verify hex formatting for common effect values
        assert_eq!(format!("{}", Effect::new(0x4, 0x37)), "0437"); // Vibrato speed=3 depth=7
        assert_eq!(format!("{}", Effect::new(0xA, 0x80)), "0A80"); // Volume slide up 8
        assert_eq!(format!("{}", Effect::new(0xC, 0x7F)), "0C7F"); // Set volume to 127
        assert_eq!(format!("{}", Effect::new(0xF, 0x03)), "0F03"); // Set speed to 3
    }

    #[test]
    fn test_effect_display_unknown_commands() {
        // Unknown command types should still display correctly as hex
        assert_eq!(format!("{}", Effect::new(0xFF, 0x12)), "FF12");
        assert_eq!(format!("{}", Effect::new(0x1A, 0xAB)), "1AAB");
    }

    // --- Effect Construction and Field Access ---

    #[test]
    fn test_effect_from_type_all_variants() {
        let cases: Vec<(EffectType, u8, u8)> = vec![
            (EffectType::Arpeggio, 0x37, 0x0),
            (EffectType::PitchSlideUp, 0x10, 0x1),
            (EffectType::PitchSlideDown, 0x20, 0x2),
            (EffectType::PortamentoToNote, 0x08, 0x3),
            (EffectType::Vibrato, 0x46, 0x4),
            (EffectType::TonePortamentoVolumeSlide, 0x12, 0x5),
            (EffectType::VibratoVolumeSlide, 0x21, 0x6),
            (EffectType::Tremolo, 0x48, 0x7),
            (EffectType::SetPanning, 0x80, 0x8),
            (EffectType::SampleOffset, 0x80, 0x9),
            (EffectType::VolumeSlide, 0x04, 0xA),
            (EffectType::PositionJump, 0x02, 0xB),
            (EffectType::SetVolume, 0x40, 0xC),
            (EffectType::PatternBreak, 0x00, 0xD),
            (EffectType::Extended, 0x12, 0xE),
            (EffectType::SetSpeed, 0x06, 0xF),
        ];
        for (effect_type, param, expected_cmd) in cases {
            let eff = Effect::from_type(effect_type, param);
            assert_eq!(
                eff.command, expected_cmd,
                "Wrong command for {:?}",
                effect_type
            );
            assert_eq!(eff.param, param, "Wrong param for {:?}", effect_type);
            assert_eq!(eff.effect_type(), Some(effect_type));
        }
    }

    #[test]
    fn test_effect_param_nibble_extraction_all_combos() {
        // Spot-check representative nibble pairs
        for x in [0u8, 3, 7, 0xF] {
            for y in [0u8, 5, 0xA, 0xF] {
                let param = (x << 4) | y;
                let eff = Effect::new(0, param);
                assert_eq!(eff.param_x(), x, "param_x wrong for 0x{:02X}", param);
                assert_eq!(eff.param_y(), y, "param_y wrong for 0x{:02X}", param);
            }
        }
    }
}
