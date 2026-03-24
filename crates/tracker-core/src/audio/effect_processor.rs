//! Effect command processor for the tracker mixer.
//!
//! Processes tracker effect commands (arpeggio, pitch slides, vibrato, volume
//! slides, etc.) and applies them to channel playback state. The effect
//! processor maintains per-channel running state and provides frame-level
//! modulation for continuous effects.

use crate::audio::pitch::{PitchCalculator, SlideMode};
use crate::pattern::effect::{Effect, EffectMode, EffectType};

/// Commands that effects can send to the transport system.
///
/// These are returned from `process_row()` and must be handled by the
/// caller (typically the mixer or app layer) to affect playback position
/// and tempo.
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
    /// The parameter (0x00-0xFF) is passed to the script handler.
    ScriptTrigger { channel: usize, param: u8 },
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
    /// Base channel volume set by IT/XM master effects (Mxx). 0.0 - 1.0.
    pub channel_volume: f32,
    /// Effective panning position (0.0 = left, 0.5 = center, 1.0 = right).
    pub pan_override: Option<f32>,
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
    /// The frequency of the note that triggered this voice (for portamento).
    pub triggered_note_freq: f64,

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

    // --- Panning ---
    /// Current effect-controlled panning position (0.0 = left, 0.5 = centre, 1.0 = right).
    /// `None` means no panning override from effects; the channel strip pan is used instead.
    pub panning_override: Option<f32>,

    // --- Volume ---
    /// Current effect-controlled volume (0.0 - 1.0).
    /// `None` means no volume override from effects.
    pub volume_override: Option<f32>,
    /// Volume slide up speed per row.
    pub volume_slide_up: u8,
    /// Volume slide down speed per row.
    pub volume_slide_down: u8,

    // --- Finetune ---
    /// Per-row finetune override (E5x effect).
    pub finetune_override: Option<i8>,

    // --- Pattern Loop (E6x) ---
    /// Row index where the loop starts.
    pub pattern_loop_start_row: Option<usize>,
    /// Number of remaining loop repetitions.
    pub pattern_loop_count: u8,

    // --- Sub-row Timing (E9x, ECx, EDx) ---
    /// Retrigger interval in ticks (E9x).
    pub retrigger_interval: Option<u8>,
    /// Volume action for retrigger (0=none, 1..5=down, 6..A=up etc).
    pub retrigger_volume_action: u8,
    /// Tick to cut the note (ECx).
    pub note_cut_tick: Option<u8>,
    /// Tick to delay the note trigger (EDx).
    pub note_delay_tick: Option<u8>,

    // --- Frame tracking ---
    /// Frames elapsed within the current row (for sub-row modulation).
    pub row_frame_counter: u32,
    /// Total frames per row (set from BPM and sample rate).
    pub frames_per_row: u32,
    /// Number of ticks per row (default 6).
    pub ticks_per_row: u8,
    /// Pitch slide up speed (per row approx).
    pub pitch_slide_up: u8,
    /// Pitch slide down speed (per row approx).
    pub pitch_slide_down: u8,
    /// Pitch calculation mode: linear semitone-based (default) or Amiga period-based.
    ///
    /// Set to `SlideMode::AmigaPeriod` when playing back MOD or S3M files.
    pub slide_mode: SlideMode,
    /// Effective Amiga period clock for this channel (AmigaPeriod mode only).
    /// Set by the mixer when a note triggers: AMIGA_PAL_CLOCK * base_freq / sample_rate
    pub period_clock: f64,

    // --- Advanced XM/IT Effects ---
    /// Base channel volume set by Mxx (0.0 - 1.0).
    pub channel_volume: f32,
    /// Channel volume slide up speed.
    pub channel_volume_slide_up: u8,
    /// Channel volume slide down speed.
    pub channel_volume_slide_down: u8,

    /// Panning slide right speed.
    pub panning_slide_right: u8,
    /// Panning slide left speed.
    pub panning_slide_left: u8,

    /// Envelope position override in ticks (Lxx). Consumed by Mixer.
    pub envelope_position_override: Option<usize>,

    /// Tremor on-time (in ticks).
    pub tremor_on: u8,
    /// Tremor off-time (in ticks).
    pub tremor_off: u8,
    /// Tremor active state.
    pub tremor_active: bool,

    /// Panbrello phase.
    pub panbrello_phase: f64,
    /// Panbrello speed.
    pub panbrello_speed: u8,
    /// Panbrello depth.
    pub panbrello_depth: u8,
    /// Panbrello active state.
    pub panbrello_active: bool,
}

impl Default for ChannelEffectState {
    fn default() -> Self {
        Self {
            arpeggio_x: 0,
            arpeggio_y: 0,
            arpeggio_active: false,
            pitch_ratio: 1.0,
            triggered_note_freq: 440.0,
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
            panning_override: None,
            volume_override: None,
            volume_slide_up: 0,
            volume_slide_down: 0,
            finetune_override: None,
            pattern_loop_start_row: None,
            pattern_loop_count: 0,
            retrigger_interval: None,
            retrigger_volume_action: 0,
            note_cut_tick: None,
            note_delay_tick: None,
            row_frame_counter: 0,
            frames_per_row: 6000, // ~125ms at 48kHz (120 BPM default)
            ticks_per_row: 6,
            pitch_slide_up: 0,
            pitch_slide_down: 0,
            slide_mode: SlideMode::default(),
            period_clock: crate::audio::pitch::AMIGA_PAL_CLOCK,
            channel_volume: 1.0,
            channel_volume_slide_up: 0,
            channel_volume_slide_down: 0,
            panning_slide_right: 0,
            panning_slide_left: 0,
            envelope_position_override: None,
            tremor_on: 0,
            tremor_off: 0,
            tremor_active: false,
            panbrello_phase: 0.0,
            panbrello_speed: 0,
            panbrello_depth: 0,
            panbrello_active: false,
        }
    }
}

impl ChannelEffectState {
    /// Reset all effect state for this channel.
    pub fn reset(&mut self) {
        let fpr = self.frames_per_row;
        let tpr = self.ticks_per_row;
        let mode = self.slide_mode;
        let clock = self.period_clock;
        let channel_vol = self.channel_volume;

        *self = Self {
            frames_per_row: fpr,
            ticks_per_row: tpr,
            slide_mode: mode,
            period_clock: clock,
            channel_volume: channel_vol,
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
        self.pitch_slide_up = 0;
        self.pitch_slide_down = 0;
        self.sample_offset = None;
        self.channel_volume_slide_up = 0;
        self.channel_volume_slide_down = 0;
        self.panning_slide_right = 0;
        self.panning_slide_left = 0;
        self.finetune_override = None;
        self.retrigger_interval = None;
        self.retrigger_volume_action = 0;
        self.note_cut_tick = None;
        self.note_delay_tick = None;
        self.envelope_position_override = None;
        self.tremor_active = false;
        self.panbrello_active = false;
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
    pub fn advance_vibrato(&mut self, _sample_rate: u32) {
        if !self.vibrato_active || self.vibrato_speed == 0 {
            return;
        }

        // FT2/XM Vibrato: phase increments by speed every tick.
        // There are 64 units of phase per full cycle (2*PI).
        // Total phase added per row = (speed / 64.0) * ticks_per_row * 2*PI.
        let cycles_per_row = (self.vibrato_speed as f64 / 64.0) * self.ticks_per_row as f64;
        let phase_inc_per_frame =
            (cycles_per_row * 2.0 * std::f64::consts::PI) / self.frames_per_row as f64;

        self.vibrato_phase += phase_inc_per_frame;

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
    pub fn advance_tremolo(&mut self, _sample_rate: u32) {
        if !self.tremolo_active || self.tremolo_speed == 0 {
            return;
        }

        // Same cycle logic as vibrato
        let cycles_per_row = (self.tremolo_speed as f64 / 64.0) * self.ticks_per_row as f64;
        let phase_inc_per_frame =
            (cycles_per_row * 2.0 * std::f64::consts::PI) / self.frames_per_row as f64;

        self.tremolo_phase += phase_inc_per_frame;

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
        if self.volume_override.is_none() && !self.tremolo_active && !self.tremor_active {
            return None;
        }
        let mut base_vol = self.volume_override.unwrap_or(1.0);
        let tremolo = self.tremolo_amplitude_modulation();

        if self.tremor_active {
            let total_ticks = (self.tremor_on + self.tremor_off) as u32;
            if total_ticks > 0 {
                let ticks_per_row = self.ticks_per_row.max(1) as u32;
                let frames_per_tick = self.frames_per_row.max(1) / ticks_per_row;
                let current_tick = self.row_frame_counter / frames_per_tick.max(1);
                // Tremor cycle: ON for `on` ticks, then OFF for `off` ticks.
                if current_tick % total_ticks >= self.tremor_on as u32 {
                    base_vol = 0.0;
                }
            }
        }

        Some((base_vol * tremolo * self.channel_volume).clamp(0.0, 2.0))
    }

    /// Advance portamento frequency by one tick.
    ///
    /// Delegates all pitch math to [`PitchCalculator`] using the channel's
    /// [`SlideMode`].  For `Linear` mode `portamento_speed` is in semitone
    /// units (effect param / 64).  For `AmigaPeriod` mode it should be the
    /// raw period delta (effect param as f64) set by the format loader.
    pub fn advance_portamento_tick(&mut self) {
        let (Some(target_freq), triggered_freq) =
            (self.portamento_target, self.triggered_note_freq)
        else {
            return;
        };

        if self.portamento_speed <= 0.0 || triggered_freq <= 0.0 {
            return;
        }

        let current_freq = self.pitch_ratio * triggered_freq;

        // Snap to target when already close enough (avoids floating-point drift).
        if (current_freq - target_freq).abs() < 0.001 {
            self.portamento_freq = Some(target_freq);
            self.pitch_ratio = target_freq / triggered_freq;
            return;
        }

        let new_freq = PitchCalculator::apply_portamento(
            current_freq,
            target_freq,
            self.portamento_speed,
            self.slide_mode,
            self.period_clock,
        );

        self.portamento_freq = Some(new_freq);
        self.pitch_ratio = new_freq / triggered_freq;
    }
    /// Advance pitch slide by one tick.
    ///
    /// Delegates all pitch math to [`PitchCalculator`] using the channel's
    /// [`SlideMode`].  For `AmigaPeriod` mode the slide units are raw period
    /// deltas rather than 1/64th-semitone steps.
    pub fn advance_pitch_slide_tick(&mut self) {
        if self.pitch_slide_up > 0 || self.pitch_slide_down > 0 {
            let current_freq = self.pitch_ratio * self.triggered_note_freq;
            let new_freq = PitchCalculator::apply_slide(
                current_freq,
                self.pitch_slide_up,
                self.pitch_slide_down,
                self.slide_mode,
                self.period_clock,
            );
            if self.triggered_note_freq > 0.0 {
                self.pitch_ratio = new_freq / self.triggered_note_freq;
            }
        }
    }

    /// Advance volume slide by one tick.
    pub fn advance_volume_slide_tick(&mut self) {
        if self.volume_slide_up > 0 || self.volume_slide_down > 0 {
            let current_vol = self.volume_override.unwrap_or(1.0);
            let delta_per_tick =
                (self.volume_slide_up as f32 - self.volume_slide_down as f32) / 64.0;
            self.volume_override = Some((current_vol + delta_per_tick).clamp(0.0, 2.0));
        }

        if self.channel_volume_slide_up > 0 || self.channel_volume_slide_down > 0 {
            let delta_per_tick = (self.channel_volume_slide_up as f32
                - self.channel_volume_slide_down as f32)
                / 64.0;
            self.channel_volume = (self.channel_volume + delta_per_tick).clamp(0.0, 1.0);
        }
    }

    /// Advance panning slide by one tick.
    pub fn advance_panning_slide_tick(&mut self) {
        if self.panning_slide_right > 0 || self.panning_slide_left > 0 {
            let current_pan = self.panning_override.unwrap_or(0.5);
            let delta_per_tick =
                (self.panning_slide_right as f32 - self.panning_slide_left as f32) / 64.0;
            self.panning_override = Some((current_pan + delta_per_tick).clamp(0.0, 1.0));
        }
    }

    /// Advance panbrello LFO phase.
    pub fn advance_panbrello(&mut self, _sample_rate: u32) {
        if !self.panbrello_active || self.panbrello_speed == 0 {
            return;
        }

        // Same cycle logic as vibrato
        let cycles_per_row = (self.panbrello_speed as f64 / 64.0) * self.ticks_per_row as f64;
        let phase_inc_per_frame =
            (cycles_per_row * 2.0 * std::f64::consts::PI) / self.frames_per_row as f64;

        self.panbrello_phase += phase_inc_per_frame;

        if self.panbrello_phase > 2.0 * std::f64::consts::PI {
            self.panbrello_phase -= 2.0 * std::f64::consts::PI;
        }
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
    /// Project-level effect interpretation mode.
    pub mode: EffectMode,
    pub global_volume: f32,
    pub global_volume_slide_up: u8,
    pub global_volume_slide_down: u8,
}

impl TrackerEffectProcessor {
    /// Create a new effect processor for the given number of channels.
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        Self {
            channels: vec![ChannelEffectState::default(); num_channels],
            sample_rate,
            mode: EffectMode::default(),
            global_volume: 1.0,
            global_volume_slide_up: 0,
            global_volume_slide_down: 0,
        }
    }

    /// Resize the per-channel vector dynamically when loading new modules.
    pub fn resize_channels(&mut self, num_channels: usize) {
        if num_channels > self.channels.len() {
            self.channels
                .resize(num_channels, ChannelEffectState::default());
        } else {
            self.channels.truncate(num_channels);
        }
    }

    /// Set the effect interpretation mode.
    pub fn set_effect_mode(&mut self, mode: EffectMode) {
        self.mode = mode;
    }

    /// Set the pitch slide mode for all channels.
    ///
    /// Call with `SlideMode::AmigaPeriod` when playing back MOD or S3M files,
    /// so that 1xx/2xx/3xx effects use Amiga hardware period arithmetic.
    pub fn set_slide_mode(&mut self, mode: SlideMode) {
        for ch in &mut self.channels {
            ch.slide_mode = mode;
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

        if let Some(freq) = note_frequency {
            let has_tone_porta = effects.iter().any(|e| {
                matches!(
                    e.effect_type(),
                    Some(EffectType::PortamentoToNote)
                        | Some(EffectType::TonePortamentoVolumeSlide)
                )
            });

            if !has_tone_porta {
                state.pitch_ratio = 1.0;
                state.triggered_note_freq = freq;
                state.portamento_target = None;
                state.portamento_freq = None;
            } else {
                state.portamento_target = Some(freq);
                // We keep the old pitch_ratio and triggered_note_freq
                // Tone portamento will slide pitch_ratio towards target/triggered
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
                    // Set pitch slide up speed
                    state.pitch_slide_up = effect.param;
                }

                EffectType::PitchSlideDown => {
                    // Set pitch slide down speed
                    state.pitch_slide_down = effect.param;
                }

                EffectType::PortamentoToNote => {
                    // Set portamento speed; target is set when a note is triggered
                    if effect.param > 0 {
                        state.portamento_speed = match state.slide_mode {
                            // Linear: 1/64th semitone per tick per unit
                            SlideMode::Linear => effect.param as f64 / 64.0,
                            // AmigaPeriod: raw period delta per tick (no scaling)
                            SlideMode::AmigaPeriod => effect.param as f64,
                        };
                    }
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
                    if effect.param_x() > 0 {
                        state.vibrato_speed = effect.param_x();
                    }
                    if effect.param_y() > 0 {
                        state.vibrato_depth = effect.param_y();
                    }
                }

                EffectType::TonePortamentoVolumeSlide => {
                    // 5xy: 3xx + Axy
                    // Uses existing portamento speed, adds volume slide
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                    }

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
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                    }
                }

                EffectType::Tremolo => {
                    state.tremolo_active = true;
                    if effect.param_x() > 0 {
                        state.tremolo_speed = effect.param_x();
                    }
                    if effect.param_y() > 0 {
                        state.tremolo_depth = effect.param_y();
                    }
                }

                EffectType::SampleOffset => {
                    // 9xx: Set sample playback offset
                    state.sample_offset = Some(effect.param as usize * 256);
                }

                EffectType::VolumeSlide => {
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                    }
                    // Volume slides in classic trackers are only applied on ticks > 0.
                    // We handle the continuous sliding smoothly in advance_frame.
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
                            // E1x: Fine Portamento Up (once per row)
                            // Standard Fine slide is 4x Extra Fine (4/64 semitones per unit)
                            let semitones = sub_param as f64 / 16.0;
                            state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                        }
                        0x2 => {
                            // E2x: Fine Portamento Down (once per row)
                            let semitones = sub_param as f64 / 16.0;
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
                        0x5 => {
                            // E5x: Set Finetune
                            // Maps 0x0-0x7 to 0..+7, 0x8-0xF to -8..-1
                            let ft = if sub_param <= 0x7 {
                                sub_param as i8
                            } else {
                                (sub_param as i8) - 16
                            };
                            state.finetune_override = Some(ft);
                        }
                        0x6 => {
                            // E6x: Pattern Loop
                            commands.push(TransportCommand::PatternLoop(sub_param));
                        }
                        0x7 => {
                            // E7x: Set Tremolo Waveform
                            state.tremolo_waveform = sub_param;
                        }
                        0x9 => {
                            // E9x: Retrigger Note
                            state.retrigger_interval = Some(sub_param);
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
                            state.note_cut_tick = Some(sub_param);
                        }
                        0xD => {
                            // EDx: Note Delay
                            state.note_delay_tick = Some(sub_param);
                        }
                        0xE => {
                            // EEx: Pattern Delay
                            commands.push(TransportCommand::PatternDelay(sub_param));
                        }
                        _ => {}
                    }
                }

                EffectType::SetPanning => {
                    // param 0x00=full left, 0x80=centre, 0xFF=full right
                    state.panning_override = Some(effect.param as f32 / 255.0);
                }

                EffectType::SetSpeed => {
                    if effect.param >= 32 {
                        // Values 0x20–0xFF set BPM
                        commands.push(TransportCommand::SetBpm(effect.param as f64));
                    } else if effect.param > 0 {
                        // Values 0x01–0x1F set ticks per line
                        commands.push(TransportCommand::SetTpl(effect.param as u32));
                    }
                }

                EffectType::SetGlobalVolume => {
                    // IT uses 0-128 for global volume.
                    self.global_volume = (effect.param as f32 / 128.0).clamp(0.0, 1.0);
                }
                EffectType::GlobalVolumeSlide => {
                    self.global_volume_slide_up = effect.param_x();
                    self.global_volume_slide_down = effect.param_y();
                }
                EffectType::PanningSlide => {
                    state.panning_slide_right = effect.param_x();
                    state.panning_slide_left = effect.param_y();
                }
                EffectType::ChannelVolume => {
                    state.channel_volume = (effect.param as f32 / 64.0).clamp(0.0, 1.0);
                }
                EffectType::ChannelVolumeSlide => {
                    if effect.param_x() == 0x0A {
                        // NxF: Fine Channel Vol Up
                        state.channel_volume = (state.channel_volume
                            + (effect.param_y() as f32 / 64.0))
                            .clamp(0.0, 1.0);
                    } else if effect.param_x() == 0x0B {
                        // NFy: Fine Channel Vol Down
                        state.channel_volume = (state.channel_volume
                            - (effect.param_y() as f32 / 64.0))
                            .clamp(0.0, 1.0);
                    } else {
                        state.channel_volume_slide_up = effect.param_x();
                        state.channel_volume_slide_down = effect.param_y();
                    }
                }
                EffectType::Tremor => {
                    state.tremor_active = true;
                    if effect.param_x() > 0 {
                        state.tremor_on = effect.param_x();
                    }
                    if effect.param_y() > 0 {
                        state.tremor_off = effect.param_y();
                    }
                }
                EffectType::RetrigNoteVolSlide => {
                    state.retrigger_interval = Some(effect.param_y());
                    state.retrigger_volume_action = effect.param_x();
                }
                EffectType::SetEnvelopePosition => {
                    state.envelope_position_override = Some(effect.param as usize);
                }
                EffectType::Panbrello => {
                    state.panbrello_active = true;
                    if effect.param_x() > 0 {
                        state.panbrello_speed = effect.param_x();
                    }
                    if effect.param_y() > 0 {
                        state.panbrello_depth = effect.param_y();
                    }
                }
                EffectType::MidiMacro => {
                    commands.push(TransportCommand::ScriptTrigger {
                        channel,
                        param: effect.param,
                    });
                }
                EffectType::ExtraFinePortaUp => {
                    // X1x: Extra Fine Portamento Up (once per row)
                    match state.slide_mode {
                        SlideMode::Linear => {
                            let semitones = effect.param as f64 / 64.0;
                            state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                        }
                        SlideMode::AmigaPeriod => {
                            let freq = state.pitch_ratio * state.triggered_note_freq;
                            let new_freq = PitchCalculator::apply_slide(
                                freq,
                                effect.param,
                                0,
                                state.slide_mode,
                                state.period_clock,
                            );
                            if state.triggered_note_freq > 0.0 {
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::ExtraFinePortaDown => {
                    // X2x: Extra Fine Portamento Down (once per row)
                    match state.slide_mode {
                        SlideMode::Linear => {
                            let semitones = effect.param as f64 / 64.0;
                            state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                        }
                        SlideMode::AmigaPeriod => {
                            let freq = state.pitch_ratio * state.triggered_note_freq;
                            let new_freq = PitchCalculator::apply_slide(
                                freq,
                                0,
                                effect.param,
                                state.slide_mode,
                                state.period_clock,
                            );
                            if state.triggered_note_freq > 0.0 {
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::SlideUpFine => {
                    // S3M Fine Slide Up: info & 0x0F units, applied once per row
                    match state.slide_mode {
                        SlideMode::Linear => {
                            let semitones = (effect.param & 0x0F) as f64 / 16.0;
                            state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                        }
                        SlideMode::AmigaPeriod => {
                            let freq = state.pitch_ratio * state.triggered_note_freq;
                            // S3M Fine Slide is 4x speed of Extra Fine Slide (raw period units)
                            let new_freq = PitchCalculator::apply_slide(
                                freq,
                                (effect.param & 0x0F) * 4,
                                0,
                                state.slide_mode,
                                state.period_clock,
                            );
                            if state.triggered_note_freq > 0.0 {
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::SlideDownFine => {
                    // S3M Fine Slide Down: info & 0x0F units, applied once per row
                    match state.slide_mode {
                        SlideMode::Linear => {
                            let semitones = (effect.param & 0x0F) as f64 / 16.0;
                            state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                        }
                        SlideMode::AmigaPeriod => {
                            let freq = state.pitch_ratio * state.triggered_note_freq;
                            let new_freq = PitchCalculator::apply_slide(
                                freq,
                                0,
                                (effect.param & 0x0F) * 4,
                                state.slide_mode,
                                state.period_clock,
                            );
                            if state.triggered_note_freq > 0.0 {
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::PortamentoExtraFine => {
                    // S3M Extra Fine Portamento (GxF): sets speed and slides once per row
                    state.portamento_speed = match state.slide_mode {
                        SlideMode::Linear => (effect.param & 0x0F) as f64 / 64.0,
                        SlideMode::AmigaPeriod => (effect.param & 0x0F) as f64,
                    };
                    state.advance_portamento_tick();
                }
                EffectType::PortamentoFine => {
                    // S3M Fine Portamento (GxE): sets speed and slides once per row
                    state.portamento_speed = match state.slide_mode {
                        SlideMode::Linear => (effect.param & 0x0F) as f64 / 16.0,
                        SlideMode::AmigaPeriod => ((effect.param & 0x0F) * 4) as f64,
                    };
                    state.advance_portamento_tick();
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
        if let Some(state) = self.channels.get_mut(channel) {
            state.row_frame_counter += 1;

            let ticks_per_row = state.ticks_per_row.max(1) as u32;
            let frames_per_tick = state.frames_per_row / ticks_per_row;

            // Tick boundary detection.
            // A tick boundary occurs when we've processed a full tick's worth of frames.
            // We use row_frame_counter % frames_per_tick == 0.
            // We skip tick 0 (the start of the row) for continuous slides.
            let is_tick_boundary = if frames_per_tick > 0 {
                state.row_frame_counter % frames_per_tick == 0
            } else {
                true
            };

            let current_tick = if frames_per_tick > 0 {
                state.row_frame_counter / frames_per_tick
            } else {
                state.row_frame_counter
            };

            // Non-tick effects (smooth per-frame modulations)
            state.advance_vibrato(self.sample_rate);
            state.advance_tremolo(self.sample_rate);
            state.advance_panbrello(self.sample_rate);

            // Tick-based effects (slides updated only at tick boundaries > 0)
            if is_tick_boundary && current_tick > 0 && current_tick < ticks_per_row {
                state.advance_portamento_tick();
                state.advance_pitch_slide_tick();
                state.advance_volume_slide_tick();
                state.advance_panning_slide_tick();
            }
        }
    }

    /// Advance global frame-level modulations.
    pub fn advance_global_frame(&mut self) {
        if self.global_volume_slide_up > 0 || self.global_volume_slide_down > 0 {
            // Global volume slides are also updated once per tick.
            // Here we use the first channel's frames_per_row/ticks_per_row as the timing reference.
            if let Some(state) = self.channels.first() {
                let ticks_per_row = state.ticks_per_row.max(1) as u32;
                let frames_per_tick = state.frames_per_row / ticks_per_row;

                if frames_per_tick > 0 && state.row_frame_counter % frames_per_tick == 0 {
                    let current_tick = state.row_frame_counter / frames_per_tick;
                    if current_tick > 0 && current_tick < ticks_per_row {
                        let delta_per_tick = (self.global_volume_slide_up as f32
                            - self.global_volume_slide_down as f32)
                            / 128.0;
                        self.global_volume = (self.global_volume + delta_per_tick).clamp(0.0, 1.0);
                    }
                }
            }
        }
    }

    /// Get the combined pitch ratio for a channel (all pitch effects combined).
    pub fn pitch_ratio(&self, channel: usize) -> f64 {
        self.channels
            .get(channel)
            .map(|s| s.combined_pitch_ratio())
            .unwrap_or(1.0)
    }

    /// Get the frequency of the last triggered note on this channel.
    pub fn last_note_frequency(&self, channel: usize) -> f64 {
        self.channels
            .get(channel)
            .map(|s| s.triggered_note_freq)
            .unwrap_or(440.0)
    }

    /// Get the portamento frequency for a channel, if portamento is active.
    pub fn portamento_frequency(&self, channel: usize) -> Option<f64> {
        self.channels.get(channel).and_then(|s| s.portamento_freq)
    }

    /// Get the sample playback offset for a channel (from 9xx effect).
    pub fn sample_offset(&self, channel: usize) -> Option<usize> {
        self.channels.get(channel).and_then(|s| s.sample_offset)
    }

    /// Get the finetune override for a channel (from E5x effect).
    pub fn finetune_override(&self, channel: usize) -> Option<i8> {
        self.channels.get(channel).and_then(|s| s.finetune_override)
    }

    /// Get the panning override for a channel (from 8xx effect), as 0.0–1.0.
    pub fn channel_panning(&self, channel: usize) -> Option<f32> {
        self.channels.get(channel).and_then(|s| {
            if s.panning_override.is_none() && !s.panbrello_active {
                return None;
            }
            let base = s.panning_override.unwrap_or(0.5);
            let panbrello = if s.panbrello_active {
                (s.panbrello_phase.sin() * s.panbrello_depth as f64 / 64.0) as f32
            } else {
                0.0
            };
            Some((base + panbrello).clamp(0.0, 1.0))
        })
    }

    /// Advance panbrello LFO phase.
    #[deprecated(note = "Use ChannelEffectState::advance_panbrello instead")]
    pub fn advance_panbrello(&mut self, channel: usize) {
        if let Some(state) = self.channels.get_mut(channel) {
            state.advance_panbrello(self.sample_rate);
        }
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
            channel_volume: self
                .channels
                .get(channel)
                .map(|s| s.channel_volume)
                .unwrap_or(1.0),
            pan_override: self.channel_panning(channel),
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

    pub fn set_period_clock(&mut self, channel: usize, clock: f64) {
        if let Some(ch) = self.channels.get_mut(channel) {
            ch.period_clock = clock;
        }
    }

    /// Reset all effect state (e.g., when playback stops).
    pub fn reset_all(&mut self) {
        for state in &mut self.channels {
            state.reset();
        }
    }

    /// Update the frames-per-row value for all channels based on BPM and sample rate.
    pub fn update_tempo(&mut self, bpm: f64) {
        for state in &mut self.channels {
            let seconds_per_row = (2.5 / bpm) * state.ticks_per_row as f64;
            state.frames_per_row = (seconds_per_row * self.sample_rate as f64) as u32;
            if state.frames_per_row == 0 {
                state.frames_per_row = 1;
            }
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
        let mut state = ChannelEffectState {
            frames_per_row: 12000,
            pitch_ratio: 2.0,
            volume_override: Some(0.5),
            ..Default::default()
        };
        state.reset();
        assert_eq!(state.frames_per_row, 12000);
        assert_eq!(state.pitch_ratio, 1.0);
        assert_eq!(state.volume_override, None);
    }

    #[test]
    fn test_new_row_resets_transients() {
        let mut state = ChannelEffectState {
            arpeggio_active: true,
            arpeggio_x: 3,
            vibrato_active: true,
            volume_slide_up: 4,
            row_frame_counter: 100,
            ..Default::default()
        };

        state.new_row();

        assert!(!state.arpeggio_active);
        assert_eq!(state.arpeggio_x, 0);
        assert!(!state.vibrato_active);
        assert_eq!(state.volume_slide_up, 0);
        assert_eq!(state.row_frame_counter, 0);
    }

    #[test]
    fn test_new_row_preserves_running_state() {
        let mut state = ChannelEffectState {
            pitch_ratio: 1.5,
            volume_override: Some(0.8),
            portamento_target: Some(440.0),
            ..Default::default()
        };

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

        // Should not slide immediately
        assert!((proc.pitch_ratio(0) - 1.0).abs() < 0.0001);

        // Advance to trigger
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 60; // 10 frames per tick
            state.ticks_per_row = 6;
        }
        // Advance 9 frames (within Tick 0)
        for _ in 0..9 {
            proc.advance_frame(0);
        }
        assert!(
            (proc.pitch_ratio(0) - 1.0).abs() < 0.0001,
            "Should not slide before tick boundary"
        );

        // Advance 1 more (completes Tick 0, slides for Tick 1)
        proc.advance_frame(0);
        assert!(
            proc.pitch_ratio(0) > 1.0,
            "Should slide at boundary of tick 0 and 1"
        );
    }

    #[test]
    fn test_process_row_pitch_slide_down() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PitchSlideDown, 0x10)];
        proc.process_row(0, &effects, None);

        // Should not slide immediately
        assert!((proc.pitch_ratio(0) - 1.0).abs() < 0.0001);

        // Advance to trigger (set small frames_per_row so ticks are short)
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 6; // 1 frame per tick
            state.ticks_per_row = 6;
        }

        proc.advance_frame(0);
        assert!(proc.pitch_ratio(0) < 1.0);
    }

    #[test]
    fn test_process_row_pitch_slide_accumulates() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::PitchSlideUp, 0x10)];

        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 6;
            state.ticks_per_row = 6;
        }

        // Row 1
        proc.process_row(0, &effects, None);
        // Advance through all ticks
        for _ in 0..60 {
            proc.advance_frame(0);
        }
        let ratio1 = proc.pitch_ratio(0);

        // Row 2
        proc.process_row(0, &effects, None);
        for _ in 0..60 {
            proc.advance_frame(0);
        }
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
    fn test_process_row_vibrato_volume_slide() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        // Initial volume
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);

        // 6xy: Vibrato + Volume Slide (Up 1, Down 0)
        let effects = vec![Effect::from_type(EffectType::VibratoVolumeSlide, 0x10)];
        proc.process_row(0, &effects, None);

        let state = proc.channel_state(0).unwrap();
        assert!(state.vibrato_active);
        assert_eq!(state.volume_slide_up, 1);
        assert_eq!(state.volume_slide_down, 0);

        // Should NOT change immediately
        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 1.0).abs() < 0.001);

        // Advance to trigger
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 60;
            state.ticks_per_row = 6;
        }
        // Advance 9 frames (within Tick 0)
        for _ in 0..9 {
            proc.advance_frame(0);
        }
        assert!((proc.volume_override(0).unwrap() - 1.0).abs() < 0.001);

        // Advance 1 more (completes Tick 0, slides for Tick 1)
        proc.advance_frame(0);
        let vol_after = proc.volume_override(0).unwrap();
        assert!(vol_after > 1.0);
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

        // 5xy: Porta + Vol Slide (Up 1, Down 0)
        let effects = vec![Effect::from_type(
            EffectType::TonePortamentoVolumeSlide,
            0x10,
        )]; // Up 1
        proc.process_row(0, &effects, None);

        // Should scale speed by 64.0 now (standard 1xx/2xx/3xx unit)
        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.portamento_speed, 16.0 / 64.0); // 0.25
        assert_eq!(state.volume_slide_up, 1);

        // Should NOT change volume immediately
        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_advance_frame_volume_slide() {
        let mut proc = TrackerEffectProcessor::new(1, 48000);
        // Set setup with 10 frames per tick
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 60;
            state.ticks_per_row = 6;
        }

        // Initial volume
        proc.process_row(0, &[Effect::from_type(EffectType::SetVolume, 0x40)], None);

        // Set volume slide (Up 1, Down 0)
        let effects = vec![Effect::from_type(EffectType::VolumeSlide, 0x10)];
        proc.process_row(0, &effects, None);

        let vol_after_row = proc.volume_override(0).unwrap();

        // Advance frames
        // Frame 1-9: Tick 0. No slide.
        for _ in 0..9 {
            proc.advance_frame(0);
        }
        assert_eq!(proc.volume_override(0).unwrap(), vol_after_row);

        // Frame 10: Tick 1 boundary. Slide happens.
        proc.advance_frame(0);

        let vol_after_tick_0 = proc.volume_override(0).unwrap();
        // Since frame 10 was processed, it should have slid once.
        let expected_delta = 1.0 / 64.0;
        assert!((vol_after_tick_0 - (vol_after_row + expected_delta as f32)).abs() < 0.000001);

        // Frame 11-19: no slide.
        for _ in 0..9 {
            proc.advance_frame(0);
        }
        assert_eq!(proc.volume_override(0).unwrap(), vol_after_tick_0);
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
        // No immediate change
        assert!((proc.volume_override(0).unwrap() - vol_before).abs() < 0.001);

        // Advance to trigger
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 6;
            state.ticks_per_row = 6;
        }
        proc.advance_frame(0);
        proc.advance_frame(0);
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
        // No immediate change
        assert!((proc.volume_override(0).unwrap() - vol_before).abs() < 0.001);

        // Advance to trigger
        if let Some(state) = proc.channel_state_mut(0) {
            state.frames_per_row = 6;
            state.ticks_per_row = 6;
        }
        proc.advance_frame(0);
        proc.advance_frame(0);
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
        // SetSpeed with param >= 32 sets BPM
        let effects = vec![Effect::from_type(EffectType::SetSpeed, 0x80)]; // BPM = 128
        let cmds = proc.process_row(0, &effects, None);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], TransportCommand::SetBpm(128.0));
    }

    #[test]
    fn test_process_row_set_tpl() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetSpeed, 0x06)]; // TPL
        let cmds = proc.process_row(0, &effects, None);

        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], TransportCommand::SetTpl(6));
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

        // Channel 1: no volume change, pitch starts unchanged (tick 0)
        assert_eq!(proc.volume_override(1), None);
        assert!((proc.pitch_ratio(1) - 1.0).abs() < 0.001);

        // Advance frames to trigger slide
        if let Some(state) = proc.channel_state_mut(1) {
            state.frames_per_row = 6;
            state.ticks_per_row = 6;
        }
        proc.advance_frame(1);
        proc.advance_frame(1);

        assert!(proc.pitch_ratio(1) > 1.0);
    }

    #[test]
    fn test_set_bpm_boundary() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);

        // 0x20 (32) = BPM 32 (minimum BPM-range value for SetSpeed)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0x20)], None);
        assert_eq!(cmds, vec![TransportCommand::SetBpm(32.0)]);

        // 0xFF = BPM 255 (maximum)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0xFF)], None);
        assert_eq!(cmds, vec![TransportCommand::SetBpm(255.0)]);

        // 0x1F (31) = speed 31, not BPM (just below boundary)
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::SetSpeed, 0x1F)], None);
        assert_eq!(cmds, vec![TransportCommand::SetTpl(31)]);
    }

    #[test]
    fn test_zxx_script_trigger() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let cmds = proc.process_row(0, &[Effect::from_type(EffectType::MidiMacro, 0x42)], None);
        assert_eq!(cmds.len(), 1);
        match cmds[0] {
            TransportCommand::ScriptTrigger { channel, param } => {
                assert_eq!(channel, 0);
                assert_eq!(param, 0x42);
            }
            _ => panic!("Expected ScriptTrigger"),
        }
    }

    #[test]
    fn test_zxx_script_trigger_different_channels() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let cmds1 = proc.process_row(0, &[Effect::from_type(EffectType::MidiMacro, 0x10)], None);
        let cmds2 = proc.process_row(2, &[Effect::from_type(EffectType::MidiMacro, 0x20)], None);

        match cmds1[0] {
            TransportCommand::ScriptTrigger { channel, param } => {
                assert_eq!(channel, 0);
                assert_eq!(param, 0x10);
            }
            _ => panic!("Expected ScriptTrigger on channel 0"),
        }
        match cmds2[0] {
            TransportCommand::ScriptTrigger { channel, param } => {
                assert_eq!(channel, 2);
                assert_eq!(param, 0x20);
            }
            _ => panic!("Expected ScriptTrigger on channel 2"),
        }
    }
}
