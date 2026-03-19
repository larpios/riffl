//! Audio mixer/sequencer that connects patterns to the audio engine.
//!
//! The mixer reads pattern data row by row, triggers sample playback for
//! note events, and mixes all active voices into a stereo output buffer.

use crate::audio::bus::{self, BusSystem};
use crate::audio::channel_strip::ChannelStrip;
use crate::audio::dsp::ProcessSpec;
use crate::audio::effect_processor::{TrackerEffectProcessor, TransportCommand};
use crate::audio::sample::Sample;
use crate::pattern::note::NoteEvent;
use crate::pattern::pattern::Pattern;
use crate::pattern::track::Track;
use std::sync::Arc;

/// State for a single voice playing a sample.
#[derive(Debug, Clone)]
struct Voice {
    /// Index into the mixer's sample list.
    sample_index: usize,
    /// Current read position within the sample's audio data (in frames).
    position: f64,
    /// Playback rate relative to the sample's base rate (for pitch shifting).
    playback_rate: f64,
    /// Volume multiplier derived from note velocity (0.0 - 1.0).
    velocity_gain: f32,
    /// Whether this voice is actively producing audio.
    active: bool,
    /// Current playback direction (1.0 for forward, -1.0 for reverse).
    /// Used for ping-pong loops.
    loop_direction: f64,
}

impl Voice {
    fn new(sample_index: usize, playback_rate: f64, velocity_gain: f32) -> Self {
        Self {
            sample_index,
            position: 0.0,
            playback_rate,
            velocity_gain,
            active: true,
            loop_direction: 1.0,
        }
    }

    fn with_position(mut self, position: f64) -> Self {
        self.position = position;
        self
    }
}

/// Audio mixer that reads pattern data and produces mixed audio output.
///
/// The mixer holds references to loaded samples and maintains per-channel
/// voice state. When `tick()` is called with a row index and pattern, it
/// processes note events and updates voice states. The `render()` method
/// fills an audio buffer by mixing all active voices.
///
/// Multi-track support: the mixer stores per-channel mixing state (volume,
/// pan, mute/solo) synced from track metadata. Equal-power panning is used
/// with -3dB center.
use crate::song::Instrument;

pub struct Mixer {
    /// Loaded audio samples available for playback.
    samples: Vec<Arc<Sample>>,
    /// Instrument definitions (mapping name to sample index).
    instruments: Vec<Instrument>,
    /// Per-channel voice state (one voice per channel).
    voices: Vec<Option<Voice>>,
    /// Output sample rate in Hz (used for pitch calculation).
    output_sample_rate: u32,
    /// Per-channel mixing state (volume, pan, mute, solo).
    channel_strips: Vec<ChannelStrip>,
    /// Per-channel effect processing state.
    effect_processor: TrackerEffectProcessor,
    /// Send/return bus system for effects routing.
    bus_system: BusSystem,
    /// One-shot preview sample (e.g. audition from browser). Independent of pattern voices.
    preview_sample: Option<Arc<Sample>>,
    /// Current read position (in frames) within the preview sample.
    preview_pos: f64,
}

impl Mixer {
    /// Create a new mixer with the given samples, instruments, and channel count.
    ///
    /// # Arguments
    /// * `samples` - The loaded audio samples indexed by instrument number
    /// * `instruments` - Instrument definitions
    /// * `num_channels` - Number of pattern channels (one voice per channel)
    /// * `output_sample_rate` - The output sample rate in Hz
    pub fn new(
        samples: Vec<Arc<Sample>>,
        instruments: Vec<Instrument>,
        num_channels: usize,
        output_sample_rate: u32,
    ) -> Self {
        let mut bus_system = BusSystem::new(bus::DEFAULT_NUM_BUSES);
        bus_system.prepare(ProcessSpec {
            sample_rate: output_sample_rate as f32,
            max_block_frames: 2048,
            channels: 2,
        });

        let channel_strips: Vec<ChannelStrip> = (0..num_channels)
            .map(|_| {
                let mut strip = ChannelStrip::new();
                strip.set_sample_rate(output_sample_rate as f32);
                strip.ensure_send_levels(bus_system.num_buses());
                strip
            })
            .collect();

        Self {
            samples,
            instruments,
            voices: vec![None; num_channels],
            output_sample_rate,
            channel_strips,
            effect_processor: TrackerEffectProcessor::new(num_channels, output_sample_rate),
            bus_system,
            preview_sample: None,
            preview_pos: 0.0,
        }
    }

    /// Update per-channel mixing state from track metadata.
    ///
    /// This syncs the mixer's internal mixing state with the track
    /// volume, pan, mute, and solo settings from the pattern.
    pub fn update_tracks(&mut self, tracks: &[Track]) {
        let any_soloed = tracks.iter().any(|t| t.solo);

        for (ch, strip) in self.channel_strips.iter_mut().enumerate() {
            if let Some(track) = tracks.get(ch) {
                strip.ensure_send_levels(self.bus_system.num_buses());
                strip.update_from_track(
                    track.volume,
                    track.pan,
                    track.muted,
                    any_soloed,
                    track.solo,
                    &track.send_levels,
                );
            } else {
                strip.ensure_send_levels(self.bus_system.num_buses());
                strip.update_from_track(1.0, 0.0, false, false, false, &[]);
            }
        }
    }

    /// Process a pattern row, triggering or stopping samples based on note events.
    ///
    /// For each channel in the row:
    /// - `NoteEvent::On(note)`: Start playing the sample at the instrument index,
    ///   pitched to match the note's frequency, with velocity-based volume.
    /// - `NoteEvent::Off`: Stop the voice on that channel.
    /// - No event: The existing voice continues playing.
    ///
    /// Returns any transport commands generated by effects (BPM changes,
    /// position jumps, pattern breaks).
    pub fn tick(&mut self, row_index: usize, pattern: &Pattern) -> Vec<TransportCommand> {
        // Sync track mixing state (volume, pan, mute/solo)
        self.update_tracks(pattern.tracks());

        let row = match pattern.get_row(row_index) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let mut transport_commands = Vec::new();

        for (ch, cell) in row.iter().enumerate() {
            if ch >= self.voices.len() {
                break;
            }

            // Skip muted/non-soloed channels: don't trigger new notes
            let audible = !self
                .channel_strips
                .get(ch)
                .is_some_and(ChannelStrip::is_silent);

            // Determine the note frequency for effect processing
            let note_frequency = match &cell.note {
                Some(NoteEvent::On(note)) => Some(note.frequency()),
                _ => None,
            };

            // Process effects for this channel
            let cmds = self
                .effect_processor
                .process_row(ch, &cell.effects, note_frequency);
            transport_commands.extend(cmds);

            match &cell.note {
                Some(NoteEvent::On(note)) => {
                    if !audible {
                        // Muted channel: stop any playing voice, don't start new one
                        self.voices[ch] = None;
                        continue;
                    }
                    let instrument_idx = cell.instrument.unwrap_or(note.instrument) as usize;
                    if instrument_idx < self.samples.len() {
                        let sample = &self.samples[instrument_idx];
                        // Calculate playback rate to pitch the sample to the desired note.
                        // The sample's base_note (default C-4) plays at original speed.
                        // Higher notes play faster, lower notes play slower.
                        let base_freq = sample.base_frequency();
                        let target_freq = note.frequency();
                        let sample_rate_ratio =
                            sample.sample_rate() as f64 / self.output_sample_rate as f64;
                        let mut playback_rate = (target_freq / base_freq) * sample_rate_ratio;

                        // Apply sample-level finetune (in cents)
                        if sample.finetune != 0 {
                            playback_rate *= 2.0_f64.powf(sample.finetune as f64 / 1200.0);
                        }

                        // Apply finetune from instrument or effect override
                        let finetune = if let Some(ft_override) =
                            self.effect_processor.finetune_override(ch)
                        {
                            ft_override
                        } else {
                            self.instruments
                                .get(instrument_idx)
                                .map(|inst| inst.finetune)
                                .unwrap_or(0)
                        };

                        if finetune != 0 {
                            // ProTracker finetune formula: 1 unit = 1/8th of a semitone
                            playback_rate *= 2.0_f64.powf(finetune as f64 / (12.0 * 8.0));
                        }

                        // Map velocity 0-127 to gain 0.0-1.0, scaled by instrument and sample volume
                        let inst_vol = self
                            .instruments
                            .get(instrument_idx)
                            .map(|inst| inst.volume)
                            .unwrap_or(1.0);
                        let velocity_gain =
                            (note.velocity as f32 / 127.0) * inst_vol * sample.volume;

                        let mut voice = Voice::new(instrument_idx, playback_rate, velocity_gain);
                        if let Some(offset) = self.effect_processor.sample_offset(ch) {
                            voice = voice.with_position(offset as f64);
                        }

                        self.voices[ch] = Some(voice);
                    }
                }
                Some(NoteEvent::Off) => {
                    self.voices[ch] = None;
                    if let Some(s) = self.effect_processor.channel_state_mut(ch) {
                        s.reset();
                    }
                }
                None => {
                    // No event — existing voice continues
                }
            }
        }

        transport_commands
    }

    /// Render audio into a stereo interleaved f32 buffer.
    ///
    /// Mixes all active voices into the output buffer. Each frame consists
    /// of two samples (left, right). Mono samples are duplicated to both channels.
    ///
    /// # Arguments
    /// * `output` - Mutable slice of f32 samples to fill (stereo interleaved: L, R, L, R, ...)
    pub fn render(&mut self, output: &mut [f32]) {
        let num_frames = output.len() / 2;

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
            let voice = match voice_slot {
                Some(v) if v.active => v,
                _ => continue,
            };

            let sample = match samples.get(voice.sample_index) {
                Some(s) => s,
                None => {
                    voice.active = false;
                    continue;
                }
            };

            let sample_data = sample.data();
            let sample_channels = sample.channels() as usize;
            let sample_frames = sample.frame_count();

            if sample_frames == 0 {
                voice.active = false;
                continue;
            }

            let strip = &mut channel_strips[ch];

            for frame in 0..num_frames {
                let render_state = effect_processor.voice_render_state(ch);

                let src_frame = voice.position as usize;

                use crate::audio::sample::LoopMode;
                match sample.loop_mode {
                    LoopMode::NoLoop => {
                        if src_frame >= sample_frames {
                            voice.active = false;
                            break;
                        }
                    }
                    LoopMode::Forward => {
                        if src_frame > sample.loop_end {
                            let loop_len = (sample.loop_end - sample.loop_start + 1) as f64;
                            voice.position -= loop_len;
                        }
                    }
                    LoopMode::PingPong => {
                        if voice.loop_direction > 0.0 && src_frame > sample.loop_end {
                            voice.loop_direction = -1.0;
                            voice.position = sample.loop_end as f64;
                        } else if voice.loop_direction < 0.0 && src_frame < sample.loop_start {
                            voice.loop_direction = 1.0;
                            voice.position = sample.loop_start as f64;
                        }
                    }
                }

                // Final safety check for buffer access
                let src_frame = voice.position as usize;
                if src_frame >= sample_frames {
                    voice.active = false;
                    break;
                }

                let effective_rate =
                    voice.playback_rate * render_state.pitch_ratio * voice.loop_direction;

                // Read sample data with linear interpolation
                let (left, right) = {
                    let pos_floor = voice.position.floor() as usize;
                    let frac = (voice.position - pos_floor as f64) as f32;
                    let next_frame = {
                        let next = pos_floor + 1;
                        match sample.loop_mode {
                            LoopMode::NoLoop => next,
                            LoopMode::Forward => {
                                if next > sample.loop_end {
                                    sample.loop_start
                                } else {
                                    next
                                }
                            }
                            LoopMode::PingPong => {
                                // For ping-pong, if we're at the boundary, the interpolation
                                // should go towards the next position based on loop_direction.
                                if voice.loop_direction > 0.0 {
                                    if next > sample.loop_end {
                                        sample.loop_end
                                    } else {
                                        next
                                    }
                                } else {
                                    // Reverse direction: frac is effectively the distance from pos_floor.
                                    // But position is decreasing.
                                    // Actually, linear interpolation usually assumes pos1 + (pos2-pos1)*frac.
                                    // If we're moving backwards, frac is the distance from the lower integer.
                                    if pos_floor > sample.loop_start {
                                        pos_floor - 1
                                    } else {
                                        sample.loop_start
                                    }
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

                let effect_gain = render_state.gain.unwrap_or(1.0);
                let (left_gain, right_gain) = strip.next_gains();

                let out_idx = frame * 2;
                let post_l = left * voice.velocity_gain * effect_gain * left_gain;
                let post_r = right * voice.velocity_gain * effect_gain * right_gain;

                output[out_idx] += post_l;
                output[out_idx + 1] += post_r;

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

        bus_system.process_and_mix(output, num_frames);
        } // end main voice block — field borrows released

        // Preview voice: renders a one-shot sample directly into output,
        // bypassing channel strips (no mute/solo/pan, preview volume = 0.7).
        let preview_done = if let Some(ref pv) = self.preview_sample {
            let pv_rate = pv.sample_rate() as f64 / self.output_sample_rate as f64;
            let pv_frames = pv.frame_count();
            let pv_channels = pv.channels() as usize;
            let pv_data = pv.data();
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
            }
            done
        } else {
            false
        };
        if preview_done {
            self.preview_sample = None;
        }

        // Clamp output to [-1.0, 1.0] to prevent clipping distortion
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// Trigger a one-shot preview of the given sample at its natural pitch.
    /// The preview plays at the sample's recorded rate, bypassing pattern voices.
    pub fn trigger_preview(&mut self, sample: Arc<Sample>) {
        self.preview_pos = 0.0;
        self.preview_sample = Some(sample);
    }

    /// Stop any currently playing preview.
    pub fn stop_preview(&mut self) {
        self.preview_sample = None;
    }

    /// Get the number of currently active voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| matches!(v, Some(voice) if voice.active))
            .count()
    }

    /// Add a sample to the instrument list and return its instrument index.
    pub fn add_sample(&mut self, sample: Arc<Sample>) -> usize {
        let idx = self.samples.len();
        self.samples.push(sample);
        idx
    }

    /// Replace the instrument definitions used for volume/finetune lookup.
    pub fn set_instruments(&mut self, instruments: Vec<Instrument>) {
        self.instruments = instruments;
    }

    /// Clear all loaded samples.
    pub fn clear_samples(&mut self) {
        self.samples.clear();
        self.stop_all();
    }

    /// Get the number of loaded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Get the name of a loaded sample by index.
    pub fn sample_name(&self, index: usize) -> Option<&str> {
        self.samples.get(index).and_then(|s| s.name())
    }

    /// Get a reference to the loaded samples.
    pub fn samples(&self) -> &[Arc<Sample>] {
        &self.samples
    }

    /// Stop all voices immediately and reset effect state.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            *voice = None;
        }
        self.effect_processor.reset_all();
        self.bus_system.reset();
    }

    /// Update the effect processor's tempo (frames per row).
    pub fn update_tempo(&mut self, bpm: f64) {
        self.effect_processor.update_tempo(bpm);
    }

    /// Get a reference to the effect processor.
    pub fn effect_processor(&self) -> &TrackerEffectProcessor {
        &self.effect_processor
    }

    /// Get a mutable reference to the bus system for configuration.
    pub fn bus_system_mut(&mut self) -> &mut BusSystem {
        &mut self.bus_system
    }

    /// Get a reference to the bus system.
    pub fn bus_system(&self) -> &BusSystem {
        &self.bus_system
    }

    /// Returns whether a channel is currently silenced by mute/solo state.
    pub fn is_channel_silent(&self, channel: usize) -> bool {
        self.channel_strips
            .get(channel)
            .is_some_and(ChannelStrip::is_silent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::note::{Note, Pitch};
    use crate::pattern::row::Cell;

    /// Create a simple sine wave sample at 440Hz for testing.
    /// Base note is set to A-4 (MIDI 57) to match the 440Hz content.
    fn make_test_sample(sample_rate: u32, duration_secs: f32) -> Sample {
        let num_samples = (sample_rate as f32 * duration_secs) as usize;
        let mut data = Vec::with_capacity(num_samples);
        let freq = 440.0;
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            data.push((2.0 * std::f32::consts::PI * freq * t).sin());
        }
        Sample::new(data, sample_rate, 1, Some("sine440".to_string())).with_base_note(57)
        // A-4
    }

    #[test]
    fn test_mixer_creation() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_triggers_voice() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        let note = Note::new(Pitch::A, 4, 100, 0);
        pattern.set_note(0, 0, note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_tick_note_off() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        pattern.set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_empty_row_continues() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Empty row — voice should continue
        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_tick_out_of_bounds_row() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let pattern = Pattern::new(16, 4);

        // Should not panic on out-of-bounds row
        mixer.tick(100, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_invalid_instrument() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Instrument index 5 doesn't exist (only one sample loaded)
        let note = Note::new(Pitch::A, 4, 100, 5);
        pattern.set_note(0, 0, note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_render_silence_when_no_voices() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut output = vec![0.0f32; 512];

        mixer.render(&mut output);

        // Output should be all zeros
        assert!(output.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_mixer_render_produces_audio() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // Output should contain non-zero samples
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Render should produce non-zero audio data");
    }

    #[test]
    fn test_mixer_render_velocity_scaling() {
        let sample = Arc::new(make_test_sample(44100, 0.25));
        let mut mixer_loud = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut mixer_quiet = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);

        let mut pattern_loud = Pattern::new(16, 4);
        pattern_loud.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        let mut pattern_quiet = Pattern::new(16, 4);
        pattern_quiet.set_note(0, 0, Note::new(Pitch::A, 4, 32, 0));

        mixer_loud.tick(0, &pattern_loud);
        mixer_quiet.tick(0, &pattern_quiet);

        let mut output_loud = vec![0.0f32; 512];
        let mut output_quiet = vec![0.0f32; 512];

        mixer_loud.render(&mut output_loud);
        mixer_quiet.render(&mut output_quiet);

        // Loud output should have higher peak amplitude
        let peak_loud: f32 = output_loud.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_quiet: f32 = output_quiet.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_loud > peak_quiet,
            "Loud peak ({}) should exceed quiet peak ({})",
            peak_loud,
            peak_quiet
        );
    }

    #[test]
    fn test_mixer_render_multiple_voices() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_note(0, 1, Note::new(Pitch::E, 4, 100, 0));
        pattern.set_note(0, 2, Note::new(Pitch::G, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 3);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Multiple voices should produce audio");
    }

    #[test]
    fn test_mixer_render_clamping() {
        // Create a loud sample
        let num_samples = 4410;
        let data: Vec<f32> = vec![1.0; num_samples];
        let sample = Sample::new(data, 44100, 1, Some("loud".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Trigger on all 4 channels at max velocity
        for ch in 0..4 {
            pattern.set_note(0, ch, Note::new(Pitch::A, 4, 127, 0));
        }

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // All samples should be clamped to [-1.0, 1.0]
        assert!(
            output.iter().all(|&s| (-1.0..=1.0).contains(&s)),
            "Output should be clamped to [-1.0, 1.0]"
        );
    }

    #[test]
    fn test_mixer_stop_all() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        mixer.stop_all();
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_sample_and_instrument_volume() {
        let sample = Arc::new(make_test_sample(44100, 0.25).with_volume(0.5));
        let instruments = vec![Instrument::new("Test").with_volume(0.8)];
        let mut mixer = Mixer::new(vec![sample], instruments, 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // velocity 127 (1.0), inst volume 0.8, sample volume 0.5
        // expected final gain = 1.0 * 0.8 * 0.5 = 0.4
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        if let Some(voice) = &mixer.voices[0] {
            assert!((voice.velocity_gain - 0.4).abs() < 0.001);
        } else {
            panic!("Voice should be triggered");
        }
    }

    #[test]
    fn test_mixer_voice_ends_at_sample_boundary() {
        // Very short sample (10 frames at 44100Hz)
        let data: Vec<f32> = vec![0.5; 10];
        let sample = Sample::new(data, 44100, 1, Some("short".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render more frames than the sample contains — voice should deactivate
        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        assert_eq!(
            mixer.active_voice_count(),
            0,
            "Voice should deactivate after sample ends"
        );
    }

    #[test]
    fn test_mixer_zero_velocity() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 0, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        // Zero velocity should produce silence
        assert!(
            output.iter().all(|&s| s == 0.0),
            "Zero velocity should produce silence"
        );
    }

    #[test]
    fn test_mixer_stereo_sample() {
        // Stereo sample: L=0.5, R=-0.5 repeated
        let mut data = Vec::new();
        for _ in 0..100 {
            data.push(0.5); // Left
            data.push(-0.5); // Right
        }
        let sample = Sample::new(data, 44100, 2, Some("stereo".to_string()));

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 20]; // 10 stereo frames
        mixer.render(&mut output);

        // Left and right channels should have different signs
        // (at least for the first frame where position is 0)
        let left = output[0];
        let right = output[1];
        assert!(left > 0.0, "Left channel should be positive, got {}", left);
        assert!(
            right < 0.0,
            "Right channel should be negative, got {}",
            right
        );
    }

    #[test]
    fn test_mixer_c4_plays_at_original_rate() {
        // A sample with default base_note C-4: playing C-4 should give playback_rate ~1.0
        // (when sample rate matches output rate)
        let data: Vec<f32> = vec![0.5; 4410];
        let sample = Sample::new(data, 44100, 1, Some("test".to_string()));
        assert_eq!(sample.base_note(), 48); // C-4 default

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut pattern = Pattern::new(16, 4);
        // C-4 note should play at original rate
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render and verify audio is produced
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "C-4 on C-4-based sample should produce audio");
    }

    #[test]
    fn test_mixer_higher_note_plays_faster() {
        // Higher notes should consume the sample faster (higher playback rate)
        let data: Vec<f32> = (0..4410).map(|i| i as f32 / 4410.0).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut mixer_low = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut mixer_high = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);

        let mut pattern_low = Pattern::new(16, 4);
        pattern_low.set_note(0, 0, Note::new(Pitch::C, 3, 100, 0)); // C-3: one octave below base

        let mut pattern_high = Pattern::new(16, 4);
        pattern_high.set_note(0, 0, Note::new(Pitch::C, 5, 100, 0)); // C-5: one octave above base

        mixer_low.tick(0, &pattern_low);
        mixer_high.tick(0, &pattern_high);

        // Render same number of frames
        let mut output_low = vec![0.0f32; 512];
        let mut output_high = vec![0.0f32; 512];
        mixer_low.render(&mut output_low);
        mixer_high.render(&mut output_high);

        // The high-pitched version should have progressed further through the ramp sample,
        // producing higher average values in the output (since the ramp goes 0→1)
        let avg_low: f32 =
            output_low.iter().map(|s| s.abs()).sum::<f32>() / output_low.len() as f32;
        let avg_high: f32 =
            output_high.iter().map(|s| s.abs()).sum::<f32>() / output_high.len() as f32;
        assert!(
            avg_high > avg_low,
            "Higher note should progress faster through sample (avg_high={} > avg_low={})",
            avg_high,
            avg_low
        );
    }

    #[test]
    fn test_mixer_custom_base_note() {
        // Sample with base_note set to A-4 (57): playing A-4 should be original rate
        let data: Vec<f32> = vec![0.8; 4410];
        let sample = Sample::new(data, 44100, 1, Some("a4_sample".to_string())).with_base_note(57); // A-4

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);

        // At original rate (1.0), each frame reads consecutive samples
        // All samples are 0.8, so output should be ~0.8 * (100/127) * pan_gain
        // Default center pan with equal-power law: gain = 1/√2 ≈ 0.707
        let velocity_gain = 100.0 / 127.0;
        let center_pan_gain = std::f32::consts::FRAC_1_SQRT_2;
        let expected_val = 0.8 * velocity_gain * center_pan_gain;
        assert!(
            (output[0] - expected_val).abs() < 0.01,
            "A-4 on A-4-based sample should play at original rate, got {} expected ~{}",
            output[0],
            expected_val
        );
    }

    #[test]
    fn test_mixer_instrument_lookup_by_index() {
        // Create two distinct samples and verify instrument index selects the right one
        let sample_a = Sample::new(vec![0.3; 4410], 44100, 1, Some("A".to_string()));
        let sample_b = Sample::new(vec![0.9; 4410], 44100, 1, Some("B".to_string()));

        let mut mixer = Mixer::new(
            vec![Arc::new(sample_a), Arc::new(sample_b)],
            Vec::new(),
            4,
            44100,
        );

        let mut pattern = Pattern::new(16, 4);
        // Channel 0: instrument 0 (quieter sample)
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        // Channel 1: instrument 1 (louder sample)
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 127, 1));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        let mut output_a = vec![0.0f32; 64];
        let mut output_b = vec![0.0f32; 64];

        // Render with only instrument 0
        let mut mixer_a = Mixer::new(
            vec![Arc::new(Sample::new(vec![0.3; 4410], 44100, 1, None))],
            Vec::new(),
            4,
            44100,
        );
        let mut pat_a = Pattern::new(16, 4);
        pat_a.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer_a.tick(0, &pat_a);
        mixer_a.render(&mut output_a);

        // Render with only instrument 1 (louder)
        let mut mixer_b = Mixer::new(
            vec![
                Arc::new(Sample::new(vec![0.0; 1], 44100, 1, None)),
                Arc::new(Sample::new(vec![0.9; 4410], 44100, 1, None)),
            ],
            Vec::new(),
            4,
            44100,
        );
        let mut pat_b = Pattern::new(16, 4);
        pat_b.set_note(0, 0, Note::new(Pitch::C, 4, 127, 1));
        mixer_b.tick(0, &pat_b);
        mixer_b.render(&mut output_b);

        let peak_a: f32 = output_a.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_b: f32 = output_b.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_b > peak_a,
            "Instrument 1 (0.9) should be louder than instrument 0 (0.3): {} vs {}",
            peak_b,
            peak_a
        );
    }

    #[test]
    fn test_mixer_note_off_stops_sample() {
        let sample = Sample::new(vec![0.5; 44100], 44100, 1, None); // 1 second sample
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        // Trigger note
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render some audio to confirm it's playing
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(
            output.iter().any(|&s| s != 0.0),
            "Voice should be producing audio"
        );

        // Note off
        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);

        // Render after note-off should be silent
        let mut output2 = vec![0.0f32; 64];
        mixer.render(&mut output2);
        assert!(
            output2.iter().all(|&s| s == 0.0),
            "After note-off, output should be silent"
        );
    }

    #[test]
    fn test_sample_base_frequency() {
        // C-4 default base note should give ~261.63 Hz
        let sample = Sample::new(vec![], 44100, 1, None);
        assert!((sample.base_frequency() - 261.63).abs() < 0.1);

        // A-4 base note should give 440 Hz
        let sample_a4 = Sample::new(vec![], 44100, 1, None).with_base_note(57);
        assert!((sample_a4.base_frequency() - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_mixer_empty_sample_deactivates() {
        let sample = Sample::new(vec![], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        // Voice was created but sample is empty

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        assert_eq!(
            mixer.active_voice_count(),
            0,
            "Empty sample should deactivate voice"
        );
    }

    #[test]
    fn test_mixer_add_sample() {
        let sample1 = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![Arc::new(sample1)], Vec::new(), 4, 44100);
        assert_eq!(mixer.sample_count(), 1);

        let sample2 = make_test_sample(44100, 0.5);
        let idx = mixer.add_sample(Arc::new(sample2));
        assert_eq!(idx, 1);
        assert_eq!(mixer.sample_count(), 2);
    }

    #[test]
    fn test_mixer_sample_name() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
        assert_eq!(mixer.sample_name(0), Some("sine440"));
        assert_eq!(mixer.sample_name(1), None);
    }

    #[test]
    fn test_mixer_add_sample_playback() {
        let sample1 = make_test_sample(44100, 0.25);
        let sample2 = Sample::new(vec![0.8; 4410], 44100, 1, Some("loud".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample1)], Vec::new(), 4, 44100);
        let idx = mixer.add_sample(Arc::new(sample2));

        let mut pattern = Pattern::new(16, 4);
        // Use the newly added sample (instrument 1)
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, idx as u8));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        let has_audio = output.iter().any(|&s| s != 0.0);
        assert!(has_audio, "Added sample should produce audio");
    }

    // --- Multi-track mixing tests ---

    #[test]
    fn test_mixer_muted_channel_produces_silence() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));
        // Mute channel 0
        pattern.get_track_mut(0).unwrap().toggle_mute();

        mixer.tick(0, &pattern);
        // Muted channels should not trigger voices
        assert_eq!(mixer.active_voice_count(), 0);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(
            output.iter().all(|&s| s == 0.0),
            "Muted channel should produce silence"
        );
    }

    #[test]
    fn test_mixer_solo_filters_non_soloed() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 127, 0));
        // Solo channel 1 only
        pattern.get_track_mut(1).unwrap().toggle_solo();

        mixer.tick(0, &pattern);
        // Channel 0 not soloed → no voice; channel 1 soloed → voice
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_track_volume_applied() {
        let sample = Arc::new(Sample::new(
            vec![1.0; 4410],
            44100,
            1,
            Some("test".to_string()),
        ));

        // Full volume
        let mut mixer_full = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut pattern_full = Pattern::new(16, 4);
        pattern_full.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer_full.tick(0, &pattern_full);

        // Half volume
        let mut mixer_half = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
        let mut pattern_half = Pattern::new(16, 4);
        pattern_half.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern_half.get_track_mut(0).unwrap().set_volume(0.5);
        mixer_half.tick(0, &pattern_half);

        let mut output_full = vec![0.0f32; 64];
        let mut output_half = vec![0.0f32; 64];
        for _ in 0..10 {
            mixer_full.render(&mut output_full);
            mixer_half.render(&mut output_half);
        }

        let peak_full: f32 = output_full.iter().map(|s| s.abs()).fold(0.0, f32::max);
        let peak_half: f32 = output_half.iter().map(|s| s.abs()).fold(0.0, f32::max);

        assert!(
            peak_full > peak_half,
            "Full volume ({}) should be louder than half volume ({})",
            peak_full,
            peak_half
        );
        // Half volume should be roughly half the peak (within pan law)
        let ratio = peak_half / peak_full;
        assert!(
            (ratio - 0.5).abs() < 0.1,
            "Half volume ratio should be ~0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_mixer_pan_left_only_left_channel() {
        let sample = Sample::new(vec![1.0; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern.get_track_mut(0).unwrap().set_pan(-1.0); // Full left

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        for _ in 0..10 {
            mixer.render(&mut output);
        }

        // Check that right channel is silent
        let right_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);
        let left_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);

        assert!(left_peak > 0.0, "Left channel should have audio");
        assert!(
            right_peak < 0.001,
            "Right channel should be silent with full-left pan, got {}",
            right_peak
        );
    }

    #[test]
    fn test_mixer_pan_right_only_right_channel() {
        let sample = Sample::new(vec![1.0; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        pattern.get_track_mut(0).unwrap().set_pan(1.0); // Full right

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        for _ in 0..10 {
            mixer.render(&mut output);
        }

        let left_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);
        let right_peak: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);

        assert!(right_peak > 0.0, "Right channel should have audio");
        assert!(
            left_peak < 0.001,
            "Left channel should be silent with full-right pan, got {}",
            left_peak
        );
    }

    #[test]
    fn test_mixer_update_tracks_syncs_state() {
        let sample = Sample::new(vec![0.8; 4410], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut tracks = vec![
            Track::new("Kick"),
            Track::new("Snare"),
            Track::new("Hat"),
            Track::new("Bass"),
        ];
        tracks[0].set_volume(0.5);
        tracks[0].set_pan(-0.5);
        tracks[1].muted = true;
        tracks[2].solo = true;

        mixer.update_tracks(&tracks);

        assert!(mixer.is_channel_silent(0));
        assert!(mixer.is_channel_silent(1));
        assert!(!mixer.is_channel_silent(2));
        assert!(mixer.is_channel_silent(3));
    }

    #[test]
    fn test_mixer_muted_voice_still_advances_position() {
        let sample = Sample::new(vec![0.5; 4410], 44100, 1, None);
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        // Start with unmuted to trigger voice
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        pattern.get_track_mut(0).unwrap().toggle_mute();
        mixer.update_tracks(pattern.tracks());

        let mut output = vec![0.0f32; 512];
        for _ in 0..4 {
            mixer.render(&mut output);
        }

        let peak: f32 = output.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(peak < 0.0001, "Muted render should be effectively silent");
    }

    #[test]
    fn test_mixer_multi_track_independent_mix() {
        let sample = Sample::new(vec![0.2; 4410], 44100, 1, Some("test".to_string()));
        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        // Channel 0: full volume, center
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        // Channel 1: full volume, full left
        pattern.set_note(0, 1, Note::new(Pitch::E, 4, 127, 0));
        pattern.get_track_mut(1).unwrap().set_pan(-1.0);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);

        let left: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2].abs())
            .fold(0.0, f32::max);
        let right: f32 = (0..output.len() / 2)
            .map(|i| output[i * 2 + 1].abs())
            .fold(0.0, f32::max);

        assert!(
            left > right,
            "Left ({}) should exceed right ({}) due to left-panned track",
            left,
            right
        );
    }

    #[test]
    fn test_mixer_forward_loop() {
        use crate::audio::sample::LoopMode;
        // 10 frame sample, loop frames 5-9
        let data = vec![0.5; 10];
        let sample = Sample::new(data, 44100, 1, None).with_loop(LoopMode::Forward, 5, 9);

        let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render 20 frames (more than sample length)
        let mut output = vec![0.0f32; 40];
        mixer.render(&mut output);

        // Voice should still be active because it's looping
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_subframe_interpolation() {
        // Sample data: 0.0, 1.0 (at frames 0 and 1)
        let data: Vec<f32> = vec![0.0, 1.0];
        let sample = Arc::new(Sample::new(data, 44100, 1, None));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

        mixer.tick(0, &pattern);

        // Manually set position to 0.5 — without interpolation we'd get 0.0 or 1.0
        // with linear interpolation we should get 0.5
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 0.5;
        }

        let mut output = vec![0.0f32; 2];
        mixer.render(&mut output);

        // Center pan gain is ~0.707
        let expected = 0.5 * 0.70710677;
        assert!(
            (output[0] - expected).abs() < 0.01,
            "Expected ~{}, got {}",
            expected,
            output[0]
        );
    }

    #[test]
    fn test_mixer_forward_loop_boundary() {
        use crate::audio::sample::LoopMode;
        // 10 frame sample, loop frames 5-9 (end inclusive)
        // Values: 0.0, 0.1, 0.2, ..., 0.9
        let data: Vec<f32> = (0..10).map(|i| i as f32 / 10.0).collect();
        let sample = Arc::new(Sample::new(data, 44100, 1, None).with_loop(LoopMode::Forward, 5, 9));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);

        // Position just at the end of loop
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 9.5;
        }

        let mut output = vec![0.0f32; 2];
        mixer.render(&mut output);

        // Interpolation between 0.9 (end) and 0.5 (start)
        // pos 9.5: l1=0.9, l2=0.5, frac=0.5 => 0.9 + (0.5-0.9)*0.5 = 0.7
        let expected = 0.7 * 0.70710677;
        assert!(
            (output[0] - expected).abs() < 0.01,
            "Expected ~{}, got {}",
            expected,
            output[0]
        );
    }

    #[test]
    fn test_mixer_pingpong_loop_boundary() {
        use crate::audio::sample::LoopMode;
        // Values: 0.0, 0.1, 0.2, ..., 0.9
        let data: Vec<f32> = (0..10).map(|i| i as f32 / 10.0).collect();
        let sample =
            Arc::new(Sample::new(data, 44100, 1, None).with_loop(LoopMode::PingPong, 5, 9));

        let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
        let mut pattern = Pattern::new(16, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer.tick(0, &pattern);

        // Position just at the end of loop, moving forward
        if let Some(voice) = mixer.voices[0].as_mut() {
            voice.position = 8.5;
            voice.loop_direction = 1.0;
        }

        // Render 3 frames (6 samples)
        let mut output = vec![0.0f32; 6];
        mixer.render(&mut output);

        // Frame 1: pos 8.5. No reversal. pos -> 9.5
        // Frame 2: pos 9.5. No reversal (9 > 9 is false). pos -> 10.5
        // Frame 3: pos 10.5. src_frame = 10. 10 > 9 is true. REVERSAL.
        //          loop_dir -> -1.0. pos -> 9.0.
        //          Then it renders pos 9.0. pos -> 9.0 + (-1.0) = 8.0.

        let voice = mixer.voices[0].as_ref().unwrap();
        assert_eq!(voice.loop_direction, -1.0);
        assert_eq!(voice.position, 8.0);
    }
}
