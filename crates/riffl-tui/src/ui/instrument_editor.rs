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

use riffl_core::audio::sample::Sample;
use riffl_core::pattern::note::{Note, Pitch};
use riffl_core::song::Instrument;

use crate::ui::theme::Theme;

/// Tabs within the instrument editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentTab {
    General,
    Sample,
    Envelopes,
    Lfos,
}

impl InstrumentTab {
    pub const ALL: &'static [InstrumentTab] = &[
        InstrumentTab::General,
        InstrumentTab::Sample,
        InstrumentTab::Envelopes,
        InstrumentTab::Lfos,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Sample => "Sample",
            Self::Envelopes => "Envelopes",
            Self::Lfos => "LFOs",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::General => Self::Sample,
            Self::Sample => Self::Envelopes,
            Self::Envelopes => Self::Lfos,
            Self::Lfos => Self::General,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::General => Self::Lfos,
            Self::Sample => Self::General,
            Self::Envelopes => Self::Sample,
            Self::Lfos => Self::Envelopes,
        }
    }
}

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
    KeyzoneList,
    KeyzoneNoteMin,
    KeyzoneNoteMax,
    KeyzoneVelMin,
    KeyzoneVelMax,
    KeyzoneSample,
    KeyzoneBaseNote,
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
        InstrumentField::KeyzoneList,
        InstrumentField::KeyzoneNoteMin,
        InstrumentField::KeyzoneNoteMax,
        InstrumentField::KeyzoneVelMin,
        InstrumentField::KeyzoneVelMax,
        InstrumentField::KeyzoneSample,
        InstrumentField::KeyzoneBaseNote,
    ];

    pub fn next(self, tab: InstrumentTab) -> Self {
        match tab {
            InstrumentTab::General => match self {
                Self::Name => Self::BaseNote,
                Self::BaseNote => Self::Volume,
                Self::Volume => Self::Finetune,
                Self::Finetune => Self::KeyzoneList,
                Self::KeyzoneList => Self::KeyzoneNoteMin,
                Self::KeyzoneNoteMin => Self::KeyzoneNoteMax,
                Self::KeyzoneNoteMax => Self::KeyzoneVelMin,
                Self::KeyzoneVelMin => Self::KeyzoneVelMax,
                Self::KeyzoneVelMax => Self::KeyzoneSample,
                Self::KeyzoneSample => Self::KeyzoneBaseNote,
                Self::KeyzoneBaseNote => Self::Name,
                _ => Self::Name,
            },
            InstrumentTab::Sample => match self {
                Self::LoopMode => Self::LoopStart,
                Self::LoopStart => Self::LoopEnd,
                Self::LoopEnd => Self::LoopMode,
                _ => Self::LoopMode,
            },
            _ => self,
        }
    }

    pub fn prev(self, tab: InstrumentTab) -> Self {
        match tab {
            InstrumentTab::General => match self {
                Self::Name => Self::KeyzoneBaseNote,
                Self::BaseNote => Self::Name,
                Self::Volume => Self::BaseNote,
                Self::Finetune => Self::Volume,
                Self::KeyzoneList => Self::Finetune,
                Self::KeyzoneNoteMin => Self::KeyzoneList,
                Self::KeyzoneNoteMax => Self::KeyzoneNoteMin,
                Self::KeyzoneVelMin => Self::KeyzoneNoteMax,
                Self::KeyzoneVelMax => Self::KeyzoneVelMin,
                Self::KeyzoneSample => Self::KeyzoneVelMax,
                Self::KeyzoneBaseNote => Self::KeyzoneSample,
                _ => Self::Name,
            },
            InstrumentTab::Sample => match self {
                Self::LoopMode => Self::LoopEnd,
                Self::LoopStart => Self::LoopMode,
                Self::LoopEnd => Self::LoopStart,
                _ => Self::LoopMode,
            },
            _ => self,
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
            Self::KeyzoneList => "Keyzones",
            Self::KeyzoneNoteMin => "Key Min",
            Self::KeyzoneNoteMax => "Key Max",
            Self::KeyzoneVelMin => "Vel Min",
            Self::KeyzoneVelMax => "Vel Max",
            Self::KeyzoneSample => "Sample",
            Self::KeyzoneBaseNote => "Root Note",
        }
    }

    /// Returns true if this field can be adjusted via mouse drag.
    pub fn is_draggable(self) -> bool {
        matches!(
            self,
            Self::BaseNote
                | Self::Volume
                | Self::Finetune
                | Self::LoopStart
                | Self::LoopEnd
                | Self::KeyzoneNoteMin
                | Self::KeyzoneNoteMax
                | Self::KeyzoneVelMin
                | Self::KeyzoneVelMax
        )
    }
}

/// Get the InstrumentField at the given row offset within the instrument editor.
/// Returns None if the row doesn't correspond to any field or the field isn't draggable.
/// The row offset is relative to the inner area (after border).
pub fn field_at_row(row_offset: u16) -> Option<InstrumentField> {
    match row_offset {
        1 => Some(InstrumentField::Name),
        3 => Some(InstrumentField::BaseNote),
        5 => Some(InstrumentField::Volume),
        7 => Some(InstrumentField::Finetune),
        9 => Some(InstrumentField::LoopMode),
        11 => Some(InstrumentField::LoopStart),
        13 => Some(InstrumentField::LoopEnd),
        15 => Some(InstrumentField::KeyzoneList),
        17 => Some(InstrumentField::KeyzoneNoteMin),
        19 => Some(InstrumentField::KeyzoneNoteMax),
        21 => Some(InstrumentField::KeyzoneVelMin),
        23 => Some(InstrumentField::KeyzoneVelMax),
        25 => Some(InstrumentField::KeyzoneSample),
        27 => Some(InstrumentField::KeyzoneBaseNote),
        _ => None,
    }
}

/// State for the instrument editor panel.
#[derive(Debug, Clone)]
pub struct InstrumentEditorState {
    /// Whether the editor panel is focused (vs the list above).
    pub focused: bool,
    /// Currently active tab.
    pub tab: InstrumentTab,
    /// Currently focused field.
    pub field: InstrumentField,
    /// Whether we are in inline text-edit mode for the Name field.
    pub text_editing: bool,
    /// Text buffer used when editing the Name field.
    pub input_buffer: String,
    /// Currently dragging field (for mouse drag interaction).
    pub dragging_field: Option<InstrumentField>,
    /// Last mouse position seen during a drag.
    pub drag_last_position: Option<(u16, u16)>,
    /// Currently selected keyzone index (for editing).
    pub selected_keyzone: Option<usize>,
}

impl Default for InstrumentEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            tab: InstrumentTab::General,
            field: InstrumentField::Name,
            text_editing: false,
            input_buffer: String::new(),
            dragging_field: None,
            drag_last_position: None,
            selected_keyzone: None,
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

    /// Move to the next tab.
    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.reset_field_for_tab();
    }

    /// Move to the previous tab.
    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.reset_field_for_tab();
    }

    fn reset_field_for_tab(&mut self) {
        self.text_editing = false;
        self.field = match self.tab {
            InstrumentTab::General => InstrumentField::Name,
            InstrumentTab::Sample => InstrumentField::LoopMode,
            InstrumentTab::Envelopes => InstrumentField::Name, // Placeholder as envelopes manage their own
            InstrumentTab::Lfos => InstrumentField::Name,      // Placeholder
        };
    }

    /// Move to the next field.
    pub fn next_field(&mut self) {
        self.text_editing = false;
        self.field = self.field.next(self.tab);
    }

    /// Move to the previous field.
    pub fn prev_field(&mut self) {
        self.text_editing = false;
        self.field = self.field.prev(self.tab);
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

    /// Start dragging a field (on mouse down).
    pub fn start_drag(&mut self, field: InstrumentField, column: u16, row: u16) {
        self.dragging_field = Some(field);
        self.drag_last_position = Some((column, row));
    }

    /// End dragging (on mouse up).
    pub fn end_drag(&mut self) {
        self.dragging_field = None;
        self.drag_last_position = None;
    }

    /// Get the currently dragging field, if any.
    pub fn dragging(&self) -> Option<InstrumentField> {
        self.dragging_field
    }

    /// Get the currently selected keyzone index for editing, if any.
    pub fn selected_keyzone_index(&self) -> Option<usize> {
        self.selected_keyzone
    }

    /// Update the drag position and return the delta from the previous event.
    pub fn update_drag_position(&mut self, column: u16, row: u16) -> Option<(i16, i16)> {
        let (prev_col, prev_row) = self.drag_last_position?;
        self.drag_last_position = Some((column, row));
        Some((
            column as i16 - prev_col as i16,
            row as i16 - prev_row as i16,
        ))
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
    let has_loop = sample.loop_mode != riffl_core::audio::sample::LoopMode::NoLoop;
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
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };

    // Title: instrument name + hint when unfocused
    let title_spans = if state.focused {
        vec![
            Span::raw(" "),
            Span::styled(
                &instrument.name,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::raw(" "),
            Span::styled(
                &instrument.name,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Enter: edit ", Style::default().fg(theme.text_dimmed)),
        ]
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Line::from(title_spans))
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;
    if width < 20 || height < 2 {
        return;
    }

    // Split each row into left/right halves for a 2-column layout.
    let half = width / 2;

    let active = |field: InstrumentField| state.focused && state.field == field;
    let field_style = |field: InstrumentField| {
        if active(field) {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        }
    };
    let label_style = |field: InstrumentField| {
        if active(field) {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.cursor_normal_bg)
        } else {
            Style::default().fg(theme.text_secondary)
        }
    };
    let dim = Style::default().fg(theme.text_dimmed);

    // Build a single 2-column row.  Each half is `half` chars wide.
    // `left_spans` and `right_spans` are the contents for each half.
    let make_row = |left: Vec<Span<'static>>, right: Vec<Span<'static>>| -> Line<'static> {
        // Pad left side to `half` chars (approximate — spans don't track width cleanly,
        // so we just concatenate and let the terminal clip).
        let mut spans = left;
        // Separator
        spans.push(Span::styled("  ", dim));
        spans.extend(right);
        Line::from(spans)
    };

    let sample_rate = sample.map(|s| s.sample_rate()).unwrap_or(44100);

    // ── Row 1: Name (left) | Sample file (right) ─────────────────────
    let name_value = if active(InstrumentField::Name) && state.text_editing {
        format!("{}▌", state.input_buffer)
    } else {
        instrument.name.clone()
    };
    let name_w = half.saturating_sub(14);
    let sample_str = instrument
        .sample_path
        .as_deref()
        .map(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
                .to_string()
        })
        .unwrap_or_else(|| "— none —".to_string());
    let sample_w = half.saturating_sub(14);
    let row1 = make_row(
        vec![
            Span::styled(format!("{:<9}", "Name"), label_style(InstrumentField::Name)),
            Span::raw(" "),
            Span::styled(
                format!("{:<w$}", name_value, w = name_w.max(1)),
                field_style(InstrumentField::Name),
            ),
        ],
        vec![
            Span::styled("Sample   ", dim),
            Span::styled(format!("{:<w$}", sample_str, w = sample_w.max(1)), dim),
        ],
    );

    // ── Row 2: Base Note (left) | Volume bar (right) ─────────────────
    let base_note_str = instrument.base_note.display_str();
    let vol_pct = (instrument.volume * 100.0).round() as u32;
    let bar_w = half.saturating_sub(22).max(4);
    let filled = (vol_pct as usize * bar_w / 100).min(bar_w);
    let vol_bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_w - filled));
    let row2 = make_row(
        vec![
            Span::styled(
                format!("{:<9}", "Base Note"),
                label_style(InstrumentField::BaseNote),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<5}", base_note_str),
                field_style(InstrumentField::BaseNote),
            ),
            Span::styled(format!(" MIDI {:3}", instrument.base_note.midi_note()), dim),
        ],
        vec![
            Span::styled(
                format!("{:<9}", "Volume"),
                label_style(InstrumentField::Volume),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:3}%", vol_pct),
                field_style(InstrumentField::Volume),
            ),
            Span::raw(" "),
            Span::styled(vol_bar, Style::default().fg(theme.primary)),
        ],
    );

    // ── Row 3: Finetune (left) | Loop mode + points (right) ──────────
    let ft = instrument.finetune;
    let cents = ft as f32 * 12.5;
    let loop_mode_str = sample
        .map(|s| match s.loop_mode {
            riffl_core::audio::sample::LoopMode::NoLoop => "Off",
            riffl_core::audio::sample::LoopMode::Forward => "Fwd",
            riffl_core::audio::sample::LoopMode::PingPong => "P-P",
        })
        .unwrap_or("Off");
    let loop_start_val = sample.map(|s| s.loop_start).unwrap_or(0);
    let loop_end_val = sample.map(|s| s.loop_end).unwrap_or(0);
    let row3 = make_row(
        vec![
            Span::styled(
                format!("{:<9}", "Finetune"),
                label_style(InstrumentField::Finetune),
            ),
            Span::raw(" "),
            Span::styled(format!("{:+3}", ft), field_style(InstrumentField::Finetune)),
            Span::styled(format!(" ({:+.0}c)", cents), dim),
        ],
        vec![
            Span::styled(
                format!("{:<9}", "Loop"),
                label_style(InstrumentField::LoopMode),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:<3}", loop_mode_str),
                field_style(InstrumentField::LoopMode),
            ),
            Span::styled("  Srt:", dim),
            Span::styled(
                format!("{}", loop_start_val),
                field_style(InstrumentField::LoopStart),
            ),
            Span::styled("  End:", dim),
            Span::styled(
                format!("{}", loop_end_val),
                field_style(InstrumentField::LoopEnd),
            ),
        ],
    );

    // ── Row 4: Keyzones ──────────────────────────────────────────────
    let keyzones = &instrument.keyzones;
    let keyzone_count = keyzones.len();
    let kz_summary = if keyzone_count == 0 {
        "none".to_string()
    } else if let Some(kz_idx) = state.selected_keyzone.filter(|&i| i < keyzone_count) {
        let kz = &keyzones[kz_idx];
        let note_min_str = (|| {
            let pitch = Pitch::from_semitone(kz.note_min % 12)?;
            Some(Note::simple(pitch, kz.note_min / 12).display_str())
        })()
        .unwrap_or_else(|| "---".to_string());
        let note_max_str = (|| {
            let pitch = Pitch::from_semitone(kz.note_max % 12)?;
            Some(Note::simple(pitch, kz.note_max / 12).display_str())
        })()
        .unwrap_or_else(|| "---".to_string());
        format!(
            "[{}] {}-{} vel:{}-{} smp:{}",
            kz_idx, note_min_str, note_max_str, kz.velocity_min, kz.velocity_max, kz.sample_index
        )
    } else {
        format!("{} zone(s)  +/-: select", keyzone_count)
    };
    let row4 = Line::from(vec![
        Span::styled(
            format!("{:<9}", "Keyzones"),
            label_style(InstrumentField::KeyzoneList),
        ),
        Span::raw(" "),
        Span::styled(kz_summary, field_style(InstrumentField::KeyzoneList)),
    ]);

    let mut lines: Vec<Line> = vec![row1, row2, row3, row4];

    // ── Waveform (2 rows) if space allows ────────────────────────────
    if let Some(s) = sample.filter(|s| !s.is_empty()) {
        if height >= lines.len() + 3 {
            let ch_str = if s.channels() == 1 { "mono" } else { "stereo" };
            let dur_str = format!("{:.2}s · {}Hz · {}", s.duration(), s.sample_rate(), ch_str);
            lines.push(Line::from(vec![
                Span::styled("─── Waveform ", dim),
                Span::styled(dur_str, dim),
            ]));
            let wf_width = width.saturating_sub(0);
            let [top_row, bot_row] = build_waveform(s, wf_width, theme);
            lines.push(top_row);
            lines.push(bot_row);
        }
    }

    // ── Loop start/end detail rows if those fields are active ─────────
    if active(InstrumentField::LoopStart) || active(InstrumentField::LoopEnd) {
        lines.push(Line::from(vec![Span::styled(
            format!(
                "  Srt: {:>6} ({:.2}s)   End: {:>6} ({:.2}s)",
                loop_start_val,
                loop_start_val as f32 / sample_rate as f32,
                loop_end_val,
                loop_end_val as f32 / sample_rate as f32,
            ),
            dim,
        )]));
    }

    // ── Hint bar ─────────────────────────────────────────────────────
    while lines.len() < height.saturating_sub(1) {
        lines.push(Line::from(""));
    }
    if height >= 2 {
        let hint = if state.focused {
            "  j/k: field   +/-: value   e: name   spc: loop   Esc: exit"
        } else {
            "  Enter: edit instrument"
        };
        lines.push(Line::from(Span::styled(hint, dim)));
    }

    let content = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(content, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffl_core::{pattern::note::Note, song::Instrument};

    fn make_inst() -> Instrument {
        let mut i = Instrument::new("TestInst");
        i.base_note = Note::simple(riffl_core::pattern::note::Pitch::C, 4);
        i.volume = 0.75;
        i.finetune = 2;
        i
    }

    #[test]
    fn test_field_cycle() {
        let mut s = InstrumentEditorState::default();
        // General Tab
        s.tab = InstrumentTab::General;
        s.field = InstrumentField::Name;
        s.next_field();
        assert_eq!(s.field, InstrumentField::BaseNote);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Volume);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Finetune);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneList);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneNoteMin);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneNoteMax);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneVelMin);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneVelMax);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneSample);
        s.next_field();
        assert_eq!(s.field, InstrumentField::KeyzoneBaseNote);
        s.next_field();
        assert_eq!(s.field, InstrumentField::Name);

        // Sample Tab
        s.tab = InstrumentTab::Sample;
        s.field = InstrumentField::LoopMode;
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopStart);
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopEnd);
        s.next_field();
        assert_eq!(s.field, InstrumentField::LoopMode);
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
    fn test_drag_tracking() {
        let mut s = InstrumentEditorState::default();
        s.start_drag(InstrumentField::Volume, 10, 5);

        assert_eq!(s.dragging(), Some(InstrumentField::Volume));
        assert_eq!(s.update_drag_position(13, 7), Some((3, 2)));

        s.end_drag();
        assert_eq!(s.dragging(), None);
        assert_eq!(s.update_drag_position(14, 8), None);
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
        use riffl_core::audio::sample::Sample;

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
        use riffl_core::audio::sample::Sample;
        let theme = Theme::default();
        let sample = Sample::default();
        let [top, bot] = build_waveform(&sample, 40, &theme);
        // Should return dash lines for empty sample.
        assert!(!top.spans.is_empty());
        assert!(!bot.spans.is_empty());
    }

    #[test]
    fn test_build_waveform_sine() {
        use riffl_core::audio::sample::Sample;
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
