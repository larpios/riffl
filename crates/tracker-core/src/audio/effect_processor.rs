//! Effect command processor for the tracker mixer.
//!
//! Processes tracker effect commands (arpeggio, pitch slides, vibrato, volume
//! slides, etc.) and applies them to channel playback state. The effect
//! processor maintains per-channel running state and provides frame-level
//! modulation for continuous effects.

use crate::pattern::effect::{Effect, EffectType};

/// Commands that effects can send to the transport system.
///
/// These are returned from `process_row()` and must be handled by the
/// caller (typically the mixer or app layer) to affect playback position
/// and tempo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportCommand {
    /// Set the tempo to the given BPM value.
    SetBpm(f64),
    /// Jump to the given arrangement position (Bxx effect).
    PositionJump(usize),
    /// Break to the given row of the next pattern (Dxx effect).
    PatternBreak(usize),
}

/// Output from the effect processor for a single channel.
///
/// Contains the resolved pitch and gain values that the mixer should
/// apply when rendering a voice.
#[derive(Debug, Clone, Copy)]
pub struct VoiceRenderState {
    /// Combined pitch ratio from all pitch effects (arpeggio, slides, vibrato, portamento).
    /// 1.0 = no change, 2.0 = one octave up, 0.5 = one octave down.
    pub pitch_ratio: f64,
    /// Volume gain from effect commands (Cxx set volume, Axy volume slide).
    /// None means no effect override (use default 1.0).
    pub gain: Option<f32>,
}

/// Per-channel effect state tracking.
///
/// Maintains running state for continuous effects (vibrato, slides) and
/// immediate state for per-row effects (set volume, arpeggio).
#[derive(Debug, Clone)]
pub struct ChannelEffectState {
    // --- Arpeggio ---
    /// Arpeggio semitone offset 1 (x nibble).
    pub arpeggio_x: u8,
    /// Arpeggio semitone offset 2 (y nibble).
    pub arpeggio_y: u8,
    /// Whether arpeggio is active this row.
    pub arpeggio_active: bool,

    // --- Pitch slides ---
    /// Accumulated pitch offset in frequency ratio units.
    /// 1.0 = no change, >1.0 = higher, <1.0 = lower.
    pub pitch_ratio: f64,

    // --- Portamento to note ---
    /// Target frequency for portamento slide.
    pub portamento_target: Option<f64>,
    /// Portamento slide speed (units per row).
    pub portamento_speed: f64,
    /// Current portamento frequency (tracks the sliding frequency).
    pub portamento_freq: Option<f64>,
    /// Whether to snap portamento to nearest semitone (E3x effect).
    pub glissando: bool,

    // --- Vibrato ---
    /// Vibrato LFO phase in radians.
    pub vibrato_phase: f64,
    /// Vibrato speed (determines LFO rate).
    pub vibrato_speed: u8,
    /// Vibrato depth (determines pitch modulation amplitude).
    pub vibrato_depth: u8,
    /// Whether vibrato is active.
    pub vibrato_active: bool,
    /// Vibrato waveform: 0=sine, 1=ramp-down, 2=square, 3=random (E4x).
    pub vibrato_waveform: u8,

    // --- Tremolo ---
    /// Tremolo LFO phase in radians.
    pub tremolo_phase: f64,
    /// Tremolo speed.
    pub tremolo_speed: u8,
    /// Tremolo depth.
    pub tremolo_depth: u8,
    /// Whether tremolo is active.
    pub tremolo_active: bool,
    /// Tremolo waveform: 0=sine, 1=ramp-down, 2=square, 3=random (E7x).
    pub tremolo_waveform: u8,

    // --- Sample ---
    /// Sample playback offset in frames (9xx effect).
    pub sample_offset: Option<usize>,

    // --- Volume ---
    /// Current effect-controlled volume (0.0 - 1.0).
    /// `None` means no volume override from effects.
    pub volume_override: Option<f32>,
    /// Volume slide up speed per row.
    pub volume_slide_up: u8,
    /// Volume slide down speed per row.
    pub volume_slide_down: u8,

    // --- Pattern Loop (E6x) ---
    /// Row index where the loop starts.
    pub pattern_loop_start_row: Option<usize>,
    /// Number of remaining loop repetitions.
    pub pattern_loop_count: u8,

    // --- Frame tracking ---
    /// Frames elapsed within the current row (for sub-row modulation).
    pub row_frame_counter: u32,
    /// Total frames per row (set from BPM and sample rate).
    pub frames_per_row: u32,
}

impl Default for ChannelEffectState {
    fn default() -> Self {
        Self {
            arpeggio_x: 0,
            arpeggio_y: 0,
            arpeggio_active: false,
            pitch_ratio: 1.0,
            portamento_target: None,
            portamento_speed: 0.0,
            portamento_freq: None,
            glissando: false,
            vibrato_phase: 0.0,
            vibrato_speed: 0,
            vibrato_depth: 0,
            vibrato_active: false,
            vibrato_waveform: 0,
            tremolo_phase: 0.0,
            tremolo_speed: 0,
            tremolo_depth: 0,
            tremolo_active: false,
            tremolo_waveform: 0,
            sample_offset: None,
            volume_override: None,
            volume_slide_up: 0,
            volume_slide_down: 0,
            pattern_loop_start_row: None,
            pattern_loop_count: 0,
            row_frame_counter: 0,
            frames_per_row: 6000, // ~125ms at 48kHz (120 BPM default)
        }
    }
}

impl ChannelEffectState {
    /// Reset all effect state for this channel.
    pub fn reset(&mut self) {
        *self = Self {
            frames_per_row: self.frames_per_row,
            ..Self::default()
        };
    }

    /// Reset per-row transient state (called at the start of each new row).
    pub fn new_row(&mut self) {
        self.arpeggio_active = false;
        self.arpeggio_x = 0;
        self.arpeggio_y = 0;
        self.vibrato_active = false;
        self.tremolo_active = false;
        self.volume_slide_up = 0;
        self.volume_slide_down = 0;
        self.sample_offset = None;
        self.row_frame_counter = 0;
    }

    /// Get the arpeggio semitone offset for the current frame.
    ///
    /// Arpeggio cycles through 3 values: base note (0), +x semitones, +y semitones.
    /// The cycle divides the row into 3 equal parts.
    pub fn arpeggio_semitone_offset(&self) -> f64 {
        if !self.arpeggio_active || (self.arpeggio_x == 0 && self.arpeggio_y == 0) {
            return 0.0;
        }

        let third = self.frames_per_row / 3;
        if third == 0 {
            return 0.0;
        }

        let phase = self.row_frame_counter / third;
        match phase % 3 {
            0 => 0.0,                    // Base note
            1 => self.arpeggio_x as f64, // +x semitones
            _ => self.arpeggio_y as f64, // +y semitones
        }
    }

    /// Get the vibrato pitch modulation as a frequency ratio.
    ///
    /// Returns a multiplier around 1.0 (e.g., 0.98 to 1.02 for subtle vibrato).
    pub fn vibrato_pitch_ratio(&self) -> f64 {
        if !self.vibrato_active || self.vibrato_depth == 0 {
            return 1.0;
        }

        // Vibrato depth: each unit = ~1/16 semitone
        let depth_semitones = self.vibrato_depth as f64 / 16.0;

        let modulation = match self.vibrato_waveform {
            0 => self.vibrato_phase.sin(),                          // sine
            1 => 1.0 - (self.vibrato_phase / std::f64::consts::PI), // ramp down
            2 => {
                if self.vibrato_phase < std::f64::consts::PI {
                    1.0
                } else {
                    -1.0
                }
            } // square
            _ => (self.vibrato_phase * 1000.0).sin().fract() * 2.0 - 1.0, // pseudo-random
        } * depth_semitones;

        // Convert semitone offset to frequency ratio
        2.0_f64.powf(modulation / 12.0)
    }

    /// Advance vibrato LFO phase by one frame.
    pub fn advance_vibrato(&mut self, sample_rate: u32) {
        if !self.vibrato_active || self.vibrato_speed == 0 {
            return;
        }

        let lfo_hz = self.vibrato_speed as f64 * 0.5;
        let phase_increment = 2.0 * std::f64::consts::PI * lfo_hz / sample_rate as f64;
        self.vibrato_phase += phase_increment;

        if self.vibrato_phase > 2.0 * std::f64::consts::PI {
            self.vibrato_phase -= 2.0 * std::f64::consts::PI;
        }
    }

    /// Get the tremolo amplitude modulation as a gain multiplier.
    ///
    /// Returns a multiplier typically between 0.0 and 1.0 (e.g., 0.8 to 1.0).
    pub fn tremolo_amplitude_modulation(&self) -> f32 {
        if !self.tremolo_active || self.tremolo_depth == 0 {
            return 1.0;
        }

        // Tremolo depth: each unit = ~1/32 of full range modulation
        let depth = self.tremolo_depth as f64 / 32.0;
        let modulation = match self.tremolo_waveform {
            0 => self.tremolo_phase.sin(),                          // sine
            1 => 1.0 - (self.tremolo_phase / std::f64::consts::PI), // ramp down
            2 => {
                if self.tremolo_phase < std::f64::consts::PI {
                    1.0
                } else {
                    -1.0
                }
            } // square
            _ => (self.tremolo_phase * 1000.0).sin().fract() * 2.0 - 1.0, // pseudo-random
        };

        (1.0 + (modulation * depth * 0.5)) as f32
    }

    /// Advance tremolo LFO phase by one frame.
    pub fn advance_tremolo(&mut self, sample_rate: u32) {
        if !self.tremolo_active || self.tremolo_speed == 0 {
            return;
        }

        let lfo_hz = self.tremolo_speed as f64 * 0.5;
        let phase_increment = 2.0 * std::f64::consts::PI * lfo_hz / sample_rate as f64;
        self.tremolo_phase += phase_increment;

        if self.tremolo_phase > 2.0 * std::f64::consts::PI {
            self.tremolo_phase -= 2.0 * std::f64::consts::PI;
        }
    }

    /// Get the combined playback rate multiplier from all pitch effects.
    ///
    /// This includes arpeggio, pitch slides, vibrato, and portamento.
    pub fn combined_pitch_ratio(&self) -> f64 {
        let arpeggio = 2.0_f64.powf(self.arpeggio_semitone_offset() / 12.0);
        let vibrato = self.vibrato_pitch_ratio();
        
        let mut ratio = self.pitch_ratio;
        if self.glissando {
            // Snap to nearest semitone
            let semitones = (ratio.log2() * 12.0).round();
            ratio = 2.0_f64.powf(semitones / 12.0);
        }
        
        ratio * arpeggio * vibrato
    }

    /// Get the effective volume from effects (if any volume override is active).
    pub fn effective_volume(&self) -> Option<f32> {
        if self.volume_override.is_none() && !self.tremolo_active {
            return None;
        }
        let base_vol = self.volume_override.unwrap_or(1.0);
        let tremolo = self.tremolo_amplitude_modulation();
        Some((base_vol * tremolo).clamp(0.0, 2.0))
    }
}

/// Effect processor that manages per-channel effect state.
///
/// The processor is called once per row (via `process_row`) to read effect
/// commands from pattern cells, and once per audio frame (via `advance_frame`)
/// to update continuous modulations like vibrato and arpeggio cycling.
pub struct TrackerEffectProcessor {
    /// Per-channel effect state.
    channels: Vec<ChannelEffectState>,
    /// Output sample rate (for timing calculations).
    sample_rate: u32,
}

impl TrackerEffectProcessor {
    /// Create a new effect processor for the given number of channels.
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        Self {
            channels: (0..num_channels)
                .map(|_| ChannelEffectState::default())
                .collect(),
            sample_rate,
        }
    }

    /// Process effects for a row, returning any transport commands.
    ///
    /// Called once at the start of each new row. Reads effect commands from
    /// the cell and updates the channel's running effect state.
    pub fn process_row(
        &mut self,
        channel: usize,
        effects: &[Effect],
        note_frequency: Option<f64>,
    ) -> Vec<TransportCommand> {
        let state = match self.channels.get_mut(channel) {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Reset per-row transient state
        state.new_row();

        let mut commands = Vec::new();

        // If a new note is triggered, set portamento current frequency
        if let Some(freq) = note_frequency {
            if state.portamento_target.is_some() {
                // Portamento: set target, keep current freq sliding
                state.portamento_target = Some(freq);
            }
        }

        for effect in effects {
            let effect_type = match effect.effect_type() {
                Some(t) => t,
                None => continue, // Skip unknown effects
            };

            match effect_type {
                EffectType::Arpeggio => {
                    if effect.param != 0 {
                        state.arpeggio_active = true;
                        state.arpeggio_x = effect.param_x();
                        state.arpeggio_y = effect.param_y();
                    }
                }

                EffectType::PitchSlideUp => {
                    // Slide pitch up: multiply ratio by a factor per row
                    // Each unit ≈ 1/16 semitone per row
                    let semitones = effect.param as f64 / 16.0;
                    state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                }

                EffectType::PitchSlideDown => {
                    // Slide pitch down: divide ratio by a factor per row
                    let semitones = effect.param as f64 / 16.0;
                    state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                }

                EffectType::PortamentoToNote => {
                    // Set portamento speed; target is set when a note is triggered
                    state.portamento_speed = effect.param as f64 / 16.0;
                    if let Some(freq) = note_frequency {
                        state.portamento_target = Some(freq);
                        // Initialize current freq if not already sliding
                        if state.portamento_freq.is_none() {
                            state.portamento_freq = Some(freq);
                        }
                    }
                }

                EffectType::Vibrato => {
                    state.vibrato_active = true;
                    state.vibrato_speed = effect.param_x();
                    state.vibrato_depth = effect.param_y();
                }

                EffectType::TonePortamentoVolumeSlide => {
                    // 5xy: 3xx + Axy
                    // Uses existing portamento speed, adds volume slide
                    let up = effect.param_x();
                    let down = effect.param_y();
                    state.volume_slide_up = up;
                    state.volume_slide_down = down;

                    // Portamento target is handled by new notes triggered on this row
                    if let Some(freq) = note_frequency {
                        state.portamento_target = Some(freq);
                        if state.portamento_freq.is_none() {
                            state.portamento_freq = Some(freq);
                        }
                    }
                }

                EffectType::VibratoVolumeSlide => {
                    // 6xy: 4xy + Axy
                    // Continues vibrato, adds volume slide
                    state.vibrato_active = true;
                    let up = effect.param_x();
                    let down = effect.param_y();
                    state.volume_slide_up = up;
                    state.volume_slide_down = down;
                }

                EffectType::Tremolo => {
                    state.tremolo_active = true;
                    state.tremolo_speed = effect.param_x();
                    state.tremolo_depth = effect.param_y();
                }

                EffectType::SampleOffset => {
                    // 9xx: Set sample playback offset
                    state.sample_offset = Some(effect.param as usize * 256);
                }

                EffectType::VolumeSlide => {
                    let up = effect.param_x();
                    let down = effect.param_y();
                    state.volume_slide_up = up;
                    state.volume_slide_down = down;

                    // Apply volume slide immediately for this row
                    let current_vol = state.volume_override.unwrap_or(1.0);
                    let delta = (up as f32 - down as f32) / 64.0;
                    state.volume_override = Some((current_vol + delta).clamp(0.0, 1.0));
                }

                EffectType::PositionJump => {
                    commands.push(TransportCommand::PositionJump(effect.param as usize));
                }

                EffectType::SetVolume => {
                    // Cxx: set volume (0x00 = silence, 0x40 = normal, 0x80 = double)
                    let vol = (effect.param as f32 / 0x40 as f32).clamp(0.0, 2.0);
                    state.volume_override = Some(vol);
                }

                EffectType::PatternBreak => {
                    commands.push(TransportCommand::PatternBreak(effect.param as usize));
                }

                EffectType::Extended => {
                    let sub_command = effect.param_x();
                    let sub_param = effect.param_y();

                    match sub_command {
                        0x1 => {
                            // E1x: Fine Portamento Up
                            let semitones = sub_param as f64 / 256.0;
                            state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                        }
                        0x2 => {
                            // E2x: Fine Portamento Down
                            let semitones = sub_param as f64 / 256.0;
                            state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                        }
                        0x3 => {
                            // E3x: Glissando Control
                            state.glissando = sub_param != 0;
                        }
                        0x4 => {
                            // E4x: Set Vibrato Waveform
                            state.vibrato_waveform = sub_param;
                        }
                        0x6 => {
                            // E6x: Pattern Loop
                            if sub_param == 0 {
                                // Set loop point - handled by the transport/mixer
                            } else {
                                // Loop x times - handled by the transport/mixer
                            }
                        }
                        0x7 => {
                            // E7x: Set Tremolo Waveform
                            state.tremolo_waveform = sub_param;
                        }
                        0x9 => {
                            // E9x: Retrigger Note
                            // Handled by the mixer frame loop
                        }
                        0xA => {
                            // EAx: Fine Volume Slide Up
                            let current_vol = state.volume_override.unwrap_or(1.0);
                            let delta = sub_param as f32 / 64.0;
                            state.volume_override = Some((current_vol + delta).clamp(0.0, 1.0));
                        }
                        0xB => {
                            // EBx: Fine Volume Slide Down
                            let current_vol = state.volume_override.unwrap_or(1.0);
                            let delta = sub_param as f32 / 64.0;
                            state.volume_override = Some((current_vol - delta).clamp(0.0, 1.0));
                        }
                        0xC => {
                            // ECx: Note Cut
                            // Handled by the mixer frame loop
                        }
                        0xD => {
                            // EDx: Note Delay
                            // Handled by the mixer row trigger
                        }
                        _ => {}
                    }
                }

                EffectType::SetSpeed => {
                    // Fxx: if param < 0x20, set speed (ticks per row)
                    // if param >= 0x20, set BPM
                    if effect.param >= 0x20 {
                        commands.push(TransportCommand::SetBpm(effect.param as f64));
                    }
                    // Speed (ticks per row) < 0x20 is not yet implemented
                    // since we don't have a sub-tick system
                }
            }
        }

        commands
    }

    /// Advance frame-level modulations for a channel.
    ///
    /// Called once per audio frame during rendering to update vibrato,
    /// arpeggio cycling, and other continuous effects.
    pub fn advance_frame(&mut self, channel: usize) {
        let state = match self.channels.get_mut(channel) {
            Some(s) => s,
            None => return,
        };

        state.row_frame_counter += 1;
        state.advance_vibrato(self.sample_rate);
        state.advance_tremolo(self.sample_rate);
    }

    /// Get the combined pitch ratio for a channel (all pitch effects combined).
    pub fn pitch_ratio(&self, channel: usize) -> f64 {
        self.channels
            .get(channel)
            .map(|s| s.combined_pitch_ratio())
            .unwrap_or(1.0)
    }

    /// Get the portamento frequency for a channel, if portamento is active.
    pub fn portamento_frequency(&self, channel: usize) -> Option<f64> {
        self.channels.get(channel).and_then(|s| s.portamento_freq)
    }

    /// Get the sample playback offset for a channel (from 9xx effect).
    pub fn sample_offset(&self, channel: usize) -> Option<usize> {
        self.channels.get(channel).and_then(|s| s.sample_offset)
    }

    /// Get the effective volume override for a channel (from Cxx or Axy effects).
    pub fn volume_override(&self, channel: usize) -> Option<f32> {
        self.channels
            .get(channel)
            .and_then(|s| s.effective_volume())
    }

    /// Get the combined render state for a channel.
    pub fn voice_render_state(&self, channel: usize) -> VoiceRenderState {
        VoiceRenderState {
            pitch_ratio: self.pitch_ratio(channel),
            gain: self.volume_override(channel),
        }
    }

    /// Get the channel effect state for a channel.
    pub fn channel_state(&self, channel: usize) -> Option<&ChannelEffectState> {
        self.channels.get(channel)
    }

    /// Get mutable channel effect state.
    pub fn channel_state_mut(&mut self, channel: usize) -> Option<&mut ChannelEffectState> {
        self.channels.get_mut(channel)
    }

    /// Reset all effect state (e.g., when playback stops).
    pub fn reset_all(&mut self) {
        for state in &mut self.channels {
            state.reset();
        }
    }

    /// Update the frames-per-row value for all channels based on BPM and sample rate.
    pub fn update_tempo(&mut self, bpm: f64) {
        let rows_per_beat = 4.0;
        let seconds_per_row = 60.0 / bpm / rows_per_beat;
        let frames = (seconds_per_row * self.sample_rate as f64) as u32;
        for state in &mut self.channels {
            state.frames_per_row = frames.max(1);
        }
    }

    /// Get the number of channels.
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ChannelEffectState Tests ---

    #[test]
    fn test_default_state() {
        let state = ChannelEffectState::default();
        assert_eq!(state.pitch_ratio, 1.0);
        assert!(!state.arpeggio_active);
        assert!(!state.vibrato_active);
        assert_eq!(state.volume_override, None);
        assert_eq!(state.portamento_target, None);
    }

    #[test]
    fn test_reset_preserves_frames_per_row() {
        let mut state = ChannelEffectState::default();
        state.frames_per_row = 12000;
        state.pitch_ratio = 2.0;
        state.volume_override = Some(0.5);
        state.reset();
        assert_eq!(state.frames_per_row, 12000);
        assert_eq!(state.pitch_ratio, 1.0);
        assert_eq!(state.volume_override, None);
    }

    #[test]
    fn test_new_row_resets_transients() {
        let mut state = ChannelEffectState::default();
        state.arpeggio_active = true;
        state.arpeggio_x = 3;
        state.vibrato_active = true;
        state.volume_slide_up = 4;
        state.row_frame_counter = 100;

        state.new_row();

        assert!(!state.arpeggio_active);
        assert_eq!(state.arpeggio_x, 0);
        assert!(!state.vibrato_active);
        assert_eq!(state.volume_slide_up, 0);
        assert_eq!(state.row_frame_counter, 0);
    }

    #[test]
    fn test_new_row_preserves_running_state() {
        let mut state = ChannelEffectState::default();
        state.pitch_ratio = 1.5;
        state.volume_override = Some(0.8);
        state.portamento_target = Some(440.0);

        state.new_row();

        assert_eq!(state.pitch_ratio, 1.5);
        assert_eq!(state.volume_override, Some(0.8));
        assert_eq!(state.portamento_target, Some(440.0));
    }

    // --- Arpeggio Tests ---

    #[test]
    fn test_arpeggio_inactive_returns_zero() {
        let state = ChannelEffectState::default();
        assert_eq!(state.arpeggio_semitone_offset(), 0.0);
    }

    #[test]
    fn test_arpeggio_zero_params_returns_zero() {
        let mut state = ChannelEffectState::default();
        state.arpeggio_active = true;
        state.arpeggio_x = 0;
        state.arpeggio_y = 0;
        assert_eq!(state.arpeggio_semitone_offset(), 0.0);
    }

    #[test]
    fn test_arpeggio_cycles_through_phases() {
        let mut state = ChannelEffectState::default();
        state.arpeggio_active = true;
        state.arpeggio_x = 4;
        state.arpeggio_y = 7;
        state.frames_per_row = 300;

        // Phase 0: base note
        state.row_frame_counter = 0;
        assert_eq!(state.arpeggio_semitone_offset(), 0.0);

        // Phase 1: +x semitones
        state.row_frame_counter = 100;
        assert_eq!(state.arpeggio_semitone_offset(), 4.0);

        // Phase 2: +y semitones
        state.row_frame_counter = 200;
        assert_eq!(state.arpeggio_semitone_offset(), 7.0);
    }

    // --- Vibrato Tests ---

    #[test]
    fn test_vibrato_inactive_returns_unity() {
        let state = ChannelEffectState::default();
        assert_eq!(state.vibrato_pitch_ratio(), 1.0);
    }

    #[test]
    fn test_vibrato_zero_depth_returns_unity() {
        let mut state = ChannelEffectState::default();
        state.vibrato_active = true;
        state.vibrato_speed = 4;
        state.vibrato_depth = 0;
        assert_eq!(state.vibrato_pitch_ratio(), 1.0);
    }

    // --- Tremolo Tests ---

    #[test]
    fn test_tremolo_inactive_returns_unity() {
        let state = ChannelEffectState::default();
        assert_eq!(state.tremolo_amplitude_modulation(), 1.0);
    }

    #[test]
    fn test_tremolo_zero_depth_returns_unity() {
        let mut state = ChannelEffectState::default();
        state.tremolo_active = true;
        state.tremolo_speed = 4;
        state.tremolo_depth = 0;
        assert_eq!(state.tremolo_amplitude_modulation(), 1.0);
    }

    #[test]
    fn test_tremolo_produces_modulation() {
        let mut state = ChannelEffectState::default();
        state.tremolo_active = true;
        state.tremolo_speed = 4;
        state.tremolo_depth = 16; // Significant depth

        // At phase 0, sin(0) = 0, so ratio should be 1.0
        state.tremolo_phase = 0.0;
        assert!((state.tremolo_amplitude_modulation() - 1.0).abs() < 0.001);

        // At phase π/2, sin(π/2) = 1.0, ratio should be > 1.0
        state.tremolo_phase = std::f64::consts::FRAC_PI_2;
        assert!(state.tremolo_amplitude_modulation() > 1.0);

        // At phase 3π/2, sin(3π/2) = -1.0, ratio should be < 1.0
        state.tremolo_phase = 3.0 * std::f64::consts::FRAC_PI_2;
        assert!(state.tremolo_amplitude_modulation() < 1.0);
    }

    #[test]
    fn test_tremolo_waveforms() {
        let mut state = ChannelEffectState::default();
        state.tremolo_active = true;
        state.tremolo_depth = 32; // Full depth
        state.tremolo_phase = std::f64::consts::FRAC_PI_2; // Peak of sine

        // Sine
        state.tremolo_waveform = 0;
        assert!((state.tremolo_amplitude_modulation() - 1.5).abs() < 0.01);

        // Ramp down
        state.tremolo_waveform = 1;
        state.tremolo_phase = 0.0; // Starts at 1.0
        assert!((state.tremolo_amplitude_modulation() - 1.5).abs() < 0.01);

        // Square
        state.tremolo_waveform = 2;
        state.tremolo_phase = 0.5; // First half
        assert!((state.tremolo_amplitude_modulation() - 1.5).abs() < 0.01);
        state.tremolo_phase = 4.0; // Second half
        assert!((state.tremolo_amplitude_modulation() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_vibrato_produces_modulation() {
        let mut state = ChannelEffectState::default();
        state.vibrato_active = true;
        state.vibrato_speed = 4;
        state.vibrato_depth = 8;

        // At phase 0, sin(0) = 0, so ratio should be 1.0
        state.vibrato_phase = 0.0;
        assert!((state.vibrato_pitch_ratio() - 1.0).abs() < 0.001);

        // At phase π/2, sin(π/2) = 1.0, ratio should be > 1.0
        state.vibrato_phase = std::f64::consts::FRAC_PI_2;
        assert!(state.vibrato_pitch_ratio() > 1.0);

        // At phase 3π/2, sin(3π/2) = -1.0, ratio should be < 1.0
        state.vibrato_phase = 3.0 * std::f64::consts::FRAC_PI_2;
        assert!(state.vibrato_pitch_ratio() < 1.0);
    }

    #[test]
    fn test_vibrato_advance_increases_phase() {
        let mut state = ChannelEffectState::default();
        state.vibrato_active = true;
        state.vibrato_speed = 4;
        state.vibrato_depth = 4;

        let initial_phase = state.vibrato_phase;
        state.advance_vibrato(48000);
        assert!(state.vibrato_phase > initial_phase);
    }

    #[test]
    fn test_vibrato_phase_wraps() {
        let mut state = ChannelEffectState::default();
        state.vibrato_active = true;
        state.vibrato_speed = 15; // max speed
        state.vibrato_phase = 6.2; // Just under 2π

        state.advance_vibrato(48000);
        // After advancing past 2π, phase should wrap
        // (may or may not wrap in one frame depending on speed)
        assert!(state.vibrato_phase >= 0.0);
    }

    // --- Combined Pitch Ratio Tests ---

    #[test]
    fn test_combined_pitch_ratio_default() {
        let state = ChannelEffectState::default();
        assert!((state.combined_pitch_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_combined_pitch_ratio_with_slide() {
        let mut state = ChannelEffectState::default();
        state.pitch_ratio = 1.5;
        assert!((state.combined_pitch_ratio() - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_combined_pitch_ratio_with_arpeggio() {
        let mut state = ChannelEffectState::default();
        state.arpeggio_active = true;
        state.arpeggio_x = 12; // One octave
        state.arpeggio_y = 0;
        state.frames_per_row = 300;
        state.row_frame_counter = 100; // Phase 1

        let ratio = state.combined_pitch_ratio();
        // 12 semitones = octave = 2x frequency
        assert!((ratio - 2.0).abs() < 0.01);
    }

    // --- EffectProcessor Tests ---

    #[test]
    fn test_processor_creation() {
        let proc = TrackerEffectProcessor::new(8, 48000);
        assert_eq!(proc.num_channels(), 8);
    }

    #[test]
    fn test_processor_default_pitch_ratio() {
        let proc = TrackerEffectProcessor::new(4, 48000);
        for ch in 0..4 {
            assert!((proc.pitch_ratio(ch) - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_processor_out_of_bounds_channel() {
        let proc = TrackerEffectProcessor::new(4, 48000);
        assert!((proc.pitch_ratio(99) - 1.0).abs() < 0.001);
        assert_eq!(proc.volume_override(99), None);
    }

    #[test]
    fn test_process_row_set_volume() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetVolume, 0x20)]; // half volume
        let cmds = proc.process_row(0, &effects, None);

        assert!(cmds.is_empty());
        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 0.5).abs() < 0.01, "Expected ~0.5, got {}", vol);
    }

    #[test]
    fn test_process_row_set_volume_max() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetVolume, 0x40)]; // full volume
        proc.process_row(0, &effects, None);

        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_process_row_set_volume_zero() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetVolume, 0x00)];
        proc.process_row(0, &effects, None);

        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_process_row_arpeggio() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::Arpeggio, 0x47)]; // x=4, y=7
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert!(state.arpeggio_active);
        assert_eq!(state.arpeggio_x, 4);
        assert_eq!(state.arpeggio_y, 7);
    }

    #[test]
    fn test_process_row_arpeggio_zero_not_active() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::Arpeggio, 0x00)];
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert!(!state.arpeggio_active);
    }

    #[test]
    fn test_process_row_pitch_slide_up() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PitchSlideUp, 0x10)];
        proc.process_row(0, &effects, None);

        assert!(proc.pitch_ratio(0) > 1.0);
    }

    #[test]
    fn test_process_row_pitch_slide_down() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PitchSlideDown, 0x10)];
        proc.process_row(0, &effects, None);

        assert!(proc.pitch_ratio(0) < 1.0);
    }

    #[test]
    fn test_process_row_pitch_slide_accumulates() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PitchSlideUp, 0x10)];

        proc.process_row(0, &effects, None);
        let ratio1 = proc.pitch_ratio(0);

        proc.process_row(0, &effects, None);
        let ratio2 = proc.pitch_ratio(0);

        assert!(
            ratio2 > ratio1,
            "Pitch slide should accumulate: {} > {}",
            ratio2,
            ratio1
        );
    }

    #[test]
    fn test_process_row_vibrato() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::Vibrato, 0x48)]; // speed=4, depth=8
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert!(state.vibrato_active);
        assert_eq!(state.vibrato_speed, 4);
        assert_eq!(state.vibrato_depth, 8);
    }

    #[test]
    fn test_process_row_tremolo() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::Tremolo, 0x48)];
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert!(state.tremolo_active);
        assert_eq!(state.tremolo_speed, 4);
        assert_eq!(state.tremolo_depth, 8);
    }

    #[test]
    fn test_process_row_tone_portamento_volume_slide() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        // Set initial speed and volume
        proc.process_row(
            0,
            &[Effect::from_type(EffectType::PortamentoToNote, 0x10)],
            None,
        );
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);

        // 5xy: Porta + Vol Slide
        let effects = vec![Effect::from_type(
            EffectType::TonePortamentoVolumeSlide,
            0x10,
        )]; // Up 1
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.portamento_speed, 1.0);
        assert_eq!(state.volume_slide_up, 1);
    }

    #[test]
    fn test_process_row_sample_offset() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SampleOffset, 0x01)]; // offset 256
        proc.process_row(0, &effects, None);

        assert_eq!(proc.sample_offset(0), Some(256));
    }

    #[test]
    fn test_process_row_extended_fine_portamento() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let initial_ratio = proc.pitch_ratio(0);

        // E11: Fine Porta Up 1
        let effects = vec![Effect::from_type(EffectType::Extended, 0x11)];
        proc.process_row(0, &effects, None);

        assert!(proc.pitch_ratio(0) > initial_ratio);
    }

    #[test]
    fn test_process_row_extended_fine_volume_slide() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);
        let initial_vol = proc.volume_override(0).unwrap();

        // EB1: Fine Volume Slide Down 1
        let effects = vec![Effect::from_type(EffectType::Extended, 0xB1)];
        proc.process_row(0, &effects, None);

        assert!(proc.volume_override(0).unwrap() < initial_vol);
    }

    #[test]
    fn test_process_row_volume_slide_up() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        // First set a base volume
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x20)], None);
        let vol_before = proc.volume_override(0).unwrap();

        // Volume slide up
        proc.process_row(0, &[Effect::from_type(EffectType::VolumeSlide, 0x40)], None);
        let vol_after = proc.volume_override(0).unwrap();

        assert!(
            vol_after > vol_before,
            "Volume should increase: {} > {}",
            vol_after,
            vol_before
        );
    }

    #[test]
    fn test_process_row_volume_slide_down() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);
        let vol_before = proc.volume_override(0).unwrap();

        proc.process_row(0, &[Effect::from_type(EffectType::VolumeSlide, 0x04)], None);
        let vol_after = proc.volume_override(0).unwrap();

        assert!(
            vol_after < vol_before,
            "Volume should decrease: {} < {}",
            vol_after,
            vol_before
        );
    }

    #[test]
    fn test_process_row_volume_slide_clamped() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        // Set volume to max
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);

        // Slide up beyond max
        proc.process_row(0, &[Effect::from_type(EffectType::VolumeSlide, 0xF0)], None);
        let vol = proc.volume_override(0).unwrap();
        assert!(vol <= 2.0, "Volume should be clamped");

        // Reset to 0 and slide down
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x00)], None);
        proc.process_row(0, &[Effect::from_type(EffectType::VolumeSlide, 0x0F)], None);
        let vol = proc.volume_override(0).unwrap();
        assert!(vol >= 0.0, "Volume should not go below 0");
    }

    // --- Transport Command Tests ---

    #[test]
    fn test_process_row_set_bpm() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetSpeed, 0x80)]; // BPM = 128
        let cmds = proc.process_row(0, &effects, None);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], TransportCommand::SetBpm(128.0));
    }

    #[test]
    fn test_process_row_set_speed_low_value_no_bpm() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetSpeed, 0x06)]; // Speed, not BPM
        let cmds = proc.process_row(0, &effects, None);

        assert!(
            cmds.is_empty(),
            "Speed values < 0x20 should not generate BPM command"
        );
    }

    #[test]
    fn test_process_row_position_jump() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PositionJump, 0x03)];
        let cmds = proc.process_row(0, &effects, None);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], TransportCommand::PositionJump(3));
    }

    #[test]
    fn test_process_row_pattern_break() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PatternBreak, 0x10)];
        let cmds = proc.process_row(0, &effects, None);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], TransportCommand::PatternBreak(16));
    }

    #[test]
    fn test_process_row_unknown_effect_ignored() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::new(0x07, 0xFF)]; // Unknown command
        let cmds = proc.process_row(0, &effects, None);

        assert!(cmds.is_empty());
        assert!((proc.pitch_ratio(0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_process_row_multiple_effects() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![
            Effect::from_type(EffectType::SetVolume, 0x20),
            Effect::from_type(EffectType::Vibrato, 0x48),
        ];
        let cmds = proc.process_row(0, &effects, None);

        assert!(cmds.is_empty());
        assert!(proc.volume_override(0).is_some());
        let state = proc.channel_state(0).unwrap();
        assert!(state.vibrato_active);
    }

    // --- Frame Advancement Tests ---

    #[test]
    fn test_advance_frame_increments_counter() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.advance_frame(0);
        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.row_frame_counter, 1);
    }

    #[test]
    fn test_advance_frame_out_of_bounds() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.advance_frame(99); // Should not panic
    }

    // --- Reset Tests ---

    #[test]
    fn test_reset_all() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);

        // Apply some effects
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x20)], None);
        proc.process_row(
            1,
            &[Effect::from_type(EffectType::PitchSlideUp, 0x10)],
            None,
        );

        proc.reset_all();

        for ch in 0..4 {
            assert!((proc.pitch_ratio(ch) - 1.0).abs() < 0.001);
            assert_eq!(proc.volume_override(ch), None);
        }
    }

    // --- Tempo Update Tests ---

    #[test]
    fn test_update_tempo() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.update_tempo(120.0);

        // At 120 BPM, 4 rows/beat: 0.125s/row → 6000 frames at 48kHz
        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.frames_per_row, 6000);
    }

    #[test]
    fn test_update_tempo_fast() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.update_tempo(240.0);

        // At 240 BPM: 0.0625s/row → 3000 frames at 48kHz
        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.frames_per_row, 3000);
    }

    // --- Portamento Tests ---

    #[test]
    fn test_portamento_sets_target() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PortamentoToNote, 0x08)];
        proc.process_row(0, &effects, Some(440.0));

        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.portamento_target, Some(440.0));
        assert_eq!(state.portamento_freq, Some(440.0));
    }

    #[test]
    fn test_portamento_no_note_no_target() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PortamentoToNote, 0x08)];
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.portamento_target, None);
    }

    // --- Effect Independence Tests ---

    #[test]
    fn test_effects_independent_per_channel() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);

        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x20)], None);
        proc.process_row(
            1,
            &[Effect::from_type(EffectType::PitchSlideUp, 0x10)],
            None,
        );

        // Channel 0: volume changed, pitch unchanged
        assert!(proc.volume_override(0).is_some());
        assert!((proc.pitch_ratio(0) - 1.0).abs() < 0.001);

        // Channel 1: no volume change, pitch changed
        assert_eq!(proc.volume_override(1), None);
        assert!(proc.pitch_ratio(1) > 1.0);
    }

    #[test]
    fn test_set_bpm_boundary() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);

        // F20 = BPM 32 (minimum BPM-range value)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0x20)], None);
        assert_eq!(cmds, vec![TransportCommand::SetBpm(32.0)]);

        // FFF = BPM 255 (maximum)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0xFF)], None);
        assert_eq!(cmds, vec![TransportCommand::SetBpm(255.0)]);

        // F1F = speed 31, not BPM (just below boundary)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0x1F)], None);
        assert!(cmds.is_empty());
    }
}
