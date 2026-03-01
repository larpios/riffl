/// Effect command types and processing for the tracker.
///
/// Effects modify playback behavior on a per-row basis. Each effect has a
/// command type (identifying what the effect does) and a parameter byte
/// controlling its intensity or target value.

use std::fmt;
use serde::{Serialize, Deserialize};

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
    /// `Axy` — Volume slide: x = up speed, y = down speed.
    VolumeSlide,
    /// `Bxx` — Position jump: jump to arrangement position xx.
    PositionJump,
    /// `Cxx` — Set volume to xx.
    SetVolume,
    /// `Dxx` — Pattern break: jump to row xx of the next pattern.
    PatternBreak,
    /// `Fxx` — Set speed/BPM.
    SetSpeed,
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
            0xA => Some(EffectType::VolumeSlide),
            0xB => Some(EffectType::PositionJump),
            0xC => Some(EffectType::SetVolume),
            0xD => Some(EffectType::PatternBreak),
            0xF => Some(EffectType::SetSpeed),
            _ => None,
        }
    }

    /// Convert an effect type to its command byte.
    pub fn to_command(self) -> u8 {
        match self {
            EffectType::Arpeggio => 0x0,
            EffectType::PitchSlideUp => 0x1,
            EffectType::PitchSlideDown => 0x2,
            EffectType::PortamentoToNote => 0x3,
            EffectType::Vibrato => 0x4,
            EffectType::VolumeSlide => 0xA,
            EffectType::SetVolume => 0xC,
            EffectType::PositionJump => 0xB,
            EffectType::PatternBreak => 0xD,
            EffectType::SetSpeed => 0xF,
        }
    }

    /// Get the mnemonic name for this effect type.
    pub fn mnemonic(&self) -> &'static str {
        match self {
            EffectType::Arpeggio => "Arpeggio",
            EffectType::PitchSlideUp => "Pitch Up",
            EffectType::PitchSlideDown => "Pitch Down",
            EffectType::PortamentoToNote => "Porta Note",
            EffectType::Vibrato => "Vibrato",
            EffectType::VolumeSlide => "Vol Slide",
            EffectType::PositionJump => "Pos Jump",
            EffectType::SetVolume => "Set Vol",
            EffectType::PatternBreak => "Pat Break",
            EffectType::SetSpeed => "Set Speed",
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
/// Display format is 3 hex characters: command nibble + param byte (e.g., "A04", "C40", "F78").
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
}

impl fmt::Display for Effect {
    /// Display as 3 hex characters: command nibble + param byte.
    /// Example: command=0xA, param=0x04 → "A04"
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:01X}{:02X}", self.command, self.param)
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
        assert_eq!(format!("{}", Effect::new(0, 0)), "000");
        assert_eq!(format!("{}", Effect::new(0xA, 0x04)), "A04");
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "C40");
        assert_eq!(format!("{}", Effect::new(0xF, 0x78)), "F78");
        assert_eq!(format!("{}", Effect::new(0xF, 0xFF)), "FFF");
    }

    #[test]
    fn test_effect_display_boundary_values() {
        let min = Effect::new(0, 0);
        assert_eq!(format!("{}", min), "000");

        let max = Effect::new(0xF, 0xFF);
        assert_eq!(format!("{}", max), "FFF");
    }

    #[test]
    fn test_effect_display_all_commands() {
        // Verify each standard command byte displays correctly
        assert_eq!(format!("{}", Effect::new(0x0, 0x37)), "037"); // Arpeggio
        assert_eq!(format!("{}", Effect::new(0x1, 0x10)), "110"); // Pitch slide up
        assert_eq!(format!("{}", Effect::new(0x2, 0x20)), "220"); // Pitch slide down
        assert_eq!(format!("{}", Effect::new(0x3, 0x08)), "308"); // Portamento
        assert_eq!(format!("{}", Effect::new(0x4, 0x46)), "446"); // Vibrato
        assert_eq!(format!("{}", Effect::new(0xA, 0x0F)), "A0F"); // Volume slide
        assert_eq!(format!("{}", Effect::new(0xB, 0x02)), "B02"); // Position jump
        assert_eq!(format!("{}", Effect::new(0xC, 0x40)), "C40"); // Set volume
        assert_eq!(format!("{}", Effect::new(0xD, 0x00)), "D00"); // Pattern break
        assert_eq!(format!("{}", Effect::new(0xF, 0x06)), "F06"); // Set speed
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
        assert_eq!(EffectType::from_command(0x1), Some(EffectType::PitchSlideUp));
        assert_eq!(EffectType::from_command(0x2), Some(EffectType::PitchSlideDown));
        assert_eq!(EffectType::from_command(0x3), Some(EffectType::PortamentoToNote));
        assert_eq!(EffectType::from_command(0x4), Some(EffectType::Vibrato));
        assert_eq!(EffectType::from_command(0xA), Some(EffectType::VolumeSlide));
        assert_eq!(EffectType::from_command(0xB), Some(EffectType::PositionJump));
        assert_eq!(EffectType::from_command(0xC), Some(EffectType::SetVolume));
        assert_eq!(EffectType::from_command(0xD), Some(EffectType::PatternBreak));
        assert_eq!(EffectType::from_command(0xF), Some(EffectType::SetSpeed));
    }

    #[test]
    fn test_effect_type_unrecognized_commands() {
        assert_eq!(EffectType::from_command(0x5), None);
        assert_eq!(EffectType::from_command(0x6), None);
        assert_eq!(EffectType::from_command(0x7), None);
        assert_eq!(EffectType::from_command(0x8), None);
        assert_eq!(EffectType::from_command(0x9), None);
        assert_eq!(EffectType::from_command(0xE), None);
    }

    #[test]
    fn test_effect_type_roundtrip() {
        let types = [
            EffectType::Arpeggio,
            EffectType::PitchSlideUp,
            EffectType::PitchSlideDown,
            EffectType::PortamentoToNote,
            EffectType::Vibrato,
            EffectType::VolumeSlide,
            EffectType::PositionJump,
            EffectType::SetVolume,
            EffectType::PatternBreak,
            EffectType::SetSpeed,
        ];
        for &effect_type in &types {
            let cmd = effect_type.to_command();
            let decoded = EffectType::from_command(cmd);
            assert_eq!(decoded, Some(effect_type), "Roundtrip failed for {:?}", effect_type);
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
        let eff_unknown = Effect::new(0x7, 0x00);
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
        assert_eq!(Effect::new(0xA, 0x04).mnemonic(), "Vol Slide");
        assert_eq!(Effect::new(0xB, 0x02).mnemonic(), "Pos Jump");
        assert_eq!(Effect::new(0xC, 0x40).mnemonic(), "Set Vol");
        assert_eq!(Effect::new(0xD, 0x00).mnemonic(), "Pat Break");
        assert_eq!(Effect::new(0xF, 0x06).mnemonic(), "Set Speed");
        assert_eq!(Effect::new(0x7, 0x00).mnemonic(), "Unknown");
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
}
