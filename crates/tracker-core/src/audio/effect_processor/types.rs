//! Shared types for effect processing.

/// Commands that effects can send to the transport system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportCommand {
    /// Set the tempo to the given BPM value.
    SetBpm(f64),
    /// Set the ticks per line (TPL).
    SetTpl(u32),
    /// Jump to the given arrangement position (Bxx effect).
    PositionJump(usize),
    /// Break to the given row of the next pattern (Dxx effect).
    PatternBreak(usize),
    /// Pattern loop (E6x): 0 = set loop point, x > 0 = loop x times.
    PatternLoop(u8),
    /// Pattern delay (EEx): delay advancing by x extra row-lengths.
    PatternDelay(u8),
    /// Zxx: Custom effect command triggering a Rhai script macro.
    ScriptTrigger { channel: usize, param: u8 },
}

/// Output from the effect processor for a single channel.
#[derive(Debug, Clone, Copy)]
pub struct VoiceRenderState {
    /// Combined pitch ratio from all pitch effects.
    pub pitch_ratio: f64,
    /// Volume gain from effect commands. None means no override.
    pub gain: Option<f32>,
    /// Base channel volume set by IT/XM master effects (Mxx). 0.0 - 1.0.
    pub channel_volume: f32,
    /// Effective panning position (0.0 = left, 0.5 = center, 1.0 = right).
    pub pan_override: Option<f32>,
}
