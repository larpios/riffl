/// LFO editor for visual display and editing of LFO settings.
///
/// Provides LFO configuration within the instrument editor panel.
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use tracker_core::song::{Instrument, Lfo, LfoWaveform};

use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoType {
    Volume,
    Panning,
    Pitch,
}

impl LfoType {
    pub const ALL: &'static [LfoType] = &[LfoType::Volume, LfoType::Panning, LfoType::Pitch];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Volume => "Volume LFO",
            Self::Panning => "Panning LFO",
            Self::Pitch => "Pitch LFO",
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct LfoEditorState {
    pub focused: bool,
    pub lfo_type: LfoType,
    pub editing_field: LfoField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfoField {
    Waveform,
    Rate,
    Depth,
    Offset,
    Enabled,
}

impl Default for LfoEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            lfo_type: LfoType::Volume,
            editing_field: LfoField::Rate,
        }
    }
}

impl LfoEditorState {
    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn cycle_lfo_type(&mut self) {
        self.lfo_type = self.lfo_type.next();
    }

    pub fn prev_lfo_type(&mut self) {
        self.lfo_type = self.lfo_type.prev();
    }

    pub fn get_lfo<'a>(&self, instrument: &'a Instrument) -> Option<&'a Lfo> {
        match self.lfo_type {
            LfoType::Volume => instrument.volume_lfo.as_ref(),
            LfoType::Panning => instrument.panning_lfo.as_ref(),
            LfoType::Pitch => instrument.pitch_lfo.as_ref(),
        }
    }

    pub fn get_lfo_mut<'a>(&mut self, instrument: &'a mut Instrument) -> Option<&'a mut Lfo> {
        match self.lfo_type {
            LfoType::Volume => instrument.volume_lfo.as_mut(),
            LfoType::Panning => instrument.panning_lfo.as_mut(),
            LfoType::Pitch => instrument.pitch_lfo.as_mut(),
        }
    }

    pub fn ensure_lfo<'a>(&mut self, instrument: &'a mut Instrument) -> &'a mut Lfo {
        let lfo = match self.lfo_type {
            LfoType::Volume => {
                if instrument.volume_lfo.is_none() {
                    instrument.volume_lfo = Some(Lfo::default());
                }
                instrument.volume_lfo.as_mut().unwrap()
            }
            LfoType::Panning => {
                if instrument.panning_lfo.is_none() {
                    instrument.panning_lfo = Some(Lfo::default());
                }
                instrument.panning_lfo.as_mut().unwrap()
            }
            LfoType::Pitch => {
                if instrument.pitch_lfo.is_none() {
                    instrument.pitch_lfo = Some(Lfo::default());
                }
                instrument.pitch_lfo.as_mut().unwrap()
            }
        };
        lfo
    }

    pub fn cycle_waveform(&mut self, instrument: &mut Instrument) {
        let lfo = self.ensure_lfo(instrument);
        lfo.waveform = match lfo.waveform {
            LfoWaveform::Sine => LfoWaveform::Triangle,
            LfoWaveform::Triangle => LfoWaveform::Square,
            LfoWaveform::Square => LfoWaveform::Sawtooth,
            LfoWaveform::Sawtooth => LfoWaveform::ReverseSaw,
            LfoWaveform::ReverseSaw => LfoWaveform::Random,
            LfoWaveform::Random => LfoWaveform::Sine,
        };
    }

    pub fn toggle_enabled(&mut self, instrument: &mut Instrument) {
        let lfo = self.ensure_lfo(instrument);
        lfo.enabled = !lfo.enabled;
    }

    pub fn change_rate(&mut self, instrument: &mut Instrument, delta: f32) {
        let lfo = self.ensure_lfo(instrument);
        lfo.rate = (lfo.rate + delta).clamp(0.0, 20.0);
    }

    pub fn change_depth(&mut self, instrument: &mut Instrument, delta: f32) {
        let lfo = self.ensure_lfo(instrument);
        lfo.depth = (lfo.depth + delta).clamp(0.0, 1.0);
    }

    pub fn change_offset(&mut self, instrument: &mut Instrument, delta: f32) {
        let lfo = self.ensure_lfo(instrument);
        lfo.offset = (lfo.offset + delta).clamp(-1.0, 1.0);
    }

    pub fn cycle_field(&mut self) {
        self.editing_field = match self.editing_field {
            LfoField::Waveform => LfoField::Rate,
            LfoField::Rate => LfoField::Depth,
            LfoField::Depth => LfoField::Offset,
            LfoField::Offset => LfoField::Enabled,
            LfoField::Enabled => LfoField::Waveform,
        };
    }

    pub fn prev_field(&mut self) {
        self.editing_field = match self.editing_field {
            LfoField::Waveform => LfoField::Enabled,
            LfoField::Rate => LfoField::Waveform,
            LfoField::Depth => LfoField::Rate,
            LfoField::Offset => LfoField::Depth,
            LfoField::Enabled => LfoField::Offset,
        };
    }
}

fn waveform_label(waveform: LfoWaveform) -> &'static str {
    match waveform {
        LfoWaveform::Sine => "Sine",
        LfoWaveform::Triangle => "Tri",
        LfoWaveform::Square => "Sqr",
        LfoWaveform::Sawtooth => "Saw",
        LfoWaveform::ReverseSaw => "RSaw",
        LfoWaveform::Random => "Rnd",
    }
}

fn draw_lfo_waveform_graphic(
    waveform: LfoWaveform,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    if width < 5 || height < 3 {
        return vec![Line::from("")];
    }

    let mut lines = Vec::with_capacity(height);
    let mid_y = height / 2;

    for y in 0..height {
        let mut spans = Vec::with_capacity(width);
        for x in 0..width {
            let phase = x as f32 / width as f32;
            let value = match waveform {
                LfoWaveform::Sine => (phase * std::f32::consts::TAU).sin(),
                LfoWaveform::Triangle => {
                    if phase < 0.5 {
                        1.0 - phase * 4.0
                    } else {
                        -1.0 + (phase - 0.5) * 4.0
                    }
                }
                LfoWaveform::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                LfoWaveform::Sawtooth => 1.0 - phase * 2.0,
                LfoWaveform::ReverseSaw => -1.0 + phase * 2.0,
                LfoWaveform::Random => 0.0,
            };

            let screen_y = if value > 0.0 {
                mid_y.saturating_sub((value * (mid_y as f32 - 1.0)) as usize)
            } else {
                mid_y + ((-value) * (mid_y as f32 - 1.0)) as usize
            }
            .min(height - 1);

            let char = if screen_y == y {
                '●'
            } else if y == mid_y {
                '─'
            } else {
                ' '
            };
            spans.push(Span::raw(String::from(char)));
        }
        lines.push(Line::from(spans));
    }

    lines
}

pub fn render_lfo_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    state: &LfoEditorState,
    theme: &Theme,
) {
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };

    let lfo_type = state.lfo_type;
    let lfo = state.get_lfo(instrument);
    let lfo_color = lfo_type.color(theme);

    let enabled_str = if let Some(l) = lfo {
        if l.enabled {
            "[ON]"
        } else {
            "[OFF]"
        }
    } else {
        "[--]"
    };

    let title = format!(" {} LFO {} ", lfo_type.short_label(), enabled_str);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title, Style::default().fg(lfo_color)))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;

    if width < 15 || height < 6 {
        return;
    }

    let help_text = if state.focused {
        "Tab: cycle type  Space: cycle waveform  e: toggle  ↑↓: adjust value"
    } else {
        "l: edit LFO"
    };

    let mut lines: Vec<Line> = Vec::new();

    if let Some(lfo) = lfo {
        let graphic_height = height.saturating_sub(6).max(3);
        let waveform_lines =
            draw_lfo_waveform_graphic(lfo.waveform, width.saturating_sub(4), graphic_height);
        lines.extend(waveform_lines);
    } else {
        lines.push(Line::from(vec![Span::styled(
            "  LFO not configured",
            Style::default().fg(theme.text_dimmed),
        )]));
    }

    lines.push(Line::from(""));

    if let Some(lfo) = lfo {
        let waveform_style = if state.focused && state.editing_field == LfoField::Waveform {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text)
        };
        let rate_style = if state.focused && state.editing_field == LfoField::Rate {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text)
        };
        let depth_style = if state.focused && state.editing_field == LfoField::Depth {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text)
        };
        let offset_style = if state.focused && state.editing_field == LfoField::Offset {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text)
        };
        let enabled_style = if state.focused && state.editing_field == LfoField::Enabled {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled("  Wave: ", Style::default().fg(theme.text_dimmed)),
            Span::styled(
                format!("{:5}", waveform_label(lfo.waveform)),
                waveform_style,
            ),
            Span::styled("  Rate: ", Style::default().fg(theme.text_dimmed)),
            Span::styled(format!("{:5.1}", lfo.rate), rate_style),
            Span::styled(" Hz", Style::default().fg(theme.text_dimmed)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Depth:", Style::default().fg(theme.text_dimmed)),
            Span::styled(format!("{:5.2}", lfo.depth), depth_style),
            Span::styled("  Offset:", Style::default().fg(theme.text_dimmed)),
            Span::styled(format!("{:5.2}", lfo.offset), offset_style),
            Span::styled("  On:", Style::default().fg(theme.text_dimmed)),
            Span::styled(if lfo.enabled { "Y" } else { "N" }, enabled_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        format!(" {}", help_text),
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
