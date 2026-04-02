use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::Theme;
use crate::editor::EditorMode;
use crate::input::keybindings::KeybindingRegistry;
use crate::registry::{
    ActionCategory, CommandCategory, CommandMetadata, CommandRegistry, Keybinding,
};
use riffl_core::pattern::effect::{EffectMode, EffectType};

/// Total line count of the taller help column. Used to cap scroll offset.
pub fn content_line_count() -> u16 {
    left_column(&Theme::default())
        .len()
        .max(right_column(&Theme::default()).len()) as u16
}

/// Total line count for effect help.
pub fn effect_help_line_count(mode: EffectMode) -> u16 {
    let mut count = 0;
    // Iterate over all possible command values (0x00 to 0x22)
    for i in 0..=0x22 {
        if let Some(t) = EffectType::from_command(i) {
            let meta = t.metadata();
            if meta.is_native || mode == EffectMode::Compatible || mode == EffectMode::Amiga {
                count += 4; // command/name line + summary + description + blank line
            }
        }
    }
    count as u16
}

/// Render help/cheatsheet overlay — two-column scrollable layout.
/// `scroll` is the vertical scroll offset in lines.
pub fn render_help(frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme, scroll: u16) {
    let help_area = super::layout::create_centered_rect(area, 84, 85);
    frame.render_widget(Clear, help_area);

    let title = " KEYBOARD SHORTCUTS  (j/k scroll · ?/Esc close) ";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(title)
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    // Split inner area into two equal columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let col_style = Style::default().fg(theme.text).bg(theme.bg_surface);
    frame.render_widget(
        Paragraph::new(left_column(theme))
            .scroll((scroll, 0))
            .style(col_style),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(right_column(theme))
            .scroll((scroll, 0))
            .style(col_style),
        cols[1],
    );
}

/// Render effect command explorer overlay.
pub fn render_effect_help(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    theme: &Theme,
    scroll: u16,
    mode: EffectMode,
) {
    let help_area = super::layout::create_centered_rect(area, 84, 85);
    frame.render_widget(Clear, help_area);

    let mode_str = match mode {
        EffectMode::RifflNative => "RIFFL NATIVE",
        EffectMode::Compatible => "COMPATIBLE",
        EffectMode::Amiga => "AMIGA (LEGACY)",
    };
    let title = format!(
        " EFFECT COMMAND EXPLORER ({})  (j/k scroll · K/Esc close) ",
        mode_str
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.info_color()))
        .title(title)
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let mut lines = Vec::new();

    for i in 0..=0x22 {
        if let Some(t) = EffectType::from_command(i) {
            let meta = t.metadata();
            if !meta.is_native && mode == EffectMode::RifflNative {
                continue;
            }

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:01X}xx - ", t.to_command()),
                    Style::default()
                        .fg(theme.success_color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    meta.name,
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(meta.summary, Style::default().fg(theme.text_secondary)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(
                    meta.description,
                    Style::default()
                        .fg(theme.text)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
            lines.push(Line::from(""));
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .scroll((scroll, 0))
            .style(Style::default().fg(theme.text).bg(theme.bg_surface)),
        inner,
    );
}

fn section(label: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        label.to_string(),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key(keys: &str, desc: &str, key_width: usize, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<width$}", keys, width = key_width),
            Style::default().fg(theme.success_color()),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.text)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

/// Generate lines for all bindings in a given mode+category, with a section header.
/// Returns empty if no bindings match.
fn category_section(
    bindings: &[(EditorMode, Keybinding)],
    mode: EditorMode,
    category: ActionCategory,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let entries: Vec<_> = bindings
        .iter()
        .filter(|(m, kb)| *m == mode && kb.category == category)
        .collect();
    if entries.is_empty() {
        return vec![];
    }
    let key_width = entries
        .iter()
        .map(|(_, kb)| kb.key.len())
        .max()
        .unwrap_or(0)
        .max(18);
    let mut lines = vec![section(category.name(), theme)];
    for (_, kb) in &entries {
        lines.push(key(&kb.key, &kb.description, key_width, theme));
    }
    lines.push(blank());
    lines
}

/// Generate lines for all bindings in a given mode, with a custom section title.
fn mode_section(
    bindings: &[(EditorMode, Keybinding)],
    mode: EditorMode,
    title: &str,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let entries: Vec<_> = bindings.iter().filter(|(m, _)| *m == mode).collect();
    if entries.is_empty() {
        return vec![];
    }
    let key_width = entries
        .iter()
        .map(|(_, kb)| kb.key.len())
        .max()
        .unwrap_or(0)
        .max(18);
    let mut lines = vec![section(title, theme)];
    for (_, kb) in &entries {
        lines.push(key(&kb.key, &kb.description, key_width, theme));
    }
    lines.push(blank());
    lines
}

/// Generate lines for commands in a given category.
fn command_category_section(category: CommandCategory, theme: &Theme) -> Vec<Line<'static>> {
    let cmds: Vec<_> = CommandRegistry::all_commands()
        .into_iter()
        .filter(|c| c.category() == category)
        .collect();
    if cmds.is_empty() {
        return vec![];
    }
    let key_width = cmds
        .iter()
        .map(|c| c.usage().len())
        .max()
        .unwrap_or(0)
        .max(18);
    let mut lines = vec![section(category.name(), theme)];
    for cmd in cmds {
        lines.push(key(cmd.usage(), cmd.description(), key_width, theme));
    }
    lines.push(blank());
    lines
}

fn left_column(theme: &Theme) -> Vec<Line<'static>> {
    let bindings = KeybindingRegistry::all_bindings();
    let mut lines = Vec::new();
    for cat in [
        ActionCategory::Navigation,
        ActionCategory::Editing,
        ActionCategory::Clipboard,
        ActionCategory::Track,
    ] {
        lines.extend(category_section(&bindings, EditorMode::Normal, cat, theme));
    }
    lines
}

fn right_column(theme: &Theme) -> Vec<Line<'static>> {
    let bindings = KeybindingRegistry::all_bindings();
    let mut lines = Vec::new();
    for cat in [
        ActionCategory::Transport,
        ActionCategory::View,
        ActionCategory::Project,
        ActionCategory::Application,
    ] {
        lines.extend(category_section(&bindings, EditorMode::Normal, cat, theme));
    }
    lines.extend(mode_section(
        &bindings,
        EditorMode::Insert,
        "Insert Mode",
        theme,
    ));
    lines.extend(mode_section(
        &bindings,
        EditorMode::Visual,
        "Visual Mode",
        theme,
    ));
    for cat in [
        CommandCategory::Project,
        CommandCategory::Pattern,
        CommandCategory::Transport,
        CommandCategory::Editing,
        CommandCategory::Track,
        CommandCategory::Navigation,
        CommandCategory::Instrument,
        CommandCategory::Misc,
    ] {
        lines.extend(command_category_section(cat, theme));
    }
    lines
}
