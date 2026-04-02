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

use super::code_editor;

pub(super) fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let status_bar_cfg = &app.config.status_bar;

    // Pattern length prompt mode: show inline length input
    if app.len_prompt.active {
        let input_style = Style::default().fg(theme.text);
        let label_style = Style::default()
            .fg(theme.cursor_fg)
            .bg(theme.info_color())
            .add_modifier(Modifier::BOLD);
        let line = Line::from(vec![
            Span::styled(" LEN ", label_style),
            Span::raw(" "),
            Span::styled(app.len_prompt.input.clone(), input_style),
            Span::styled("█", Style::default().fg(theme.primary)),
            Span::styled(
                "  Enter:apply  Esc:cancel  (16–512)",
                Style::default().fg(theme.text_secondary),
            ),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    // BPM prompt mode: show inline BPM input
    if app.bpm_prompt.active {
        let bpm_style = Style::default().fg(theme.text);
        let label_style = Style::default()
            .fg(theme.cursor_fg)
            .bg(theme.warning_color())
            .add_modifier(Modifier::BOLD);
        let line = Line::from(vec![
            Span::styled(" BPM ", label_style),
            Span::raw(" "),
            Span::styled(app.bpm_prompt.input.clone(), bpm_style),
            Span::styled("█", Style::default().fg(theme.primary)),
            Span::styled(
                "  Enter:apply  Esc:cancel",
                Style::default().fg(theme.text_secondary),
            ),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    // Command mode: show command line input instead of normal footer
    if app.command_mode {
        let cmd_style = Style::default().fg(theme.text);
        let line = Line::from(vec![
            Span::styled(":", cmd_style),
            Span::styled(app.command_input.clone(), cmd_style),
            Span::styled("█", Style::default().fg(theme.primary)),
        ]);
        frame.render_widget(ratatui::widgets::Paragraph::new(line), area);
        return;
    }

    let mode = app.editor.mode();
    let cursor_row = app.editor.cursor_row();
    let cursor_channel = app.editor.cursor_channel();

    let key_style = Style::default().fg(theme.success_color());
    // When code editor is active, show its mode; otherwise show pattern editor mode.
    // pending_replace overrides to show REPLACE mode pill.
    let (mode_label, mode_bg) = if app.pending_replace {
        ("REPLACE", theme.cursor_normal_bg)
    } else if app.is_code_editor_active() {
        match app.code_editor.mode {
            code_editor::ModeKind::Normal => ("NORMAL", theme.primary),
            code_editor::ModeKind::Insert => ("INSERT", theme.warning_color()),
            code_editor::ModeKind::Visual => ("VISUAL", theme.secondary),
        }
    } else {
        (mode.label(), theme.primary)
    };
    let mode_style = Style::default()
        .fg(theme.cursor_fg)
        .bg(mode_bg)
        .add_modifier(Modifier::BOLD);

    // ── LEFT SECTION ────────────────────────────────────────────────────────
    let mut left_spans: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled(format!(" {} ", mode_label), mode_style),
        Span::raw(" "),
    ];

    // Code editor cursor position
    if app.is_code_editor_active() {
        let ln = app.code_editor.cursor_row() + 1;
        let col = app.code_editor.cursor_col() + 1;
        let total = app.code_editor.line_count();
        left_spans.push(Span::styled(
            format!("Ln {}, Col {}  ({} lines)", ln, col, total),
            Style::default().fg(theme.text_secondary),
        ));
    }

    // Show status indicators instead of keybinding hints
    if !app.is_code_editor_active() && app.current_view == AppView::PatternEditor {
        // Octave
        left_spans.push(Span::styled(
            "OCT:",
            Style::default().fg(theme.text_secondary),
        ));
        left_spans.push(Span::styled(
            format!(" {} ", app.editor.current_octave()),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        // Instrument
        left_spans.push(Span::styled(
            "INS:",
            Style::default().fg(theme.text_secondary),
        ));
        left_spans.push(Span::styled(
            format!(" {:02X} ", app.editor.current_instrument()),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        // Step size
        left_spans.push(Span::styled(
            "STP:",
            Style::default().fg(theme.text_secondary),
        ));
        left_spans.push(Span::styled(
            format!(" {} ", app.editor.step_size()),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        left_spans.push(Span::raw(" | "));

        // Location status
        let pos = app.transport.arrangement_position();
        let total = app.song.arrangement.len();
        left_spans.push(Span::styled(
            "POS:",
            Style::default().fg(theme.text_secondary),
        ));
        left_spans.push(Span::styled(
            format!(" {:02}/{:02} ", pos, total),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ));

        if let Some(&pat_idx) = app.song.arrangement.get(pos) {
            left_spans.push(Span::styled(
                "PAT:",
                Style::default().fg(theme.text_secondary),
            ));
            left_spans.push(Span::styled(
                format!(" {:02} ", pat_idx),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Track volume and pan for the channel under the cursor
        {
            let ch = cursor_channel;
            let (vol, pan) = app
                .song
                .tracks
                .get(ch)
                .map(|t| (t.volume, t.pan))
                .unwrap_or((1.0, 0.0));
            left_spans.push(Span::styled(
                "VOL:",
                Style::default().fg(theme.text_secondary),
            ));
            let vol_pct = (vol * 100.0).round() as i32;
            let vol_color = if vol < 0.8 {
                theme.warning_color()
            } else {
                theme.primary
            };
            left_spans.push(Span::styled(
                format!(" {:3}% ", vol_pct),
                Style::default().fg(vol_color).add_modifier(Modifier::BOLD),
            ));

            left_spans.push(Span::styled(
                "PAN:",
                Style::default().fg(theme.text_secondary),
            ));
            let pan_pct = (pan * 100.0).round() as i32;
            let pan_str = if pan_pct == 0 {
                " CTR ".to_string()
            } else if pan_pct > 0 {
                format!(" R{:02} ", pan_pct)
            } else {
                format!(" L{:02} ", -pan_pct)
            };
            let pan_color = if pan_pct == 0 {
                theme.primary
            } else {
                theme.info_color()
            };
            left_spans.push(Span::styled(
                pan_str,
                Style::default().fg(pan_color).add_modifier(Modifier::BOLD),
            ));
        }

        // Effect mode badge
        {
            let (fx_label, fx_color) = match app.song.effect_mode {
                EffectMode::RifflNative => ("FX:N", theme.primary),
                EffectMode::Compatible => ("FX:C", theme.info_color()),
                EffectMode::Amiga => ("FX:A", theme.warning_color()),
            };
            left_spans.push(Span::raw(" "));
            left_spans.push(Span::styled(fx_label, Style::default().fg(fx_color)));
        }

        // Effect Description
        if app.editor.sub_column() == SubColumn::Effect {
            if let Some(cell) = app
                .editor
                .pattern()
                .get_cell(app.editor.cursor_row(), app.editor.cursor_channel())
            {
                if let Some(effect) = cell.first_effect() {
                    let desc = effect.describe(app.song.effect_mode);
                    left_spans.push(Span::raw(" | "));
                    left_spans.push(Span::styled(
                        format!("[ {} ]", desc),
                        Style::default()
                            .fg(theme.info_color())
                            .add_modifier(Modifier::BOLD),
                    ));
                }
            }
        }
    } else if !app.is_code_editor_active() {
        // View-specific hints for non-pattern views
        let hint = match app.current_view {
            AppView::InstrumentList => {
                if app.inst_editor.focused {
                    "  j/k: field   +/-: value   e: edit name   spc: loop   Tab: envelope   Esc: list"
                } else if app.env_editor.focused {
                    "  hjkl: point   a: add   x: del   0/$: first/last   Tab: waveform   Esc: list"
                } else if app.waveform_editor.focused {
                    "  h/←→: cursor   [/]: set loop   l: cycle loop   p: pencil   Esc: list"
                } else {
                    "  j/k: select   Enter/e: edit inst   Tab: envelope   n: new   Ctrl+F: load"
                }
            }
            AppView::Arrangement => "  j/k: row   h/l: col   Enter: select   n: new pattern",
            AppView::PatternList => "  j/k: select   Enter: open   n: new   d: delete   c: clone",
            AppView::SampleBrowser => {
                "  j/k: browse   l/Enter: open dir   h: up dir   Space: preview   Enter: load"
            }
            _ => "",
        };
        if !hint.is_empty() {
            left_spans.push(Span::styled(
                hint,
                Style::default().fg(theme.text_secondary),
            ));
        }
    }

    // ── RIGHT SECTION ───────────────────────────────────────────────────────
    let mut right_spans: Vec<Span> = Vec::new();

    // Metronome indicator
    if app.metronome_enabled() {
        right_spans.extend([
            Span::styled(
                " METRO ",
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(theme.info_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    // Follow mode indicator
    if app.follow_mode {
        right_spans.extend([
            Span::styled(
                " FOL ",
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    // Draw mode indicator
    if app.draw_mode {
        right_spans.extend([
            Span::styled(
                " DRW ",
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(theme.success_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    // Loop region indicator
    if let Some((loop_start, loop_end)) = app.transport.loop_region() {
        let (label, bg) = if app.transport.loop_region_active() {
            (
                format!(" LOP {:02X}-{:02X} ", loop_start, loop_end),
                theme.warning_color(),
            )
        } else {
            (
                format!(" lop {:02X}-{:02X} ", loop_start, loop_end),
                theme.text_secondary,
            )
        };
        right_spans.extend([
            Span::styled(
                label,
                Style::default()
                    .fg(theme.cursor_fg)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    // Cursor position
    if status_bar_cfg.show_pattern_row {
        right_spans.extend([
            Span::styled(
                format!("CH:{} ROW:{:02X}", cursor_channel, cursor_row),
                Style::default().fg(theme.primary),
            ),
            Span::raw("  "),
        ]);
    }

    // Selection indicator
    if status_bar_cfg.show_selection {
        if let Some(((r0, c0), (r1, c1))) = app.editor.visual_selection() {
            let sel_text = if r0 == r1 && c0 == c1 {
                format!("SEL:{}:{:02X}", c0, r0)
            } else {
                format!("SEL:{}:{:02X}-{}:{:02X}", c0, r0, c1, r1)
            };
            right_spans.extend([
                Span::styled(sel_text, Style::default().fg(theme.secondary)),
                Span::raw("  "),
            ]);
        }
    }

    // Pattern length
    if matches!(
        app.current_view,
        AppView::PatternEditor | AppView::PatternList
    ) {
        let row_count = app.editor.pattern().row_count();
        right_spans.extend([
            Span::styled(
                format!("Len:{}", row_count),
                Style::default().fg(theme.info_color()),
            ),
            Span::raw("  "),
        ]);
    }

    // Instrument count
    if status_bar_cfg.show_instrument_count {
        right_spans.extend([
            Span::styled(
                format!("Inst:{}", app.instrument_count()),
                Style::default().fg(theme.text_secondary),
            ),
            Span::raw("  "),
        ]);
    }

    // System resource indicators
    if status_bar_cfg.show_cpu || status_bar_cfg.show_memory {
        if let Some((cpu, mem)) = app.system_stats() {
            if status_bar_cfg.show_cpu {
                right_spans.extend([
                    Span::styled(
                        format!("CPU:{:.0}%", cpu),
                        Style::default().fg(if cpu > 80.0 {
                            theme.status_error
                        } else if cpu > 50.0 {
                            theme.status_warning
                        } else {
                            theme.text_dimmed
                        }),
                    ),
                    Span::raw(" "),
                ]);
            }
            if status_bar_cfg.show_memory {
                right_spans.extend([
                    Span::styled(
                        format!("MEM:{:.0}%", mem),
                        Style::default().fg(if mem > 80.0 {
                            theme.status_error
                        } else if mem > 50.0 {
                            theme.status_warning
                        } else {
                            theme.text_dimmed
                        }),
                    ),
                    Span::raw(" "),
                ]);
            }
        }
    }

    // View indicator
    let view_label = if app.split_view {
        "SPLIT"
    } else {
        match app.current_view {
            AppView::PatternEditor => "1:PAT",
            AppView::Arrangement => "2:ARR",
            AppView::InstrumentList => "3:INS",
            AppView::CodeEditor => "4:CODE",
            AppView::PatternList => "5:PATLIST",
            AppView::SampleBrowser => "6:SAMPLES",
        }
    };
    right_spans.extend([
        Span::styled(
            view_label,
            Style::default()
                .fg(theme.info_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ]);

    // ? help — always rightmost
    right_spans.extend([Span::styled("?", key_style), Span::raw(" help ")]);

    // ── COMPOSE: pad between left and right ─────────────────────────────────
    let left_width: usize = left_spans.iter().map(|s| s.content.len()).sum();
    let right_width: usize = right_spans.iter().map(|s| s.content.len()).sum();
    let total_width = area.width as usize;
    let padding = total_width.saturating_sub(left_width + right_width);

    let mut all_spans = left_spans;
    all_spans.push(Span::raw(" ".repeat(padding)));
    all_spans.extend(right_spans);

    let footer = Paragraph::new(Line::from(all_spans)).style(theme.footer_style());

    frame.render_widget(footer, area);
}
