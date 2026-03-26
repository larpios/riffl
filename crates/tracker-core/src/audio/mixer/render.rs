use crate::audio::sample::LoopMode;
use crate::audio::voice::{AdsrPhase, Voice};

impl super::Mixer {
    /// Render audio into a stereo interleaved f32 buffer.
    ///
    /// Mixes all active voices into the output buffer. Each frame consists
    /// of two samples (left, right). Mono samples are duplicated to both channels.
    ///
    /// # Arguments
    /// * `output` - Mutable slice of f32 samples to fill (stereo interleaved: L, R, L, R, ...)
    pub fn render(&mut self, output: &mut [f32]) {
        let num_frames = output.len() / 2;

        let output_sample_rate = self.output_sample_rate;

        // Wrap main voice rendering in a block so borrows are released before preview.
        {
            let channel_strips = &mut self.channel_strips;
            let bus_system = &mut self.bus_system;
            let effect_processor = &mut self.effect_processor;
            let voices = &mut self.voices;
            let samples = &self.samples;

            // Clear the buffer first
            for sample in output.iter_mut() {
                *sample = 0.0;
            }

            bus_system.clear_all(num_frames);
            let num_buses = bus_system.num_buses();

            for (ch, voice_slot) in voices.iter_mut().enumerate() {
                let strip = &mut channel_strips[ch];

                for frame in 0..num_frames {
                    // Check for pending note trigger for this channel at this frame
                    let mut triggered_now = false;
                    let current_row_frame = effect_processor
                        .channel_state(ch)
                        .unwrap()
                        .row_frame_counter;

                    if let Some(pos) = self
                        .pending_notes
                        .iter()
                        .position(|pn| pn.channel == ch && pn.trigger_frame == current_row_frame)
                    {
                        let pn = self.pending_notes.remove(pos);
                        effect_processor.set_period_clock(ch, pn.period_clock);
                        let mut voice = Voice::new(
                            self.instruments.get(pn.instrument_index),
                            pn.instrument_index,
                            pn.sample_index,
                            pn.playback_rate,
                            pn.velocity_gain,
                            pn.hz_to_rate,
                            pn.triggered_note_freq,
                        );
                        if let Some(offset) = pn.offset {
                            voice = voice.with_position(offset as f64);
                        }
                        if let Some(env_override) = effect_processor
                            .channel_state(ch)
                            .unwrap()
                            .envelope_position_override
                        {
                            voice.volume_envelope_tick = env_override;
                        }
                        *voice_slot = Some(voice);
                        triggered_now = true;
                    }

                    let voice = match voice_slot {
                        Some(v) if v.active => v,
                        _ => {
                            effect_processor.advance_frame(ch);
                            continue;
                        }
                    };

                    let render_state = effect_processor.voice_render_state(ch);
                    let ch_state = effect_processor.channel_state(ch).unwrap();

                    // Apply any envelope position overrides (Lxx) from the effect processor at row start
                    if ch_state.row_frame_counter == 0 {
                        if let Some(pos) = ch_state.envelope_position_override {
                            voice.volume_envelope_tick = pos;
                        }
                    }

                    // Sub-row timing logic
                    let frames_per_tick = ch_state.frames_per_row / ch_state.ticks_per_row as u32;
                    let current_tick = ch_state.row_frame_counter / frames_per_tick;
                    let tick_frame = ch_state.row_frame_counter % frames_per_tick;

                    // Note Cut (ECx)
                    if let Some(cut_tick) = ch_state.note_cut_tick {
                        if current_tick >= cut_tick as u32 {
                            voice.active = false;
                        }
                    }

                    // Retrigger (E9x)
                    if !triggered_now {
                        // Don't retrigger a note that just started
                        if let Some(retrigger_interval) = ch_state.retrigger_interval {
                            if retrigger_interval > 0
                                && current_tick > 0
                                && tick_frame == 0
                                && current_tick.is_multiple_of(retrigger_interval as u32)
                            {
                                voice.position = 0.0;
                                // Apply retrigger volume action
                                match ch_state.retrigger_volume_action {
                                    0 | 8 => {} // No change
                                    1 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 1.0 / 64.0).max(0.0)
                                    }
                                    2 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 2.0 / 64.0).max(0.0)
                                    }
                                    3 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 4.0 / 64.0).max(0.0)
                                    }
                                    4 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 8.0 / 64.0).max(0.0)
                                    }
                                    5 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain - 16.0 / 64.0).max(0.0)
                                    }
                                    6 => voice.velocity_gain *= 2.0 / 3.0,
                                    7 => voice.velocity_gain *= 0.5,
                                    9 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 1.0 / 64.0).min(1.0)
                                    }
                                    10 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 2.0 / 64.0).min(1.0)
                                    }
                                    11 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 4.0 / 64.0).min(1.0)
                                    }
                                    12 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 8.0 / 64.0).min(1.0)
                                    }
                                    13 => {
                                        voice.velocity_gain =
                                            (voice.velocity_gain + 16.0 / 64.0).min(1.0)
                                    }
                                    14 => voice.velocity_gain *= 3.0 / 2.0,
                                    15 => voice.velocity_gain *= 2.0,
                                    _ => {}
                                }
                            }
                        }
                    }

                    if !voice.active {
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    // Envelope and Modulation processing
                    let mut env_vol = 1.0;
                    let mut env_pan = 0.0; // Panning offset (-1.0 to 1.0)
                    let mut env_pitch = 0.0; // Pitch offset in semitones

                    if let Some(inst) = self.instruments.get(voice.instrument_index) {
                        // Update LFO phases
                        voice.lfo.update(inst, output_sample_rate, self.bpm);

                        // Fadeout processing
                        if !voice.key_on
                            && inst.fadeout > 0
                            && tick_frame == frames_per_tick.saturating_sub(1)
                        {
                            let delta = inst.fadeout as f32 / 65536.0;
                            voice.fadeout_multiplier = (voice.fadeout_multiplier - delta).max(0.0);
                            if voice.fadeout_multiplier <= 0.0001 {
                                voice.active = false;
                            }
                        }

                        // Volume Modulations
                        if let Some(adsr) = &inst.volume_adsr {
                            env_vol *=
                                voice
                                    .volume_adsr
                                    .update(adsr, voice.key_on, output_sample_rate);
                            if voice.volume_adsr.phase == AdsrPhase::Done {
                                voice.active = false;
                            }
                        }
                        if let Some(vol_env) = &inst.volume_envelope {
                            if vol_env.enabled {
                                let (val, next_tick) =
                                    vol_env.evaluate(voice.volume_envelope_tick, voice.key_on);
                                env_vol *= val;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.volume_envelope_tick = next_tick;
                                }
                                if !voice.key_on
                                    && vol_env.points.last().is_some_and(|p| {
                                        voice.volume_envelope_tick >= p.frame as usize
                                    })
                                    && val <= 0.001
                                {
                                    voice.active = false;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.volume_lfo {
                            env_vol *= (1.0 + voice.lfo.get_vol_value(lfo)).max(0.0);
                        }

                        // Panning Modulations
                        if let Some(adsr) = &inst.panning_adsr {
                            env_pan +=
                                voice
                                    .panning_adsr
                                    .update(adsr, voice.key_on, output_sample_rate)
                                    * 2.0
                                    - 1.0;
                        }
                        if let Some(pan_env) = &inst.panning_envelope {
                            if pan_env.enabled {
                                let (val, next_tick) =
                                    pan_env.evaluate(voice.panning_envelope_tick, voice.key_on);
                                env_pan += val;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.panning_envelope_tick = next_tick;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.panning_lfo {
                            env_pan += voice.lfo.get_pan_value(lfo);
                        }

                        // Pitch Modulations
                        if let Some(adsr) = &inst.pitch_adsr {
                            env_pitch +=
                                (voice
                                    .pitch_adsr
                                    .update(adsr, voice.key_on, output_sample_rate)
                                    * 2.0
                                    - 1.0)
                                    * 12.0;
                        }
                        if let Some(pitch_env) = &inst.pitch_envelope {
                            if pitch_env.enabled {
                                let (val, next_tick) =
                                    pitch_env.evaluate(voice.pitch_envelope_tick, voice.key_on);
                                env_pitch += val * 12.0;
                                if tick_frame == frames_per_tick.saturating_sub(1) {
                                    voice.pitch_envelope_tick = next_tick;
                                }
                            }
                        }
                        if let Some(lfo) = &inst.pitch_lfo {
                            env_pitch += voice.lfo.get_pitch_value(lfo) * 12.0;
                        }
                    }

                    if !voice.active {
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    let sample = match samples.get(voice.sample_index) {
                        Some(s) => s,
                        None => {
                            voice.active = false;
                            effect_processor.advance_frame(ch);
                            continue;
                        }
                    };

                    let sample_data = sample.data();
                    let sample_channels = sample.channels() as usize;
                    let sample_frames = sample.frame_count();

                    if sample_frames == 0 {
                        voice.active = false;
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    let src_frame = voice.position as usize;

                    use crate::audio::sample::LoopMode;

                    // Determine effective loop mode, start, and end.
                    let (eff_loop_mode, eff_loop_start, eff_loop_end) = if voice.key_on
                        && sample.sustain_loop_mode != LoopMode::NoLoop
                        && sample.sustain_loop_end > sample.sustain_loop_start
                    {
                        (
                            sample.sustain_loop_mode,
                            sample.sustain_loop_start,
                            sample.sustain_loop_end,
                        )
                    } else {
                        (sample.loop_mode, sample.loop_start, sample.loop_end)
                    };

                    match eff_loop_mode {
                        LoopMode::NoLoop => {
                            if src_frame >= sample_frames {
                                voice.active = false;
                                effect_processor.advance_frame(ch);
                                continue;
                            }
                        }
                        LoopMode::Forward => {
                            if voice.position >= (eff_loop_end + 1) as f64 {
                                let loop_len = (eff_loop_end - eff_loop_start + 1) as f64;
                                let offset =
                                    (voice.position - eff_loop_start as f64).rem_euclid(loop_len);
                                voice.position = eff_loop_start as f64 + offset;
                            }
                        }
                        LoopMode::PingPong => {
                            if voice.loop_direction > 0.0
                                && voice.position >= (eff_loop_end + 1) as f64
                            {
                                voice.loop_direction = -1.0;
                                // Invert fractional component when reversing direction
                                let overshoot = voice.position - (eff_loop_end + 1) as f64;
                                voice.position = eff_loop_end as f64 - overshoot;
                            } else if voice.loop_direction < 0.0
                                && voice.position < eff_loop_start as f64
                            {
                                voice.loop_direction = 1.0;
                                let overshoot = eff_loop_start as f64 - voice.position;
                                voice.position = eff_loop_start as f64 + overshoot;
                            }
                        }
                    }

                    // Final safety check for buffer access
                    let src_frame = voice.position as usize;
                    if src_frame >= sample_frames {
                        voice.active = false;
                        effect_processor.advance_frame(ch);
                        continue;
                    }

                    // Compute effective playback rate using the combined pitch ratio from effects and modulation.
                    let pitch_mod_ratio = 2.0f64.powf(env_pitch as f64 / 12.0);
                    let effective_rate = voice.triggered_note_freq
                        * voice.hz_to_rate
                        * render_state.pitch_ratio
                        * pitch_mod_ratio
                        * voice.loop_direction;

                    // Read sample data with linear interpolation
                    let (left, right) = {
                        let pos_floor = voice.position.floor() as usize;
                        let frac = (voice.position - pos_floor as f64) as f32;
                        let next_frame = {
                            let next = pos_floor + 1;
                            match eff_loop_mode {
                                LoopMode::NoLoop => next,
                                LoopMode::Forward => {
                                    if next > eff_loop_end {
                                        eff_loop_start
                                    } else {
                                        next
                                    }
                                }
                                LoopMode::PingPong => {
                                    if voice.loop_direction > 0.0 {
                                        if next > eff_loop_end {
                                            eff_loop_end
                                        } else {
                                            next
                                        }
                                    } else if pos_floor > eff_loop_start {
                                        pos_floor - 1
                                    } else {
                                        eff_loop_start
                                    }
                                }
                            }
                        };

                        let get_stereo = |f: usize| {
                            if f >= sample_frames {
                                (0.0, 0.0)
                            } else if sample_channels >= 2 {
                                let idx = f * sample_channels;
                                (sample_data[idx], sample_data[idx + 1])
                            } else {
                                (sample_data[f], sample_data[f])
                            }
                        };

                        let (l1, r1) = get_stereo(pos_floor);
                        let (l2, r2) = get_stereo(next_frame);

                        (l1 + (l2 - l1) * frac, r1 + (r2 - r1) * frac)
                    };

                    let combined_channel_gain =
                        render_state.gain.unwrap_or(render_state.channel_volume);

                    let pan_sep_mult = self.pan_separation as f32 / 128.0;

                    // Combine base pan (track or override)
                    let base_pan = render_state
                        .pan_override
                        .map(|p| p * 2.0 - 1.0)
                        .unwrap_or(strip.current_pan());

                    // IF we have a panning envelope enabled, it OVERRIDES the base panning (IT behavior)
                    let pan_env_active = self
                        .instruments
                        .get(voice.instrument_index)
                        .and_then(|i| i.panning_envelope.as_ref())
                        .map(|e| e.enabled)
                        .unwrap_or(false);

                    // When a panning envelope is active, it provides the absolute
                    // pan position (IT/XM behaviour).  When inactive, env_pan holds
                    // any ADSR / LFO offsets that should still be added to base_pan.
                    let final_pan = if pan_env_active {
                        env_pan
                    } else {
                        (base_pan + env_pan).clamp(-1.0, 1.0)
                    };

                    let total_pan = (final_pan * pan_sep_mult).clamp(-1.0, 1.0);

                    let (left_gain, right_gain) = strip.next_gains_modulated(
                        env_vol * combined_channel_gain,
                        0.0,
                        Some(total_pan),
                        self.panning_law,
                    );

                    let out_idx = frame * 2;
                    let global_vol_mult = effect_processor.global_volume;
                    let post_l = left
                        * voice.velocity_gain
                        * left_gain
                        * global_vol_mult
                        * voice.fadeout_multiplier;
                    let post_r = right
                        * voice.velocity_gain
                        * right_gain
                        * global_vol_mult
                        * voice.fadeout_multiplier;

                    output[out_idx] += post_l;
                    output[out_idx + 1] += post_r;

                    self.visualizer
                        .update_channel_levels(ch, post_l.abs(), post_r.abs());

                    // Write mono mix to oscilloscope ring buffer
                    self.visualizer
                        .record_oscilloscope_sample(ch, (post_l + post_r) * 0.5);

                    for bus_idx in 0..num_buses {
                        let send_level = strip.next_send_level(bus_idx);
                        if send_level > 0.0001 {
                            bus_system.accumulate(bus_idx, frame, post_l, post_r, send_level);
                        }
                    }

                    voice.position += effective_rate;

                    // Advance frame-level effect modulations
                    effect_processor.advance_frame(ch);
                }
            }

            effect_processor.advance_global_frame();
            bus_system.process_and_mix(output, num_frames);
        } // end main voice block — field borrows released

        // Preview voice: renders a one-shot sample directly into output,
        // bypassing channel strips (no mute/solo/pan, preview volume = 0.7).
        let preview_done = if let Some(ref pv) = self.preview_sample {
            let pv_rate = self.preview_rate;
            let pv_frames = pv.frame_count();
            let pv_channels = pv.channels() as usize;
            let pv_data = pv.data();
            let pv_loop_mode = pv.loop_mode;
            let pv_loop_start = pv.loop_start;
            let pv_loop_end = pv.loop_end.min(pv_frames.saturating_sub(1));
            let looping = pv_loop_mode != LoopMode::NoLoop && pv_loop_end > pv_loop_start;
            let mut done = false;
            for frame in 0..num_frames {
                let pos = self.preview_pos as usize;
                if pos >= pv_frames {
                    done = true;
                    break;
                }
                let l = pv_data[pos * pv_channels];
                let r = if pv_channels > 1 {
                    pv_data[pos * pv_channels + 1]
                } else {
                    l
                };
                output[frame * 2] += l * 0.7;
                output[frame * 2 + 1] += r * 0.7;
                self.preview_pos += pv_rate;
                if looping && self.preview_pos >= (pv_loop_end + 1) as f64 {
                    let loop_len = (pv_loop_end - pv_loop_start + 1) as f64;
                    let offset = (self.preview_pos - pv_loop_start as f64).rem_euclid(loop_len);
                    self.preview_pos = pv_loop_start as f64 + offset;
                }
            }
            done
        } else {
            false
        };
        if preview_done {
            self.preview_sample = None;
        }

        // Write mono mix to FFT capture buffer
        for frame in 0..num_frames {
            let mono = (output[frame * 2] + output[frame * 2 + 1]) * 0.5;
            self.visualizer.record_fft_sample_mut(mono);
        }

        // Clamp output to [-1.0, 1.0] to prevent clipping distortion
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}
