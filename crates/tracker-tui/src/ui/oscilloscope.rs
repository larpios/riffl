/// Per-channel oscilloscope widget for the pattern editor.
///
/// Renders a waveform sparkline from the mixer's ring buffer data,
/// showing the most recent audio output for each channel.
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

/// Braille-based waveform characters for 2-row rendering.
/// Maps a normalized amplitude (0.0 to 1.0) to a vertical bar character.
const WAVE_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render oscilloscope waveforms for each channel.
///
/// `waveforms` is a slice of per-channel sample data (e.g., from `Mixer::oscilloscope_data`).
/// Each waveform is rendered as a compact single-row sparkline within its column.
pub fn render_oscilloscopes(frame: &mut Frame, area: Rect, waveforms: &[Vec<f32>], theme: &Theme) {
    if waveforms.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }

    let num_channels = waveforms.len() as u16;
    let constraints: Vec<Constraint> = (0..num_channels)
        .map(|_| Constraint::Ratio(1, num_channels as u32))
        .collect();

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (ch, waveform) in waveforms.iter().enumerate() {
        let col_area = columns[ch];
        if col_area.width < 2 {
            continue;
        }

        let width = col_area.width as usize;
        let line = render_waveform_line(waveform, width, theme);
        frame.render_widget(Paragraph::new(line), col_area);
    }
}

/// Downsample a waveform buffer to `width` characters and produce a styled Line.
fn render_waveform_line(samples: &[f32], width: usize, theme: &Theme) -> Line<'static> {
    if samples.is_empty() || width == 0 {
        return Line::default();
    }

    let step = samples.len() as f64 / width as f64;
    let wave_color = theme.primary;

    let spans: Vec<Span> = (0..width)
        .map(|i| {
            let start = (i as f64 * step) as usize;
            let end = ((i + 1) as f64 * step) as usize;
            let end = end.min(samples.len());

            // Find peak absolute value in this segment
            let peak = samples[start..end]
                .iter()
                .fold(0.0f32, |acc, &s| acc.max(s.abs()));

            let idx = (peak.clamp(0.0, 1.0) * 8.0) as usize;
            let idx = idx.min(8);
            let ch = WAVE_CHARS[idx];

            let color = if peak >= 0.85 {
                theme.status_error
            } else if peak >= 0.60 {
                theme.status_warning
            } else {
                wave_color
            };

            Span::styled(ch.to_string(), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
}

/// Map a waveform amplitude (-1.0 to 1.0) to a character suitable for
/// a centered waveform display (bipolar mode).
pub fn amplitude_to_char(amplitude: f32) -> char {
    let normalized = ((amplitude + 1.0) / 2.0).clamp(0.0, 1.0);
    let idx = (normalized * 8.0) as usize;
    WAVE_CHARS[idx.min(8)]
}

/// Compute the RMS level of a waveform buffer.
pub fn rms_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_chars_length() {
        assert_eq!(WAVE_CHARS.len(), 9);
    }

    #[test]
    fn test_amplitude_to_char_silent() {
        assert_eq!(amplitude_to_char(0.0), '▄'); // center
    }

    #[test]
    fn test_amplitude_to_char_max() {
        assert_eq!(amplitude_to_char(1.0), '█');
    }

    #[test]
    fn test_amplitude_to_char_min() {
        assert_eq!(amplitude_to_char(-1.0), ' ');
    }

    #[test]
    fn test_rms_level_silence() {
        assert_eq!(rms_level(&[0.0; 100]), 0.0);
    }

    #[test]
    fn test_rms_level_dc() {
        let level = rms_level(&[0.5; 100]);
        assert!((level - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rms_level_empty() {
        assert_eq!(rms_level(&[]), 0.0);
    }

    #[test]
    fn test_render_waveform_line_empty() {
        let theme = Theme::default();
        let line = render_waveform_line(&[], 10, &theme);
        assert!(line.spans.is_empty());
    }

    #[test]
    fn test_render_waveform_line_silent() {
        let theme = Theme::default();
        let samples = vec![0.0f32; 512];
        let line = render_waveform_line(&samples, 20, &theme);
        assert_eq!(line.spans.len(), 20);
        // All should be space (silent)
        for span in &line.spans {
            assert_eq!(span.content.as_ref(), " ");
        }
    }

    #[test]
    fn test_render_waveform_line_loud() {
        let theme = Theme::default();
        let samples = vec![1.0f32; 512];
        let line = render_waveform_line(&samples, 10, &theme);
        assert_eq!(line.spans.len(), 10);
        for span in &line.spans {
            assert_eq!(span.content.as_ref(), "█");
        }
    }
}
