//! Export dialog UI for rendering songs to WAV files.
//!
//! Provides a modal dialog for configuring and executing audio export,
//! with sample rate selection, bit depth selection, progress indicator,
//! and completion messages.

use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::layout::create_centered_rect;
use super::theme::Theme;
use riffl_core::export::{BitDepth, ExportConfig};

/// Which field the user is currently editing in the export dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportField {
    /// Sample rate selection (44100 / 48000)
    SampleRate,
    /// Bit depth selection (16-bit / 24-bit)
    BitDepth,
    /// Confirm / cancel row
    Confirm,
}

/// State of the export dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPhase {
    /// User is configuring export settings.
    Configure,
    /// Export is in progress (stores progress 0-100).
    Exporting,
    /// Export completed successfully.
    Done,
    /// Export failed with an error.
    Failed,
}

/// Export dialog state.
#[derive(Debug, Clone)]
pub struct ExportDialog {
    /// Whether the dialog is visible.
    pub active: bool,
    /// Current dialog phase.
    pub phase: ExportPhase,
    /// Currently focused field during configuration.
    pub focused_field: ExportField,
    /// Selected sample rate (44100 or 48000).
    pub sample_rate: u32,
    /// Selected bit depth.
    pub bit_depth: BitDepth,
    /// Output file path.
    pub output_path: String,
    /// Export progress (0-100).
    pub progress: u8,
    /// Result message after export completes.
    pub result_message: String,
}

impl ExportDialog {
    /// Create a new export dialog with default settings.
    pub fn new() -> Self {
        Self {
            active: false,
            phase: ExportPhase::Configure,
            focused_field: ExportField::SampleRate,
            sample_rate: 44100,
            bit_depth: BitDepth::Bits16,
            output_path: String::new(),
            progress: 0,
            result_message: String::new(),
        }
    }

    /// Open the dialog with the given default output path.
    pub fn open(&mut self, default_path: &str) {
        self.active = true;
        self.phase = ExportPhase::Configure;
        self.focused_field = ExportField::SampleRate;
        self.sample_rate = 44100;
        self.bit_depth = BitDepth::Bits16;
        self.output_path = default_path.to_string();
        self.progress = 0;
        self.result_message.clear();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Move focus to the next field.
    pub fn next_field(&mut self) {
        self.focused_field = match self.focused_field {
            ExportField::SampleRate => ExportField::BitDepth,
            ExportField::BitDepth => ExportField::Confirm,
            ExportField::Confirm => ExportField::SampleRate,
        };
    }

    /// Move focus to the previous field.
    pub fn prev_field(&mut self) {
        self.focused_field = match self.focused_field {
            ExportField::SampleRate => ExportField::Confirm,
            ExportField::BitDepth => ExportField::SampleRate,
            ExportField::Confirm => ExportField::BitDepth,
        };
    }

    /// Toggle the current field's value.
    pub fn toggle_value(&mut self) {
        match self.focused_field {
            ExportField::SampleRate => {
                self.sample_rate = if self.sample_rate == 44100 {
                    48000
                } else {
                    44100
                };
            }
            ExportField::BitDepth => {
                self.bit_depth = match self.bit_depth {
                    BitDepth::Bits16 => BitDepth::Bits24,
                    BitDepth::Bits24 => BitDepth::Bits16,
                };
            }
            ExportField::Confirm => {
                // No toggle on confirm — handled by Enter
            }
        }
    }

    /// Build an ExportConfig from the current dialog settings.
    pub fn to_config(&self) -> ExportConfig {
        ExportConfig {
            sample_rate: self.sample_rate,
            bit_depth: self.bit_depth,
        }
    }

    /// Set the dialog to the exporting phase.
    pub fn start_export(&mut self) {
        self.phase = ExportPhase::Exporting;
        self.progress = 0;
    }

    /// Update the progress percentage.
    pub fn set_progress(&mut self, percent: u8) {
        self.progress = percent.min(100);
    }

    /// Mark export as completed with a success message.
    pub fn finish_success(&mut self, message: String) {
        self.phase = ExportPhase::Done;
        self.progress = 100;
        self.result_message = message;
    }

    /// Mark export as failed with an error message.
    pub fn finish_error(&mut self, message: String) {
        self.phase = ExportPhase::Failed;
        self.result_message = message;
    }
}

impl Default for ExportDialog {
    fn default() -> Self {
        Self::new()
    }
}

/// Render the export dialog overlay.
pub fn render_export_dialog(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    dialog: &ExportDialog,
    theme: &Theme,
) {
    let dialog_area = create_centered_rect(area, 55, 50);
    frame.render_widget(Clear, dialog_area);

    let border_color = match dialog.phase {
        ExportPhase::Configure => theme.info_color(),
        ExportPhase::Exporting => theme.warning_color(),
        ExportPhase::Done => theme.success_color(),
        ExportPhase::Failed => theme.error_color(),
    };

    let title = match dialog.phase {
        ExportPhase::Configure => " Export to WAV ",
        ExportPhase::Exporting => " Exporting... ",
        ExportPhase::Done => " Export Complete ",
        ExportPhase::Failed => " Export Failed ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    let inner_area = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut lines: Vec<Line> = Vec::new();

    match dialog.phase {
        ExportPhase::Configure => {
            render_configure_phase(&mut lines, dialog, theme);
        }
        ExportPhase::Exporting => {
            render_exporting_phase(&mut lines, dialog, theme);
        }
        ExportPhase::Done => {
            render_done_phase(&mut lines, dialog, theme);
        }
        ExportPhase::Failed => {
            render_failed_phase(&mut lines, dialog, theme);
        }
    }

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(theme.text_style());

    frame.render_widget(paragraph, inner_area);
}

fn render_configure_phase(lines: &mut Vec<Line>, dialog: &ExportDialog, theme: &Theme) {
    let key_style = Style::default().fg(theme.success_color());
    let focused_style = Style::default()
        .fg(theme.cursor_fg)
        .bg(theme.info_color())
        .add_modifier(Modifier::BOLD);
    let normal_style = Style::default().fg(theme.text);
    let label_style = Style::default().fg(theme.text_secondary);

    lines.push(Line::from(""));

    // Output path
    lines.push(Line::from(vec![
        Span::styled("  Output: ", label_style),
        Span::styled(
            dialog.output_path.clone(),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    lines.push(Line::from(""));

    // Sample rate field
    let sr_label = format!("  {} Hz", dialog.sample_rate);
    let sr_style = if dialog.focused_field == ExportField::SampleRate {
        focused_style
    } else {
        normal_style
    };
    lines.push(Line::from(vec![
        Span::styled("  Sample Rate:  ", label_style),
        Span::styled(sr_label, sr_style),
    ]));

    // Bit depth field
    let bd_label = format!("  {}-bit", dialog.bit_depth.bits_per_sample());
    let bd_style = if dialog.focused_field == ExportField::BitDepth {
        focused_style
    } else {
        normal_style
    };
    lines.push(Line::from(vec![
        Span::styled("  Bit Depth:    ", label_style),
        Span::styled(bd_label, bd_style),
    ]));

    lines.push(Line::from(""));

    // Confirm button
    let confirm_style = if dialog.focused_field == ExportField::Confirm {
        Style::default()
            .fg(theme.cursor_fg)
            .bg(theme.success_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.success_color())
    };
    lines.push(Line::from(vec![
        Span::raw("        "),
        Span::styled("  [ Export ]  ", confirm_style),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Footer instructions
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", key_style),
        Span::raw(":navigate  "),
        Span::styled("l/h/Space", key_style),
        Span::raw(":toggle  "),
        Span::styled("Enter", key_style),
        Span::raw(":export  "),
        Span::styled("Esc", Style::default().fg(theme.error_color())),
        Span::raw(":cancel"),
    ]));
}

fn render_exporting_phase(lines: &mut Vec<Line>, dialog: &ExportDialog, theme: &Theme) {
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled(
            "  Exporting to: ",
            Style::default().fg(theme.text_secondary),
        ),
        Span::styled(
            dialog.output_path.clone(),
            Style::default().fg(theme.primary),
        ),
    ]));

    lines.push(Line::from(""));

    // Progress bar
    let bar_width = 30;
    let filled = (dialog.progress as usize * bar_width / 100).min(bar_width);
    let empty = bar_width - filled;
    let bar = format!(
        "  [{}{}] {}%",
        "#".repeat(filled),
        "-".repeat(empty),
        dialog.progress
    );
    lines.push(Line::from(Span::styled(
        bar,
        Style::default()
            .fg(theme.warning_color())
            .add_modifier(Modifier::BOLD),
    )));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Please wait...",
        Style::default().fg(theme.text_dimmed),
    )));
}

fn render_done_phase(lines: &mut Vec<Line>, dialog: &ExportDialog, theme: &Theme) {
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    for line in dialog.result_message.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", line),
            Style::default().fg(theme.text),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Press "),
        Span::styled("ESC", Style::default().fg(theme.success_color())),
        Span::raw(" or "),
        Span::styled("Enter", Style::default().fg(theme.success_color())),
        Span::raw(" to close"),
    ]));
}

fn render_failed_phase(lines: &mut Vec<Line>, dialog: &ExportDialog, theme: &Theme) {
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "  Export failed:",
        Style::default()
            .fg(theme.error_color())
            .add_modifier(Modifier::BOLD),
    )));

    lines.push(Line::from(""));

    for line in dialog.result_message.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {}", line),
            Style::default().fg(theme.text),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::raw("  Press "),
        Span::styled("ESC", Style::default().fg(theme.error_color())),
        Span::raw(" or "),
        Span::styled("Enter", Style::default().fg(theme.error_color())),
        Span::raw(" to close"),
    ]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_dialog_new_defaults() {
        let dialog = ExportDialog::new();
        assert!(!dialog.active);
        assert_eq!(dialog.phase, ExportPhase::Configure);
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
        assert_eq!(dialog.sample_rate, 44100);
        assert_eq!(dialog.bit_depth, BitDepth::Bits16);
        assert_eq!(dialog.progress, 0);
    }

    #[test]
    fn test_export_dialog_open() {
        let mut dialog = ExportDialog::new();
        dialog.open("my_song.wav");
        assert!(dialog.active);
        assert_eq!(dialog.output_path, "my_song.wav");
        assert_eq!(dialog.phase, ExportPhase::Configure);
    }

    #[test]
    fn test_export_dialog_close() {
        let mut dialog = ExportDialog::new();
        dialog.open("test.wav");
        assert!(dialog.active);
        dialog.close();
        assert!(!dialog.active);
    }

    #[test]
    fn test_next_field_cycles() {
        let mut dialog = ExportDialog::new();
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
        dialog.next_field();
        assert_eq!(dialog.focused_field, ExportField::BitDepth);
        dialog.next_field();
        assert_eq!(dialog.focused_field, ExportField::Confirm);
        dialog.next_field();
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
    }

    #[test]
    fn test_prev_field_cycles() {
        let mut dialog = ExportDialog::new();
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
        dialog.prev_field();
        assert_eq!(dialog.focused_field, ExportField::Confirm);
        dialog.prev_field();
        assert_eq!(dialog.focused_field, ExportField::BitDepth);
        dialog.prev_field();
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
    }

    #[test]
    fn test_toggle_sample_rate() {
        let mut dialog = ExportDialog::new();
        dialog.focused_field = ExportField::SampleRate;
        assert_eq!(dialog.sample_rate, 44100);
        dialog.toggle_value();
        assert_eq!(dialog.sample_rate, 48000);
        dialog.toggle_value();
        assert_eq!(dialog.sample_rate, 44100);
    }

    #[test]
    fn test_toggle_bit_depth() {
        let mut dialog = ExportDialog::new();
        dialog.focused_field = ExportField::BitDepth;
        assert_eq!(dialog.bit_depth, BitDepth::Bits16);
        dialog.toggle_value();
        assert_eq!(dialog.bit_depth, BitDepth::Bits24);
        dialog.toggle_value();
        assert_eq!(dialog.bit_depth, BitDepth::Bits16);
    }

    #[test]
    fn test_toggle_confirm_is_noop() {
        let mut dialog = ExportDialog::new();
        dialog.focused_field = ExportField::Confirm;
        let sr_before = dialog.sample_rate;
        let bd_before = dialog.bit_depth;
        dialog.toggle_value();
        assert_eq!(dialog.sample_rate, sr_before);
        assert_eq!(dialog.bit_depth, bd_before);
    }

    #[test]
    fn test_to_config() {
        let mut dialog = ExportDialog::new();
        dialog.sample_rate = 48000;
        dialog.bit_depth = BitDepth::Bits24;
        let config = dialog.to_config();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.bit_depth, BitDepth::Bits24);
    }

    #[test]
    fn test_start_export() {
        let mut dialog = ExportDialog::new();
        dialog.open("test.wav");
        dialog.start_export();
        assert_eq!(dialog.phase, ExportPhase::Exporting);
        assert_eq!(dialog.progress, 0);
    }

    #[test]
    fn test_set_progress_clamps() {
        let mut dialog = ExportDialog::new();
        dialog.set_progress(50);
        assert_eq!(dialog.progress, 50);
        dialog.set_progress(150);
        assert_eq!(dialog.progress, 100);
    }

    #[test]
    fn test_finish_success() {
        let mut dialog = ExportDialog::new();
        dialog.open("test.wav");
        dialog.start_export();
        dialog.finish_success("Exported 8.0s to test.wav".to_string());
        assert_eq!(dialog.phase, ExportPhase::Done);
        assert_eq!(dialog.progress, 100);
        assert_eq!(dialog.result_message, "Exported 8.0s to test.wav");
    }

    #[test]
    fn test_finish_error() {
        let mut dialog = ExportDialog::new();
        dialog.open("test.wav");
        dialog.start_export();
        dialog.finish_error("Permission denied".to_string());
        assert_eq!(dialog.phase, ExportPhase::Failed);
        assert_eq!(dialog.result_message, "Permission denied");
    }

    #[test]
    fn test_open_resets_state() {
        let mut dialog = ExportDialog::new();
        dialog.open("first.wav");
        dialog.focused_field = ExportField::Confirm;
        dialog.sample_rate = 48000;
        dialog.bit_depth = BitDepth::Bits24;
        dialog.phase = ExportPhase::Done;
        dialog.progress = 100;

        // Re-opening should reset everything
        dialog.open("second.wav");
        assert_eq!(dialog.output_path, "second.wav");
        assert_eq!(dialog.focused_field, ExportField::SampleRate);
        assert_eq!(dialog.sample_rate, 44100);
        assert_eq!(dialog.bit_depth, BitDepth::Bits16);
        assert_eq!(dialog.phase, ExportPhase::Configure);
        assert_eq!(dialog.progress, 0);
    }

    #[test]
    fn test_export_phase_equality() {
        assert_eq!(ExportPhase::Configure, ExportPhase::Configure);
        assert_eq!(ExportPhase::Exporting, ExportPhase::Exporting);
        assert_eq!(ExportPhase::Done, ExportPhase::Done);
        assert_eq!(ExportPhase::Failed, ExportPhase::Failed);
        assert_ne!(ExportPhase::Configure, ExportPhase::Exporting);
    }

    #[test]
    fn test_export_field_equality() {
        assert_eq!(ExportField::SampleRate, ExportField::SampleRate);
        assert_eq!(ExportField::BitDepth, ExportField::BitDepth);
        assert_eq!(ExportField::Confirm, ExportField::Confirm);
        assert_ne!(ExportField::SampleRate, ExportField::BitDepth);
    }
}
