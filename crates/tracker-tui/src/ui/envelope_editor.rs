/// Envelope editor for visual point-to-point editing of Volume, Panning, and Pitch envelopes.
///
/// Provides visual envelope editing within the instrument editor panel.
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use tracker_core::song::{Envelope, EnvelopePoint, Instrument};

use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeType {
    Volume,
    Panning,
    Pitch,
}

impl EnvelopeType {
    pub const ALL: &'static [EnvelopeType] = &[
        EnvelopeType::Volume,
        EnvelopeType::Panning,
        EnvelopeType::Pitch,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Volume => "Volume",
            Self::Panning => "Panning",
            Self::Pitch => "Pitch",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            Self::Volume => "VOL",
            Self::Panning => "PAN",
            Self::Pitch => "PIT",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Volume => Self::Panning,
            Self::Panning => Self::Pitch,
            Self::Pitch => Self::Volume,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Volume => Self::Pitch,
            Self::Panning => Self::Volume,
            Self::Pitch => Self::Panning,
        }
    }

    pub fn color(&self, theme: &Theme) -> ratatui::style::Color {
        match self {
            Self::Volume => theme.primary,
            Self::Panning => theme.secondary,
            Self::Pitch => theme.status_info,
        }
    }

    pub fn value_range(&self) -> (f32, f32) {
        match self {
            Self::Volume | Self::Panning => (0.0, 1.0),
            Self::Pitch => (-1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeEditorState {
    pub focused: bool,
    pub envelope_type: EnvelopeType,
    pub selected_point: Option<usize>,
    pub drag_value_delta: Option<f32>,
}

impl Default for EnvelopeEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            envelope_type: EnvelopeType::Volume,
            selected_point: None,
            drag_value_delta: None,
        }
    }
}

impl EnvelopeEditorState {
    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
    }

    pub fn cycle_envelope_type(&mut self) {
        self.envelope_type = self.envelope_type.next();
        self.selected_point = None;
    }

    pub fn prev_envelope_type(&mut self) {
        self.envelope_type = self.envelope_type.prev();
        self.selected_point = None;
    }

    pub fn select_point(&mut self, idx: Option<usize>) {
        self.selected_point = idx;
    }

    pub fn select_first_point(&mut self, envelope: &Envelope) {
        self.selected_point = if envelope.points.is_empty() {
            None
        } else {
            Some(0)
        };
    }

    pub fn select_last_point(&mut self, envelope: &Envelope) {
        self.selected_point = if envelope.points.is_empty() {
            None
        } else {
            Some(envelope.points.len().saturating_sub(1))
        };
    }

    pub fn move_point_left(&mut self, envelope: &mut Envelope) {
        if let Some(idx) = self.selected_point {
            if idx > 0 {
                if let Some(point) = envelope.points.get_mut(idx) {
                    if point.frame > 0 {
                        point.frame = point.frame.saturating_sub(1);
                    }
                }
                let new_idx = idx - 1;
                self.selected_point = Some(new_idx);
            }
        }
    }

    pub fn move_point_right(&mut self, envelope: &mut Envelope) {
        if let Some(idx) = self.selected_point {
            if let Some(point) = envelope.points.get_mut(idx) {
                point.frame = point.frame.saturating_add(1);
            }
            if idx < envelope.points.len() - 1 {
                self.selected_point = Some(idx + 1);
            }
        }
    }

    pub fn move_point_up(&mut self, envelope: &mut Envelope, env_type: EnvelopeType) {
        if let Some(idx) = self.selected_point {
            if let Some(point) = envelope.points.get_mut(idx) {
                let step = 0.05;
                let (_min_val, max_val) = env_type.value_range();
                point.value = (point.value + step).min(max_val);
            }
        }
    }

    pub fn move_point_down(&mut self, envelope: &mut Envelope, env_type: EnvelopeType) {
        if let Some(idx) = self.selected_point {
            if let Some(point) = envelope.points.get_mut(idx) {
                let step = 0.05;
                let (min_val, _max_val) = env_type.value_range();
                point.value = (point.value - step).max(min_val);
            }
        }
    }

    pub fn change_value(&mut self, envelope: &mut Envelope, delta: f32) {
        if let Some(idx) = self.selected_point {
            if let Some(point) = envelope.points.get_mut(idx) {
                let (min_val, max_val) = self.envelope_type.value_range();
                point.value = (point.value + delta).clamp(min_val, max_val);
            }
        }
    }

    pub fn add_point_at(&mut self, envelope: &mut Envelope, frame: u16, value: f32) {
        let idx = envelope
            .points
            .iter()
            .position(|p| p.frame > frame)
            .unwrap_or(envelope.points.len());
        envelope.points.insert(idx, EnvelopePoint { frame, value });
        self.selected_point = Some(idx);
    }

    pub fn delete_selected_point(&mut self, envelope: &mut Envelope) {
        if let Some(idx) = self.selected_point {
            if envelope.points.len() > 1 {
                envelope.points.remove(idx);
                self.selected_point = Some(idx.min(envelope.points.len().saturating_sub(1)));
            }
        }
    }

    pub fn toggle_envelope_enabled(&self, instrument: &mut Instrument) {
        let envelope = self.get_envelope_mut(instrument);
        if !envelope.points.is_empty() {
            envelope.enabled = !envelope.enabled;
        }
    }

    pub fn get_envelope<'a>(&self, instrument: &'a Instrument) -> &'a Envelope {
        match self.envelope_type {
            EnvelopeType::Volume => instrument
                .volume_envelope
                .as_ref()
                .unwrap_or(&EMPTY_ENVELOPE),
            EnvelopeType::Panning => instrument
                .panning_envelope
                .as_ref()
                .unwrap_or(&EMPTY_ENVELOPE),
            EnvelopeType::Pitch => instrument
                .pitch_envelope
                .as_ref()
                .unwrap_or(&EMPTY_ENVELOPE),
        }
    }

    pub fn get_envelope_mut<'a>(&self, instrument: &'a mut Instrument) -> &'a mut Envelope {
        match self.envelope_type {
            EnvelopeType::Volume => {
                if instrument.volume_envelope.is_none() {
                    instrument.volume_envelope = Some(Envelope::default());
                }
                instrument.volume_envelope.as_mut().unwrap()
            }
            EnvelopeType::Panning => {
                if instrument.panning_envelope.is_none() {
                    instrument.panning_envelope = Some(Envelope::default());
                }
                instrument.panning_envelope.as_mut().unwrap()
            }
            EnvelopeType::Pitch => {
                if instrument.pitch_envelope.is_none() {
                    instrument.pitch_envelope = Some(Envelope::default());
                }
                instrument.pitch_envelope.as_mut().unwrap()
            }
        }
    }

    /// Start dragging from the current selected point position.
    pub fn start_drag(&mut self, _initial_delta: f32) {
        if self.selected_point.is_some() {
            self.drag_value_delta = Some(0.0);
        }
    }

    /// Update the drag delta (call during drag).
    pub fn update_drag(&mut self, delta: f32) {
        if let Some(current) = self.drag_value_delta.take() {
            self.drag_value_delta = Some(current + delta);
        }
    }

    /// Apply the current drag delta to the envelope point and reset.
    pub fn apply_drag(&mut self, envelope: &mut Envelope) {
        if let (Some(idx), Some(delta)) = (self.selected_point, self.drag_value_delta.take()) {
            if let Some(point) = envelope.points.get_mut(idx) {
                let (min_val, max_val) = self.envelope_type.value_range();
                point.value = (point.value + delta).clamp(min_val, max_val);
            }
        }
    }

    /// End dragging without applying.
    pub fn end_drag(&mut self) {
        self.drag_value_delta = None;
    }

    /// Check if currently dragging.
    pub fn is_dragging(&self) -> bool {
        self.drag_value_delta.is_some()
    }
}

static EMPTY_ENVELOPE: Envelope = Envelope {
    points: Vec::new(),
    enabled: false,
    sustain_enabled: false,
    sustain_start_point: 0,
    sustain_end_point: 0,
    loop_enabled: false,
    loop_start_point: 0,
    loop_end_point: 0,
};

const MIN_ENVELOPE_FRAMES: u16 = 0;
const MAX_ENVELOPE_FRAMES: u16 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EnvelopeGraphMetrics {
    label_width: usize,
    grid_width: usize,
    grid_height: usize,
}

fn value_label_width(env_type: EnvelopeType) -> usize {
    match env_type {
        EnvelopeType::Volume | EnvelopeType::Panning => 7,
        EnvelopeType::Pitch => 6,
    }
}

fn graph_metrics(
    env_type: EnvelopeType,
    width: usize,
    height: usize,
) -> Option<EnvelopeGraphMetrics> {
    if width < 4 || height < 2 {
        return None;
    }

    Some(EnvelopeGraphMetrics {
        label_width: value_label_width(env_type),
        grid_width: width - 2,
        grid_height: height.saturating_sub(1),
    })
}

fn format_value(value: f32, env_type: EnvelopeType) -> String {
    match env_type {
        EnvelopeType::Volume => format!("{:5.1}%", value * 100.0),
        EnvelopeType::Panning => {
            if value < -0.01 {
                format!("{:5.1}L", (-value) * 100.0)
            } else if value > 0.01 {
                format!("{:5.1}R", value * 100.0)
            } else {
                " C   ".to_string()
            }
        }
        EnvelopeType::Pitch => {
            if value.abs() < 0.01 {
                "  0  ".to_string()
            } else if value > 0.0 {
                format!("+{:.2}", value)
            } else {
                format!("{:.2}", value)
            }
        }
    }
}

fn draw_envelope_graphic(
    envelope: &Envelope,
    env_type: EnvelopeType,
    width: usize,
    height: usize,
    theme: &Theme,
    selected_point: Option<usize>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(height);

    let Some(metrics) = graph_metrics(env_type, width, height) else {
        return vec![Line::from(vec![Span::raw("")])];
    };

    let grid_width = metrics.grid_width;
    let grid_height = metrics.grid_height;

    let (min_val, max_val) = env_type.value_range();
    let val_range = max_val - min_val;

    let mut grid: Vec<Vec<char>> = vec![vec![' '; grid_width]; grid_height];

    if !envelope.points.is_empty() {
        let max_frame = envelope
            .points
            .iter()
            .map(|p| p.frame)
            .max()
            .unwrap()
            .max(1);

        for window in envelope.points.windows(2) {
            let p1 = &window[0];
            let p2 = &window[1];

            let x1 = ((p1.frame as f32 / max_frame as f32) * (grid_width - 1) as f32) as usize;
            let x2 = ((p2.frame as f32 / max_frame as f32) * (grid_width - 1) as f32) as usize;
            let y1 = ((p1.value - min_val) / val_range * (grid_height - 1) as f32) as usize;
            let y2 = ((p2.value - min_val) / val_range * (grid_height - 1) as f32) as usize;

            let y1 = grid_height.saturating_sub(1).min(y1);
            let y2 = grid_height.saturating_sub(1).min(y2);

            let min_x = x1.min(x2);
            let max_x = x1.max(x2);

            if max_x == min_x {
                if y1 < grid_height {
                    grid[y1][min_x] = '●';
                }
            } else {
                let range_len = max_x - min_x + 1;
                for i in 0..range_len {
                    let x = min_x + i;
                    let progress = i as f32 / (max_x - min_x) as f32;
                    let y = (y1 as f32 + (y2 as f32 - y1 as f32) * progress) as usize;
                    let y = grid_height.saturating_sub(1).min(y);
                    if y < grid_height {
                        grid[y][x] = '─';
                    }
                }
                if y1 < grid_height {
                    grid[y1][x1.min(x2)] = '├';
                }
                if y2 < grid_height {
                    grid[y2][x2] = '┤';
                }
            }
        }

        if let Some(last_point) = envelope.points.last() {
            let x =
                ((last_point.frame as f32 / max_frame as f32) * (grid_width - 1) as f32) as usize;
            let y = ((last_point.value - min_val) / val_range * (grid_height - 1) as f32) as usize;
            let y = grid_height.saturating_sub(1).min(y);
            if y < grid_height && x < grid_width {
                grid[y][x] = '●';
            }
        }

        if let Some(idx) = selected_point {
            if let Some(point) = envelope.points.get(idx) {
                let x =
                    ((point.frame as f32 / max_frame as f32) * (grid_width - 1) as f32) as usize;
                let y = ((point.value - min_val) / val_range * (grid_height - 1) as f32) as usize;
                let y = grid_height.saturating_sub(1).min(y);
                if y < grid_height && x < grid_width {
                    grid[y][x] = '◆';
                }
            }
        }
    }

    let color = env_type.color(theme);
    let val_style = Style::default().fg(color);

    for (row_idx, row) in grid.iter().enumerate().take(height - 1) {
        let val_at_row = max_val - (row_idx as f32 / (grid_height - 1).max(1) as f32) * val_range;
        let val_str = format_value(val_at_row, env_type);
        let row_str: String = row.iter().collect();

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", val_str), val_style),
            Span::raw(row_str),
        ]));
    }

    if !lines.is_empty() {
        let last = lines.len() - 1;
        let val_str = format_value(min_val, env_type);
        let last_line = &mut lines[last];
        let mut spans = last_line.spans.clone();
        spans.insert(0, Span::styled(format!("{} ", val_str), val_style));
        lines[last] = Line::from(spans);
    }

    lines
}

pub fn point_at_position(
    envelope: &Envelope,
    env_type: EnvelopeType,
    width: usize,
    height: usize,
    local_x: u16,
    local_y: u16,
) -> Option<usize> {
    let metrics = graph_metrics(env_type, width, height)?;
    if envelope.points.is_empty()
        || (local_x as usize) < metrics.label_width
        || (local_y as usize) >= metrics.grid_height
    {
        return None;
    }

    let graph_x = (local_x as usize).saturating_sub(metrics.label_width);
    if graph_x >= metrics.grid_width {
        return None;
    }

    let (min_val, max_val) = env_type.value_range();
    let val_range = max_val - min_val;
    let max_frame = envelope
        .points
        .iter()
        .map(|p| p.frame)
        .max()
        .unwrap_or(1)
        .max(1);

    envelope
        .points
        .iter()
        .enumerate()
        .filter_map(|(idx, point)| {
            let px =
                ((point.frame as f32 / max_frame as f32) * (metrics.grid_width - 1) as f32) as i32;
            let py =
                ((point.value - min_val) / val_range * (metrics.grid_height - 1) as f32) as usize;
            let py = metrics.grid_height.saturating_sub(1).min(py) as i32;
            let py = metrics.grid_height as i32 - 1 - py;

            let dx = px - graph_x as i32;
            let dy = py - local_y as i32;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq <= 4 {
                Some((idx, dist_sq))
            } else {
                None
            }
        })
        .min_by_key(|(_, dist_sq)| *dist_sq)
        .map(|(idx, _)| idx)
}

pub fn update_point_from_position(
    envelope: &mut Envelope,
    env_type: EnvelopeType,
    selected_point: usize,
    width: usize,
    height: usize,
    local_x: u16,
    local_y: u16,
) {
    let Some(metrics) = graph_metrics(env_type, width, height) else {
        return;
    };
    if selected_point >= envelope.points.len()
        || (local_x as usize) < metrics.label_width
        || (local_y as usize) >= metrics.grid_height
    {
        return;
    }

    let graph_x = (local_x as usize)
        .saturating_sub(metrics.label_width)
        .min(metrics.grid_width.saturating_sub(1));
    let graph_y = (local_y as usize).min(metrics.grid_height.saturating_sub(1));

    let max_frame = envelope
        .points
        .iter()
        .map(|p| p.frame)
        .max()
        .unwrap_or(1)
        .clamp(1, MAX_ENVELOPE_FRAMES);
    let (min_val, max_val) = env_type.value_range();
    let normalized_x = if metrics.grid_width <= 1 {
        0.0
    } else {
        graph_x as f32 / (metrics.grid_width - 1) as f32
    };
    let normalized_y = if metrics.grid_height <= 1 {
        0.0
    } else {
        1.0 - graph_y as f32 / (metrics.grid_height - 1) as f32
    };

    let min_frame = if selected_point == 0 {
        MIN_ENVELOPE_FRAMES
    } else {
        envelope.points[selected_point - 1].frame.saturating_add(1)
    };
    let max_frame_for_point = if selected_point + 1 >= envelope.points.len() {
        max_frame
    } else {
        envelope.points[selected_point + 1].frame.saturating_sub(1)
    };

    let frame = (normalized_x * max_frame as f32).round() as u16;
    let value = min_val + normalized_y * (max_val - min_val);

    if let Some(point) = envelope.points.get_mut(selected_point) {
        point.frame = frame.clamp(min_frame, max_frame_for_point.max(min_frame));
        point.value = value.clamp(min_val, max_val);
    }
}

pub fn render_envelope_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    state: &EnvelopeEditorState,
    theme: &Theme,
) {
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };

    let env_type = state.envelope_type;
    let envelope = state.get_envelope(instrument);

    // Build a tab-bar title: " Envelope  [VOL]  PAN  PIT "
    // Active tab is bracketed and coloured; inactive tabs are dimmed.
    let mut title_spans = vec![Span::raw(" Envelope ")];
    for tab in EnvelopeType::ALL {
        let is_active = *tab == env_type;
        let tab_env = match tab {
            EnvelopeType::Volume => instrument.volume_envelope.as_ref(),
            EnvelopeType::Panning => instrument.panning_envelope.as_ref(),
            EnvelopeType::Pitch => instrument.pitch_envelope.as_ref(),
        };
        let enabled = tab_env.map_or(false, |e| e.enabled);
        let label = if is_active {
            format!("[{}{}] ", tab.short_label(), if enabled { "·" } else { "" })
        } else {
            format!(" {}{}  ", tab.short_label(), if enabled { "·" } else { "" })
        };
        let style = if is_active {
            Style::default()
                .fg(tab.color(theme))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dimmed)
        };
        title_spans.push(Span::styled(label, style));
    }
    if !state.focused {
        title_spans.push(Span::styled(" Tab ", Style::default().fg(theme.text_dimmed)));
    }

    let title_line = Line::from(title_spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title_line)
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;

    if width < 10 || height < 2 {
        return;
    }

    // When focused: reserve 2 bottom rows for point info + help text.
    // When not focused: use the full height for the graph.
    let (graph_height, show_info) = if state.focused && height >= 4 {
        (height - 2, true)
    } else {
        (height, false)
    };

    let graphic = draw_envelope_graphic(
        envelope,
        env_type,
        width,
        graph_height,
        theme,
        state.selected_point,
    );

    let mut lines: Vec<Line> = Vec::new();
    lines.extend(graphic);

    if show_info {
        let point_info = if let Some(idx) = state.selected_point {
            if let Some(point) = envelope.points.get(idx) {
                let value_str = format_value(point.value, env_type);
                format!("  Pt {}: frame={} val={}  k/j: value  h/l: time  a: add  x: del  e: on/off  Esc", idx, point.frame, value_str)
            } else {
                String::new()
            }
        } else {
            "  No point selected — a: add   Tab/S-Tab: cycle type   Esc: exit".to_string()
        };

        lines.push(Line::from(vec![Span::styled(
            point_info,
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "  hjkl: navigate   a: add pt   x: del pt   0/$: first/last   Tab: cycle type   e: on/off   Esc",
            Style::default().fg(theme.text_dimmed),
        )]));
    }

    let content = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(content, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracker_core::song::Envelope;

    fn make_envelope() -> Envelope {
        Envelope {
            points: vec![
                EnvelopePoint {
                    frame: 0,
                    value: 0.0,
                },
                EnvelopePoint {
                    frame: 64,
                    value: 1.0,
                },
                EnvelopePoint {
                    frame: 128,
                    value: 0.5,
                },
                EnvelopePoint {
                    frame: 256,
                    value: 0.0,
                },
            ],
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_envelope_type_labels() {
        assert_eq!(EnvelopeType::Volume.label(), "Volume");
        assert_eq!(EnvelopeType::Panning.label(), "Panning");
        assert_eq!(EnvelopeType::Pitch.label(), "Pitch");
        assert_eq!(EnvelopeType::Volume.short_label(), "VOL");
        assert_eq!(EnvelopeType::Panning.short_label(), "PAN");
        assert_eq!(EnvelopeType::Pitch.short_label(), "PIT");
    }

    #[test]
    fn test_envelope_type_cycle() {
        assert_eq!(EnvelopeType::Volume.next(), EnvelopeType::Panning);
        assert_eq!(EnvelopeType::Panning.next(), EnvelopeType::Pitch);
        assert_eq!(EnvelopeType::Pitch.next(), EnvelopeType::Volume);
        assert_eq!(EnvelopeType::Volume.prev(), EnvelopeType::Pitch);
        assert_eq!(EnvelopeType::Panning.prev(), EnvelopeType::Volume);
        assert_eq!(EnvelopeType::Pitch.prev(), EnvelopeType::Panning);
    }

    #[test]
    fn test_envelope_value_range() {
        assert_eq!(EnvelopeType::Volume.value_range(), (0.0, 1.0));
        assert_eq!(EnvelopeType::Panning.value_range(), (0.0, 1.0));
        assert_eq!(EnvelopeType::Pitch.value_range(), (-1.0, 1.0));
    }

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(1.0, EnvelopeType::Volume), "100.0%");
        assert_eq!(format_value(0.5, EnvelopeType::Volume), " 50.0%");
        assert_eq!(format_value(0.0, EnvelopeType::Panning), " C   ");
        assert_eq!(format_value(0.5, EnvelopeType::Panning), " 50.0R");
        assert_eq!(format_value(-0.5, EnvelopeType::Panning), " 50.0L");
        assert_eq!(format_value(0.0, EnvelopeType::Pitch), "  0  ");
        assert_eq!(format_value(0.5, EnvelopeType::Pitch), "+0.50");
        assert_eq!(format_value(-0.5, EnvelopeType::Pitch), "-0.50");
    }

    #[test]
    fn test_envelope_editor_state_defaults() {
        let state = EnvelopeEditorState::default();
        assert!(!state.focused);
        assert_eq!(state.envelope_type, EnvelopeType::Volume);
        assert!(state.selected_point.is_none());
    }

    #[test]
    fn test_focus_unfocus() {
        let mut state = EnvelopeEditorState::default();
        state.focus();
        assert!(state.focused);
        state.unfocus();
        assert!(!state.focused);
    }

    #[test]
    fn test_cycle_envelope_type() {
        let mut state = EnvelopeEditorState::default();
        assert_eq!(state.envelope_type, EnvelopeType::Volume);
        state.cycle_envelope_type();
        assert_eq!(state.envelope_type, EnvelopeType::Panning);
        state.cycle_envelope_type();
        assert_eq!(state.envelope_type, EnvelopeType::Pitch);
        state.cycle_envelope_type();
        assert_eq!(state.envelope_type, EnvelopeType::Volume);
    }

    #[test]
    fn test_select_point() {
        let mut state = EnvelopeEditorState::default();
        state.select_point(Some(5));
        assert_eq!(state.selected_point, Some(5));
        state.select_point(None);
        assert!(state.selected_point.is_none());
    }

    #[test]
    fn test_select_first_last_point() {
        let envelope = make_envelope();
        let mut state = EnvelopeEditorState::default();

        state.select_first_point(&envelope);
        assert_eq!(state.selected_point, Some(0));

        state.select_last_point(&envelope);
        assert_eq!(state.selected_point, Some(3));
    }

    #[test]
    fn test_draw_envelope_graphic() {
        let theme = Theme::default();
        let envelope = make_envelope();

        let graphic =
            draw_envelope_graphic(&envelope, EnvelopeType::Volume, 40, 10, &theme, Some(1));

        assert!(!graphic.is_empty());
    }

    #[test]
    fn test_draw_envelope_graphic_empty() {
        let theme = Theme::default();
        let envelope = Envelope::default();

        let graphic = draw_envelope_graphic(&envelope, EnvelopeType::Volume, 40, 10, &theme, None);

        assert!(!graphic.is_empty());
    }

    #[test]
    fn test_point_at_position_selects_nearby_point() {
        let envelope = make_envelope();
        let idx = point_at_position(&envelope, EnvelopeType::Volume, 40, 10, 7, 8);
        assert_eq!(idx, Some(0));
    }

    #[test]
    fn test_update_point_from_position_updates_frame_and_value() {
        let mut envelope = make_envelope();
        update_point_from_position(&mut envelope, EnvelopeType::Volume, 1, 40, 10, 20, 2);

        let point = &envelope.points[1];
        assert!(point.frame >= envelope.points[0].frame);
        assert!(point.frame <= envelope.points[2].frame);
        assert!(point.value > 0.5);
    }

    #[test]
    fn test_render_no_panic() {
        let inst = Instrument::new("TestInst");
        let state = EnvelopeEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_envelope_editor(frame, frame.area(), &inst, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_envelope_no_panic() {
        let mut inst = Instrument::new("TestInst");
        inst.volume_envelope = Some(make_envelope());
        let state = EnvelopeEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_envelope_editor(frame, frame.area(), &inst, &state, &theme);
            })
            .unwrap();
    }

    #[test]
    fn test_render_focused_no_panic() {
        let inst = Instrument::new("TestInst");
        let mut state = EnvelopeEditorState::default();
        state.focus();
        state.select_point(Some(0));
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_envelope_editor(frame, frame.area(), &inst, &state, &theme);
            })
            .unwrap();
    }
}
