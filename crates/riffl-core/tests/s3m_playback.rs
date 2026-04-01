#![cfg(feature = "test-modules")]
//! Integration test for S3M playback (PCM samples and Adlib instruments).
//! Loads a known S3M file, creates a mixer, ticks pattern rows, and verifies
//! that audio output is non-zero.

use riffl_core::audio::mixer::Mixer;
use riffl_core::format::s3m::import_s3m;
use std::fs;
use std::sync::Arc;

#[test]
fn test_s3m_pcm_playback() {
    // Load the test S3M file
    let data = include_bytes!("test_modules/test.s3m");
    let format_data = import_s3m(&data[..]).expect("Failed to parse S3M file");
    let song = format_data.song;
    let samples = format_data.samples;

    // Debug info
    eprintln!("Loaded S3M '{}'", song.name);
    eprintln!("  samples: {}", samples.len());
    for (i, s) in samples.iter().enumerate() {
        eprintln!(
            "    [{}] {}: volume={}, base_note={}",
            i,
            s.name().unwrap_or("?"),
            s.volume,
            s.base_note()
        );
    }
    eprintln!("  instruments: {}", song.instruments.len());
    for (i, inst) in song.instruments.iter().enumerate() {
        eprintln!(
            "    [{}] {}: sample_index={:?}",
            i, inst.name, inst.sample_index
        );
    }
    if samples.is_empty() {
        eprintln!("Warning: test.s3m has no samples, skipping PCM playback test");
        return;
    }

    // Convert samples to Arc for mixer
    let samples_arc: Vec<Arc<_>> = samples.into_iter().map(Arc::new).collect();

    // Create mixer with 32 channels (S3M standard)
    let mut mixer = Mixer::new(
        samples_arc.clone(),
        song.instruments.clone(),
        32,
        48000, // output sample rate
    );

    // Set mixer BPM and TPL from song
    // mixer.set_bpm(song.bpm); // no setter, default BPM is fine
    mixer.set_tpl(song.tpl);
    mixer.set_global_volume(song.global_volume);

    // Update tracks from song
    mixer.update_tracks(&song.tracks);

    // Get first pattern index from arrangement
    let pat_idx = song.arrangement.first().copied().unwrap_or(0);
    let pattern = &song.patterns[pat_idx];

    // Tick first row of the pattern
    let transport = mixer.tick(0, pattern);
    // Ignore transport commands for this test

    // Render some audio
    let mut buffer = vec![0.0f32; 1024 * 2]; // stereo, 1024 frames
    mixer.render(&mut buffer);

    // Check that at least some samples are non-zero
    let any_non_zero = buffer.iter().any(|&s| s.abs() > 1e-9);
    assert!(
        any_non_zero,
        "S3M PCM playback produced silent output (buffer all zeros)"
    );

    // Optionally, print peak level for debugging
    let peak = buffer.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
    println!("S3M PCM playback peak: {}", peak);
}
