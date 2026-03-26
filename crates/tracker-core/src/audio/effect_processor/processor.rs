//! Effect processor implementation.

use super::state::ChannelEffectState;
use super::types::{TransportCommand, VoiceRenderState};
use crate::audio::pitch::{PitchCalculator, SlideMode};
use crate::pattern::effect::{Effect, EffectMode, EffectType};

/// Effect processor that manages per-channel effect state.
pub struct TrackerEffectProcessor {
    /// Per-channel effect state.
    channels: Vec<ChannelEffectState>,
    /// Output sample rate (for timing calculations).
    sample_rate: u32,
    /// Project-level effect interpretation mode.
    pub mode: EffectMode,
    /// Scale factor for global volume effects (64.0 for S3M/XM, 128.0 for IT).
    pub global_volume_range: f32,
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
            global_volume_range: 128.0, // Default to IT/standard range
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
    pub fn set_slide_mode(&mut self, mode: SlideMode) {
        for ch in &mut self.channels {
            ch.slide_mode = mode;
        }
    }

    /// Reset global slide state at the start of a row.
    pub fn reset_row_slides(&mut self) {
        self.global_volume_slide_up = 0;
        self.global_volume_slide_down = 0;
    }

    /// Set high-resolution period math (14.3MHz clock) for all channels.
    pub fn set_use_high_res_periods(&mut self, use_high_res: bool) {
        for ch in &mut self.channels {
            ch.use_high_res_periods = use_high_res;
        }
    }

    /// Process effects for a row, returning any transport commands.
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
                        | Some(EffectType::PortamentoFine)
                        | Some(EffectType::PortamentoExtraFine)
                )
            });

            if !has_tone_porta {
                state.pitch_ratio = 1.0;
                state.triggered_note_freq = freq;
                state.portamento_target = None;
                state.portamento_freq = None;
            } else {
                state.portamento_target = Some(freq);
            }
        }

        for effect in effects {
            let effect_type = match effect.effect_type() {
                Some(t) => t,
                None => continue,
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
                    if effect.param > 0 {
                        let speed = match state.slide_mode {
                            SlideMode::Linear => effect.param as f64 * 4.0,
                            SlideMode::AmigaPeriod => {
                                let mut s = effect.param as f64;
                                if state.use_high_res_periods {
                                    s *= 4.0;
                                }
                                s
                            }
                        };
                        state.pitch_slide_up = speed;
                        state.prev_pitch_slide_up = speed;
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.pitch_slide_up = state.prev_pitch_slide_up;
                    } else {
                        state.pitch_slide_up = 0.0;
                    }
                }

                EffectType::PitchSlideDown => {
                    if effect.param > 0 {
                        let speed = match state.slide_mode {
                            SlideMode::Linear => effect.param as f64 * 4.0,
                            SlideMode::AmigaPeriod => {
                                let mut s = effect.param as f64;
                                if state.use_high_res_periods {
                                    s *= 4.0;
                                }
                                s
                            }
                        };
                        state.pitch_slide_down = speed;
                        state.prev_pitch_slide_down = speed;
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.pitch_slide_down = state.prev_pitch_slide_down;
                    } else {
                        state.pitch_slide_down = 0.0;
                    }
                }

                EffectType::PortamentoToNote => {
                    if effect.param > 0 {
                        let speed = match state.slide_mode {
                            SlideMode::Linear => effect.param as f64 / 16.0,
                            SlideMode::AmigaPeriod => {
                                let mut s = effect.param as f64;
                                if state.use_high_res_periods {
                                    s *= 4.0;
                                }
                                s
                            }
                        };
                        state.portamento_speed = speed;
                        state.prev_portamento_speed = speed;
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.portamento_speed = state.prev_portamento_speed;
                    } else {
                        state.portamento_speed = 0.0;
                    }

                    if let Some(freq) = note_frequency {
                        state.portamento_target = Some(freq);
                        if state.portamento_freq.is_none() && state.triggered_note_freq > 0.0 {
                            let current_freq = state.pitch_ratio * state.triggered_note_freq;
                            state.portamento_freq = Some(current_freq);
                        }
                    }
                }

                EffectType::Vibrato => {
                    state.vibrato_active = true;
                    state.is_fine_vibrato = false;
                    if effect.param_x() > 0 {
                        state.vibrato_speed = effect.param_x();
                        state.prev_vibrato_speed = effect.param_x();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.vibrato_speed = state.prev_vibrato_speed;
                    }
                    if effect.param_y() > 0 {
                        state.vibrato_depth = effect.param_y();
                        state.prev_vibrato_depth = effect.param_y();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.vibrato_depth = state.prev_vibrato_depth;
                    }
                }

                EffectType::FineVibrato => {
                    state.vibrato_active = true;
                    state.is_fine_vibrato = true;
                    if effect.param_x() > 0 {
                        state.vibrato_speed = effect.param_x();
                        state.prev_vibrato_speed = effect.param_x();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.vibrato_speed = state.prev_vibrato_speed;
                    }
                    if effect.param_y() > 0 {
                        state.vibrato_depth = effect.param_y();
                        state.prev_vibrato_depth = effect.param_y();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.vibrato_depth = state.prev_vibrato_depth;
                    }
                }

                EffectType::TonePortamentoVolumeSlide => {
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                        state.prev_volume_slide_up = effect.param_x();
                        state.prev_volume_slide_down = effect.param_y();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.volume_slide_up = state.prev_volume_slide_up;
                        state.volume_slide_down = state.prev_volume_slide_down;
                    }
                    state.portamento_speed = state.prev_portamento_speed;
                    if let Some(freq) = note_frequency {
                        state.portamento_target = Some(freq);
                        if state.portamento_freq.is_none() && state.triggered_note_freq > 0.0 {
                            let current_freq = state.pitch_ratio * state.triggered_note_freq;
                            state.portamento_freq = Some(current_freq);
                        }
                    }
                }

                EffectType::VibratoVolumeSlide => {
                    state.vibrato_active = true;
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                        state.prev_volume_slide_up = effect.param_x();
                        state.prev_volume_slide_down = effect.param_y();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.volume_slide_up = state.prev_volume_slide_up;
                        state.volume_slide_down = state.prev_volume_slide_down;
                    }
                    state.vibrato_speed = state.prev_vibrato_speed;
                    state.vibrato_depth = state.prev_vibrato_depth;
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
                    state.sample_offset = Some(effect.param as usize * 256);
                }

                EffectType::VolumeSlide => {
                    if effect.param > 0 {
                        state.volume_slide_up = effect.param_x();
                        state.volume_slide_down = effect.param_y();
                        state.prev_volume_slide_up = effect.param_x();
                        state.prev_volume_slide_down = effect.param_y();
                    } else if self.mode == EffectMode::Compatible || self.mode == EffectMode::Amiga
                    {
                        state.volume_slide_up = state.prev_volume_slide_up;
                        state.volume_slide_down = state.prev_volume_slide_down;
                    }
                }

                EffectType::PositionJump => {
                    commands.push(TransportCommand::PositionJump(effect.param as usize));
                }

                EffectType::SetVolume => {
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
                        0x1 => match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = sub_param as f64 / 16.0;
                                state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let delta = sub_param as f64;
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    delta,
                                    0.0,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                if state.triggered_note_freq > 0.0 {
                                    state.pitch_ratio = new_freq / state.triggered_note_freq;
                                }
                            }
                        },
                        0x2 => match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = sub_param as f64 / 16.0;
                                state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let delta = sub_param as f64;
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    0.0,
                                    delta,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                if state.triggered_note_freq > 0.0 {
                                    state.pitch_ratio = new_freq / state.triggered_note_freq;
                                }
                            }
                        },
                        0x3 => {
                            state.glissando = sub_param != 0;
                        }
                        0x4 => {
                            state.vibrato_waveform = sub_param;
                        }
                        0x5 => {
                            let ft = if sub_param <= 0x7 {
                                sub_param as i8
                            } else {
                                (sub_param as i8) - 16
                            };
                            state.finetune_override = Some(ft);
                        }
                        0x6 => {
                            commands.push(TransportCommand::PatternLoop(sub_param));
                        }
                        0x7 => {
                            state.tremolo_waveform = sub_param;
                        }
                        0x9 => {
                            state.retrigger_interval = Some(sub_param);
                        }
                        0xA => {
                            let current_vol = state.volume_override.unwrap_or(1.0);
                            let delta = sub_param as f32 / 64.0;
                            state.volume_override = Some((current_vol + delta).clamp(0.0, 1.0));
                        }
                        0xB => {
                            let current_vol = state.volume_override.unwrap_or(1.0);
                            let delta = sub_param as f32 / 64.0;
                            state.volume_override = Some((current_vol - delta).clamp(0.0, 1.0));
                        }
                        0xC => {
                            state.note_cut_tick = Some(sub_param);
                        }
                        0xD => {
                            state.note_delay_tick = Some(sub_param);
                        }
                        0xE => {
                            commands.push(TransportCommand::PatternDelay(sub_param));
                        }
                        _ => {}
                    }
                }

                EffectType::SetPanning => {
                    state.panning_override = Some(effect.param as f32 / 255.0);
                }

                EffectType::SetSpeed => {
                    if effect.param >= 32 {
                        commands.push(TransportCommand::SetBpm(effect.param as f64));
                    } else if effect.param > 0 {
                        commands.push(TransportCommand::SetTpl(effect.param as u32));
                    }
                }

                EffectType::SetGlobalVolume => {
                    self.global_volume =
                        (effect.param as f32 / self.global_volume_range).clamp(0.0, 1.0);
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
                        state.channel_volume = (state.channel_volume
                            + (effect.param_y() as f32 / 64.0))
                            .clamp(0.0, 1.0);
                    } else if effect.param_x() == 0x0B {
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
                    if state.triggered_note_freq > 0.0 {
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = effect.param as f64 / 64.0;
                                state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let delta = effect.param as f64;
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    delta,
                                    0.0,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::ExtraFinePortaDown => {
                    if state.triggered_note_freq > 0.0 {
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = effect.param as f64 / 64.0;
                                state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let delta = effect.param as f64;
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    0.0,
                                    delta,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::SlideUpFine => {
                    if state.triggered_note_freq > 0.0 {
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = effect.param as f64 / 16.0;
                                state.pitch_ratio *= 2.0_f64.powf(semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let mut delta = effect.param as f64;
                                if state.use_high_res_periods {
                                    delta *= 4.0;
                                }
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    delta,
                                    0.0,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::SlideDownFine => {
                    if state.triggered_note_freq > 0.0 {
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let semitones = effect.param as f64 / 16.0;
                                state.pitch_ratio *= 2.0_f64.powf(-semitones / 12.0);
                            }
                            SlideMode::AmigaPeriod => {
                                let freq = state.pitch_ratio * state.triggered_note_freq;
                                let mut delta = effect.param as f64;
                                if state.use_high_res_periods {
                                    delta *= 4.0;
                                }
                                let new_freq = PitchCalculator::apply_slide(
                                    freq,
                                    0.0,
                                    delta,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::PortamentoExtraFine => {
                    if state.triggered_note_freq > 0.0 {
                        let current_freq = state.pitch_ratio * state.triggered_note_freq;
                        let target_freq = state.portamento_target.unwrap_or(current_freq);
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let speed = effect.param as f64 / 64.0;
                                let new_freq = PitchCalculator::apply_portamento(
                                    current_freq,
                                    target_freq,
                                    speed,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                            SlideMode::AmigaPeriod => {
                                let delta = effect.param as f64;
                                let new_freq = PitchCalculator::apply_portamento(
                                    current_freq,
                                    target_freq,
                                    delta,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
                EffectType::PortamentoFine => {
                    if state.triggered_note_freq > 0.0 {
                        let current_freq = state.pitch_ratio * state.triggered_note_freq;
                        let target_freq = state.portamento_target.unwrap_or(current_freq);
                        match state.slide_mode {
                            SlideMode::Linear => {
                                let speed = effect.param as f64 / 16.0;
                                let new_freq = PitchCalculator::apply_portamento(
                                    current_freq,
                                    target_freq,
                                    speed,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                            SlideMode::AmigaPeriod => {
                                let delta = effect.param as f64;
                                let new_freq = PitchCalculator::apply_portamento(
                                    current_freq,
                                    target_freq,
                                    delta,
                                    state.slide_mode,
                                    state.period_clock,
                                );
                                state.pitch_ratio = new_freq / state.triggered_note_freq;
                            }
                        }
                    }
                }
            }
        }

        commands
    }

    pub fn advance_frame(&mut self, channel: usize) {
        if let Some(state) = self.channels.get_mut(channel) {
            state.row_frame_counter += 1;

            let ticks_per_row = state.ticks_per_row.max(1) as u32;
            let frames_per_tick = state.frames_per_row / ticks_per_row;

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

            state.advance_vibrato(self.sample_rate);
            state.advance_tremolo(self.sample_rate);
            state.advance_panbrello(self.sample_rate);

            if is_tick_boundary && current_tick > 0 && current_tick < ticks_per_row {
                state.advance_portamento_tick();
                state.advance_pitch_slide_tick();
                state.advance_volume_slide_tick();
                state.advance_panning_slide_tick();
            }
        }
    }

    pub fn advance_global_frame(&mut self) {
        if self.global_volume_slide_up > 0 || self.global_volume_slide_down > 0 {
            if let Some(state) = self.channels.first() {
                let ticks_per_row = state.ticks_per_row.max(1) as u32;
                let frames_per_tick = state.frames_per_row / ticks_per_row;

                if frames_per_tick > 0 && state.row_frame_counter % frames_per_tick == 0 {
                    let current_tick = state.row_frame_counter / frames_per_tick;
                    if current_tick > 0 && current_tick < ticks_per_row {
                        let delta_per_tick = (self.global_volume_slide_up as f32
                            - self.global_volume_slide_down as f32)
                            / self.global_volume_range;
                        self.global_volume = (self.global_volume + delta_per_tick).clamp(0.0, 1.0);
                    }
                }
            }
        }
    }

    pub fn pitch_ratio(&self, channel: usize) -> f64 {
        self.channels
            .get(channel)
            .map(|s| s.combined_pitch_ratio())
            .unwrap_or(1.0)
    }

    pub fn effective_frequency(&self, channel: usize) -> Option<f64> {
        self.channels
            .get(channel)
            .map(|s| s.combined_pitch_ratio() * s.triggered_note_freq)
    }

    pub fn last_note_frequency(&self, channel: usize) -> f64 {
        self.channels
            .get(channel)
            .map(|s| s.triggered_note_freq)
            .unwrap_or(440.0)
    }

    pub fn portamento_frequency(&self, channel: usize) -> Option<f64> {
        self.channels.get(channel).and_then(|s| s.portamento_freq)
    }

    pub fn sample_offset(&self, channel: usize) -> Option<usize> {
        self.channels.get(channel).and_then(|s| s.sample_offset)
    }

    pub fn finetune_override(&self, channel: usize) -> Option<i8> {
        self.channels.get(channel).and_then(|s| s.finetune_override)
    }

    pub fn channel_panning(&self, channel: usize) -> Option<f32> {
        self.channels.get(channel).and_then(|s| {
            if s.panning_override.is_none()
                && s.instrument_pan_override.is_none()
                && !s.panbrello_active
            {
                return None;
            }
            let base = s
                .panning_override
                .or(s.instrument_pan_override)
                .unwrap_or(0.5);
            let panbrello = if s.panbrello_active {
                (s.panbrello_phase.sin() * s.panbrello_depth as f64 / 64.0) as f32
            } else {
                0.0
            };
            Some((base + panbrello).clamp(0.0, 1.0))
        })
    }

    pub fn channel_effect_panning(&self, channel: usize) -> Option<f32> {
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

    pub fn volume_override(&self, channel: usize) -> Option<f32> {
        self.channels
            .get(channel)
            .and_then(|s| s.effective_volume())
    }

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

    pub fn channel_state(&self, channel: usize) -> Option<&ChannelEffectState> {
        self.channels.get(channel)
    }

    pub fn channel_state_mut(&mut self, channel: usize) -> Option<&mut ChannelEffectState> {
        self.channels.get_mut(channel)
    }

    pub fn set_period_clock(&mut self, channel: usize, clock: f64) {
        if let Some(ch) = self.channels.get_mut(channel) {
            ch.period_clock = clock;
        }
    }

    pub fn reset_all(&mut self) {
        for state in &mut self.channels {
            state.reset();
        }
    }

    pub fn update_tempo(&mut self, bpm: f64) {
        for state in &mut self.channels {
            let seconds_per_row = (2.5 / bpm) * state.ticks_per_row as f64;
            state.frames_per_row = (seconds_per_row * self.sample_rate as f64) as u32;
            if state.frames_per_row == 0 {
                state.frames_per_row = 1;
            }
        }
    }

    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::effect::{Effect, EffectType};

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
            portamento_target: Some(440.0),
            portamento_speed: 1.0,
            prev_portamento_speed: 1.0,
            portamento_freq: Some(445.0),
            triggered_note_freq: 440.0,
            ..Default::default()
        };
        state.reset();
        assert_eq!(state.frames_per_row, 12000);
        // Portamento state should be preserved for pause/resume
        assert_eq!(state.pitch_ratio, 2.0);
        assert_eq!(state.portamento_target, Some(440.0));
        assert_eq!(state.portamento_speed, 1.0);
        assert_eq!(state.prev_portamento_speed, 1.0);
        assert_eq!(state.portamento_freq, Some(445.0));
        assert_eq!(state.triggered_note_freq, 440.0);
        // Non-portamento state should still reset
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
    fn test_processor_creation() {
        let proc = TrackerEffectProcessor::new(8, 48000);
        assert_eq!(proc.num_channels(), 8);
    }

    #[test]
    fn test_process_row_set_volume() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        let effects = vec![Effect::from_type(EffectType::SetVolume, 0x20)]; // half volume
        proc.process_row(0, &effects, None);

        let vol = proc.volume_override(0).unwrap();
        assert!((vol - 0.5).abs() < 0.01, "Expected ~0.5, got {}", vol);
    }

    #[test]
    fn test_advance_frame_increments_counter() {
        let mut proc = TrackerEffectProcessor::new(4, 48000);
        proc.advance_frame(0);
        let state = proc.channel_state(0).unwrap();
        assert_eq!(state.row_frame_counter, 1);
    }
}
