//! Audio sample loading from files
//!
//! Loads audio files (WAV, FLAC, OGG/Vorbis) using symphonia and decodes
//! them into `Sample` instances ready for playback.

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::error::{AudioError, AudioResult};
use crate::audio::sample::Sample;

/// Load an audio sample from a file on disk.
///
/// Supports WAV, FLAC, and OGG/Vorbis formats. The audio data is decoded
/// to interleaved `f32` samples normalized to the -1.0..1.0 range. Mono
/// files are automatically converted to stereo by duplicating the channel.
/// If the file's sample rate differs from `target_sample_rate`, basic
/// linear interpolation resampling is applied.
///
/// The file name (without directory) is stored in `Sample.name`.
pub fn load_sample(path: &Path, target_sample_rate: u32) -> AudioResult<Sample> {
    let file = std::fs::File::open(path).map_err(|e| {
        AudioError::LoadError(format!("failed to open file '{}': {}", path.display(), e))
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AudioError::LoadError(format!("unsupported format: {}", e)))?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or_else(|| AudioError::LoadError("no audio track found".into()))?;

    let codec_params = track.codec_params.clone();
    let track_id = track.id;

    let file_sample_rate = codec_params
        .sample_rate
        .ok_or_else(|| AudioError::LoadError("unknown sample rate".into()))?;

    let file_channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::LoadError(format!("codec error: {}", e)))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => {
                return Err(AudioError::LoadError(format!("decode error: {}", e)));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| AudioError::LoadError(format!("decode error: {}", e)))?;

        let spec = *decoded.spec();
        let num_frames = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(num_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        all_samples.extend_from_slice(sample_buf.samples());
    }

    if all_samples.is_empty() {
        return Err(AudioError::LoadError("decoded audio is empty".into()));
    }

    // Convert mono to stereo by duplicating channels
    let (data, channels) = if file_channels == 1 {
        let stereo: Vec<f32> = all_samples.iter().flat_map(|&s| [s, s]).collect();
        (stereo, 2u16)
    } else {
        (all_samples, file_channels)
    };

    // Resample if needed (basic linear interpolation)
    let data = if file_sample_rate != target_sample_rate {
        resample_linear(&data, channels, file_sample_rate, target_sample_rate)
    } else {
        data
    };

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    Ok(Sample::new(data, target_sample_rate, channels, name))
}

/// Basic linear interpolation resampling.
///
/// Converts interleaved audio data from `src_rate` to `dst_rate`.
fn resample_linear(data: &[f32], channels: u16, src_rate: u32, dst_rate: u32) -> Vec<f32> {
    let ch = channels as usize;
    let src_frames = data.len() / ch;
    let ratio = src_rate as f64 / dst_rate as f64;
    let dst_frames = ((src_frames as f64) / ratio).ceil() as usize;

    let mut out = Vec::with_capacity(dst_frames * ch);

    for frame_idx in 0..dst_frames {
        let src_pos = frame_idx as f64 * ratio;
        let src_idx = src_pos as usize;
        let frac = (src_pos - src_idx as f64) as f32;

        let idx_a = src_idx.min(src_frames - 1);
        let idx_b = (src_idx + 1).min(src_frames - 1);

        for c in 0..ch {
            let a = data[idx_a * ch + c];
            let b = data[idx_b * ch + c];
            out.push(a + (b - a) * frac);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: write a minimal WAV file (PCM 16-bit) to a temp path and return it.
    fn write_test_wav(path: &Path, sample_rate: u32, channels: u16, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let file_size = 36 + data_len;
        let byte_rate = sample_rate * channels as u32 * 2;
        let block_align = channels * 2;

        let mut f = std::fs::File::create(path).unwrap();
        // RIFF header
        f.write_all(b"RIFF").unwrap();
        f.write_all(&file_size.to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        // fmt chunk
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap(); // chunk size
        f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        f.write_all(&channels.to_le_bytes()).unwrap();
        f.write_all(&sample_rate.to_le_bytes()).unwrap();
        f.write_all(&byte_rate.to_le_bytes()).unwrap();
        f.write_all(&block_align.to_le_bytes()).unwrap();
        f.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample

        // data chunk
        f.write_all(b"data").unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        for &s in samples {
            f.write_all(&s.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn test_load_stereo_wav() {
        let dir = std::env::temp_dir().join("tracker_rs_test_stereo");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("stereo.wav");

        // 4 stereo frames at 44100 Hz
        let samples: Vec<i16> = vec![1000, -1000, 2000, -2000, 3000, -3000, 4000, -4000];
        write_test_wav(&path, 44100, 2, &samples);

        let sample = load_sample(&path, 44100).unwrap();
        assert_eq!(sample.channels(), 2);
        assert_eq!(sample.sample_rate(), 44100);
        assert_eq!(sample.frame_count(), 4);
        assert_eq!(sample.name(), Some("stereo.wav"));

        // Verify data is normalized f32
        for &v in sample.data() {
            assert!(v >= -1.0 && v <= 1.0, "sample value {} out of range", v);
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_mono_converts_to_stereo() {
        let dir = std::env::temp_dir().join("tracker_rs_test_mono");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mono.wav");

        // 4 mono frames
        let samples: Vec<i16> = vec![1000, 2000, 3000, 4000];
        write_test_wav(&path, 44100, 1, &samples);

        let sample = load_sample(&path, 44100).unwrap();
        assert_eq!(sample.channels(), 2, "mono should be converted to stereo");
        assert_eq!(sample.frame_count(), 4, "frame count should stay the same");
        // Each mono sample should be duplicated: L=R
        let data = sample.data();
        for frame in 0..4 {
            let l = data[frame * 2];
            let r = data[frame * 2 + 1];
            assert!(
                (l - r).abs() < 1e-6,
                "L and R should be equal for mono->stereo, frame {}",
                frame
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_missing_file() {
        let result = load_sample(Path::new("/nonexistent/file.wav"), 48000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            AudioError::LoadError(msg) => {
                assert!(msg.contains("failed to open"), "error: {}", msg);
            }
            other => panic!("expected LoadError, got: {:?}", other),
        }
    }

    #[test]
    fn test_load_invalid_file() {
        let dir = std::env::temp_dir().join("tracker_rs_test_invalid");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("garbage.wav");
        std::fs::write(&path, b"this is not audio data").unwrap();

        let result = load_sample(&path, 48000);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_with_correct_metadata() {
        let dir = std::env::temp_dir().join("tracker_rs_test_meta");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test_meta.wav");

        // 100 stereo frames at 48000 Hz
        let samples: Vec<i16> = (0..200).map(|i| (i * 100) as i16).collect();
        write_test_wav(&path, 48000, 2, &samples);

        let sample = load_sample(&path, 48000).unwrap();
        assert_eq!(sample.sample_rate(), 48000);
        assert_eq!(sample.channels(), 2);
        assert_eq!(sample.frame_count(), 100);
        // Duration should be 100 / 48000 ≈ 0.00208 seconds
        let expected_duration = 100.0 / 48000.0;
        assert!(
            (sample.duration() - expected_duration).abs() < 1e-6,
            "duration mismatch: {} vs {}",
            sample.duration(),
            expected_duration
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_resample_changes_sample_count() {
        let dir = std::env::temp_dir().join("tracker_rs_test_resample");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("resample.wav");

        // 441 stereo frames at 44100 Hz = 0.01s of audio
        let samples: Vec<i16> = (0..882).map(|i| ((i % 100) * 300) as i16).collect();
        write_test_wav(&path, 44100, 2, &samples);

        let sample = load_sample(&path, 48000).unwrap();
        assert_eq!(sample.sample_rate(), 48000);
        assert_eq!(sample.channels(), 2);
        // After resampling 441 frames from 44100→48000, we expect ~480 frames
        let expected_frames = ((441.0_f64 * 48000.0) / 44100.0).ceil() as usize;
        assert_eq!(sample.frame_count(), expected_frames);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_mono_values_duplicated_correctly() {
        let dir = std::env::temp_dir().join("tracker_rs_test_mono_vals");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mono_vals.wav");

        // Known mono samples
        let samples: Vec<i16> = vec![16383, -16383, 0, 8191];
        write_test_wav(&path, 44100, 1, &samples);

        let sample = load_sample(&path, 44100).unwrap();
        let data = sample.data();
        assert_eq!(data.len(), 8, "4 mono frames -> 8 stereo samples");

        // Each mono sample should appear as identical L and R
        for frame in 0..4 {
            let l = data[frame * 2];
            let r = data[frame * 2 + 1];
            assert!(
                (l - r).abs() < 1e-6,
                "frame {} L={} R={} should match",
                frame,
                l,
                r
            );
        }

        // Verify values are non-zero where expected
        assert!(data[0].abs() > 0.1, "first sample should be significant");
        assert!(data[4].abs() < 1e-6, "third sample (0) should be near zero");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_duration_calculation() {
        let dir = std::env::temp_dir().join("tracker_rs_test_duration");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("duration.wav");

        // 44100 stereo frames at 44100 Hz = exactly 1 second
        let samples: Vec<i16> = vec![0i16; 44100 * 2];
        write_test_wav(&path, 44100, 2, &samples);

        let sample = load_sample(&path, 44100).unwrap();
        assert!(
            (sample.duration() - 1.0).abs() < 0.001,
            "expected ~1.0s, got {}",
            sample.duration()
        );
        assert_eq!(sample.frame_count(), 44100);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_resample_linear_basic() {
        // 2 stereo frames: [1.0, 0.5, 0.0, -0.5]
        let data = vec![1.0f32, 0.5, 0.0, -0.5];
        let result = resample_linear(&data, 2, 1, 2);
        // Going from 1 Hz to 2 Hz with 2 source frames should produce ~4 frames
        assert!(
            result.len() >= 4,
            "should produce more frames when upsampling"
        );
        // All values should be in a reasonable range
        for &v in &result {
            assert!(v >= -1.0 && v <= 1.5, "resampled value {} out of range", v);
        }
    }
}
