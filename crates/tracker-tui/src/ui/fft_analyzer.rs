/// Master bus FFT spectrum analyzer widget.
///
/// Computes a real-valued FFT on the latest master bus audio samples and
/// renders the magnitude spectrum as a bar chart in the terminal.
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

const BAR_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Compute a radix-2 Cooley-Tukey FFT in-place.
/// `real` and `imag` must have the same power-of-2 length.
fn fft_in_place(real: &mut [f32], imag: &mut [f32]) {
    let n = real.len();
    if n <= 1 {
        return;
    }
    debug_assert!(n.is_power_of_two());
    debug_assert_eq!(real.len(), imag.len());

    // Bit-reversal permutation
    let mut j = 0;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly operations
    let mut step = 2;
    while step <= n {
        let half = step / 2;
        let angle_step = -2.0 * std::f32::consts::PI / step as f32;
        for k in (0..n).step_by(step) {
            for i in 0..half {
                let angle = angle_step * i as f32;
                let wr = angle.cos();
                let wi = angle.sin();
                let idx1 = k + i;
                let idx2 = k + i + half;
                let tr = real[idx2] * wr - imag[idx2] * wi;
                let ti = real[idx2] * wi + imag[idx2] * wr;
                real[idx2] = real[idx1] - tr;
                imag[idx2] = imag[idx1] - ti;
                real[idx1] += tr;
                imag[idx1] += ti;
            }
        }
        step <<= 1;
    }
}

/// Compute magnitude spectrum from time-domain samples.
/// Returns `n/2` magnitude bins (DC to Nyquist).
pub fn compute_spectrum(samples: &[f32]) -> Vec<f32> {
    let n = samples.len().next_power_of_two();
    let mut real = vec![0.0f32; n];
    let mut imag = vec![0.0f32; n];

    // Apply Hann window
    for (i, &s) in samples.iter().enumerate() {
        let w = 0.5
            * (1.0
                - (2.0 * std::f32::consts::PI * i as f32 / (samples.len() - 1).max(1) as f32)
                    .cos());
        real[i] = s * w;
    }

    fft_in_place(&mut real, &mut imag);

    // Compute magnitudes (only first half is meaningful for real input)
    let half = n / 2;
    let mut magnitudes = Vec::with_capacity(half);
    let scale = 2.0 / n as f32;
    for i in 0..half {
        let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt() * scale;
        magnitudes.push(mag);
    }
    magnitudes
}

/// Render the FFT spectrum analyzer as a single-row bar chart.
///
/// `samples` is the time-domain audio from the master bus (typically 1024 samples).
/// The spectrum is binned down to the available `area.width`.
pub fn render_fft_analyzer(frame: &mut Frame, area: Rect, samples: &[f32], theme: &Theme) {
    if area.width == 0 || area.height == 0 || samples.is_empty() {
        return;
    }

    let spectrum = compute_spectrum(samples);
    let width = area.width as usize;

    // Bin the spectrum into `width` bars (logarithmic-ish grouping for musical relevance)
    let num_bins = spectrum.len();
    let step = (num_bins as f64 / width as f64).max(1.0);

    let spans: Vec<Span> = (0..width)
        .map(|i| {
            let start = (i as f64 * step) as usize;
            let end = (((i + 1) as f64 * step) as usize).min(num_bins);
            let peak = spectrum[start..end]
                .iter()
                .fold(0.0f32, |acc, &m| acc.max(m));

            // Convert to dB-like scale (compress dynamic range)
            let db = if peak > 0.0001 {
                (20.0 * peak.log10()).max(-60.0)
            } else {
                -60.0
            };
            let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);

            let idx = (normalized * 8.0) as usize;
            let ch = BAR_CHARS[idx.min(8)];

            let color = if normalized >= 0.85 {
                theme.status_error
            } else if normalized >= 0.60 {
                theme.status_warning
            } else {
                theme.primary
            };

            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect();

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_silence() {
        let samples = vec![0.0f32; 1024];
        let spectrum = compute_spectrum(&samples);
        assert_eq!(spectrum.len(), 512);
        assert!(spectrum.iter().all(|&m| m < 0.001));
    }

    #[test]
    fn test_fft_dc() {
        let mut samples = vec![1.0f32; 1024];
        // DC signal should show energy in bin 0
        let spectrum = compute_spectrum(&samples);
        assert!(spectrum[0] > spectrum[10]);
    }

    #[test]
    fn test_fft_sine_peak() {
        let sample_rate = 1024.0;
        let freq = 100.0;
        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
            .collect();
        let spectrum = compute_spectrum(&samples);

        // The peak should be near bin 100 (freq * N / sample_rate)
        let peak_bin = spectrum
            .iter()
            .enumerate()
            .skip(1) // skip DC
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // Allow some leakage due to windowing
        assert!(
            (peak_bin as f32 - 100.0).abs() < 5.0,
            "Peak at bin {} expected near 100",
            peak_bin
        );
    }

    #[test]
    fn test_fft_non_power_of_two() {
        // Should still work by zero-padding
        let samples = vec![0.5f32; 500];
        let spectrum = compute_spectrum(&samples);
        assert!(!spectrum.is_empty());
    }

    #[test]
    fn test_fft_small_input() {
        let samples = vec![1.0f32; 4];
        let spectrum = compute_spectrum(&samples);
        assert!(!spectrum.is_empty());
    }
}
