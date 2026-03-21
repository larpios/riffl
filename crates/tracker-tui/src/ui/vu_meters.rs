/// VU meter bar widget for the pattern editor channel header.
///
/// Renders per-channel peak level indicators as colored character bars.
/// Each channel is shown with left and right bars side-by-side, colored
/// green → yellow → red based on amplitude.
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

/// Number of bar segments used to represent the full 0.0–1.0 range.
const BAR_SEGMENTS: u16 = 8;

/// Full block character used for filled bar segments.
const BLOCK: char = '█';

/// Map a normalized level (0.0–1.0) to a bar string of `width` characters.
///
/// Clamps `level` to `[0.0, 1.0]` before computing the fill count.
pub fn level_to_bar(level: f32, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let level = level.clamp(0.0, 1.0);
    let filled = (level * width as f32).round() as u16;
    let filled = filled.min(width);
    let empty = width - filled;
    let mut s = String::with_capacity(width as usize);
    for _ in 0..filled {
        s.push(BLOCK);
    }
    for _ in 0..empty {
        s.push(' ');
    }
    s
}

/// Map a normalized level (0.0–1.0) to a theme-appropriate color.
///
/// - 0.00–0.60 → `status_success` (green)
/// - 0.60–0.85 → `status_warning` (yellow)
/// - 0.85–1.00 → `status_error`   (red)
pub fn level_to_color(level: f32, theme: &Theme) -> Color {
    if level >= 0.85 {
        theme.status_error
    } else if level >= 0.60 {
        theme.status_warning
    } else {
        theme.status_success
    }
}

/// Render VU meter bars for each channel into the given area.
///
/// `levels` is a slice of `(left, right)` normalized peak values (0.0–1.0),
/// one entry per channel. The area is divided into equal-width columns, one
/// per channel. Each column shows `L` and `R` bar segments side-by-side.
///
/// If the area is too narrow to show any bars the function returns early
/// without rendering.
pub fn render_vu_meters(frame: &mut Frame, area: Rect, levels: &[(f32, f32)], theme: &Theme) {
    if levels.is_empty() || area.width == 0 || area.height == 0 {
        return;
    }

    let num_channels = levels.len() as u16;
    let constraints: Vec<Constraint> = (0..num_channels)
        .map(|_| Constraint::Ratio(1, num_channels as u32))
        .collect();

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (ch, &(left, right)) in levels.iter().enumerate() {
        let col_area: Rect = columns[ch];
        if col_area.width < 2 {
            continue;
        }

        // Each column: half-width for L, half-width for R.
        let half = col_area.width / 2;
        let l_width = half;
        let r_width = col_area.width - half; // absorbs odd remainder

        let l_bar = level_to_bar(left, l_width.min(BAR_SEGMENTS));
        let r_bar = level_to_bar(right, r_width.min(BAR_SEGMENTS));

        let l_color = level_to_color(left, theme);
        let r_color = level_to_color(right, theme);

        let line = Line::from(vec![
            Span::styled(l_bar, Style::default().fg(l_color)),
            Span::styled(r_bar, Style::default().fg(r_color)),
        ]);

        frame.render_widget(Paragraph::new(line), col_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::Theme;

    fn theme() -> Theme {
        Theme::dark()
    }

    // ── level_to_bar ────────────────────────────────────────────────────────

    #[test]
    fn test_level_to_bar_zero() {
        let bar = level_to_bar(0.0, 8);
        assert_eq!(bar.chars().count(), 8);
        assert!(bar.chars().all(|c| c == ' '));
    }

    #[test]
    fn test_level_to_bar_full() {
        let bar = level_to_bar(1.0, 8);
        assert_eq!(bar.chars().count(), 8);
        assert!(bar.chars().all(|c| c == BLOCK));
    }

    #[test]
    fn test_level_to_bar_half() {
        let bar = level_to_bar(0.5, 8);
        assert_eq!(bar.chars().count(), 8);
        // Exactly 4 filled blocks, 4 empty
        assert_eq!(bar.chars().filter(|&c| c == BLOCK).count(), 4);
        assert_eq!(bar.chars().filter(|&c| c == ' ').count(), 4);
    }

    #[test]
    fn test_level_to_bar_clamps_above_one() {
        let bar = level_to_bar(2.0, 8);
        assert!(bar.chars().all(|c| c == BLOCK));
    }

    #[test]
    fn test_level_to_bar_clamps_below_zero() {
        let bar = level_to_bar(-1.0, 8);
        assert!(bar.chars().all(|c| c == ' '));
    }

    #[test]
    fn test_level_to_bar_zero_width() {
        let bar = level_to_bar(1.0, 0);
        assert_eq!(bar, "");
    }

    #[test]
    fn test_level_to_bar_width_one() {
        assert_eq!(level_to_bar(0.0, 1), " ");
        assert_eq!(level_to_bar(1.0, 1), "█");
    }

    // ── level_to_color ──────────────────────────────────────────────────────

    #[test]
    fn test_level_to_color_low_is_success() {
        let t = theme();
        assert_eq!(level_to_color(0.0, &t), t.status_success);
        assert_eq!(level_to_color(0.3, &t), t.status_success);
        assert_eq!(level_to_color(0.59, &t), t.status_success);
    }

    #[test]
    fn test_level_to_color_mid_is_warning() {
        let t = theme();
        assert_eq!(level_to_color(0.60, &t), t.status_warning);
        assert_eq!(level_to_color(0.70, &t), t.status_warning);
        assert_eq!(level_to_color(0.84, &t), t.status_warning);
    }

    #[test]
    fn test_level_to_color_high_is_error() {
        let t = theme();
        assert_eq!(level_to_color(0.85, &t), t.status_error);
        assert_eq!(level_to_color(0.95, &t), t.status_error);
        assert_eq!(level_to_color(1.0, &t), t.status_error);
    }
}
