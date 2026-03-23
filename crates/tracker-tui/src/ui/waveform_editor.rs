/// Waveform editor for manual sample data drawing/editing.
///
/// Provides point-by-point waveform editing within a dedicated editor panel.
/// "Pencil mode" allows drawing sample values using keyboard controls.
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use tracker_core::audio::sample::Sample;
use tracker_core::song::Instrument;

use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformEditMode {
    Navigate,
    Pencil,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformEditorState {
    pub focused: bool,
    pub edit_mode: WaveformEditMode,
    pub cursor_sample: usize,
    pub pencil_value: f32,
}

impl Default for WaveformEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            edit_mode: WaveformEditMode::Navigate,
            cursor_sample: 0,
            pencil_value: 0.0,
        }
    }
}

impl WaveformEditorState {
    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
        self.edit_mode = WaveformEditMode::Navigate;
    }

    pub fn enter_pencil_mode(&mut self) {
        self.edit_mode = WaveformEditMode::Pencil;
    }

    pub fn exit_pencil_mode(&mut self) {
        self.edit_mode = WaveformEditMode::Navigate;
    }

    pub fn toggle_pencil_mode(&mut self) {
        self.edit_mode = match self.edit_mode {
            WaveformEditMode::Navigate => WaveformEditMode::Pencil,
            WaveformEditMode::Pencil => WaveformEditMode::Navigate,
        };
    }

    pub fn move_cursor_left(&mut self, max_samples: usize) {
        self.cursor_sample = self
            .cursor_sample
            .saturating_sub(1)
            .min(max_samples.saturating_sub(1));
    }

    pub fn move_cursor_right(&mut self, max_samples: usize) {
        self.cursor_sample = self
            .cursor_sample
            .saturating_add(1)
            .min(max_samples.saturating_sub(1));
    }

    pub fn set_cursor(&mut self, sample: usize) {
        self.cursor_sample = sample;
    }

    pub fn pencil_value_up(&mut self) {
        self.pencil_value = (self.pencil_value + 0.1).min(1.0);
    }

    pub fn pencil_value_down(&mut self) {
        self.pencil_value = (self.pencil_value - 0.1).max(-1.0);
    }

    pub fn draw_at_cursor(&self, sample: &mut Sample) {
        if self.cursor_sample < sample.frame_count() {
            sample.set_sample(self.cursor_sample, self.pencil_value);
        }
    }
}

const MAX_WAVEFORM_DISPLAY_HEIGHT: usize = 8;

#[allow(clippy::needless_range_loop)]
fn draw_waveform_graphic(
    sample: &Sample,
    width: usize,
    height: usize,
    theme: &Theme,
    cursor_sample: Option<usize>,
    edit_mode: WaveformEditMode,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(height.min(MAX_WAVEFORM_DISPLAY_HEIGHT));

    let frame_count = sample.frame_count();
    if frame_count == 0 || width < 4 {
        let dash = "─".repeat(width.max(4));
        let s = Style::default().fg(theme.text_dimmed);
        for _ in 0..height.min(MAX_WAVEFORM_DISPLAY_HEIGHT) {
            lines.push(Line::from(Span::styled(dash.clone(), s)));
        }
        return lines;
    }

    let grid_width = width.saturating_sub(2);
    let grid_height = height.saturating_sub(1).min(MAX_WAVEFORM_DISPLAY_HEIGHT);

    if grid_height < 2 {
        let dash = "─".repeat(width.max(4));
        let s = Style::default().fg(theme.text_dimmed);
        lines.push(Line::from(Span::styled(dash, s)));
        return lines;
    }

    let center_row = grid_height / 2;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; grid_width]; grid_height];

    let mut cursor_col: Option<usize> = None;
    let cursor_char = match edit_mode {
        WaveformEditMode::Pencil => '◆',
        WaveformEditMode::Navigate => '│',
    };

    for col in 0..grid_width {
        let start_frame = (col * frame_count) / grid_width;
        let end_frame = ((col + 1) * frame_count / grid_width)
            .max(start_frame + 1)
            .min(frame_count);

        let mut peak_pos: f32 = 0.0;
        let mut peak_neg: f32 = 0.0;
        for frame_idx in start_frame..end_frame {
            let channels = sample.channels() as usize;
            if frame_idx * channels < sample.data().len() {
                let v = sample.data()[frame_idx * channels];
                if v > peak_pos {
                    peak_pos = v;
                }
                if -v > peak_neg {
                    peak_neg = -v;
                }
            }
        }

        let top_row = center_row
            .saturating_sub((peak_pos * (center_row as f32)) as usize)
            .min(center_row);
        let bot_row =
            (center_row + 1 + (peak_neg * ((grid_height - center_row - 1) as f32)) as usize)
                .min(grid_height - 1);

        if peak_pos > 0.01 {
            for row in top_row..=center_row {
                if row < grid_height {
                    grid[row][col] = '█';
                }
            }
        }
        if peak_neg > 0.01 {
            for row in center_row..=bot_row {
                if row < grid_height {
                    grid[row][col] = '█';
                }
            }
        }

        if peak_pos > 0.01 && peak_neg > 0.01 && center_row < grid_height {
            grid[center_row][col] = '█';
        }

        if let Some(cur) = cursor_sample {
            let cur_col = (cur * grid_width) / frame_count;
            if cur_col == col {
                cursor_col = Some(col);
                if let Some(row) = grid.get_mut(center_row) {
                    row[col] = cursor_char;
                }
            }
        }
    }

    let wf_style = Style::default().fg(theme.primary);
    let cursor_style = if edit_mode == WaveformEditMode::Pencil {
        Style::default()
            .fg(theme.warning_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.cursor_fg)
    };
    let center_style = Style::default().fg(theme.text_dimmed);

    for (row_idx, row) in grid.iter().enumerate() {
        let row_str: String = row.iter().collect();
        let row_style = if row_idx == center_row {
            center_style
        } else {
            wf_style
        };
        lines.push(Line::from(vec![Span::styled(row_str, row_style)]));
    }

    if let Some(col) = cursor_col {
        let line_idx = center_row.min(lines.len().saturating_sub(1));
        if let Some(line) = lines.get_mut(line_idx) {
            let mut spans = line.spans.clone();
            let char_at_cursor = if edit_mode == WaveformEditMode::Pencil {
                '◆'
            } else {
                '│'
            };
            if col < spans.len() {
                spans[col] = Span::styled(char_at_cursor.to_string(), cursor_style);
            }
            lines[line_idx] = Line::from(spans);
        }
    }

    lines
}

fn format_sample_value(value: f32) -> String {
    if value.abs() < 0.001 {
        "  0.00".to_string()
    } else if value >= 0.0 {
        format!("+{:.2}", value)
    } else {
        format!("{:.2}", value)
    }
}

#[allow(unused_variables)]
pub fn render_waveform_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    sample: Option<&Sample>,
    state: &WaveformEditorState,
    theme: &Theme,
) {
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };

    let title = format!(
        " Waveform Editor {} ",
        match state.edit_mode {
            WaveformEditMode::Navigate => "[NAV]",
            WaveformEditMode::Pencil => "[PENCIL]",
        }
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;

    if width < 10 || height < 4 {
        return;
    }

    let sample_info = if let Some(s) = sample {
        if s.is_empty() {
            "No sample loaded".to_string()
        } else {
            let ch = s.channels();
            let sr = s.sample_rate();
            let frames = s.frame_count();
            let dur = s.duration();
            format!("{:.2}s · {}Hz · {}ch · {} frames", dur, sr, ch, frames)
        }
    } else {
        "No sample".to_string()
    };

    let help_text = if state.focused {
        match state.edit_mode {
            WaveformEditMode::Navigate => "←→: navigate  p: pencil mode  Esc: exit",
            WaveformEditMode::Pencil => "↑↓: draw value  ←→: move  Enter: draw  p: exit",
        }
    } else {
        "Enter to edit"
    };

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(sample_info, Style::default().fg(theme.text_secondary)),
    ]));

    lines.push(Line::from(""));

    if let Some(s) = sample {
        let graphic = draw_waveform_graphic(
            s,
            width,
            height.saturating_sub(4),
            theme,
            Some(state.cursor_sample),
            state.edit_mode,
        );
        for line in graphic {
            let mut spans = vec![Span::raw("  ")];
            spans.extend(line.spans);
            lines.push(Line::from(spans));
        }

        if state.focused && state.edit_mode == WaveformEditMode::Pencil {
            let current_value = if state.cursor_sample < s.frame_count() {
                s.data()[state.cursor_sample * s.channels() as usize]
            } else {
                0.0
            };
            let pencil_val = state.pencil_value;
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "  Cursor: {:4}  Pencil: {}  ",
                    state.cursor_sample,
                    format_sample_value(pencil_val)
                ),
                Style::default().fg(theme.text),
            )]));
            lines.push(Line::from(vec![Span::styled(
                format!("  Current: {}  ", format_sample_value(current_value)),
                Style::default().fg(theme.text_dimmed),
            )]));
        }
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  No sample to edit. Load a sample first.",
            Style::default().fg(theme.text_dimmed),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!("  {}", help_text),
        Style::default().fg(theme.text_dimmed),
    )]));

    let content = Paragraph::new(lines).alignment(Alignment::Left);

    let inner_with_border = ratatui::layout::Rect::new(
        inner.x.saturating_sub(1),
        inner.y.saturating_sub(1),
        inner.width + 2,
        inner.height + 2,
    );
    frame.render_widget(content, inner_with_border);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample() -> Sample {
        let data: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        Sample::new(data, 44100, 1, Some("test".to_string()))
    }

    #[test]
    fn test_waveform_editor_state_defaults() {
        let state = WaveformEditorState::default();
        assert!(!state.focused);
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
        assert_eq!(state.cursor_sample, 0);
    }

    #[test]
    fn test_focus_unfocus() {
        let mut state = WaveformEditorState::default();
        state.focus();
        assert!(state.focused);
        state.unfocus();
        assert!(!state.focused);
    }

    #[test]
    fn test_pencil_mode_toggle() {
        let mut state = WaveformEditorState::default();
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
        state.enter_pencil_mode();
        assert_eq!(state.edit_mode, WaveformEditMode::Pencil);
        state.exit_pencil_mode();
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = WaveformEditorState::default();
        state.move_cursor_right(100);
        assert_eq!(state.cursor_sample, 1);
        state.move_cursor_left(100);
        assert_eq!(state.cursor_sample, 0);
        state.move_cursor_left(100);
        assert_eq!(state.cursor_sample, 0);
    }

    #[test]
    fn test_pencil_value() {
        let mut state = WaveformEditorState::default();
        state.pencil_value_up();
        assert!((state.pencil_value - 0.1).abs() < 0.001);
        state.pencil_value_down();
        state.pencil_value_down();
        assert!((state.pencil_value - (-0.1)).abs() < 0.001);
    }

    #[test]
    fn test_sample_set_sample() {
        let mut sample = make_sample();
        let original = sample.data()[0];
        sample.set_sample(0, 0.5);
        assert_eq!(sample.data()[0], 0.5);
        sample.set_sample(0, original);
    }

    #[test]
    fn test_sample_set_sample_clamp() {
        let mut sample = make_sample();
        sample.set_sample(0, 2.0);
        assert_eq!(sample.data()[0], 1.0);
        sample.set_sample(0, -2.0);
        assert_eq!(sample.data()[0], -1.0);
    }

    #[test]
    fn test_render_no_panic() {
        let inst = Instrument::new("TestInst");
        let state = WaveformEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(frame, frame.area(), &inst, None, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_sample_no_panic() {
        let inst = Instrument::new("TestInst");
        let sample = make_sample();
        let state = WaveformEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(frame, frame.area(), &inst, Some(&sample), &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_render_focused_pencil_no_panic() {
        let inst = Instrument::new("TestInst");
        let sample = make_sample();
        let mut state = WaveformEditorState::default();
        state.focus();
        state.enter_pencil_mode();
        state.cursor_sample = 50;
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(frame, frame.area(), &inst, Some(&sample), &state, &theme);
            })
            .unwrap();
    }
}
