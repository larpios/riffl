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

use super::fft_analyzer;

pub(super) fn render_header(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let theme = &app.theme;
    let pattern = app.editor.pattern();

    let (play_icon, play_status) = match app.transport.state() {
        TransportState::Playing => ("\u{25B6}", "PLAYING"),
        TransportState::Paused => ("\u{23F8}", "PAUSED"),
        TransportState::Stopped => ("\u{23F9}", "STOPPED"),
    };
    let play_color = match app.transport.state() {
        TransportState::Playing => theme.success_color(),
        TransportState::Paused => theme.warning_color(),
        TransportState::Stopped => theme.text_dimmed,
    };

    let mode_indicator = match app.transport.playback_mode() {
        PlaybackMode::Pattern => "PAT",
        PlaybackMode::Song => "SONG",
    };
    let loop_indicator = if app.transport.loop_enabled() {
        " L"
    } else {
        ""
    };
    let dirty_marker = if app.is_dirty { "*" } else { "" };
    let song_label = if app.song.name.is_empty() {
        "riffl".to_string()
    } else if app.song.artist.is_empty() {
        format!(
            "{}{}",
            app.song.name,
            if dirty_marker.is_empty() { "" } else { " *" }
        )
    } else {
        format!(
            "{} — {}{}",
            app.song.artist,
            app.song.name,
            if dirty_marker.is_empty() { "" } else { " *" }
        )
    };
    let title = format!(
        " {} | BPM: {:.0} | TPL: {} | {} {}{} [{}] ",
        song_label,
        app.transport.bpm(),
        app.transport.tpl(),
        play_icon,
        play_status,
        loop_indicator,
        mode_indicator
    );

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.border_style())
        .title(title)
        .title_alignment(Alignment::Center);

    let app_label = if app.song.name.is_empty() {
        "riffl".to_string()
    } else {
        app.song.name.clone()
    };
    let mut status_spans = vec![
        Span::styled(
            app_label,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("BPM: {:.0}", app.transport.bpm()),
            Style::default().fg(theme.text),
        ),
        Span::raw("  "),
        Span::styled(
            format!("TPL: {}", app.transport.tpl()),
            Style::default().fg(theme.text),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} [{}]", play_icon, play_status),
            Style::default().fg(play_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

    // Show arrangement position + row in Song mode, just row in Pattern mode
    match app.transport.playback_mode() {
        PlaybackMode::Song => {
            status_spans.push(Span::styled(
                format!(
                    "Arr: {:02X}/{:02X}  Row: {:02X}/{:02X}",
                    app.transport.arrangement_position(),
                    app.song.arrangement.len(),
                    app.transport.current_row(),
                    pattern.num_rows(),
                ),
                Style::default().fg(theme.text_secondary),
            ));
        }
        PlaybackMode::Pattern => {
            status_spans.push(Span::styled(
                format!(
                    "Row: {:02X}/{:02X}",
                    app.transport.current_row(),
                    pattern.num_rows()
                ),
                Style::default().fg(theme.text_secondary),
            ));
        }
    }

    // Playback mode indicator
    let mode_color = match app.transport.playback_mode() {
        PlaybackMode::Pattern => theme.text_secondary,
        PlaybackMode::Song => theme.warning_color(),
    };
    status_spans.push(Span::raw("  "));
    status_spans.push(Span::styled(
        format!("[{}]", mode_indicator),
        Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
    ));

    if app.transport.loop_enabled() {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            "[LOOP]",
            Style::default()
                .fg(theme.info_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.live_mode {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            "[LIVE]",
            Style::default()
                .fg(theme.error_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    if app.follow_mode {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            "[FOLLOW]",
            Style::default()
                .fg(theme.info_color())
                .add_modifier(Modifier::BOLD),
        ));
    }

    // Show channel scroll position and reset hint
    if app.channel_scroll > 0 {
        status_spans.push(Span::raw("  "));
        status_spans.push(Span::styled(
            format!("CH:{} \u{2190}", app.channel_scroll),
            Style::default().fg(theme.text_secondary),
        ));
    }

    let header_text = Paragraph::new(Line::from(status_spans))
        .block(header_block)
        .alignment(Alignment::Center)
        .style(theme.header_style());

    frame.render_widget(header_text, area);

    if area.height >= 2 {
        let fft_area = Rect::new(area.x, area.y + 1, area.width, 1);
        let fft_samples = app.fft_data();
        if !fft_samples.is_empty() {
            fft_analyzer::render_fft_analyzer(frame, fft_area, &fft_samples, theme);
        }
    }
}
