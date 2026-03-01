//! Audio mixer/sequencer that connects patterns to the audio engine.
//!
//! The mixer reads pattern data row by row, triggers sample playback for
//! note events, and mixes all active voices into a stereo output buffer.

use crate::audio::sample::Sample;
use crate::pattern::note::NoteEvent;
use crate::pattern::pattern::Pattern;

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
}

impl Voice {
    fn new(sample_index: usize, playback_rate: f64, velocity_gain: f32) -> Self {
        Self {
            sample_index,
            position: 0.0,
            playback_rate,
            velocity_gain,
            active: true,
        }
    }
}

/// Audio mixer that reads pattern data and produces mixed audio output.
///
/// The mixer holds references to loaded samples and maintains per-channel
/// voice state. When `tick()` is called with a row index and pattern, it
/// processes note events and updates voice states. The `render()` method
/// fills an audio buffer by mixing all active voices.
pub struct Mixer {
    /// Loaded audio samples available for playback.
    samples: Vec<Sample>,
    /// Per-channel voice state (one voice per channel).
    voices: Vec<Option<Voice>>,
    /// Output sample rate in Hz (used for pitch calculation).
    output_sample_rate: u32,
}

impl Mixer {
    /// Create a new mixer with the given samples and channel count.
    ///
    /// # Arguments
    /// * `samples` - The loaded audio samples indexed by instrument number
    /// * `num_channels` - Number of pattern channels (one voice per channel)
    /// * `output_sample_rate` - The output sample rate in Hz
    pub fn new(samples: Vec<Sample>, num_channels: usize, output_sample_rate: u32) -> Self {
        Self {
            samples,
            voices: vec![None; num_channels],
            output_sample_rate,
        }
    }

    /// Process a pattern row, triggering or stopping samples based on note events.
    ///
    /// For each channel in the row:
    /// - `NoteEvent::On(note)`: Start playing the sample at the instrument index,
    ///   pitched to match the note's frequency, with velocity-based volume.
    /// - `NoteEvent::Off`: Stop the voice on that channel.
    /// - No event: The existing voice continues playing.
    pub fn tick(&mut self, row_index: usize, pattern: &Pattern) {
        let row = match pattern.get_row(row_index) {
            Some(r) => r,
            None => return,
        };

        for (ch, cell) in row.iter().enumerate() {
            if ch >= self.voices.len() {
                break;
            }

            match &cell.note {
                Some(NoteEvent::On(note)) => {
                    let instrument = cell.instrument.unwrap_or(note.instrument) as usize;
                    if instrument < self.samples.len() {
                        let sample = &self.samples[instrument];
                        // Calculate playback rate to pitch the sample to the desired note.
                        // The sample's base_note (default C-4) plays at original speed.
                        // Higher notes play faster, lower notes play slower.
                        let base_freq = sample.base_frequency();
                        let target_freq = note.frequency();
                        let sample_rate_ratio =
                            sample.sample_rate() as f64 / self.output_sample_rate as f64;
                        let playback_rate =
                            (target_freq / base_freq) * sample_rate_ratio;

                        // Map velocity 0-127 to gain 0.0-1.0
                        let velocity_gain = note.velocity as f32 / 127.0;

                        self.voices[ch] = Some(Voice::new(instrument, playback_rate, velocity_gain));
                    }
                }
                Some(NoteEvent::Off) => {
                    self.voices[ch] = None;
                }
                None => {
                    // No event — existing voice continues
                }
            }
        }
    }

    /// Render audio into a stereo interleaved f32 buffer.
    ///
    /// Mixes all active voices into the output buffer. Each frame consists
    /// of two samples (left, right). Mono samples are duplicated to both channels.
    ///
    /// # Arguments
    /// * `output` - Mutable slice of f32 samples to fill (stereo interleaved: L, R, L, R, ...)
    pub fn render(&mut self, output: &mut [f32]) {
        // Clear the buffer first
        for sample in output.iter_mut() {
            *sample = 0.0;
        }

        let num_frames = output.len() / 2;

        for voice_slot in &mut self.voices {
            let voice = match voice_slot {
                Some(v) if v.active => v,
                _ => continue,
            };

            let sample = match self.samples.get(voice.sample_index) {
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

            for frame in 0..num_frames {
                let src_frame = voice.position as usize;
                if src_frame >= sample_frames {
                    voice.active = false;
                    break;
                }

                // Read sample data (mono or stereo)
                let (left, right) = if sample_channels >= 2 {
                    let idx = src_frame * sample_channels;
                    if idx + 1 < sample_data.len() {
                        (sample_data[idx], sample_data[idx + 1])
                    } else {
                        voice.active = false;
                        break;
                    }
                } else {
                    let idx = src_frame;
                    if idx < sample_data.len() {
                        (sample_data[idx], sample_data[idx])
                    } else {
                        voice.active = false;
                        break;
                    }
                };

                // Apply velocity gain and mix into output
                let out_idx = frame * 2;
                output[out_idx] += left * voice.velocity_gain;
                output[out_idx + 1] += right * voice.velocity_gain;

                voice.position += voice.playback_rate;
            }
        }

        // Clamp output to [-1.0, 1.0] to prevent clipping distortion
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    /// Get the number of currently active voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| matches!(v, Some(voice) if voice.active))
            .count()
    }

    /// Add a sample to the instrument list and return its instrument index.
    pub fn add_sample(&mut self, sample: Sample) -> usize {
        let idx = self.samples.len();
        self.samples.push(sample);
        idx
    }

    /// Get the number of loaded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Get the name of a loaded sample by index.
    pub fn sample_name(&self, index: usize) -> Option<&str> {
        self.samples.get(index).and_then(|s| s.name())
    }

    /// Stop all voices immediately.
    pub fn stop_all(&mut self) {
        for voice in &mut self.voices {
            *voice = None;
        }
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
        Sample::new(data, sample_rate, 1, Some("sine440".to_string()))
            .with_base_note(57) // A-4
    }

    #[test]
    fn test_mixer_creation() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![sample], 4, 44100);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_triggers_voice() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        let note = Note::new(Pitch::A, 4, 100, 0);
        pattern.set_note(0, 0, note);

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);
    }

    #[test]
    fn test_mixer_tick_note_off() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);
        let pattern = Pattern::new(16, 4);

        // Should not panic on out-of-bounds row
        mixer.tick(100, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_tick_invalid_instrument() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);
        let mut output = vec![0.0f32; 512];

        mixer.render(&mut output);

        // Output should be all zeros
        assert!(output.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn test_mixer_render_produces_audio() {
        let sample = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let sample = make_test_sample(44100, 0.25);
        let mut mixer_loud = Mixer::new(vec![sample.clone()], 4, 44100);
        let mut mixer_quiet = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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

        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        pattern.set_note(0, 1, Note::new(Pitch::C, 4, 100, 0));

        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 2);

        mixer.stop_all();
        assert_eq!(mixer.active_voice_count(), 0);
    }

    #[test]
    fn test_mixer_voice_ends_at_sample_boundary() {
        // Very short sample (10 frames at 44100Hz)
        let data: Vec<f32> = vec![0.5; 10];
        let sample = Sample::new(data, 44100, 1, Some("short".to_string()));

        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
            data.push(0.5);  // Left
            data.push(-0.5); // Right
        }
        let sample = Sample::new(data, 44100, 2, Some("stereo".to_string()));

        let mut mixer = Mixer::new(vec![sample], 4, 44100);

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
        assert!(right < 0.0, "Right channel should be negative, got {}", right);
    }

    #[test]
    fn test_mixer_c4_plays_at_original_rate() {
        // A sample with default base_note C-4: playing C-4 should give playback_rate ~1.0
        // (when sample rate matches output rate)
        let data: Vec<f32> = vec![0.5; 4410];
        let sample = Sample::new(data, 44100, 1, Some("test".to_string()));
        assert_eq!(sample.base_note(), 48); // C-4 default

        let mut mixer = Mixer::new(vec![sample], 4, 44100);
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
        let sample = Sample::new(data, 44100, 1, None);

        let mut mixer_low = Mixer::new(vec![sample.clone()], 4, 44100);
        let mut mixer_high = Mixer::new(vec![sample], 4, 44100);

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
        let avg_low: f32 = output_low.iter().map(|s| s.abs()).sum::<f32>() / output_low.len() as f32;
        let avg_high: f32 = output_high.iter().map(|s| s.abs()).sum::<f32>() / output_high.len() as f32;
        assert!(
            avg_high > avg_low,
            "Higher note should progress faster through sample (avg_high={} > avg_low={})",
            avg_high, avg_low
        );
    }

    #[test]
    fn test_mixer_custom_base_note() {
        // Sample with base_note set to A-4 (57): playing A-4 should be original rate
        let data: Vec<f32> = vec![0.8; 4410];
        let sample = Sample::new(data, 44100, 1, Some("a4_sample".to_string()))
            .with_base_note(57); // A-4

        let mut mixer = Mixer::new(vec![sample], 4, 44100);
        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);

        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);

        // At original rate (1.0), each frame reads consecutive samples
        // All samples are 0.8, so output should be ~0.8 * (100/127)
        let expected_gain = 100.0 / 127.0;
        let expected_val = 0.8 * expected_gain;
        assert!(
            (output[0] - expected_val).abs() < 0.01,
            "A-4 on A-4-based sample should play at original rate, got {} expected ~{}",
            output[0], expected_val
        );
    }

    #[test]
    fn test_mixer_instrument_lookup_by_index() {
        // Create two distinct samples and verify instrument index selects the right one
        let sample_a = Sample::new(vec![0.3; 4410], 44100, 1, Some("A".to_string()));
        let sample_b = Sample::new(vec![0.9; 4410], 44100, 1, Some("B".to_string()));

        let mut mixer = Mixer::new(vec![sample_a, sample_b], 4, 44100);

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
            vec![Sample::new(vec![0.3; 4410], 44100, 1, None)],
            4, 44100,
        );
        let mut pat_a = Pattern::new(16, 4);
        pat_a.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        mixer_a.tick(0, &pat_a);
        mixer_a.render(&mut output_a);

        // Render with only instrument 1 (louder)
        let mut mixer_b = Mixer::new(
            vec![Sample::new(vec![0.0; 1], 44100, 1, None), Sample::new(vec![0.9; 4410], 44100, 1, None)],
            4, 44100,
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
            peak_b, peak_a
        );
    }

    #[test]
    fn test_mixer_note_off_stops_sample() {
        let sample = Sample::new(vec![0.5; 44100], 44100, 1, None); // 1 second sample
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_cell(1, 0, Cell::with_note(NoteEvent::Off));

        // Trigger note
        mixer.tick(0, &pattern);
        assert_eq!(mixer.active_voice_count(), 1);

        // Render some audio to confirm it's playing
        let mut output = vec![0.0f32; 64];
        mixer.render(&mut output);
        assert!(output.iter().any(|&s| s != 0.0), "Voice should be producing audio");

        // Note off
        mixer.tick(1, &pattern);
        assert_eq!(mixer.active_voice_count(), 0);

        // Render after note-off should be silent
        let mut output2 = vec![0.0f32; 64];
        mixer.render(&mut output2);
        assert!(output2.iter().all(|&s| s == 0.0), "After note-off, output should be silent");
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
        let mut mixer = Mixer::new(vec![sample], 4, 44100);

        let mut pattern = Pattern::new(16, 4);
        pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        mixer.tick(0, &pattern);
        // Voice was created but sample is empty

        let mut output = vec![0.0f32; 512];
        mixer.render(&mut output);

        assert_eq!(mixer.active_voice_count(), 0, "Empty sample should deactivate voice");
    }

    #[test]
    fn test_mixer_add_sample() {
        let sample1 = make_test_sample(44100, 0.25);
        let mut mixer = Mixer::new(vec![sample1], 4, 44100);
        assert_eq!(mixer.sample_count(), 1);

        let sample2 = make_test_sample(44100, 0.5);
        let idx = mixer.add_sample(sample2);
        assert_eq!(idx, 1);
        assert_eq!(mixer.sample_count(), 2);
    }

    #[test]
    fn test_mixer_sample_name() {
        let sample = make_test_sample(44100, 0.25);
        let mixer = Mixer::new(vec![sample], 4, 44100);
        assert_eq!(mixer.sample_name(0), Some("sine440"));
        assert_eq!(mixer.sample_name(1), None);
    }

    #[test]
    fn test_mixer_add_sample_playback() {
        let sample1 = make_test_sample(44100, 0.25);
        let sample2 = Sample::new(vec![0.8; 4410], 44100, 1, Some("loud".to_string()));
        let mut mixer = Mixer::new(vec![sample1], 4, 44100);
        let idx = mixer.add_sample(sample2);

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
}
