use riffl_core::audio::mixer::Mixer;
use riffl_core::audio::sample::{LoopMode, Sample};
use riffl_core::pattern::note::NoteEvent;
use riffl_core::pattern::note::{Note, Pitch};
use riffl_core::pattern::pattern::Pattern;
use riffl_core::pattern::row::Cell;
use riffl_core::song::{Envelope, EnvelopePoint, Instrument};
use std::sync::Arc;

#[test]
fn test_volume_envelope_sustain_looping_repro() {
    let data = vec![1.0f32; 1000];
    let mut sample = Sample::new(data, 44100, 1, None);
    sample.loop_mode = LoopMode::Forward;
    sample.loop_start = 0;
    sample.loop_end = 999;

    let mut inst = Instrument::new("looping_env");
    inst.sample_index = Some(0);
    inst.volume_envelope = Some(Envelope {
        enabled: true,
        points: vec![
            EnvelopePoint {
                frame: 0,
                value: 1.0,
            },
            EnvelopePoint {
                frame: 2,
                value: 0.0,
            },
        ],
        sustain_enabled: true,
        sustain_start_point: 0,
        sustain_end_point: 1, // Loop from Point 0 to 1 (Tick 0 to 2)
        loop_enabled: false,
        loop_start_point: 0,
        loop_end_point: 0,
    });

    let mut mixer = Mixer::new(vec![Arc::new(sample)], vec![inst], 1, 44100);
    mixer.update_tempo(125.0);
    mixer.set_tpl(6);

    let mut pattern = Pattern::new(1, 1);
    let mut cell = Cell::default();
    cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
    cell.instrument = Some(0);
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    let mut buf = vec![0.0f32; 882 * 2];

    mixer.render(&mut buf);
    let rms0 = rms_stereo(&buf);
    println!("Tick 0 RMS: {}", rms0);

    mixer.render(&mut buf);
    let rms1 = rms_stereo(&buf);
    println!("Tick 1 RMS: {}", rms1);

    mixer.render(&mut buf);
    let rms2 = rms_stereo(&buf);
    println!("Tick 2 RMS: {}", rms2);

    mixer.render(&mut buf);
    let rms3 = rms_stereo(&buf);
    println!("Tick 3 RMS: {}", rms3);

    // Loop points: Pos 0 at tick 0 (val 1.0), Pos 1 at tick 2 (val 0.0).
    // Tick 0: 1.0 -> 0.75 (Avg ~0.87). RMS ~0.54
    assert!(rms0 > 0.5);
    // Tick 1: 0.75 -> 0.5 (Avg ~0.62). RMS ~0.20
    assert!(rms1 > 0.15 && rms1 < 0.25);
    // Tick 2: Jumps from 0.5 back to start (1.0). Interpolates 0.5 -> 1.0. RMS ~0.40
    assert!(rms2 > 0.35 && rms2 < 0.45);
    // Tick 3: Start of new cycle (same as Tick 0)
    assert!(rms3 > 0.5);
}

#[test]
fn test_fast_volume_envelope_looping() {
    let data = vec![1.0f32; 1000];
    let mut sample = Sample::new(data, 44100, 1, None);
    sample.loop_mode = LoopMode::Forward;
    sample.loop_start = 0;
    sample.loop_end = 999;

    let mut inst = Instrument::new("fast_loop_env");
    inst.sample_index = Some(0);
    inst.volume_envelope = Some(Envelope {
        enabled: true,
        points: vec![
            EnvelopePoint {
                frame: 0,
                value: 1.0,
            },
            EnvelopePoint {
                frame: 1,
                value: 0.0,
            },
        ],
        sustain_enabled: true,
        sustain_start_point: 0,
        sustain_end_point: 1,
        loop_enabled: false,
        loop_start_point: 0,
        loop_end_point: 0,
    });

    let mut mixer = Mixer::new(vec![Arc::new(sample)], vec![inst], 1, 44100);
    mixer.update_tempo(125.0);
    mixer.set_tpl(6);

    let mut pattern = Pattern::new(1, 1);
    let mut cell = Cell::default();
    cell.note = Some(NoteEvent::On(Note::new(Pitch::C, 4, 127, 0)));
    cell.instrument = Some(0);
    pattern.set_cell(0, 0, cell);

    mixer.tick(0, &pattern);

    let mut buf = vec![0.0f32; 882 * 2];

    mixer.render(&mut buf);
    let rms0 = rms_stereo(&buf);

    mixer.render(&mut buf);
    let rms1 = rms_stereo(&buf);

    mixer.render(&mut buf);
    let rms2 = rms_stereo(&buf);

    println!("RMS: {}, {}, {}", rms0, rms1, rms2);

    // Cycle is exactly 2 ticks.
    // Tick 0: 1.0 -> 0.0. RMS ~0.41
    // Tick 1: 0.0 -> 1.0. RMS ~0.41
    assert!(rms0 > 0.35 && rms0 < 0.45);
    assert!(rms1 > 0.35 && rms1 < 0.45);
    assert!(rms2 > 0.35 && rms2 < 0.45);
}

fn rms_stereo(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = buf.iter().map(|x| x * x).sum();
    (sum_sq / buf.len() as f32).sqrt()
}
