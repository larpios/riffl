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

use tracker_core::song::Instrument;

use crate::ui::theme::Theme;

/// Which field is currently focused in the instrument editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentField {
    Name,
    BaseNote,
    Volume,
    Finetune,
}

impl InstrumentField {
    pub const ALL: &'static [InstrumentField] = &[
        InstrumentField::Name,
        InstrumentField::BaseNote,
        InstrumentField::Volume,
        InstrumentField::Finetune,
    ];

    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::BaseNote,
            Self::BaseNote => Self::Volume,
            Self::Volume => Self::Finetune,
            Self::Finetune => Self::Name,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::Finetune,
            Self::BaseNote => Self::Name,
            Self::Volume => Self::BaseNote,
            Self::Finetune => Self::Volume,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::BaseNote => "Base Note",
            Self::Volume => "Volume",
            Self::Finetune => "Finetune",
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

/// Render the instrument editor panel for `instrument`.
/// `area` is the panel's allocated rect.
pub fn render_instrument_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    state: &InstrumentEditorState,
    theme: &Theme,
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
    let name_value = if state.focused
        && state.field == InstrumentField::Name
        && state.text_editing
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
        Span::styled(
            format!("{:+2}", ft),
            field_style(InstrumentField::Finetune),
        ),
        Span::raw("   "),
        Span::styled(
            format!("({:+.1} cents)", cents),
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
        Span::styled(
            "Ctrl+F: assign",
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    // Pad and show key hints at the bottom
    let content_lines = lines.len();
    let visible = inner.height as usize;
    for _ in content_lines..visible.saturating_sub(2) {
        lines.push(Line::from(""));
    }
    if visible >= 2 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Tab: next field   +/-: change   e/Enter: edit name   Esc: back to list",
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
                render_instrument_editor(frame, frame.area(), &inst, &state, &theme);
            })
            .unwrap();
    }
}
