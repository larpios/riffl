/// UI rendering and components
///
/// This module contains all UI-related code including layout management,
/// theming, and modal dialogs.
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

// Submodules
pub mod arrangement;
pub mod code_editor;
pub mod envelope_editor;
pub mod export_dialog;
pub mod fft_analyzer;
pub mod file_browser;
pub mod help;
pub mod instrument_editor;
pub mod instrument_list;
pub mod layout;
pub mod lfo_editor;
pub mod modal;
pub mod oscilloscope;
pub mod pattern_list;
pub mod sample_browser;
pub mod theme;
pub mod tutor;
pub mod vu_meters;
pub mod waveform_editor;

use help::{render_effect_help, render_help};
use tutor::render_tutor;

mod footer;
mod header;
mod overlays;
mod pattern_renderer;
#[cfg(test)]
mod tests;

use footer::render_footer;
use header::render_header;
use overlays::{render_command_completions, render_file_browser, render_which_key};
use pattern_renderer::render_pattern_with_area;

/// Render the application UI
pub fn render(frame: &mut Frame, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let full_area = frame.area();

    // Fill entire frame with theme background so Catppuccin/Nord bg colors are visible
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        full_area,
    );

    // Layout: header(3) + tab_bar(1) + content(flex) + footer(1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(full_area);

    let (header_area, tabs_area, content_area, footer_area) =
        (chunks[0], chunks[1], chunks[2], chunks[3]);

    render_header(frame, header_area, app);
    render_view_tabs(frame, tabs_area, app);

    // Handle instrument expanded view (full-screen deep editing)
    if app.instrument_expanded {
        if let Some(idx) = app.instrument_selection() {
            if idx < app.song.instruments.len() {
                render_instrument_view(frame, content_area, app, idx);
            }
        }
        render_footer(frame, footer_area, app);
        return;
    }

    // Handle split view: pattern left, code editor right
    if app.split_view && app.current_view == AppView::PatternEditor {
        let (left, right) =
            layout::create_split_layout(content_area, ratatui::layout::Direction::Horizontal, 50);
        render_content(frame, left, app);
        code_editor::render_code_editor(frame, right, &app.code_editor, &app.theme);
    } else {
        // Dispatch to the correct view renderer based on the active view
        match app.current_view {
            AppView::PatternEditor => render_content(frame, content_area, app),
            AppView::Arrangement => {
                let playback_pos = if app.transport.is_playing()
                    && app.transport.playback_mode() == PlaybackMode::Song
                {
                    Some(app.transport.arrangement_position())
                } else {
                    None
                };
                arrangement::render_arrangement(
                    frame,
                    content_area,
                    &app.song,
                    &app.arrangement_view,
                    playback_pos,
                    &app.theme,
                );
            }
            AppView::InstrumentList => {
                // Always render the two-column instrument view regardless of selection
                let sel = app.instrument_selection();
                if let Some(idx) = sel.filter(|&i| i < app.song.instruments.len()) {
                    render_instrument_view(frame, content_area, app, idx);
                } else {
                    render_instrument_view_empty(frame, content_area, app);
                }
            }
            AppView::CodeEditor => {
                code_editor::render_code_editor(frame, content_area, &app.code_editor, &app.theme);
            }
            AppView::PatternList => {
                let arr_pos = app.transport.arrangement_position();
                let editing_pat_idx = app.song.arrangement.get(arr_pos).copied().unwrap_or(0);
                pattern_list::render_pattern_list(
                    frame,
                    content_area,
                    &app.song,
                    &app.theme,
                    app.pattern_selection(),
                    editing_pat_idx,
                );
            }
            AppView::SampleBrowser => {
                let (preview_pos, total_frames, sample_rate) = app.preview_cursor_state();
                sample_browser::render_sample_browser(
                    frame,
                    content_area,
                    &app.sample_browser,
                    &app.theme,
                    preview_pos,
                    total_frames,
                    sample_rate,
                );
            }
        }
    }

    render_footer(frame, footer_area, app);

    // Render file browser on top if active
    if app.has_file_browser() {
        render_file_browser(frame, full_area, app);
    }

    // Render export dialog on top if active
    if app.export_dialog.active {
        export_dialog::render_export_dialog(frame, full_area, &app.export_dialog, &app.theme);
    }

    // Render modal on top if one is active
    if let Some(active_modal) = app.current_modal() {
        modal::render_modal(frame, full_area, active_modal, &app.theme);
    }

    // Render help overlay on top if active
    if app.show_help {
        render_help(frame, full_area, &app.theme, app.help_scroll);
    }

    // Render effect help overlay on top if active
    if app.show_effect_help {
        render_effect_help(
            frame,
            full_area,
            &app.theme,
            app.effect_help_scroll,
            app.song.effect_mode,
        );
    }

    // Render tutor view on top if active
    if app.show_tutor {
        render_tutor(frame, full_area, &app.theme, app.tutor_scroll);
    }

    // Render which-key popup when a chord is pending
    if app.pending_key.is_some() {
        render_which_key(frame, full_area, app);
    }

    // Render command autocomplete above the footer when in command mode
    if app.command_mode && !app.command_input.is_empty() {
        render_command_completions(frame, footer_area, app);
    }
}

/// Render the one-row view tab bar showing all views with the active one highlighted.
fn render_view_tabs(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;

    let tabs: &[(&str, AppView)] = &[
        ("1:PAT", AppView::PatternEditor),
        ("2:ARR", AppView::Arrangement),
        ("3:INS", AppView::InstrumentList),
        ("4:CODE", AppView::CodeEditor),
        ("5:LIST", AppView::PatternList),
        ("6:SMPL", AppView::SampleBrowser),
    ];

    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (label, view) in tabs {
        let is_active = app.current_view == *view
            || (app.split_view
                && app.current_view == AppView::PatternEditor
                && *view == AppView::PatternEditor);
        if is_active {
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", label),
                Style::default().fg(theme.text_dimmed),
            ));
        }
        spans.push(Span::styled("│", Style::default().fg(theme.text_dimmed)));
    }

    // Show focused panel indicator when in instrument view
    if app.current_view == AppView::InstrumentList {
        let panel_hint = if app.inst_editor.focused {
            " [INST EDITOR]  Tab: envelope  Esc: list  j/k: field  +/-: value"
        } else if app.env_editor.focused {
            " [ENVELOPE]  hjkl: point  a: add  x: del  Tab: type  Esc: list"
        } else {
            " Tab: envelope  S-Tab: inst editor  Enter/e: edit inst  j/k: navigate"
        };
        spans.push(Span::styled(
            panel_hint,
            Style::default().fg(theme.text_secondary),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Render the two-column instrument view when no instrument is selected.
fn render_instrument_view_empty(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    instrument_list::render_instrument_list(
        frame,
        cols[0],
        &app.song,
        &app.loaded_samples(),
        &app.theme,
        app.instrument_selection(),
    );

    // Right side: placeholder
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(app.theme.border_style())
        .title(" Instrument ")
        .title_alignment(Alignment::Left);
    let inner = block.inner(cols[1]);
    frame.render_widget(block, cols[1]);
    let hint = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Select an instrument with j/k, then Enter to edit.",
            Style::default().fg(app.theme.text_dimmed),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  n: new instrument   Ctrl+F: load sample",
            Style::default().fg(app.theme.text_secondary),
        )),
    ]);
    frame.render_widget(hint, inner);
}

/// Render the two-column instrument view:
///   left  (~38%): instrument list
///   right (~62%): tabbed instrument editor
fn render_instrument_view(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, idx: usize) {
    use ratatui::layout::{Constraint, Direction, Layout};

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(area);

    instrument_list::render_instrument_list(
        frame,
        cols[0],
        &app.song,
        &app.loaded_samples(),
        &app.theme,
        Some(idx),
    );

    let sample = {
        let samples = app.loaded_samples();
        app.song.instruments[idx]
            .sample_index
            .and_then(|si| samples.get(si).cloned())
    };

    // Right side: Tabs at top, active editor below
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(cols[1]);

    render_instrument_tabs(frame, right_chunks[0], &app.inst_editor, &app.theme);

    use crate::ui::instrument_editor::InstrumentTab;
    match app.inst_editor.tab {
        InstrumentTab::General => {
            instrument_editor::render_instrument_editor(
                frame,
                right_chunks[1],
                &app.song.instruments[idx],
                &app.inst_editor,
                &app.theme,
                sample.as_deref(),
            );
        }
        InstrumentTab::Sample => {
            waveform_editor::render_waveform_editor(
                frame,
                right_chunks[1],
                &app.song.instruments[idx],
                sample.as_deref(),
                &app.waveform_editor,
                &app.theme,
            );
        }
        InstrumentTab::Envelopes => {
            envelope_editor::render_envelope_editor(
                frame,
                right_chunks[1],
                &app.song.instruments[idx],
                &app.env_editor,
                &app.theme,
            );
        }
        InstrumentTab::Lfos => {
            lfo_editor::render_lfo_editor(
                frame,
                right_chunks[1],
                &app.song.instruments[idx],
                &app.lfo_editor,
                &app.theme,
            );
        }
    }
}

/// Render the instrument editor tab bar.
fn render_instrument_tabs(
    frame: &mut Frame,
    area: Rect,
    state: &instrument_editor::InstrumentEditorState,
    theme: &theme::Theme,
) {
    use crate::ui::instrument_editor::InstrumentTab;
    let tabs = InstrumentTab::ALL;
    let mut spans = vec![Span::raw("  ")];

    for (i, tab) in tabs.iter().enumerate() {
        let is_selected = state.tab == *tab;
        let style = if is_selected {
            Style::default()
                .fg(theme.cursor_fg)
                .bg(theme.primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_dimmed)
        };

        spans.push(Span::styled(format!(" {} ", tab.label()), style));
        if i < tabs.len() - 1 {
            spans.push(Span::styled("│", Style::default().fg(theme.text_dimmed)));
        }
    }

    if !state.focused {
        spans.push(Span::styled(
            "  Alt+, / Alt+. to switch",
            Style::default().fg(theme.text_dimmed),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Width of a single channel column (including separator): 22 chars.
/// Layout: "│ " (2) + "C#4" (3) + " " (1) + "01" (2) + " " (1) + "40" (2) + " " (1)
///       + "0C20" (4) + " " (1) + "0A04" (4) + " " (1) = 22
pub const CHANNEL_COL_WIDTH: u16 = 22;

/// Width of the row number column: "  XX  " = 6
pub const ROW_NUM_WIDTH: u16 = 6;

/// Calculate the horizontal channel scroll offset to keep the cursor channel visible.
fn calculate_channel_scroll(
    cursor_channel: usize,
    available_width: u16,
    num_channels: usize,
) -> usize {
    let channel_space = available_width.saturating_sub(ROW_NUM_WIDTH);
    let visible_channels = (channel_space / CHANNEL_COL_WIDTH) as usize;
    if visible_channels == 0 {
        return 0;
    }
    if visible_channels >= num_channels {
        return 0;
    }
    if cursor_channel < visible_channels / 2 {
        0
    } else if cursor_channel + visible_channels / 2 >= num_channels {
        num_channels.saturating_sub(visible_channels)
    } else {
        cursor_channel.saturating_sub(visible_channels / 2)
    }
}

/// Render the main content area with the tracker pattern grid
fn render_content(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    // Handle mini control panel - show above pattern when enabled and instrument selected
    if app.instrument_mini_panel {
        if let Some(idx) = app.instrument_selection() {
            if idx < app.song.instruments.len() {
                // Calculate layout: mini panel takes top ~20% or 5 rows, pattern takes rest
                let mini_panel_height = 5;
                if area.height > mini_panel_height + 2 {
                    let chunks = ratatui::layout::Layout::default()
                        .direction(ratatui::layout::Direction::Vertical)
                        .constraints([
                            ratatui::layout::Constraint::Length(mini_panel_height),
                            ratatui::layout::Constraint::Min(0),
                        ])
                        .split(area);

                    // Render mini instrument control panel
                    let sample = {
                        let samples = app.loaded_samples();
                        app.song.instruments[idx]
                            .sample_index
                            .and_then(|si| samples.get(si).cloned())
                    };
                    instrument_editor::render_instrument_editor(
                        frame,
                        chunks[0],
                        &app.song.instruments[idx],
                        &app.inst_editor,
                        &app.theme,
                        sample.as_deref(),
                    );

                    // Render pattern in remaining space
                    render_pattern_with_area(frame, chunks[1], app);
                    return;
                }
            }
        }
    }

    // Default: render pattern in full area
    render_pattern_with_area(frame, area, app);
}
