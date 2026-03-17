/// Instrument list view UI.
///
/// Displays the instruments defined in the song alongside loaded sample names.
use ratatui::{
    layout::Alignment,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;
use tracker_core::song::Song;

use std::sync::Arc;
use tracker_core::audio::{LoopMode, Sample};

/// Render the instrument list view.
pub fn render_instrument_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    song: &Song,
    samples: &[Arc<Sample>],
    theme: &Theme,
    selection: Option<usize>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(" Instruments ")
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {:>3}  {:<20}  {:<12}  {:>5}  {:>4}  {:>4}  {}",
            "Idx", "Name", "Sample", "Vol", "FT", "Loop", "Base"
        ),
        Style::default().fg(theme.text_secondary),
    )]));

    // Show song instruments
    if song.instruments.is_empty() && samples.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No instruments defined. Press 'o' or F5 to load a sample.",
            Style::default().fg(theme.text_dimmed),
        )));
    } else {
        // List instruments from the song model
        for (idx, inst) in song.instruments.iter().enumerate() {
            let sample = inst.sample_index.and_then(|si| samples.get(si));

            let sample_name = sample.and_then(|s| s.name()).unwrap_or("---");
            let sample_name_display = if sample_name.len() > 12 {
                &sample_name[..12]
            } else {
                sample_name
            };

            let name_display = if inst.name.len() > 20 {
                &inst.name[..20]
            } else {
                &inst.name
            };

            let base_note = inst.base_note.display_str();

            // Loop mode display: - (none), F (forward), P (ping-pong)
            let loop_display = match sample.map(|s| s.loop_mode).unwrap_or(LoopMode::NoLoop) {
                LoopMode::NoLoop => "-",
                LoopMode::Forward => "Fwd",
                LoopMode::PingPong => "P-P",
            };

            let line_text = format!(
                "  {:3}  {:<20}  {:<12}  {:4.0}%  {:>4}  {:>4}  {}",
                format!("{:02X}", idx),
                name_display,
                sample_name_display,
                inst.volume * 100.0,
                sample.map(|s| s.finetune).unwrap_or(0),
                loop_display,
                base_note,
            );

            let is_selected = selection == Some(idx);
            let style = if is_selected {
                Style::default().fg(theme.text).bg(theme.bg_highlight)
            } else {
                Style::default().fg(theme.text)
            };

            lines.push(Line::from(Span::styled(line_text, style)));
        }

        // Also list loaded samples that don't have instrument entries yet
        let inst_count = song.instruments.len();
        for (idx, sample) in samples.iter().enumerate() {
            // Skip samples already covered by instrument entries
            if song.instruments.iter().any(|i| i.sample_index == Some(idx)) {
                continue;
            }
            let name = sample.name().unwrap_or("unnamed");
            let display_name = if name.len() > 12 { &name[..12] } else { name };
            let line_text = format!(
                "  {:3}  {:<20}  {:<12}  {:>5}  {:>4}  {:>4}  {}",
                format!("{:02X}", inst_count + idx),
                display_name,
                "(loaded)",
                "100%",
                sample.finetune,
                "-",
                "C-4",
            );
            lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(theme.text_dimmed),
            )));
        }
    }

    // Footer
    let visible_rows = inner.height as usize;
    let content_lines = lines.len();
    if content_lines < visible_rows {
        for _ in content_lines..visible_rows.saturating_sub(2) {
            lines.push(Line::from(""));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("?", Style::default().fg(theme.success_color())),
        Span::raw(" help"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracker_core::song::Song;

    fn make_sample(name: &str) -> Arc<Sample> {
        Arc::new(Sample::new(
            vec![0.0f32; 4],
            44100,
            1,
            Some(name.to_string()),
        ))
    }

    #[test]
    fn test_render_instrument_list_no_panic_empty() {
        let song = Song::new("Test", 120.0);
        let samples: Vec<Arc<Sample>> = vec![];
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_instrument_list(frame, area, &song, &samples, &theme, None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_instrument_list_no_panic_with_samples() {
        let song = Song::new("Test", 120.0);
        let samples = vec![make_sample("sine440"), make_sample("kick.wav")];
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_instrument_list(frame, area, &song, &samples, &theme, None);
            })
            .unwrap();
    }
}
