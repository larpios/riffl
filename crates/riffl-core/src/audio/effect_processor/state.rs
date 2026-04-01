//! Per-channel effect state and modulation logic.

use crate::audio::pitch::{PitchCalculator, SlideMode};

/// ProTracker/Furnace-compatible 64-entry vibrato sine table.
/// Values normalized to [-1.0, 1.0]. Computed as sin(i / 64 * 2π).
/// Phase position (0..63) indexes directly; negative values give downward pitch modulation.
static VIBRATO_TABLE: [f64; 64] = [
    0.0,
    0.09801714,
    0.19509032,
    0.29028467,
    0.38268343,
    0.47139674,
    0.55557023,
    0.63439328,
    0.70710678,
    0.77301045,
    0.83146961,
    0.88192126,
    0.92387953,
    0.95694034,
    0.98078528,
    0.99518473,
    1.0,
    0.99518473,
    0.98078528,
    0.95694034,
    0.92387953,
    0.88192126,
    0.83146961,
    0.77301045,
    0.70710678,
    0.63439328,
    0.55557023,
    0.47139674,
    0.38268343,
    0.29028467,
    0.19509032,
    0.09801714,
    0.0,
    -0.09801714,
    -0.19509032,
    -0.29028467,
    -0.38268343,
    -0.47139674,
    -0.55557023,
    -0.63439328,
    -0.70710678,
    -0.77301045,
    -0.83146961,
    -0.88192126,
    -0.92387953,
    -0.95694034,
    -0.98078528,
    -0.99518473,
    -1.0,
    -0.99518473,
    -0.98078528,
    -0.95694034,
    -0.92387953,
    -0.88192126,
    -0.83146961,
    -0.77301045,
    -0.70710678,
    -0.63439328,
    -0.55557023,
    -0.47139674,
    -0.38268343,
    -0.29028467,
    -0.19509032,
    -0.09801714,
];

/// ProTracker/Furnace-compatible 128-entry tremolo amplitude table.
/// Values represent gain multiplier ≈ 1 + depth * (1 - cos(i/128 * 2π)) / 2.
/// Range is [0.0, 2.0] at full depth; table stores 0.5*(1-cos) in [0,1] normalized.
/// Actual gain = 1.0 + (table[i] - 0.5) * 2 * depth_fraction.
/// At i=0: silent end; at i=64: loudest peak.
static TREMOLO_TABLE: [f64; 128] = [
    0.0, 0.00304, 0.01214, 0.02728, 0.04840, 0.07544, 0.10828, 0.14673, 0.19060, 0.23965, 0.29357,
    0.35200, 0.41451, 0.48066, 0.54996, 0.62188, 0.69585, 0.77130, 0.84763, 0.92421, 1.00000,
    1.07425, 1.14622, 1.21512, 1.28023, 1.34089, 1.39644, 1.44633, 1.49000, 1.52697, 1.55690,
    1.57952, 1.59459, 1.60196, 1.60153, 1.59333, 1.57747, 1.55418, 1.52378, 1.48668, 1.44338,
    1.39441, 1.34040, 1.28199, 1.21985, 1.15466, 1.08712, 1.01792, 0.94774, 0.87724, 0.80707,
    0.73779, 0.66998, 0.60413, 0.54071, 0.48013, 0.42275, 0.36888, 0.31876, 0.27255, 0.23036,
    0.19224, 0.15820, 0.12820, 0.10223, 0.08021, 0.06204, 0.04762, 0.03675, 0.02927, 0.02495,
    0.02356, 0.02495, 0.02927, 0.03675, 0.04762, 0.06204, 0.08021, 0.10223, 0.12820, 0.15820,
    0.19224, 0.23036, 0.27255, 0.31876, 0.36888, 0.42275, 0.48013, 0.54071, 0.60413, 0.66998,
    0.73779, 0.80707, 0.87724, 0.94774, 1.01792, 1.08712, 1.15466, 1.21985, 1.28199, 1.34040,
    1.39441, 1.44338, 1.48668, 1.52378, 1.55418, 1.57747, 1.59333, 1.60153, 1.60196, 1.59459,
    1.57952, 1.55690, 1.52697, 1.49000, 1.44633, 1.39644, 1.34089, 1.28023, 1.21512, 1.14622,
    1.07425, 1.00000, 0.92421, 0.84763, 0.77130, 0.69585, 0.62188,
];

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
    /// Vibrato LFO position in table space [0.0, 64.0).
    /// Integer part indexes VIBRATO_TABLE; advances by speed per tick.
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
    /// Tremolo LFO position in table space [0.0, 128.0).
    /// Integer part indexes TREMOLO_TABLE; advances by speed per tick.
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
    /// Effect-controlled panning (8xx, P-slide, panbrello). 0.0=left, 0.5=centre, 1.0=right.
    /// None means no effect override; the channel strip pan is used instead.
    pub panning_override: Option<f32>,
    /// Instrument default-panning override (set by DfP bit in IT/panning in XM).
    /// Cleared when a new note triggers on an instrument without default panning.
    /// Effect panning (`panning_override`) takes priority over this.
    pub instrument_pan_override: Option<f32>,

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
    pub pitch_slide_up: f64,
    /// Pitch slide down speed (per row approx).
    pub pitch_slide_down: f64,
    /// Pitch calculation mode: linear semitone-based (default) or Amiga period-based.
    ///
    /// Set to `SlideMode::AmigaPeriod` when playing back MOD or S3M files.
    pub slide_mode: SlideMode,
    /// If true, use 14.3MHz clock logic (S3M/XM Amiga mode) which has 4x period resolution.
    pub use_high_res_periods: bool,
    /// Effective Amiga period clock for this channel (AmigaPeriod mode only).
    /// Set by the mixer when a note triggers: AMIGA_PAL_CLOCK * base_freq / sample_rate
    pub period_clock: f64,

    // --- Effect Memory (for param 00 continuation) ---
    pub prev_pitch_slide_up: f64,
    pub prev_pitch_slide_down: f64,
    pub prev_portamento_speed: f64,
    pub prev_volume_slide_up: u8,
    pub prev_volume_slide_down: u8,
    pub prev_vibrato_speed: u8,
    pub prev_vibrato_depth: u8,
    pub is_fine_vibrato: bool,

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
            instrument_pan_override: None,
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
            pitch_slide_up: 0.0,
            pitch_slide_down: 0.0,
            slide_mode: SlideMode::default(),
            use_high_res_periods: false,
            period_clock: crate::audio::pitch::AMIGA_PAL_CLOCK,
            prev_pitch_slide_up: 0.0,
            prev_pitch_slide_down: 0.0,
            prev_portamento_speed: 0.0,
            prev_volume_slide_up: 0,
            prev_volume_slide_down: 0,
            prev_vibrato_speed: 0,
            prev_vibrato_depth: 0,
            is_fine_vibrato: false,
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
        let high_res = self.use_high_res_periods;
        let channel_vol = self.channel_volume;

        // Preserve portamento state for pause/resume
        let porta_target = self.portamento_target;
        let porta_speed = self.portamento_speed;
        let prev_porta_speed = self.prev_portamento_speed;
        let porta_freq = self.portamento_freq;
        let triggered_freq = self.triggered_note_freq;
        let pitch_ratio = self.pitch_ratio;

        *self = Self {
            frames_per_row: fpr,
            ticks_per_row: tpr,
            slide_mode: mode,
            period_clock: clock,
            use_high_res_periods: high_res,
            channel_volume: channel_vol,
            portamento_target: porta_target,
            portamento_speed: porta_speed,
            prev_portamento_speed: prev_porta_speed,
            portamento_freq: porta_freq,
            triggered_note_freq: triggered_freq,
            pitch_ratio,
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
        self.pitch_slide_up = 0.0;
        self.pitch_slide_down = 0.0;
        self.portamento_speed = 0.0;
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
    pub fn arpeggio_semitone_offset(&self) -> f64 {
        if !self.arpeggio_active || (self.arpeggio_x == 0 && self.arpeggio_y == 0) {
            return 0.0;
        }

        // Arpeggio cycles once per tick (not once per third of the row).
        // With TPL=6: tick 0→base, tick 1→+x, tick 2→+y, tick 3→base, …
        let ticks_per_row = self.ticks_per_row.max(1) as u32;
        let frames_per_tick = (self.frames_per_row / ticks_per_row).max(1);
        let current_tick = self.row_frame_counter / frames_per_tick;

        match current_tick % 3 {
            0 => 0.0,                    // Base note
            1 => self.arpeggio_x as f64, // +x semitones
            _ => self.arpeggio_y as f64, // +y semitones
        }
    }

    /// Get the vibrato pitch modulation as a frequency ratio.
    pub fn vibrato_pitch_ratio(&self) -> f64 {
        if !self.vibrato_active || self.vibrato_depth == 0 {
            return 1.0;
        }

        let mut depth_semitones = self.vibrato_depth as f64 / 16.0;
        if self.is_fine_vibrato {
            depth_semitones /= 4.0;
        }

        let pos = (self.vibrato_phase as usize) & 63; // wrap at 64
        let modulation = match self.vibrato_waveform {
            // 0: sine — use ProTracker lookup table
            0 => VIBRATO_TABLE[pos],
            // 1: ramp down — falls from +1 to -1 over the 64-entry cycle
            1 => 1.0 - (pos as f64 / 32.0),
            // 2: square — +1 for first half, -1 for second half
            2 => {
                if pos < 32 {
                    1.0
                } else {
                    -1.0
                }
            }
            // 3+: random (seeded on table position for repeatability)
            _ => VIBRATO_TABLE[(pos * 7 + 13) & 63],
        } * depth_semitones;

        2.0_f64.powf(modulation / 12.0)
    }

    /// Advance vibrato LFO by one frame.
    ///
    /// Phase advances by `speed` per tick (ProTracker/Furnace convention),
    /// stored as fractional position in [0.0, 64.0) table space.
    pub fn advance_vibrato(&mut self, _sample_rate: u32) {
        if !self.vibrato_active || self.vibrato_speed == 0 {
            return;
        }

        // Advance by speed units per tick, distributed across frames-per-tick
        let ticks_per_row = self.ticks_per_row.max(1) as f64;
        let frames_per_tick = self.frames_per_row as f64 / ticks_per_row;
        let pos_inc = self.vibrato_speed as f64 / frames_per_tick;

        self.vibrato_phase = (self.vibrato_phase + pos_inc).rem_euclid(64.0);
    }

    /// Get the tremolo amplitude modulation as a gain multiplier.
    pub fn tremolo_amplitude_modulation(&self) -> f32 {
        if !self.tremolo_active || self.tremolo_depth == 0 {
            return 1.0;
        }

        let depth = self.tremolo_depth as f64 / 64.0; // depth / 64 per tick
        let pos = (self.tremolo_phase as usize) & 127; // wrap at 128

        // TREMOLO_TABLE values are in [0.0, ~1.6] range centered at 1.0.
        // Rescale to [-1.0, 1.0] for the modulation signal:
        let modulation = match self.tremolo_waveform {
            // 0: use cosine table (starts at 0, peaks at half-cycle)
            0 => {
                let t = TREMOLO_TABLE[pos];
                (t - 0.8) / 0.8 // normalize ~[0, 1.6] → [-1, 1]
            }
            // 1: ramp down
            1 => 1.0 - (pos as f64 / 64.0),
            // 2: square
            2 => {
                if pos < 64 {
                    1.0
                } else {
                    -1.0
                }
            }
            // 3+: random
            _ => {
                let t = TREMOLO_TABLE[(pos * 7 + 13) & 127];
                (t - 0.8) / 0.8
            }
        };

        (1.0 + modulation * depth).clamp(0.0, 2.0) as f32
    }

    /// Advance tremolo LFO by one frame.
    ///
    /// Phase advances by `speed` per tick in [0.0, 128.0) table space.
    pub fn advance_tremolo(&mut self, _sample_rate: u32) {
        if !self.tremolo_active || self.tremolo_speed == 0 {
            return;
        }

        let ticks_per_row = self.ticks_per_row.max(1) as f64;
        let frames_per_tick = self.frames_per_row as f64 / ticks_per_row;
        let pos_inc = self.tremolo_speed as f64 / frames_per_tick;

        self.tremolo_phase = (self.tremolo_phase + pos_inc).rem_euclid(128.0);
    }

    /// Get the combined playback rate multiplier from all pitch effects.
    pub fn combined_pitch_ratio(&self) -> f64 {
        let arpeggio = 2.0_f64.powf(self.arpeggio_semitone_offset() / 12.0);
        let vibrato = self.vibrato_pitch_ratio();

        let mut ratio = self.pitch_ratio;
        if self.glissando {
            let semitones = (ratio.log2() * 12.0).round();
            ratio = 2.0_f64.powf(semitones / 12.0);
        }

        ratio * arpeggio * vibrato
    }

    /// Get the effective volume from effects.
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
                if current_tick % total_ticks >= self.tremor_on as u32 {
                    base_vol = 0.0;
                }
            }
        }

        Some((base_vol * tremolo * self.channel_volume).clamp(0.0, 2.0))
    }

    /// Advance portamento frequency by one tick.
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
    pub fn advance_pitch_slide_tick(&mut self) {
        if self.pitch_slide_up > 0.0 || self.pitch_slide_down > 0.0 {
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
            let current_pan = self
                .panning_override
                .or(self.instrument_pan_override)
                .unwrap_or(0.5);
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

        let cycles_per_row = (self.panbrello_speed as f64 / 64.0) * self.ticks_per_row as f64;
        let phase_inc_per_frame =
            (cycles_per_row * 2.0 * std::f64::consts::PI) / self.frames_per_row as f64;

        self.panbrello_phase += phase_inc_per_frame;

        if self.panbrello_phase > 2.0 * std::f64::consts::PI {
            self.panbrello_phase -= 2.0 * std::f64::consts::PI;
        }
    }
}
