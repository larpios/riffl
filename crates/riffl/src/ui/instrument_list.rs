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
use riffl_core::song::Song;

use riffl_core::audio::{LoopMode, Sample};
use std::sync::Arc;

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

    // Build the instrument rows (excluding header/footer).
    let mut inst_lines: Vec<Line> = Vec::new();

    if song.instruments.is_empty() && samples.is_empty() {
        inst_lines.push(Line::from(Span::styled(
            "  No instruments defined. Press Ctrl+F to load a sample.",
            Style::default().fg(theme.text_dimmed),
        )));
    } else {
        for (idx, inst) in song.instruments.iter().enumerate() {
            let sample = inst.sample_index.and_then(|si| samples.get(si));

            let sample_name = sample.and_then(|s| s.name()).unwrap_or("---");
            let sample_name_display = if sample_name.len() > 12 {
                sample_name.get(..12).unwrap_or(sample_name)
            } else {
                sample_name
            };

            let name_display = if inst.name.len() > 20 {
                inst.name.get(..20).unwrap_or(&inst.name)
            } else {
                &inst.name
            };

            let base_note = inst.base_note.display_str();

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

            inst_lines.push(Line::from(Span::styled(line_text, style)));
        }

        let inst_count = song.instruments.len();
        for (idx, sample) in samples.iter().enumerate() {
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
            inst_lines.push(Line::from(Span::styled(
                line_text,
                Style::default().fg(theme.text_dimmed),
            )));
        }
    }

    // 1 header row + 2 footer rows (blank + hint); remaining rows show instruments.
    let footer_rows = 2usize;
    let header_rows = 1usize;
    let visible_slots = (inner.height as usize).saturating_sub(header_rows + footer_rows);

    // Scroll so the selected item stays visible.
    let scroll_offset = if let Some(sel) = selection {
        if sel >= visible_slots {
            sel - visible_slots + 1
        } else {
            0
        }
    } else {
        0
    };

    let header = Line::from(vec![Span::styled(
        format!(
            "  {:>3}  {:<20}  {:<12}  {:>5}  {:>4}  {:>4}  {}",
            "Idx", "Name", "Sample", "Vol", "FT", "Loop", "Base"
        ),
        Style::default().fg(theme.text_secondary),
    )]);

    let footer = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled("?", Style::default().fg(theme.success_color())),
            Span::raw(" help   "),
            Span::styled("i", Style::default().fg(theme.secondary)),
            Span::raw(" piano roll preview   "),
            Span::styled("Esc", Style::default().fg(theme.secondary)),
            Span::raw(" exit preview   "),
        ]),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(header);

    let end = (scroll_offset + visible_slots).min(inst_lines.len());
    let visible = &inst_lines[scroll_offset..end];
    lines.extend_from_slice(visible);

    // Pad empty rows between list and footer.
    while lines.len() < inner.height as usize - footer_rows {
        lines.push(Line::from(""));
    }
    lines.extend(footer);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use riffl_core::song::Song;

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
