use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppView};
use crate::editor::{EditorMode, SubColumn};
use crate::input::keybindings::KeybindingRegistry;
use crate::registry::{CommandMetadata, CommandRegistry};
use riffl_core::pattern::effect::EffectMode;
use riffl_core::pattern::note::NoteEvent;
use riffl_core::transport::{PlaybackMode, TransportState};

use super::{file_browser, layout};

pub(super) fn render_file_browser(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let browser = &app.file_browser;

    // Create centered area (70% width, 60% height)
    let browser_area = layout::create_centered_rect(area, 70, 60);
    frame.render_widget(Clear, browser_area);

    let dir_display = browser.directory().display().to_string();
    let title = format!(" Load File  {}  ", dir_display);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(title)
        .title_alignment(Alignment::Left)
        .style(Style::default().bg(Color::Black));

    let inner_area = block.inner(browser_area);
    frame.render_widget(block, browser_area);

    let mut lines: Vec<Line> = Vec::new();

    if browser.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No files found (.wav, .flac, .ogg, .mod)",
            Style::default().fg(theme.text_dimmed),
        )));
        lines.push(Line::from(""));
    } else {
        // Calculate scroll for the file list
        let visible_rows = inner_area.height.saturating_sub(3) as usize; // reserve header + footer
        let selected = browser.selected_index();
        let total = browser.entries().len();
        let scroll_offset = if visible_rows >= total || selected < visible_rows / 2 {
            0
        } else if selected + visible_rows / 2 >= total {
            total.saturating_sub(visible_rows)
        } else {
            selected.saturating_sub(visible_rows / 2)
        };

        // Header line: describe what Enter will do for the selected file
        let selected_ext = browser
            .selected_path()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        let action_hint = match selected_ext.as_deref() {
            Some("mod") => "  Enter → import MOD as new song (replaces current)".to_string(),
            _ => format!(
                "  Enter → load as instrument {:02X} (adds to instrument list)",
                app.song.instruments.len()
            ),
        };
        lines.push(Line::from(Span::styled(
            format!("  {} file(s)", total),
            Style::default().fg(theme.text_secondary),
        )));
        lines.push(Line::from(Span::styled(
            action_hint,
            Style::default().fg(theme.info_color()),
        )));
        lines.push(Line::from(""));

        for idx in scroll_offset..(scroll_offset + visible_rows).min(total) {
            let entry = &browser.entries()[idx];
            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("???");

            let is_selected = idx == selected;
            let prefix = if is_selected { "▸ " } else { "  " };
            let text = format!("{}{}", prefix, name);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(theme.info_color())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            lines.push(Line::from(Span::styled(text, style)));
        }
    }

    // Footer with instructions
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k ↑↓", Style::default().fg(theme.success_color())),
        Span::raw(":navigate  "),
        Span::styled("Enter", Style::default().fg(theme.success_color())),
        Span::raw(":load  "),
        Span::styled("Esc", Style::default().fg(theme.error_color())),
        Span::raw(":close  "),
        Span::styled(
            ".wav .flac .ogg .mod",
            Style::default().fg(theme.text_dimmed),
        ),
    ]));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(theme.text_style());

    frame.render_widget(paragraph, inner_area);
}
pub(super) fn render_command_completions(frame: &mut Frame, footer_area: ratatui::layout::Rect, app: &App) {
    let input = app.command_input.trim();
    let input_word = input.split_whitespace().next().unwrap_or(input);

    let matches: Vec<(String, String)> = CommandRegistry::all_commands()
        .into_iter()
        .filter_map(|cmd| {
            let name = cmd.name();
            let aliases = cmd.aliases();
            let all_names: Vec<&str> = std::iter::once(name)
                .chain(aliases.iter().copied())
                .collect();

            let _ = all_names.iter().find(|&&n| n.starts_with(input_word))?;
            let usage = cmd.usage().strip_prefix(':').unwrap_or(cmd.usage());
            Some((usage.to_string(), cmd.description().to_string()))
        })
        .collect();

    if matches.is_empty() {
        return;
    }

    let theme = &app.theme;
    let width = 30u16;
    let height = matches.len() as u16 + 2;
    let x = 0;
    let y = footer_area.y.saturating_sub(height);
    let area = ratatui::layout::Rect::new(x, y, width, height);

    frame.render_widget(Clear, area);

    let lines: Vec<Line> = matches
        .iter()
        .map(|(cmd, desc)| {
            Line::from(vec![
                Span::raw(" :"),
                Span::styled(
                    cmd.clone(),
                    Style::default()
                        .fg(theme.success_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}", desc)),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.text_dimmed))
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, area);
}

/// Render a which-key popup showing completions for the current pending key
/// or when which_key_mode is manually triggered.
pub(super) fn render_which_key(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Handle manually triggered which-key menu
    if app.which_key_mode && app.pending_key.is_none() {
        render_which_key_menu(frame, area, app);
        return;
    }

    // Handle pending key (chord) completion
    let pending = match app.pending_key {
        Some(c) => c,
        None => return,
    };

    let theme = &app.theme;
    let entries = KeybindingRegistry::get_which_key_entries(pending);

    if entries.is_empty() {
        return;
    }

    let width = 24u16;
    let height = entries.len() as u16 + 2;
    let x = 0;
    let y = area.height.saturating_sub(height + 1);
    let popup_area = ratatui::layout::Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let title = format!(" {}… ", pending);
    let lines: Vec<Line> = entries
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    key.clone(),
                    Style::default()
                        .fg(theme.success_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}", desc)),
            ])
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title(title)
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(lines).block(block);
    frame.render_widget(para, popup_area);
}

/// Render the full which-key menu when manually triggered.
pub(super) fn render_which_key_menu(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;

    // Get all which-key entries
    let all_entries = KeybindingRegistry::get_all_which_key_entries();

    let width = 40u16;
    let height = 25u16;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let popup_area = ratatui::layout::Rect::new(x, y, width, height);

    frame.render_widget(Clear, popup_area);

    let mut lines = Vec::new();
    for (key, desc) in all_entries.iter() {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {:3} ", key),
                Style::default()
                    .fg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(desc),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title(" KEYBINDINGS  (Esc close) ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(theme.primary));
    frame.render_widget(para, popup_area);
}
