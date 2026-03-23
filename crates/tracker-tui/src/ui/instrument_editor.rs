/// Instrument editor panel — shown below the instrument list when an instrument is selected.
///
/// Fields: Name, Base Note, Volume, Finetune
/// Keys (when editor is focused):
///   Tab / Shift+Tab  — cycle fields
///   +/-              — increment/decrement numeric fields
///   e / Enter        — enter text-edit mode for Name field
///   Esc              — exit text-edit / exit editor focus
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

/// Which field is currently focused in the instrument editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentField {
    Name,
    BaseNote,
    Volume,
    Finetune,
    LoopMode,
    LoopStart,
    LoopEnd,
}

impl InstrumentField {
    pub const ALL: &'static [InstrumentField] = &[
        InstrumentField::Name,
        InstrumentField::BaseNote,
        InstrumentField::Volume,
        InstrumentField::Finetune,
        InstrumentField::LoopMode,
        InstrumentField::LoopStart,
        InstrumentField::LoopEnd,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::BaseNote,
            Self::BaseNote => Self::Volume,
            Self::Volume => Self::Finetune,
            Self::Finetune => Self::LoopMode,
            Self::LoopMode => Self::LoopStart,
            Self::LoopStart => Self::LoopEnd,
            Self::LoopEnd => Self::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::LoopEnd,
            Self::BaseNote => Self::Name,
            Self::Volume => Self::BaseNote,
            Self::Finetune => Self::Volume,
            Self::LoopMode => Self::Finetune,
            Self::LoopStart => Self::LoopMode,
            Self::LoopEnd => Self::LoopStart,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::BaseNote => "Base Note",
            Self::Volume => "Volume",
            Self::Finetune => "Finetune",
            Self::LoopMode => "Loop Mode",
            Self::LoopStart => "Loop Start",
            Self::LoopEnd => "Loop End",
        }
    }
}

/// State for the instrument editor panel.
#[derive(Debug, Clone)]
pub struct InstrumentEditorState {
    /// Whether the editor panel is focused (vs the list above).
    pub focused: bool,
    /// Currently focused field.
    pub field: InstrumentField,
    /// Whether we are in inline text-edit mode for the Name field.
    pub text_editing: bool,
    /// Text buffer used when editing the Name field.
    pub input_buffer: String,
}

impl Default for InstrumentEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            field: InstrumentField::Name,
            text_editing: false,
            input_buffer: String::new(),
        }
    }
}

impl InstrumentEditorState {
    /// Enter focus on the editor panel, starting at the Name field.
    pub fn focus(&mut self) {
        self.focused = true;
        self.field = InstrumentField::Name;
        self.text_editing = false;
    }

    /// Leave editor focus.
    pub fn unfocus(&mut self) {
        self.focused = false;
        self.text_editing = false;
    }

    /// Move to the next field.
    pub fn next_field(&mut self) {
        self.text_editing = false;
        self.field = self.field.next();
    }

    /// Move to the previous field.
    pub fn prev_field(&mut self) {
        self.text_editing = false;
        self.field = self.field.prev();
    }

    /// Start text-editing the Name field.
    pub fn start_text_edit(&mut self, current_name: &str) {
        if self.field == InstrumentField::Name {
            self.text_editing = true;
            self.input_buffer = current_name.to_string();
        }
    }

    /// Finish text editing, returning the entered name (or None if cancelled).
    pub fn finish_text_edit(&mut self) -> Option<String> {
        if self.text_editing {
            self.text_editing = false;
            Some(self.input_buffer.trim().to_string())
        } else {
            None
        }
    }

    /// Cancel text editing without applying.
    pub fn cancel_text_edit(&mut self) {
        self.text_editing = false;
        self.input_buffer.clear();
    }
}

/// Build a 2-row waveform visualization from sample data.
///
/// Each character cell uses `▀`, `▄`, or `█` to show positive and negative
/// peak amplitudes within a time window. Returns two `Line`s (top row, bottom row).
/// Loop start/end positions are marked with dim vertical-bar characters.
fn build_waveform(sample: &Sample, width: usize, theme: &Theme) -> [Line<'static>; 2] {
    let data = sample.data();
    let channels = sample.channels() as usize;
    let frame_count = sample.frame_count();

    if frame_count == 0 || width == 0 || channels == 0 {
        let dash = "─".repeat(width);
        let s = Style::default().fg(theme.text_dimmed);
        return [
            Line::from(Span::styled(dash.clone(), s)),
            Line::from(Span::styled(dash, s)),
        ];
    }

    // Precompute per-column peak positive and peak negative amplitudes.
    let mut peaks: Vec<(f32, f32)> = Vec::with_capacity(width);
    for col in 0..width {
        let start_frame = (col * frame_count) / width;
        let end_frame = ((col + 1) * frame_count / width)
            .max(start_frame + 1)
            .min(frame_count);

        let mut peak_pos: f32 = 0.0;
        let mut peak_neg: f32 = 0.0;
        for frame_idx in start_frame..end_frame {
            // Use left channel only (index 0 of each interleaved frame).
            let v = data[frame_idx * channels];
            if v > peak_pos {
                peak_pos = v;
            }
            if -v > peak_neg {
                peak_neg = -v;
            }
        }
        peaks.push((peak_pos, peak_neg));
    }

    // Loop marker column positions (if the sample has loop points).
    let has_loop = sample.loop_mode != tracker_core::audio::sample::LoopMode::NoLoop;
    let loop_start_col = if has_loop {
        Some(sample.loop_start * width / frame_count.max(1))
    } else {
        None
    };
    let loop_end_col = if has_loop {
        Some(sample.loop_end * width / frame_count.max(1))
    } else {
        None
    };

    let wf_style = Style::default().fg(theme.primary);
    let loop_start_style = Style::default()
        .fg(theme.status_success)
        .add_modifier(Modifier::BOLD);
    let loop_end_style = Style::default()
        .fg(theme.status_error)
        .add_modifier(Modifier::BOLD);
    let loop_region_style = Style::default().fg(theme.text_dimmed);
    let center_style = Style::default().fg(theme.text_dimmed);

    // Threshold below which we treat the signal as silence at that level.
    const THRESH: f32 = 0.03;

    let mut top_styles: Vec<(String, Style)> = Vec::new();
    let mut bot_styles: Vec<(String, Style)> = Vec::new();

    for (col, &(peak_pos, peak_neg)) in peaks.iter().enumerate().take(width) {
        let is_loop_start = loop_start_col == Some(col);
        let is_loop_end = loop_end_col == Some(col);

        if is_loop_start {
            top_styles.push(("◁".to_string(), loop_start_style));
            bot_styles.push(("◁".to_string(), loop_start_style));
            continue;
        }

        if is_loop_end {
            top_styles.push(("▷".to_string(), loop_end_style));
            bot_styles.push(("▷".to_string(), loop_end_style));
            continue;
        }

        let in_loop_region = has_loop
            && loop_start_col.is_some_and(|s| col > s)
            && loop_end_col.is_some_and(|e| col < e);

        // Top row: upper half = positive peak > 0.5, lower half = positive peak > THRESH.
        let top_upper = peak_pos > 0.5;
        let top_lower = peak_pos > THRESH;
        let top_ch = match (top_upper, top_lower) {
            (true, _) => '█',
            (false, true) => '▄',
            _ => '─',
        };
        let top_style = if top_upper || top_lower {
            if in_loop_region {
                loop_region_style
            } else {
                wf_style
            }
        } else {
            center_style
        };

        // Bottom row: upper half = negative peak > THRESH, lower half = negative peak > 0.5.
        let bot_upper = peak_neg > THRESH;
        let bot_lower = peak_neg > 0.5;
        let bot_ch = match (bot_upper, bot_lower) {
            (_, true) => '█',
            (true, false) => '▀',
            _ => '─',
        };
        let bot_style = if bot_upper || bot_lower {
            if in_loop_region {
                loop_region_style
            } else {
                wf_style
            }
        } else {
            center_style
        };

        top_styles.push((top_ch.to_string(), top_style));
        bot_styles.push((bot_ch.to_string(), bot_style));
    }

    // Coalesce consecutive runs of the same style for efficiency.
    fn coalesce(parts: Vec<(String, Style)>) -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut cur_text = String::new();
        let mut cur_style = Style::default();
        for (text, style) in parts {
            if style == cur_style {
                cur_text.push_str(&text);
            } else {
                if !cur_text.is_empty() {
                    spans.push(Span::styled(cur_text.clone(), cur_style));
                }
                cur_text = text;
                cur_style = style;
            }
        }
        if !cur_text.is_empty() {
            spans.push(Span::styled(cur_text, cur_style));
        }
        Line::from(spans)
    }

    [coalesce(top_styles), coalesce(bot_styles)]
}

/// Render the instrument editor panel for `instrument`.
/// `area` is the panel's allocated rect.
/// `sample` is the audio sample assigned to this instrument (if loaded).
pub fn render_instrument_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    state: &InstrumentEditorState,
    theme: &Theme,
    sample: Option<&Sample>,
) {
    let title = format!(" Edit: {} ", instrument.name);
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    let mut lines: Vec<Line> = Vec::new();

    // Helper: style a field label+value row
    let field_style = |field: InstrumentField| {
        if state.focused && state.field == field {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        }
    };
    let label_style = |field: InstrumentField| {
        if state.focused && state.field == field {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_secondary)
        }
    };

    // Blank line at top for breathing room
    lines.push(Line::from(""));

    // ── Name ─────────────────────────────────────────────────────────
    let name_value = if state.focused && state.field == InstrumentField::Name && state.text_editing
    {
        format!("{}▌", state.input_buffer)
    } else {
        instrument.name.clone()
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::Name.label()),
            label_style(InstrumentField::Name),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<24}", name_value),
            field_style(InstrumentField::Name),
        ),
        Span::raw("   "),
        Span::styled("e/Enter: edit name", Style::default().fg(theme.text_dimmed)),
    ]));

    lines.push(Line::from(""));

    // ── Base Note ────────────────────────────────────────────────────
    let base_note_str = instrument.base_note.display_str();
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::BaseNote.label()),
            label_style(InstrumentField::BaseNote),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<6}", base_note_str),
            field_style(InstrumentField::BaseNote),
        ),
        Span::raw("   "),
        Span::styled(
            format!("(MIDI {:3})", instrument.base_note.midi_note()),
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Volume ───────────────────────────────────────────────────────
    let vol_pct = (instrument.volume * 100.0).round() as u32;
    let vol_bar = {
        let filled = (vol_pct / 5) as usize; // 0-20 blocks
        format!(
            "{}{}",
            "█".repeat(filled.min(20)),
            "░".repeat(20usize.saturating_sub(filled))
        )
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::Volume.label()),
            label_style(InstrumentField::Volume),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:3}%", vol_pct),
            field_style(InstrumentField::Volume),
        ),
        Span::raw("  "),
        Span::styled(vol_bar, Style::default().fg(theme.primary)),
    ]));

    lines.push(Line::from(""));

    // ── Finetune ─────────────────────────────────────────────────────
    // Each unit = 1/8 semitone = 12.5 cents
    let ft = instrument.finetune;
    let cents = ft as f32 * 12.5;
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::Finetune.label()),
            label_style(InstrumentField::Finetune),
        ),
        Span::raw("  "),
        Span::styled(format!("{:+2}", ft), field_style(InstrumentField::Finetune)),
        Span::raw("   "),
        Span::styled(
            format!("({:+.1} cents)", cents),
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Loop Mode ───────────────────────────────────────────────────────
    let loop_mode_str = sample
        .map(|s| match s.loop_mode {
            tracker_core::audio::sample::LoopMode::NoLoop => "Off",
            tracker_core::audio::sample::LoopMode::Forward => "Forward",
            tracker_core::audio::sample::LoopMode::PingPong => "Ping-Pong",
        })
        .unwrap_or("Off");
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::LoopMode.label()),
            label_style(InstrumentField::LoopMode),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", loop_mode_str),
            field_style(InstrumentField::LoopMode),
        ),
        Span::raw("   "),
        Span::styled("space: cycle", Style::default().fg(theme.text_dimmed)),
    ]));

    lines.push(Line::from(""));

    // ── Loop Start ─────────────────────────────────────────────────────
    let loop_start_val = sample.map(|s| s.loop_start).unwrap_or(0);
    let sample_rate = sample.map(|s| s.sample_rate()).unwrap_or(44100);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::LoopStart.label()),
            label_style(InstrumentField::LoopStart),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:>6}", loop_start_val),
            field_style(InstrumentField::LoopStart),
        ),
        Span::raw("   "),
        Span::styled(
            format!("({:.2}s)", loop_start_val as f32 / sample_rate as f32),
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Loop End ───────────────────────────────────────────────────────
    let loop_end_val = sample.map(|s| s.loop_end).unwrap_or(0);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<10}", InstrumentField::LoopEnd.label()),
            label_style(InstrumentField::LoopEnd),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{:>6}", loop_end_val),
            field_style(InstrumentField::LoopEnd),
        ),
        Span::raw("   "),
        Span::styled(
            format!("({:.2}s)", loop_end_val as f32 / sample_rate as f32),
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    lines.push(Line::from(""));

    // ── Sample path ──────────────────────────────────────────────────
    let sample_str = instrument
        .sample_path
        .as_deref()
        .map(|p| {
            // Show only the filename part
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
                .to_string()
        })
        .unwrap_or_else(|| "— no sample —".to_string());
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Sample    ", Style::default().fg(theme.text_secondary)),
        Span::raw("  "),
        Span::styled(sample_str, Style::default().fg(theme.text_dimmed)),
        Span::raw("   "),
        Span::styled("Ctrl+F: assign", Style::default().fg(theme.text_dimmed)),
    ]));

    // ── Waveform ──────────────────────────────────────────────────────
    // Show a 2-row waveform if we have sample data and enough space.
    // We need at least 4 more rows: 1 blank + 1 header + 2 waveform rows.
    let content_lines_before_waveform = lines.len();
    let visible = inner.height as usize;
    let waveform_rows = 4; // blank + label + top + bottom
    let hints_rows = 2; // blank + hints line
    let has_waveform_space = visible >= content_lines_before_waveform + waveform_rows + hints_rows;

    if let Some(s) = sample.filter(|s| !s.is_empty() && has_waveform_space) {
        lines.push(Line::from(""));

        // Duration label, e.g. "0.42s · 44100Hz · mono"
        let ch_str = if s.channels() == 1 { "mono" } else { "stereo" };
        let dur_str = format!("{:.2}s · {}Hz · {}", s.duration(), s.sample_rate(), ch_str,);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Waveform  ", Style::default().fg(theme.text_secondary)),
            Span::styled(dur_str, Style::default().fg(theme.text_dimmed)),
        ]));

        // Waveform area width = inner width minus 2-char left indent.
        let wf_width = (inner.width as usize).saturating_sub(2);
        let [top_row, bot_row] = build_waveform(s, wf_width, theme);

        let mut top_spans = vec![Span::raw("  ")];
        top_spans.extend(top_row.spans);
        lines.push(Line::from(top_spans));

        let mut bot_spans = vec![Span::raw("  ")];
        bot_spans.extend(bot_row.spans);
        lines.push(Line::from(bot_spans));
    }

    // Pad and show key hints at the bottom
    let content_lines = lines.len();
    for _ in content_lines..visible.saturating_sub(2) {
        lines.push(Line::from(""));
    }
    if visible >= 2 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Tab: next  +/-: change  e/Enter: edit  space: loop mode  Esc: back",
                Style::default().fg(theme.text_dimmed),
            ),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracker_core::{pattern::note::Note, song::Instrument};

    fn make_inst() -> Instrument {
        let mut i = Instrument::new("TestInst");
        i.base_note = Note::simple(tracker_core::pattern::note::Pitch::C, 4);
        i.volume = 0.75;
        i.finetune = 2;
        i
    }

    #[test]
    fn test_field_cycle() {
        let mut s = InstrumentEditorState::default();
        s.field = InstrumentField::Name;
        s.next_field();
        assert_eq!(s.field, InstrumentField::BaseNote);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Volume);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Finetune);
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopMode);
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopStart);
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopEnd);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Name);
    }

    #[test]
    fn test_text_edit_lifecycle() {
        let mut s = InstrumentEditorState::default();
        s.focus();
        s.start_text_edit("hello");
        assert!(s.text_editing);
        assert_eq!(s.input_buffer, "hello");
        let result = s.finish_text_edit();
        assert_eq!(result, Some("hello".to_string()));
        assert!(!s.text_editing);
    }

    #[test]
    fn test_render_no_panic() {
        let inst = make_inst();
        let state = InstrumentEditorState::default();
        let theme = Theme::default();
        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_instrument_editor(frame, frame.area(), &inst, &state, &theme, None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_sample_no_panic() {
        use tracker_core::audio::sample::Sample;

        let inst = make_inst();
        let state = InstrumentEditorState::default();
        let theme = Theme::default();

        // Sine wave — gives non-trivial waveform with positive and negative peaks.
        let data: Vec<f32> = (0..4410)
            .map(|i| (i as f32 * std::f32::consts::TAU * 440.0 / 44100.0).sin())
            .collect();
        let sample = Sample::new(data, 44100, 1, Some("sine".to_string()));

        let backend = ratatui::backend::TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_instrument_editor(frame, frame.area(), &inst, &state, &theme, Some(&sample));
            })
            .unwrap();
    }

    #[test]
    fn test_build_waveform_empty_sample() {
        use tracker_core::audio::sample::Sample;
        let theme = Theme::default();
        let sample = Sample::default();
        let [top, bot] = build_waveform(&sample, 40, &theme);
        // Should return dash lines for empty sample.
        assert!(!top.spans.is_empty());
        assert!(!bot.spans.is_empty());
    }

    #[test]
    fn test_build_waveform_sine() {
        use tracker_core::audio::sample::Sample;
        let theme = Theme::default();
        let data: Vec<f32> = (0..4410)
            .map(|i| (i as f32 * std::f32::consts::TAU * 440.0 / 44100.0).sin())
            .collect();
        let sample = Sample::new(data, 44100, 1, None);
        let [top, bot] = build_waveform(&sample, 60, &theme);
        // Both rows must be non-empty and contain waveform content.
        let top_text: String = top.spans.iter().map(|s| s.content.as_ref()).collect();
        let bot_text: String = bot.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(top_text.chars().count(), 60);
        assert_eq!(bot_text.chars().count(), 60);
        // A sine wave has both positive and negative peaks — should see █ in both rows.
        assert!(top_text.contains('█') || top_text.contains('▄'));
        assert!(bot_text.contains('█') || bot_text.contains('▀'));
    }
}
