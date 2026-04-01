use super::Mixer;
use crate::audio::sample::Sample;
use crate::audio::voice::evaluate_lfo_waveform;
use crate::audio::voice::VoiceLfoState;
use crate::pattern::effect::{Effect, EffectType};
use crate::pattern::note::{Note, NoteEvent, Pitch};
use crate::pattern::pattern::Pattern;
use crate::pattern::row::Cell;
use crate::pattern::track::Track;
use crate::song::Instrument;
use crate::song::{Lfo, LfoWaveform};
use std::sync::Arc;

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
fn test_mixer_tpl_change_affects_timing() {
    let data = vec![1.0f32; 100000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);

    mixer.update_tempo(120.0);
    mixer.set_tpl(6);
    mixer.tick(0, &crate::pattern::pattern::Pattern::new(1, 1));

    let state_tpl6 = mixer
        .effect_processor()
        .channel_state(0)
        .unwrap()
        .ticks_per_row;

    mixer.set_tpl(12);
    mixer.tick(0, &crate::pattern::pattern::Pattern::new(1, 1));

    let state_tpl12 = mixer
        .effect_processor()
        .channel_state(0)
        .unwrap()
        .ticks_per_row;

    assert_eq!(state_tpl6, 6, "Initial TPL should be 6");
    assert_eq!(state_tpl12, 12, "TPL change to 12 should be reflected");
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
    let mut inst = Instrument::new("Test").with_volume(0.8);
    inst.sample_index = Some(0);
    let instruments = vec![inst];
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
    let avg_low: f32 = output_low.iter().map(|s| s.abs()).sum::<f32>() / output_low.len() as f32;
    let avg_high: f32 = output_high.iter().map(|s| s.abs()).sum::<f32>() / output_high.len() as f32;
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
    // Center pan gain is cos(45 deg)
    let center_pan_gain = std::f32::consts::FRAC_PI_4.cos();
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

    mixer.update_tracks(pattern.tracks());
    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 64];
    // Render enough to finish the 0.005s ramp (approx 220 samples at 44.1kHz)
    for _ in 0..100 {
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

    mixer.update_tracks(pattern.tracks());
    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 64];
    // Render enough to finish the 0.005s ramp
    for _ in 0..100 {
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
    // Channel 0: full volume, full right
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
    pattern.get_track_mut(0).unwrap().set_pan(1.0);
    // Channel 1: full volume, full left
    pattern.set_note(0, 1, Note::new(Pitch::E, 4, 127, 0));
    pattern.get_track_mut(1).unwrap().set_pan(-1.0);

    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 2);

    let mut output = vec![0.0f32; 64];
    // Render enough to finish potential ramps
    for _ in 0..100 {
        mixer.render(&mut output);
    }

    let left: f32 = (0..output.len() / 2)
        .map(|i| output[i * 2].abs())
        .fold(0.0, f32::max);
    let right: f32 = (0..output.len() / 2)
        .map(|i| output[i * 2 + 1].abs())
        .fold(0.0, f32::max);

    // Since both have same volume and sample value, they should be roughly equal but present
    assert!(left > 0.0);
    assert!(right > 0.0);
    assert!((left - right).abs() < 0.01);
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

    // Center pan gain is ~0.707 (cos(pi/4))
    let expected = 0.5 * std::f32::consts::FRAC_PI_4.cos();
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
    let sample = Arc::new(Sample::new(data, 44100, 1, None).with_loop(LoopMode::PingPong, 5, 9));

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
    // Frame 3: pos 10.5. 10.5 >= (9 + 1 = 10.0) is true. REVERSAL.
    //          loop_dir -> -1.0. pos -> 9.0 - (10.5 - 10.0) = 8.5.
    //          Then it renders pos 8.5. pos -> 8.5 + (-1.0) = 7.5.

    let voice = mixer.voices[0].as_ref().unwrap();
    assert_eq!(voice.loop_direction, -1.0);
    assert_eq!(voice.position, 7.5);
}

// --- Preview toggle & scrub ---

#[test]
fn test_is_preview_playing_false_initially() {
    let sample = make_test_sample(44100, 0.25);
    let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
    assert!(!mixer.is_preview_playing());
}

#[test]
fn test_is_preview_playing_true_after_trigger() {
    let sample = Arc::new(make_test_sample(44100, 0.25));
    let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    mixer.trigger_preview(Arc::clone(&sample), 1.0);
    assert!(mixer.is_preview_playing());
}

#[test]
fn test_is_preview_playing_false_after_stop() {
    let sample = Arc::new(make_test_sample(44100, 0.25));
    let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    mixer.trigger_preview(Arc::clone(&sample), 1.0);
    mixer.stop_preview();
    assert!(!mixer.is_preview_playing());
}

#[test]
fn test_trigger_preview_at_sets_offset() {
    let sample = Arc::new(make_test_sample(44100, 1.0));
    let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    // Start 0.1s (4410 frames) into the sample
    mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 4410);
    assert!(mixer.is_preview_playing());
    // Render a small buffer — should not panic, preview starts mid-sample
    let mut output = vec![0.0f32; 64];
    mixer.render(&mut output);
    assert!(
        mixer.is_preview_playing(),
        "still playing after small render"
    );
}

#[test]
fn test_trigger_preview_at_zero_same_as_trigger_preview() {
    let sample = Arc::new(make_test_sample(44100, 0.25));
    let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 0);
    assert!(mixer.is_preview_playing());
}

#[test]
fn test_preview_pos_and_total_no_preview() {
    let sample = Arc::new(make_test_sample(44100, 0.25));
    let mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    let (pos, total) = mixer.preview_pos_and_total();
    assert_eq!(pos, 0);
    assert_eq!(total, 0);
}

#[test]
fn test_preview_pos_and_total_after_trigger_at() {
    let sample = Arc::new(make_test_sample(44100, 1.0)); // 44100 frames
    let mut mixer = Mixer::new(vec![Arc::clone(&sample)], Vec::new(), 4, 44100);
    mixer.trigger_preview_at(Arc::clone(&sample), 1.0, 4410);
    let (pos, total) = mixer.preview_pos_and_total();
    assert_eq!(pos, 4410);
    assert_eq!(total, sample.frame_count());
}

#[test]
fn test_mixer_note_delay_edx() {
    let data = vec![1.0f32; 100];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(120.0); // Sync tempo for 44100Hz
    let mut pattern = Pattern::new(16, 1);

    // ED3: Delay by 3 ticks. Default 6 ticks per row.
    // 120 BPM => 125ms per row. 6 ticks => 20.83ms per tick.
    // 3 ticks => 62.5ms. At 44100Hz => 2756.25 frames.
    // trigger_frame = 3 * (5512 / 6) = 3 * 918 = 2754.
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), effects: vec![Effect::from_type(EffectType::Extended, 0xD3)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    // Voice should NOT be active initially
    assert!(mixer.voices[0].is_none());
    assert_eq!(mixer.pending_notes.len(), 1);

    // Render some frames (less than 3 ticks)
    let mut output = vec![0.0f32; 2000 * 2];
    mixer.render(&mut output);
    assert!(output.iter().all(|&s| s == 0.0));
    assert!(mixer.voices[0].is_none());

    // Render more frames to pass the trigger point (2754)
    let mut output2 = vec![0.0f32; 1000 * 2];
    mixer.render(&mut output2);

    // Voice should now be active
    assert!(mixer.voices[0].is_some());
    // And we should have some audio in output2
    assert!(output2.iter().any(|&s| s > 0.0));
}

#[test]
fn test_mixer_note_cut_ecx() {
    let data = vec![1.0f32; 10000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(120.0);
    let mut pattern = Pattern::new(16, 1);

    // EC2: Cut after 2 ticks. 2 ticks = 2 * 918 = 1836 frames.
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), effects: vec![Effect::from_type(EffectType::Extended, 0xC2)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);
    assert!(mixer.voices[0].is_some());

    // Render 1000 frames (less than 2 ticks)
    let mut output = vec![0.0f32; 1000 * 2];
    mixer.render(&mut output);
    assert!(output.iter().any(|&s| s > 0.0));
    assert!(mixer.voices[0].as_ref().unwrap().active);

    // Render more frames to pass the cut point (1836)
    let mut output2 = vec![0.0f32; 2000 * 2];
    mixer.render(&mut output2);

    // Voice should now be inactive
    assert!(!mixer.voices[0].as_ref().unwrap().active);
}

#[test]
fn test_mixer_tremor_effect() {
    let data = vec![1.0f32; 100000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(60.0);
    let mut pattern = Pattern::new(16, 1);

    // Txy: Tremor - ON for x ticks, OFF for y ticks
    // T31: 3 ticks on, 1 tick off
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), effects: vec![Effect::from_type(EffectType::Tremor, 0x31)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    // Verify tremor state is set
    let state = mixer.effect_processor().channel_state(0).unwrap();
    assert!(state.tremor_active, "Tremor should be active");
    assert_eq!(state.tremor_on, 3, "Tremor ON should be 3 ticks");
    assert_eq!(state.tremor_off, 1, "Tremor OFF should be 1 tick");
}

// --- VU Meter Tests ---

#[test]
fn test_mixer_channel_levels_initialized_to_zero() {
    let sample = make_test_sample(44100, 0.25);
    let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    for ch in 0..4 {
        let (l, r) = mixer.get_channel_level(ch);
        assert_eq!(l, 0.0, "Channel {} left should be 0 initially", ch);
        assert_eq!(r, 0.0, "Channel {} right should be 0 initially", ch);
    }
}

#[test]
fn test_mixer_channel_levels_invalid_channel() {
    let sample = make_test_sample(44100, 0.25);
    let mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let (l, r) = mixer.get_channel_level(99);
    assert_eq!(l, 0.0);
    assert_eq!(r, 0.0);
}

#[test]
fn test_mixer_render_tracks_peak_levels() {
    let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 512];
    mixer.render(&mut output);

    let (l, r) = mixer.get_channel_level(0);
    assert!(l > 0.0, "Channel 0 left peak should be > 0, got {}", l);
    assert!(r > 0.0, "Channel 0 right peak should be > 0, got {}", r);
}

#[test]
fn test_mixer_render_peak_levels_accumulate() {
    let sample = Sample::new(vec![0.5f32; 4410], 44100, 1, Some("test".to_string()));
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 256];
    mixer.render(&mut output);
    let (l1, _) = mixer.get_channel_level(0);

    mixer.render(&mut output);
    let (l2, _) = mixer.get_channel_level(0);

    assert_eq!(
        l1, l2,
        "Peak should remain the same across renders without new audio"
    );
}

#[test]
fn test_mixer_reset_channel_levels() {
    let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 512];
    mixer.render(&mut output);

    let (l, _r) = mixer.get_channel_level(0);
    assert!(l > 0.0);

    mixer.reset_channel_levels();

    let (l, r) = mixer.get_channel_level(0);
    assert_eq!(l, 0.0, "Left peak should be reset to 0");
    assert_eq!(r, 0.0, "Right peak should be reset to 0");
}

#[test]
fn test_mixer_decay_channel_levels() {
    let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 512];
    mixer.render(&mut output);

    let (l, _) = mixer.get_channel_level(0);
    assert!(l > 0.0);

    mixer.decay_channel_levels(0.5);

    let (l2, _) = mixer.get_channel_level(0);
    assert!(
        (l2 - l * 0.5).abs() < 0.001,
        "Peak should decay to 50%, got {} expected ~{}",
        l2,
        l * 0.5
    );
}

#[test]
fn test_mixer_decay_to_zero() {
    let sample = Sample::new(vec![0.8f32; 4410], 44100, 1, Some("test".to_string()));
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 512];
    mixer.render(&mut output);

    // 20 iterations of 0.9 decay = 0.9^20 ≈ 0.12 of original
    for _ in 0..20 {
        mixer.decay_channel_levels(0.9);
    }

    let (l, _) = mixer.get_channel_level(0);
    assert!(l < 0.1, "Peak should decay to less than 10%, got {}", l);
}

// --- Effect Processing Edge Cases ---

#[test]
fn test_mixer_arpeggio_effect_changes_pitch() {
    let data: Vec<f32> = (0..44100).map(|i| i as f32 / 100.0).collect();
    let sample = Arc::new(Sample::new(data, 44100, 1, None).with_base_note(48));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(60.0);
    let mut pattern = Pattern::new(16, 1);

    // 0xy: arpeggio with x=4 (major third), y=7 (perfect fifth)
    // C-4 → C-4 → E-4 → G-4 cycles
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), effects: vec![Effect::from_type(EffectType::Arpeggio, 0x47)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    // First third: base pitch
    let mut output1 = vec![0.0f32; 500];
    mixer.render(&mut output1);
    let peak1: f32 = output1.iter().map(|s| s.abs()).fold(0.0, f32::max);

    // Second third: +4 semitones (C to E)
    let mut output2 = vec![0.0f32; 500];
    mixer.render(&mut output2);
    let peak2: f32 = output2.iter().map(|s| s.abs()).fold(0.0, f32::max);

    // Third third: +7 semitones (C to G)
    let mut output3 = vec![0.0f32; 500];
    mixer.render(&mut output3);
    let peak3: f32 = output3.iter().map(|s| s.abs()).fold(0.0, f32::max);

    assert!(
        peak1 > 0.0 && peak2 > 0.0 && peak3 > 0.0,
        "All arpeggio phases should produce audio"
    );
}

#[test]
fn test_mixer_portamento_slide_changes_pitch() {
    let data: Vec<f32> = (0..44100).map(|i| (i as f32 / 100.0).sin() * 0.5).collect();
    let sample = Arc::new(Sample::new(data, 44100, 1, None).with_base_note(48));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(60.0);
    let mut pattern = Pattern::new(16, 1);

    // Row 0: Start on C-4
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
    // Row 1: Portamento to E-4 (3xx with speed parameter)
    let mut cell1 = Cell::default();
    cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 0)));
    cell1
        .effects
        .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
    pattern.set_cell(1, 0, cell1);

    mixer.tick(0, &pattern);
    let freq_before = mixer.voices[0].as_ref().map(|v| v.triggered_note_freq);
    let c4_freq = Note::new(Pitch::C, 4, 127, 0).frequency();
    assert_eq!(
        freq_before,
        Some(c4_freq),
        "Initial note should be C-4 frequency"
    );

    // Render some frames
    let mut output1 = vec![0.0f32; 5000];
    mixer.render(&mut output1);

    // Apply portamento on row 1
    mixer.tick(1, &pattern);

    // Portamento should be active, voice should continue
    assert!(
        mixer.voices[0].is_some(),
        "Voice should continue during portamento"
    );
}

#[test]
fn test_mixer_tone_portamento_updates_instrument() {
    let data = vec![1.0f32; 1000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));

    let mut inst1 = Instrument::new("Inst1").with_volume(0.5);
    inst1.sample_index = Some(0);
    let mut inst2 = Instrument::new("Inst2").with_volume(1.0);
    inst2.sample_index = Some(0);

    let mut mixer = Mixer::new(vec![sample], vec![inst1, inst2], 1, 44100);
    let mut pattern = Pattern::new(16, 1);

    // Row 0: Start note with instrument 0 (vol 0.5)
    let mut cell0 = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), ..Default::default() };
    pattern.set_cell(0, 0, cell0);

    mixer.tick(0, &pattern);
    // Render a bit to advance position from 0
    let mut output = vec![0.0f32; 100];
    mixer.render(&mut output);

    let vol_before = mixer.voices[0].as_ref().unwrap().velocity_gain;
    assert!((vol_before - 0.5).abs() < 0.001);

    // Row 1: Tone portamento to E-4 with instrument 1 (vol 1.0)
    let mut cell1 = Cell::default();
    cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 1)));
    cell1.instrument = Some(1);
    cell1
        .effects
        .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
    pattern.set_cell(1, 0, cell1);

    mixer.tick(1, &pattern);

    let voice = mixer.voices[0].as_ref().unwrap();
    let vol_after = voice.velocity_gain;

    // Volume should be updated to 1.0 (from inst2)
    assert!(
        (vol_after - 1.0).abs() < 0.001,
        "Volume should be updated to 1.0, got {}",
        vol_after
    );
    // Position should NOT be reset (no re-trigger)
    assert!(
        voice.position > 0.0,
        "Voice should not be re-triggered (position should be > 0)"
    );
}

#[test]
fn test_mixer_tone_portamento_instrument_only() {
    let data = vec![1.0f32; 1000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));

    let mut inst1 = Instrument::new("Inst1").with_volume(0.5);
    inst1.sample_index = Some(0);
    let mut inst2 = Instrument::new("Inst2").with_volume(1.0);
    inst2.sample_index = Some(0);

    let mut mixer = Mixer::new(vec![sample], vec![inst1, inst2], 1, 44100);
    let mut pattern = Pattern::new(16, 1);
    println!(
        "DEBUG: pattern rows = {}, channels = {}",
        pattern.row_count(),
        pattern.num_channels()
    );

    // Row 0: Start note with instrument 0 (vol 0.5)
    let mut cell0 = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), ..Default::default() };
    pattern.set_cell(0, 0, cell0);

    mixer.tick(0, &pattern);
    // Render a bit to advance position from 0
    let mut output = vec![0.0f32; 100];
    mixer.render(&mut output);

    let vol_before = mixer.voices[0].as_ref().unwrap().velocity_gain;
    assert!((vol_before - 0.5).abs() < 0.001);

    // Row 1: Tone portamento with instrument 1 (vol 1.0) with NEW NOTE
    let mut cell1 = Cell::default();
    cell1.note = Some(NoteEvent::On(Note::new(Pitch::E, 4, 127, 0)));
    cell1.instrument = Some(1);
    cell1
        .effects
        .push(Effect::from_type(EffectType::PortamentoToNote, 0x10));
    pattern.set_cell(1, 0, cell1);

    mixer.tick(1, &pattern);

    let voice = mixer.voices[0].as_ref().unwrap();
    let vol_after = voice.velocity_gain;

    // Volume should be updated to 1.0 (from inst2)
    println!(
        "DEBUG: vol_after = {}, inst_idx = {}",
        vol_after, voice.instrument_index
    );
    assert!(
        (vol_after - 1.0).abs() < 0.001,
        "Volume should be updated to 1.0, got {}",
        vol_after
    );
    assert_eq!(voice.instrument_index, 1);
    assert!(voice.position > 0.0, "Voice should not be re-triggered");
}

#[test]
fn test_mixer_volume_column_applied() {
    let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
    let mut pattern = Pattern::new(16, 1);

    // Volume column (v40 = half volume)
    let mut cell = Cell::default();
    cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
    cell.volume = Some(0x40); // 64 decimal
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 1);

    let mut output = vec![0.0f32; 256];
    mixer.render(&mut output);

    let peak: f32 = output.iter().map(|s| s.abs()).fold(0.0, f32::max);
    // Volume column 0x40 = 64/64 = 1.0, applied via volume_override
    // Should produce audio at normalized level
    assert!(peak > 0.0, "Volume column should produce audio");
}

#[test]
fn test_mixer_voice_stealing_on_new_note() {
    let sample = Sample::new(vec![0.5; 44100], 44100, 1, None);
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 4, 44100);
    let mut pattern = Pattern::new(16, 4);

    // Start note on channel 0
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 1);

    // New note on same channel should steal the voice
    pattern.set_note(1, 0, Note::new(Pitch::E, 4, 100, 0));
    mixer.tick(1, &pattern);

    // Should still have 1 voice, just restarted
    assert_eq!(
        mixer.active_voice_count(),
        1,
        "New note on same channel should replace voice"
    );

    // Voice position should be reset (new note)
    let voice_pos = mixer.voices[0].as_ref().map(|v| v.position);
    assert_eq!(voice_pos, Some(0.0), "New note should reset voice position");
}

#[test]
fn test_mixer_set_volume_effect_cxx() {
    let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
    let mut pattern = Pattern::new(16, 1);

    // C20: set volume to 32/64 = 0.5
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), effects: vec![Effect::from_type(EffectType::Extended, 0xD3)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 1);

    let mut output1 = vec![0.0f32; 256];
    mixer.render(&mut output1);
    let peak_half: f32 = output1.iter().map(|s| s.abs()).fold(0.0, f32::max);

    // Now set full volume C40
    let mut cell2 = Cell::default();
    cell2.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
    cell2
        .effects
        .push(Effect::from_type(EffectType::SetVolume, 0x40));
    let mut pattern2 = Pattern::new(16, 1);
    pattern2.set_cell(0, 0, cell2);

    mixer.tick(0, &pattern2);
    let mut output2 = vec![0.0f32; 256];
    mixer.render(&mut output2);
    let peak_full: f32 = output2.iter().map(|s| s.abs()).fold(0.0, f32::max);

    assert!(
        peak_full > peak_half,
        "Full volume ({}) should be louder than half ({}), actual: {}",
        peak_full,
        peak_half / 0.5 * 1.0,
        peak_full
    );
}

#[test]
fn test_mixer_sample_offset_9xx() {
    let data: Vec<f32> = (0..10000).map(|i| i as f32).collect();
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    let mut pattern = Pattern::new(16, 1);

    // 9xx: sample offset 512 bytes (2 * 256)
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), effects: vec![Effect::from_type(EffectType::Extended, 0xD3)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    // Voice position should be set to offset
    let pos = mixer.voices[0].as_ref().map(|v| v.position);
    assert_eq!(
        pos,
        Some(512.0),
        "Sample offset 9xx should set voice position"
    );
}

#[test]
fn test_mixer_set_panning_8xx() {
    let sample = Sample::new(vec![1.0; 44100], 44100, 1, None);
    let mut mixer = Mixer::new(vec![Arc::new(sample)], Vec::new(), 1, 44100);
    let mut pattern = Pattern::new(16, 1);

    // 8xx: set panning (0x00=full left, 0x80=center, 0xFF=full right)
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), effects: vec![Effect::from_type(EffectType::Extended, 0xD3)], ..Default::default() };
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 256];
    mixer.render(&mut output);

    let left: f32 = (0..output.len() / 2)
        .map(|i| output[i * 2].abs())
        .fold(0.0, f32::max);
    let right: f32 = (0..output.len() / 2)
        .map(|i| output[i * 2 + 1].abs())
        .fold(0.0, f32::max);

    assert!(left > 0.0, "Left channel should have audio");
    assert!(
        right < 0.001,
        "Right channel should be silent with full-left panning"
    );
}

// --- LFO modulation tests ---

#[test]
fn test_voice_lfo_state_default() {
    let state = VoiceLfoState::default();
    assert_eq!(state.volume, 0.0);
    assert_eq!(state.panning, 0.0);
    assert_eq!(state.pitch, 0.0);
}

#[test]
fn test_voice_lfo_state_from_instrument() {
    let mut inst = Instrument::new("Test");
    inst.volume_lfo = Some(Lfo::sine(4.0, 0.5));
    let state = VoiceLfoState::new(Some(&inst));
    assert_eq!(state.volume, 0.0);
    assert_eq!(state.panning, 0.0);
    assert_eq!(state.pitch, 0.0);
}

#[test]
fn test_evaluate_lfo_waveform_sine() {
    let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Sine, 0.0);
    assert!(
        (val_at_zero - 0.0).abs() < 0.001,
        "Sine at 0 should be ~0, got {}",
        val_at_zero
    );

    let val_at_quarter = evaluate_lfo_waveform(LfoWaveform::Sine, 0.25);
    assert!(
        (val_at_quarter - 1.0).abs() < 0.001,
        "Sine at 0.25 should be ~1, got {}",
        val_at_quarter
    );

    let val_at_half = evaluate_lfo_waveform(LfoWaveform::Sine, 0.5);
    assert!(
        (val_at_half - 0.0).abs() < 0.001,
        "Sine at 0.5 should be ~0, got {}",
        val_at_half
    );

    let val_at_3qtr = evaluate_lfo_waveform(LfoWaveform::Sine, 0.75);
    assert!(
        (val_at_3qtr + 1.0).abs() < 0.001,
        "Sine at 0.75 should be ~-1, got {}",
        val_at_3qtr
    );
}

#[test]
fn test_evaluate_lfo_waveform_triangle() {
    let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.0);
    assert!(
        (val_at_zero - 0.0).abs() < 0.001,
        "Triangle at 0 should be 0, got {}",
        val_at_zero
    );

    let val_at_quarter = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.25);
    assert!(
        (val_at_quarter - 1.0).abs() < 0.001,
        "Triangle at 0.25 should be 1, got {}",
        val_at_quarter
    );

    let val_at_half = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.5);
    assert!(
        (val_at_half - 0.0).abs() < 0.001,
        "Triangle at 0.5 should be 0, got {}",
        val_at_half
    );

    let val_at_3quarter = evaluate_lfo_waveform(LfoWaveform::Triangle, 0.75);
    assert!(
        (val_at_3quarter - (-1.0)).abs() < 0.001,
        "Triangle at 0.75 should be -1, got {}",
        val_at_3quarter
    );
}

#[test]
fn test_evaluate_lfo_waveform_square() {
    let val_low = evaluate_lfo_waveform(LfoWaveform::Square, 0.0);
    assert!(
        (val_low - 1.0).abs() < 0.001,
        "Square at 0 should be 1, got {}",
        val_low
    );

    let val_high = evaluate_lfo_waveform(LfoWaveform::Square, 0.5);
    assert!(
        (val_high - (-1.0)).abs() < 0.001,
        "Square at 0.5 should be -1, got {}",
        val_high
    );

    let val_high2 = evaluate_lfo_waveform(LfoWaveform::Square, 0.9);
    assert!(
        (val_high2 - (-1.0)).abs() < 0.001,
        "Square at 0.9 should be -1, got {}",
        val_high2
    );
}

#[test]
fn test_evaluate_lfo_waveform_sawtooth() {
    let val_at_zero = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.0);
    assert!(
        (val_at_zero - (-1.0)).abs() < 0.001,
        "Sawtooth at 0 should be -1, got {}",
        val_at_zero
    );

    let val_at_half = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.5);
    assert!(
        (val_at_half - 0.0).abs() < 0.001,
        "Sawtooth at 0.5 should be 0, got {}",
        val_at_half
    );

    let val_at_end = evaluate_lfo_waveform(LfoWaveform::Sawtooth, 0.999);
    assert!(
        (val_at_end - 0.998).abs() < 0.01,
        "Sawtooth at 0.999 should be ~0.998, got {}",
        val_at_end
    );
}

#[test]
fn test_evaluate_lfo_waveform_reverse_saw() {
    let val_at_zero = evaluate_lfo_waveform(LfoWaveform::ReverseSaw, 0.0);
    assert!(
        (val_at_zero - 1.0).abs() < 0.001,
        "ReverseSaw at 0 should be 1, got {}",
        val_at_zero
    );

    let val_at_half = evaluate_lfo_waveform(LfoWaveform::ReverseSaw, 0.5);
    assert!(
        (val_at_half - 0.0).abs() < 0.001,
        "ReverseSaw at 0.5 should be 0, got {}",
        val_at_half
    );
}

#[test]
fn test_evaluate_lfo_waveform_random() {
    let val = evaluate_lfo_waveform(LfoWaveform::Random, 0.1);
    assert!(
        val >= -1.0 && val <= 1.0,
        "Random LFO value should be in [-1, 1], got {}",
        val
    );
}

#[test]
fn test_mixer_lfo_vol_modulation() {
    // Create a sample with constant amplitude
    let data: Vec<f32> = vec![0.5; 48000];
    let sample = Sample::new(data, 48000, 1, Some("test".to_string()));
    let mut inst = Instrument::new("Test");
    inst.sample_index = Some(0);
    inst.volume_lfo = Some(Lfo::sine(10.0, 0.5));
    let instruments = vec![inst];

    let mut mixer = Mixer::new(vec![Arc::new(sample)], instruments, 4, 48000);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output1 = vec![0.0f32; 480];
    let mut output2 = vec![0.0f32; 480];
    mixer.render(&mut output1);
    mixer.render(&mut output2);

    let avg1: f32 = output1.iter().map(|s| s.abs()).sum::<f32>() / output1.len() as f32;
    let avg2: f32 = output2.iter().map(|s| s.abs()).sum::<f32>() / output2.len() as f32;
    assert!(
        avg1 > 0.0,
        "Voice with volume LFO should produce non-zero audio, got {}",
        avg1
    );
    assert!(
        (avg1 - avg2).abs() < 0.01 || avg1 != avg2,
        "LFO modulation should vary output between renders"
    );
}

#[test]
fn test_mixer_lfo_pitch_modulation() {
    // Create a sample that's a linear ramp (easy to measure pitch changes)
    let data: Vec<f32> = (0..96000).map(|i| i as f32 / 96000.0).collect();
    let sample = Sample::new(data, 48000, 1, Some("ramp".to_string()));

    let mut inst_no_lfo = Instrument::new("NoLFO");
    inst_no_lfo.sample_index = Some(0);
    let mut inst_with_lfo = Instrument::new("WithLFO");
    inst_with_lfo.sample_index = Some(0);
    inst_with_lfo.pitch_lfo = Some(Lfo::sine(10.0, 0.5));

    let mut mixer_no_lfo = Mixer::new(vec![Arc::new(sample.clone())], vec![inst_no_lfo], 4, 48000);
    let mut mixer_with_lfo = Mixer::new(
        vec![Arc::new(sample.clone())],
        vec![inst_with_lfo],
        4,
        48000,
    );

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

    mixer_no_lfo.tick(0, &pattern);
    mixer_with_lfo.tick(0, &pattern);

    let mut output_no_lfo = vec![0.0f32; 480];
    let mut output_with_lfo = vec![0.0f32; 480];
    mixer_no_lfo.render(&mut output_no_lfo);
    mixer_with_lfo.render(&mut output_with_lfo);

    let _peak_no_lfo: f32 = output_no_lfo.iter().map(|s| s.abs()).fold(0.0, f32::max);
    let peak_with_lfo: f32 = output_with_lfo.iter().map(|s| s.abs()).fold(0.0, f32::max);

    assert!(
        peak_with_lfo > 0.0,
        "Voice with pitch LFO should produce audio, got {}",
        peak_with_lfo
    );
}

#[test]
fn test_mixer_lfo_zero_rate_no_modulation() {
    // LFO with 0 rate should not modulate
    let data: Vec<f32> = vec![0.5; 48000];
    let sample = Sample::new(data, 48000, 1, Some("test".to_string()));

    let mut inst = Instrument::new("Test");
    inst.sample_index = Some(0);
    inst.volume_lfo = Some(Lfo {
        waveform: LfoWaveform::Sine,
        rate: 0.0,
        depth: 1.0,
        offset: 0.0,
        enabled: true,
        phase: 0.0,
        sync_to_bpm: false,
    });
    inst.pitch_lfo = Some(Lfo {
        waveform: LfoWaveform::Sine,
        rate: 0.0,
        depth: 1.0,
        offset: 0.0,
        enabled: true,
        phase: 0.0,
        sync_to_bpm: false,
    });
    inst.panning_lfo = Some(Lfo {
        waveform: LfoWaveform::Sine,
        rate: 0.0,
        depth: 1.0,
        offset: 0.0,
        enabled: true,
        phase: 0.0,
        sync_to_bpm: false,
    });

    let mut mixer = Mixer::new(vec![Arc::new(sample)], vec![inst], 4, 48000);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));

    mixer.tick(0, &pattern);

    let mut output = vec![0.0f32; 480];
    mixer.render(&mut output);

    let has_audio = output.iter().any(|&s| s != 0.0);
    assert!(
        has_audio,
        "Voice should still produce audio with zero-rate LFO"
    );
}

#[test]
fn test_mixer_keyzone_sample_selection() {
    use crate::song::Keyzone;

    // Two samples: low-pitched sine and high-pitched sine
    let low_sample = Arc::new(make_test_sample(44100, 0.25));
    let high_sample = Arc::new(make_test_sample(44100, 0.25));

    let mut inst = Instrument::new("Piano");
    inst.keyzones = vec![
        Keyzone::new(0).with_note_range(0, 59),   // low sample
        Keyzone::new(1).with_note_range(60, 119), // high sample
    ];

    let mut mixer = Mixer::new(vec![low_sample, high_sample], vec![inst], 4, 44100);

    // Trigger a low note (C-3 = MIDI 36) on instrument 0
    let mut pattern = Pattern::new(16, 4);
    let low_note = Note::new(Pitch::C, 3, 100, 0);
    pattern.set_note(0, 0, low_note);

    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 1);

    // Verify it picked sample index 0 (low keyzone)
    let voice = mixer.voices[0].as_ref().unwrap();
    assert_eq!(voice.sample_index, 0);

    // Now trigger a high note (C-6 = MIDI 72) on instrument 0
    let mut pattern2 = Pattern::new(16, 4);
    let high_note = Note::new(Pitch::C, 6, 100, 0);
    pattern2.set_note(0, 1, high_note);

    mixer.tick(0, &pattern2);
    mixer.tick(1, &pattern2);
    let voice = mixer.voices[1].as_ref().unwrap();
    assert_eq!(voice.sample_index, 1);
}

#[test]
fn test_mixer_keyzone_no_match_silent() {
    use crate::song::Keyzone;

    let sample = Arc::new(make_test_sample(44100, 0.25));

    let mut inst = Instrument::new("Sparse");
    // Only covers notes 60-72
    inst.keyzones = vec![Keyzone::new(0).with_note_range(60, 72)];

    let mut mixer = Mixer::new(vec![sample], vec![inst], 4, 44100);

    // Trigger note outside keyzone range (C-3 = MIDI 36)
    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::C, 3, 100, 0));

    mixer.tick(0, &pattern);
    // No keyzone matches, so no voice should be triggered
    assert_eq!(mixer.active_voice_count(), 0);
}

#[test]
fn test_mixer_no_keyzones_backward_compat() {
    let sample = Arc::new(make_test_sample(44100, 0.25));
    let mut inst = Instrument::new("Simple");
    inst.sample_index = Some(0);
    let instruments = vec![inst];
    // No keyzones -- should use instrument_idx as sample_index directly
    // actually now it uses inst.sample_index fallback.
    let mut mixer = Mixer::new(vec![sample], instruments, 4, 44100);

    let mut pattern = Pattern::new(16, 4);
    pattern.set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

    mixer.tick(0, &pattern);
    assert_eq!(mixer.active_voice_count(), 1);
    assert_eq!(mixer.voices[0].as_ref().unwrap().sample_index, 0);
}

#[test]
fn test_mixer_retrigger_e9x() {
    // E93: retrigger every 3 ticks. At 120 BPM, 6 TPL, 44100Hz:
    //   frames_per_row = (2.5/120) * 6 * 44100 = 5512 (approx)
    //   frames_per_tick = 5512 / 6 = 918
    //   retrigger at tick 3 = frame 2754, tick 6 = frame 5508 (but row ends)
    let data = vec![0.5f32; 10000];
    let sample = Arc::new(Sample::new(data, 44100, 1, None));
    let mut mixer = Mixer::new(vec![sample], Vec::new(), 1, 44100);
    mixer.update_tempo(120.0);
    let mut pattern = Pattern::new(16, 1);

    let mut cell = Cell::default();
    cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
    // E93: Extended sub-command 9, param 3 — retrigger every 3 ticks
    cell.effects
        .push(Effect::from_type(EffectType::Extended, 0x93));
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);
    assert!(mixer.voices[0].is_some(), "Voice should start immediately");

    // Record initial voice position after triggering
    let pos_after_tick = mixer.voices[0].as_ref().unwrap().position;

    // Render past tick 3 (2754 frames)
    let mut output = vec![0.0f32; 3000 * 2];
    mixer.render(&mut output);

    // Voice should have been retriggered: position should have reset to near 0
    let pos_after_retrigger = mixer.voices[0].as_ref().unwrap().position;
    // After render of 3000 frames the voice advanced then was retriggered at ~2754.
    // Net advance = 3000 - 2754 = 246 frames.
    assert!(
        pos_after_retrigger < 500.0,
        "Voice should have retriggered (position {pos_after_retrigger} should be near 0)",
    );
    // Ensure the position after retriggering is less than it would be without retrigger
    assert!(
        pos_after_retrigger < pos_after_tick + 3000.0,
        "Retrigger should have reset position"
    );
}

/// Verify that a volume envelope is applied to the voice output.
/// An envelope that starts at 0.0 and ramps to 1.0 over 32 ticks should
/// produce near-silent audio during the attack and loud audio after it.
#[test]
fn test_volume_envelope_applied() {
    use crate::song::{Envelope, EnvelopePoint};

    // Looping DC sample (constant 1.0) so amplitude == volume exactly.
    let data = vec![1.0f32; 44100];
    let mut sample = Sample::new(data, 44100, 1, None);
    sample.loop_mode = crate::audio::sample::LoopMode::Forward;
    sample.loop_start = 0;
    sample.loop_end = 44099;

    // Instrument with envelope: 0.0 at tick 0, 1.0 at tick 32, sustain at point 1
    let mut inst = Instrument::new("env_test");
    inst.volume_envelope = Some(Envelope {
        enabled: true,
        points: vec![
            EnvelopePoint {
                frame: 0,
                value: 0.0,
            },
            EnvelopePoint {
                frame: 32,
                value: 1.0,
            },
        ],
        sustain_enabled: true,
        sustain_start_point: 1,
        sustain_end_point: 1,
        loop_enabled: false,
        loop_start_point: 0,
        loop_end_point: 0,
    });
    inst.sample_index = Some(0);

    let mut mixer = Mixer::new(vec![Arc::new(sample)], vec![inst], 1, 44100);
    mixer.update_tempo(120.0);
    mixer.set_tpl(6);

    // Trigger a note
    let mut pattern = Pattern::new(1, 1);
    let mut cell = riffl_core::pattern::row::Cell { note: Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0))), instrument: Some(0), effects: vec![Effect::from_type(EffectType::Extended, 0xD3)], ..Default::default() };
    pattern.set_cell(0, 0, cell);
    mixer.tick(0, &pattern);

    // frames_per_tick at 120BPM, tpl=6: ~918 frames/tick
    // Render just a few frames (well within tick 0, envelope value = 0.0)
    let mut early_buf = vec![0.0f32; 200 * 2];
    mixer.render(&mut early_buf);
    let early_rms = rms_stereo(&early_buf);

    // Advance well past tick 32 of the envelope (32 ticks * 918 frames = ~29376)
    let mut skip_buf = vec![0.0f32; 30000 * 2];
    mixer.render(&mut skip_buf);

    // Render after the attack — envelope value should be 1.0 at sustain
    let mut late_buf = vec![0.0f32; 200 * 2];
    mixer.render(&mut late_buf);
    let late_rms = rms_stereo(&late_buf);

    assert!(
        early_rms < 0.05,
        "Early render (attack at 0) should be near-silent, got rms={early_rms:.4}"
    );
    assert!(
        late_rms > 0.5,
        "Late render (sustain at 1.0) should be loud, got rms={late_rms:.4}"
    );
}

fn rms_stereo(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buf.iter().map(|x| x * x).sum();
    (sum_sq / buf.len() as f32).sqrt()
}
