//! Audio export functionality for rendering songs to WAV files.
//!
//! Provides offline rendering of the full song arrangement through the mixer,
//! writing the output to a WAV file using the `hound` crate.

use std::path::Path;

use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};

use std::sync::Arc;

use crate::audio::mixer::Mixer;
use crate::audio::sample::Sample;
use crate::song::Song;

/// Supported bit depths for WAV export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitDepth {
    /// 16-bit integer samples.
    Bits16,
    /// 24-bit integer samples.
    Bits24,
}

impl BitDepth {
    /// Get the bits per sample for this bit depth.
    pub fn bits_per_sample(self) -> u16 {
        match self {
            BitDepth::Bits16 => 16,
            BitDepth::Bits24 => 24,
        }
    }
}

/// Configuration for WAV export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Output sample rate in Hz (e.g., 44100 or 48000).
    pub sample_rate: u32,
    /// Bit depth for the output WAV file.
    pub bit_depth: BitDepth,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            bit_depth: BitDepth::Bits16,
        }
    }
}

/// Rows per beat constant (same as transport.rs).
const ROWS_PER_BEAT: f64 = 4.0;

/// Export a song to a WAV file.
///
/// Performs offline rendering by processing the entire song arrangement
/// row-by-row through the mixer, then writing the resulting audio to a
/// WAV file.
///
/// # Arguments
/// * `path` - Output file path for the WAV file
/// * `song` - The song to render (patterns, arrangement, BPM)
/// * `samples` - Loaded audio samples referenced by instruments
/// * `config` - Export configuration (sample rate, bit depth)
/// * `progress` - Optional callback receiving progress as a percentage (0.0 to 1.0)
///
/// # Returns
/// `Ok(())` on success, or an error if rendering or writing fails.
pub fn export_wav<F>(
    path: &Path,
    song: &Song,
    samples: &[Arc<Sample>],
    config: &ExportConfig,
    mut progress: F,
) -> Result<()>
where
    F: FnMut(f32),
{
    let num_channels = song.patterns.first().map_or(8, |p| p.num_channels());
    let mut mixer = Mixer::new(samples.to_vec(), num_channels, config.sample_rate);
    mixer.update_tempo(song.bpm);

    // Calculate frames per row based on BPM
    let seconds_per_row = 60.0 / song.bpm / ROWS_PER_BEAT;
    let frames_per_row = (seconds_per_row * config.sample_rate as f64).round() as usize;

    // Stereo interleaved buffer for one row of audio
    let mut row_buffer = vec![0.0f32; frames_per_row * 2];

    // Calculate total rows for progress reporting
    let total_rows: usize = song
        .arrangement
        .iter()
        .map(|&pat_idx| song.patterns.get(pat_idx).map_or(0, |p| p.num_rows()))
        .sum();

    if total_rows == 0 {
        // Nothing to render — write an empty WAV
        let spec = wav_spec(config);
        WavWriter::create(path, spec).context("Failed to create WAV file")?;
        progress(1.0);
        return Ok(());
    }

    // Create WAV writer
    let spec = wav_spec(config);
    let mut writer = WavWriter::create(path, spec).context("Failed to create WAV file")?;

    let mut rows_rendered: usize = 0;

    // Process each pattern in the arrangement
    for &pattern_idx in &song.arrangement {
        let pattern = match song.patterns.get(pattern_idx) {
            Some(p) => p,
            None => continue,
        };

        let num_rows = pattern.num_rows();

        for row in 0..num_rows {
            // Process the row (trigger notes, apply effects)
            let _transport_cmds = mixer.tick(row, pattern);

            // Render audio for this row
            row_buffer.iter_mut().for_each(|s| *s = 0.0);
            mixer.render(&mut row_buffer);

            // Write samples to WAV
            match config.bit_depth {
                BitDepth::Bits16 => {
                    for &sample in &row_buffer {
                        let scaled = (sample * i16::MAX as f32) as i16;
                        writer
                            .write_sample(scaled)
                            .context("Failed to write 16-bit sample")?;
                    }
                }
                BitDepth::Bits24 => {
                    for &sample in &row_buffer {
                        let scaled = (sample * 8_388_607.0) as i32; // 2^23 - 1
                        writer
                            .write_sample(scaled)
                            .context("Failed to write 24-bit sample")?;
                    }
                }
            }

            rows_rendered += 1;
            progress(rows_rendered as f32 / total_rows as f32);
        }
    }

    writer.finalize().context("Failed to finalize WAV file")?;
    Ok(())
}

/// Build a `WavSpec` from the export configuration.
fn wav_spec(config: &ExportConfig) -> WavSpec {
    WavSpec {
        channels: 2, // Stereo output
        sample_rate: config.sample_rate,
        bits_per_sample: config.bit_depth.bits_per_sample(),
        sample_format: SampleFormat::Int,
    }
}

/// Calculate the expected duration of a song in seconds.
pub fn song_duration(song: &Song) -> f64 {
    let seconds_per_row = 60.0 / song.bpm / ROWS_PER_BEAT;
    let total_rows: usize = song
        .arrangement
        .iter()
        .map(|&pat_idx| song.patterns.get(pat_idx).map_or(0, |p| p.num_rows()))
        .sum();
    total_rows as f64 * seconds_per_row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::sample::Sample;
    use crate::pattern::note::{Note, Pitch};
    use crate::pattern::pattern::Pattern;
    use crate::song::Song;
    /// Create a simple sine wave test sample at 440Hz.
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

    fn temp_wav_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("tracker_rs_tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn test_export_wav_creates_valid_file() {
        let path = temp_wav_path("test_export_basic.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Test", 120.0);
        // Add a note to the first row
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig::default();
        let mut progress_values = Vec::new();

        export_wav(&path, &song, &[std::sync::Arc::new(sample)], &config, |p| {
            progress_values.push(p);
        })
        .unwrap();

        // Verify file exists and is a valid WAV
        assert!(path.exists());
        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, SampleFormat::Int);

        // Progress should reach 1.0
        assert!(!progress_values.is_empty());
        let last = *progress_values.last().unwrap();
        assert!(
            (last - 1.0).abs() < 0.01,
            "Final progress should be ~1.0, got {}",
            last
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_correct_duration() {
        let path = temp_wav_path("test_export_duration.wav");
        let sample = make_test_sample(44100, 2.0);

        let mut song = Song::new("Test", 120.0);
        // Default: 1 pattern, 64 rows, BPM 120, 4 rows/beat
        // Duration = 64 rows * (60 / 120 / 4) = 64 * 0.125 = 8 seconds
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig {
            sample_rate: 44100,
            bit_depth: BitDepth::Bits16,
        };

        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        let num_samples = reader.len();
        let num_frames = num_samples / spec.channels as u32;
        let duration = num_frames as f64 / spec.sample_rate as f64;

        // Should be ~8 seconds
        assert!(
            (duration - 8.0).abs() < 0.1,
            "Expected ~8s duration, got {}s",
            duration
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_silence_produces_near_zero() {
        let path = temp_wav_path("test_export_silence.wav");
        let sample = make_test_sample(44100, 1.0);

        // Song with no notes (all cells empty)
        let song = Song::new("Silent", 120.0);

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let max_abs: i16 = reader
            .samples::<i16>()
            .map(|s| s.unwrap().abs())
            .max()
            .unwrap_or(0);

        assert_eq!(max_abs, 0, "Silent song should produce zero samples");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_with_notes_produces_audio() {
        let path = temp_wav_path("test_export_audio.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Audio", 120.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 127, 0));
        song.patterns[0].set_note(4, 1, Note::new(Pitch::C, 4, 100, 0));

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let max_abs: i16 = reader
            .samples::<i16>()
            .map(|s| s.unwrap().abs())
            .max()
            .unwrap_or(0);

        assert!(
            max_abs > 0,
            "Song with notes should produce non-zero audio, got max_abs={}",
            max_abs
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_48khz() {
        let path = temp_wav_path("test_export_48k.wav");
        let sample = make_test_sample(48000, 1.0);

        let mut song = Song::new("48k", 120.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig {
            sample_rate: 48000,
            bit_depth: BitDepth::Bits16,
        };

        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().sample_rate, 48000);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_24bit() {
        let path = temp_wav_path("test_export_24bit.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("24bit", 120.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig {
            sample_rate: 44100,
            bit_depth: BitDepth::Bits24,
        };

        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        assert_eq!(reader.spec().bits_per_sample, 24);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_config_default() {
        let config = ExportConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.bit_depth, BitDepth::Bits16);
    }

    #[test]
    fn test_bit_depth_bits_per_sample() {
        assert_eq!(BitDepth::Bits16.bits_per_sample(), 16);
        assert_eq!(BitDepth::Bits24.bits_per_sample(), 24);
    }

    #[test]
    fn test_song_duration_calculation() {
        let song = Song::new("Test", 120.0);
        // Default: 1 pattern, 64 rows, 120 BPM, 4 rows/beat
        // Duration = 64 * (60 / 120 / 4) = 64 * 0.125 = 8.0s
        let dur = song_duration(&song);
        assert!((dur - 8.0).abs() < 0.001, "Expected 8.0s, got {}", dur);
    }

    #[test]
    fn test_song_duration_multiple_patterns() {
        let mut song = Song::new("Multi", 120.0);
        song.add_pattern(Pattern::new(32, 8)); // Pattern 1: 32 rows
        song.arrangement = vec![0, 1]; // 64 + 32 = 96 rows

        // Duration = 96 * 0.125 = 12.0s
        let dur = song_duration(&song);
        assert!((dur - 12.0).abs() < 0.001, "Expected 12.0s, got {}", dur);
    }

    #[test]
    fn test_export_wav_progress_monotonic() {
        let path = temp_wav_path("test_export_progress.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Test", 120.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig::default();
        let mut progress_values = Vec::new();

        export_wav(&path, &song, &[std::sync::Arc::new(sample)], &config, |p| {
            progress_values.push(p);
        })
        .unwrap();

        // Progress should be monotonically increasing
        for window in progress_values.windows(2) {
            assert!(
                window[1] >= window[0],
                "Progress should be monotonic: {} -> {}",
                window[0],
                window[1]
            );
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_multi_arrangement() {
        let path = temp_wav_path("test_export_multi_arr.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Multi", 120.0);
        // Pattern 0: 64 rows with a note
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        // Pattern 1: 32 rows
        let mut pat1 = Pattern::new(32, 8);
        pat1.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        song.add_pattern(pat1);
        song.arrangement = vec![0, 1];

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        let num_frames = reader.len() / 2; // stereo
        let duration = num_frames as f64 / 44100.0;

        // 64 + 32 = 96 rows * 0.125s = 12.0s
        assert!(
            (duration - 12.0).abs() < 0.1,
            "Expected ~12s, got {}s",
            duration
        );

        std::fs::remove_file(&path).ok();
    }

    // === Additional audio export tests (Phase-05 task 7) ===

    #[test]
    fn test_export_wav_valid_file_readable_by_hound() {
        // Verify WAV is structurally valid: hound can open and iterate all samples
        let path = temp_wav_path("test_export_valid_structure.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Valid", 120.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        song.patterns[0].set_note(8, 0, Note::new(Pitch::E, 4, 100, 0));
        song.patterns[0].set_note(16, 0, Note::new(Pitch::G, 4, 100, 0));

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        // Reading all samples should not error (proves file integrity)
        let mut reader = hound::WavReader::open(&path).unwrap();
        let sample_count: usize = reader.samples::<i16>().map(|s| s.unwrap()).count();
        assert!(sample_count > 0, "WAV should contain samples");

        // Sample count should be even (stereo interleaved)
        assert_eq!(
            sample_count % 2,
            0,
            "Stereo WAV must have even sample count"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_metadata_all_fields_correct() {
        // Comprehensive metadata check: sample rate, channels, bits, format, and duration
        let path = temp_wav_path("test_export_metadata_full.wav");
        let sample = make_test_sample(48000, 2.0);

        let mut song = Song::new("MetaCheck", 140.0);
        song.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));

        let config = ExportConfig {
            sample_rate: 48000,
            bit_depth: BitDepth::Bits24,
        };

        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();

        // All metadata fields
        assert_eq!(spec.channels, 2, "Should be stereo");
        assert_eq!(spec.sample_rate, 48000, "Sample rate should match config");
        assert_eq!(spec.bits_per_sample, 24, "Bit depth should match config");
        assert_eq!(
            spec.sample_format,
            SampleFormat::Int,
            "Format should be integer"
        );

        // Duration at 140 BPM: 64 rows * (60 / 140 / 4) = 64 * ~0.10714 ≈ 6.857s
        let num_frames = reader.len() / spec.channels as u32;
        let duration = num_frames as f64 / spec.sample_rate as f64;
        let expected = 64.0 * (60.0 / 140.0 / 4.0);
        assert!(
            (duration - expected).abs() < 0.1,
            "Expected ~{:.2}s duration at 140 BPM, got {:.2}s",
            expected,
            duration
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_silence_all_channels_empty() {
        // Verify silence across multiple patterns with no notes triggers zero output
        let path = temp_wav_path("test_export_silence_multi_pat.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("MultiSilence", 120.0);
        // Add a second empty pattern
        song.add_pattern(Pattern::new(32, 8));
        song.arrangement = vec![0, 1];

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let all_zero = reader.samples::<i16>().all(|s| s.unwrap() == 0);
        assert!(
            all_zero,
            "Empty multi-pattern song should produce only zero samples"
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_notes_produce_varying_audio() {
        // Verify that different notes on different channels produce non-zero, distinct audio
        let path = temp_wav_path("test_export_varying_audio.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Varying", 120.0);
        // Put notes on multiple channels at different rows
        song.patterns[0].set_note(0, 0, Note::new(Pitch::C, 4, 127, 0));
        song.patterns[0].set_note(0, 1, Note::new(Pitch::E, 4, 100, 0));
        song.patterns[0].set_note(16, 0, Note::new(Pitch::G, 4, 80, 0));
        song.patterns[0].set_note(32, 2, Note::new(Pitch::A, 4, 127, 0));

        let config = ExportConfig::default();
        export_wav(
            &path,
            &song,
            &[std::sync::Arc::new(sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let mut reader = hound::WavReader::open(&path).unwrap();
        let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

        let max_abs = samples.iter().map(|s| s.abs()).max().unwrap_or(0);
        assert!(
            max_abs > 100,
            "Multi-note song should produce significant audio, got max_abs={}",
            max_abs
        );

        // Verify audio isn't all the same value (not just DC offset)
        let unique_values: std::collections::HashSet<i16> = samples.iter().copied().collect();
        assert!(
            unique_values.len() > 10,
            "Audio should have variety, got only {} unique values",
            unique_values.len()
        );

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_export_wav_different_bpm_changes_duration() {
        // Verify that different BPM values produce different file durations
        let sample = std::sync::Arc::new(make_test_sample(44100, 2.0));
        let config = ExportConfig::default();

        let path_slow = temp_wav_path("test_export_bpm_slow.wav");
        let mut song_slow = Song::new("Slow", 60.0); // 60 BPM
        song_slow.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        export_wav(
            &path_slow,
            &song_slow,
            &[std::sync::Arc::clone(&sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let path_fast = temp_wav_path("test_export_bpm_fast.wav");
        let mut song_fast = Song::new("Fast", 240.0); // 240 BPM
        song_fast.patterns[0].set_note(0, 0, Note::new(Pitch::A, 4, 100, 0));
        export_wav(
            &path_fast,
            &song_fast,
            &[std::sync::Arc::clone(&sample)],
            &config,
            |_| {},
        )
        .unwrap();

        let reader_slow = hound::WavReader::open(&path_slow).unwrap();
        let reader_fast = hound::WavReader::open(&path_fast).unwrap();

        let frames_slow = reader_slow.len() / 2;
        let frames_fast = reader_fast.len() / 2;

        // 60 BPM should produce 4x more audio than 240 BPM for the same rows
        let ratio = frames_slow as f64 / frames_fast as f64;
        assert!(
            (ratio - 4.0).abs() < 0.1,
            "60 BPM should be 4x longer than 240 BPM, got ratio {:.2}",
            ratio
        );

        std::fs::remove_file(&path_slow).ok();
        std::fs::remove_file(&path_fast).ok();
    }

    #[test]
    fn test_export_wav_empty_arrangement() {
        // Song with empty arrangement should produce a valid but empty WAV
        let path = temp_wav_path("test_export_empty_arr.wav");
        let sample = make_test_sample(44100, 1.0);

        let mut song = Song::new("Empty", 120.0);
        song.arrangement = vec![]; // No patterns in arrangement

        let config = ExportConfig::default();
        let mut final_progress = 0.0f32;
        export_wav(&path, &song, &[std::sync::Arc::new(sample)], &config, |p| {
            final_progress = p;
        })
        .unwrap();

        assert!(
            path.exists(),
            "WAV file should be created even with empty arrangement"
        );
        assert!(
            (final_progress - 1.0).abs() < 0.01,
            "Progress should reach 1.0 for empty arrangement"
        );

        std::fs::remove_file(&path).ok();
    }
}
