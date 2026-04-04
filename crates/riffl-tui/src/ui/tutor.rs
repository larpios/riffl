use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::theme::Theme;

/// Total line count of the tutor content. Used to cap scroll offset.
pub fn content_line_count() -> u16 {
    tutor_content(&Theme::default()).len() as u16
}

/// Render the :tutor full-screen scrollable help view.
/// `scroll` is the vertical scroll offset in lines.
/// `filter` highlights / filters lines containing the term.
/// `filter_active` determines whether the search bar cursor is shown.
pub fn render_tutor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    theme: &Theme,
    scroll: u16,
    filter: &str,
    filter_active: bool,
) {
    let tutor_area = super::layout::create_centered_rect(area, 90, 92);
    frame.render_widget(Clear, tutor_area);

    let title = " RIFFL TUTOR  (j/k · Ctrl+D/U · / search · q/Esc close) ";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title(title)
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme.bg_surface));

    let inner = block.inner(tutor_area);
    frame.render_widget(block, tutor_area);

    let (content_area, search_area) = if filter_active || !filter.is_empty() {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (split[0], Some(split[1]))
    } else {
        (inner, None)
    };

    let content = if filter.is_empty() {
        tutor_content(theme)
    } else {
        filter_tutor_lines(tutor_content(theme), filter)
    };

    frame.render_widget(
        Paragraph::new(content)
            .scroll((scroll, 0))
            .style(Style::default().fg(theme.text).bg(theme.bg_surface)),
        content_area,
    );

    if let Some(bar) = search_area {
        render_search_bar(frame, bar, filter, filter_active, theme);
    }
}

/// Render a one-line search bar at the bottom of the overlay.
fn render_search_bar(frame: &mut Frame, area: Rect, filter: &str, active: bool, theme: &Theme) {
    let cursor = if active { "_" } else { "" };
    let bar_line = Line::from(vec![
        Span::styled(
            "/ ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{}{}", filter, cursor),
            Style::default().fg(theme.text),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(bar_line).style(Style::default().bg(theme.bg_surface)),
        area,
    );
}

/// Filter tutor lines to those containing `filter` (case-insensitive).
/// Section headers (bold primary lines) are retained only when they have matching content nearby.
fn filter_tutor_lines(lines: Vec<Line<'static>>, filter: &str) -> Vec<Line<'static>> {
    let filter_lower = filter.to_lowercase();
    lines
        .into_iter()
        .filter(|line| {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            text.to_lowercase().contains(&filter_lower)
        })
        .collect()
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn section(label: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {}", label),
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

fn subsection(label: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", label),
        Style::default()
            .fg(theme.info_color())
            .add_modifier(Modifier::BOLD),
    ))
}

fn key(keys: &str, desc: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {:<22}", keys),
            Style::default().fg(theme.success_color()),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.text)),
    ])
}

fn effect_row(cmd: &str, param: &str, name: &str, desc: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {:<6}", cmd),
            Style::default()
                .fg(theme.warning_color())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<12}", param),
            Style::default().fg(theme.success_color()),
        ),
        Span::styled(
            format!("{:<14}", name),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.text_dimmed)),
    ])
}

fn note_row(key_str: &str, note: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("    {:<6}", key_str),
            Style::default().fg(theme.success_color()),
        ),
        Span::styled(note.to_string(), Style::default().fg(theme.text)),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn text(s: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", s),
        Style::default().fg(theme.text_dimmed),
    ))
}

// ── main content ─────────────────────────────────────────────────────────────

#[allow(clippy::vec_init_then_push)]
fn tutor_content(theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // ── Welcome ──────────────────────────────────────────────────────────────
    lines.push(blank());
    lines.push(section("WELCOME TO RIFFL", theme));
    lines.push(blank());
    lines.push(text(
        "Riffl is a Rust-based music tracker inspired by FastTracker 2, Impulse Tracker,",
        theme,
    ));
    lines.push(text(
        "and modern live-coding tools. It uses a grid-based pattern editor where rows",
        theme,
    ));
    lines.push(text(
        "are steps in time and columns are tracks (channels).",
        theme,
    ));
    lines.push(blank());
    lines.push(text(
        "Each cell can hold: a Note, an Instrument number, a Volume, and an Effect.",
        theme,
    ));
    lines.push(text(
        "Type  :tutor  at any time to return here.  Press q or Esc to close.",
        theme,
    ));
    lines.push(blank());

    // ── Modes ────────────────────────────────────────────────────────────────
    lines.push(section("MODES", theme));
    lines.push(blank());
    lines.push(subsection("Normal mode  (default)", theme));
    lines.push(text(
        "Navigate the pattern with h/j/k/l or arrow keys. Execute commands with :",
        theme,
    ));
    lines.push(text(
        "Press i to enter Insert mode, v to enter Visual mode.",
        theme,
    ));
    lines.push(blank());
    lines.push(subsection("Insert mode  (i)", theme));
    lines.push(text(
        "Type notes using the piano keyboard layout (see NOTE ENTRY below).",
        theme,
    ));
    lines.push(text(
        "Type hex digits to fill instrument, volume, and effect columns.",
        theme,
    ));
    lines.push(text("Press Esc to return to Normal mode.", theme));
    lines.push(blank());
    lines.push(subsection("Visual mode  (v)", theme));
    lines.push(text(
        "Extend a selection with h/j/k/l, then copy/paste/delete the selection.",
        theme,
    ));
    lines.push(text("Press Esc to return to Normal mode.", theme));
    lines.push(blank());

    // ── Note entry ───────────────────────────────────────────────────────────
    lines.push(section(
        "NOTE ENTRY  (Insert mode — piano keyboard layout)",
        theme,
    ));
    lines.push(blank());
    lines.push(text(
        "The keyboard maps to piano keys.  Bottom row = white keys, top row = black.",
        theme,
    ));
    lines.push(blank());
    lines.push(text("  Top row (black keys):", theme));
    lines.push(blank());
    lines.push(note_row("W", "C#  (C-sharp)", theme));
    lines.push(note_row("E", "D#  (D-sharp / Eb)", theme));
    lines.push(note_row("T", "F#  (F-sharp)", theme));
    lines.push(note_row("Y", "G#  (G-sharp / Ab)", theme));
    lines.push(note_row("U", "A#  (A-sharp / Bb)", theme));
    lines.push(blank());
    lines.push(text("  Bottom row (white keys):", theme));
    lines.push(blank());
    lines.push(note_row("A", "C   (do)", theme));
    lines.push(note_row("S", "D   (re)", theme));
    lines.push(note_row("D", "E   (mi)", theme));
    lines.push(note_row("F", "F   (fa)", theme));
    lines.push(note_row("G", "G   (sol)", theme));
    lines.push(note_row("H", "A   (la)", theme));
    lines.push(note_row("J", "B   (si)", theme));
    lines.push(note_row("K", "C+1 (C in the next octave)", theme));
    lines.push(blank());
    lines.push(text("  Other Insert-mode keys:", theme));
    lines.push(blank());
    lines.push(key("0 – 9", "Set current octave (0–9)", theme));
    lines.push(key("~  (tilde)", "Enter note-off  ===", theme));
    lines.push(key("Del", "Enter note-cut  ^^^  (hard silence)", theme));
    lines.push(key("{ / }", "Decrease / increase step size", theme));
    lines.push(key(":step N", "Set step size to N rows (0–8)", theme));
    lines.push(blank());
    lines.push(text(
        "  Octave is shown in the status bar. Adjust it before entering notes.",
        theme,
    ));
    lines.push(blank());

    // ── Pattern columns ──────────────────────────────────────────────────────
    lines.push(section("PATTERN COLUMNS", theme));
    lines.push(blank());
    lines.push(text(
        "Each cell in the pattern grid has four sub-columns:",
        theme,
    ));
    lines.push(blank());
    lines.push(key("NOTE", "e.g. C-4  D#5  ===  ^^^", theme));
    lines.push(key(
        "INST  (2 hex digits)",
        "Instrument number 00–FF",
        theme,
    ));
    lines.push(key(
        "VOL   (2 hex digits)",
        "Volume override 00–40 (40 = full)",
        theme,
    ));
    lines.push(key(
        "EFF   (1 + 2 hex)",
        "Effect command + parameter",
        theme,
    ));
    lines.push(blank());
    lines.push(text(
        "Navigate sub-columns with h / l (left/right within a cell).",
        theme,
    ));
    lines.push(text(
        "Navigate tracks (channels) with Tab / Shift+Tab.",
        theme,
    ));
    lines.push(blank());

    // ── Effect table ─────────────────────────────────────────────────────────
    lines.push(section("EFFECT TABLE", theme));
    lines.push(blank());
    lines.push(text(
        "Effects are entered in the last sub-column of each cell.",
        theme,
    ));
    lines.push(text(
        "Format: Cxx  where C = command nibble (0–F), xx = parameter byte.",
        theme,
    ));
    lines.push(blank());
    lines.push(Line::from(vec![Span::styled(
        "    CMD   PARAM   NAME          DESCRIPTION",
        Style::default()
            .fg(theme.text_dimmed)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(Span::styled(
        "    ─────────────────────────────────────────────────────────────────────",
        Style::default().fg(theme.text_dimmed),
    )));
    lines.push(effect_row(
        "0xy",
        "x,y",
        "Arpeggio",
        "Cycle base note → +x semitones → +y semitones per tick",
        theme,
    ));
    lines.push(effect_row(
        "1xx",
        "speed",
        "Pitch Up",
        "Slide pitch up by xx units per row",
        theme,
    ));
    lines.push(effect_row(
        "2xx",
        "speed",
        "Pitch Down",
        "Slide pitch down by xx units per row",
        theme,
    ));
    lines.push(effect_row(
        "3xx",
        "speed",
        "Portamento",
        "Slide smoothly to the new note at speed xx",
        theme,
    ));
    lines.push(effect_row(
        "4xy",
        "x=spd,y=dep",
        "Vibrato",
        "Oscillate pitch — speed x, depth y",
        theme,
    ));
    lines.push(effect_row(
        "5xy",
        "x=up,y=dn",
        "Porta+VolSlide",
        "Portamento-to-note + volume slide simultaneously",
        theme,
    ));
    lines.push(effect_row(
        "6xy",
        "x=up,y=dn",
        "Vib+VolSlide",
        "Vibrato + volume slide simultaneously",
        theme,
    ));
    lines.push(effect_row(
        "7xy",
        "x=spd,y=dep",
        "Tremolo",
        "Oscillate amplitude — speed x, depth y",
        theme,
    ));
    lines.push(effect_row(
        "9xx",
        "offset",
        "Sample Offset",
        "Start sample playback at xx × 256 samples",
        theme,
    ));
    lines.push(effect_row(
        "Axy",
        "x=up,y=dn",
        "Volume Slide",
        "Raise volume by x or lower by y each tick",
        theme,
    ));
    lines.push(effect_row(
        "Bxx",
        "pos",
        "Position Jump",
        "Jump to arrangement position xx",
        theme,
    ));
    lines.push(effect_row(
        "Cxx",
        "vol",
        "Set Volume",
        "Set channel volume to xx (00–40)",
        theme,
    ));
    lines.push(effect_row(
        "Dxx",
        "row",
        "Pattern Break",
        "Jump to row xx of the next pattern",
        theme,
    ));
    lines.push(effect_row(
        "Exy",
        "x=cmd,y=val",
        "Extended",
        "Sub-commands: E1y=fine up, E2y=fine down, ECy=note cut at tick y, EBy=loop",
        theme,
    ));
    lines.push(effect_row(
        "8xx",
        "bpm",
        "Set Tempo",
        "Set playback BPM to xx (00-FF)",
        theme,
    ));
    lines.push(effect_row(
        "Fxx",
        "tpl",
        "Set Speed/TPL",
        "Set ticks-per-line (speed) to xx",
        theme,
    ));
    lines.push(blank());

    // ── Commands ─────────────────────────────────────────────────────────────
    lines.push(section("COMMAND LINE  (press : to open)", theme));
    lines.push(blank());
    lines.push(key(":w", "Save project", theme));
    lines.push(key(":w <file>", "Save project to <file>", theme));
    lines.push(key(":wq  / :x", "Save and quit", theme));
    lines.push(key(":q", "Quit (prompts if unsaved)", theme));
    lines.push(key(":q!", "Force quit without saving", theme));
    lines.push(key(":e <file>", "Open / load project from <file>", theme));
    lines.push(key(":bpm <n>", "Set BPM to n (20–999)", theme));
    lines.push(key(":t <n>  / :tempo <n>", "Alias for :bpm", theme));
    lines.push(key(":step <n>", "Set cursor step size (0–8)", theme));
    lines.push(key(":volume <n>", "Set global volume 0–100", theme));
    lines.push(key(
        ":speed <n>  / :tpl <n>",
        "Set ticks per line (1–31)",
        theme,
    ));
    lines.push(key(
        ":len <n>  / :length <n>",
        "Resize current pattern (16–512 rows)",
        theme,
    ));
    lines.push(key(
        ":transpose <n>  / :tr <n>",
        "Transpose selection by n semitones",
        theme,
    ));
    lines.push(key(":quantize", "Quantize selection to step grid", theme));
    lines.push(key(
        ":interpolate  / :interp",
        "Interpolate volume across visual selection",
        theme,
    ));
    lines.push(key(":clear", "Clear all cells in current pattern", theme));
    lines.push(key(
        ":fill <note> [<step>]",
        "Fill current channel with note every step rows",
        theme,
    ));
    lines.push(key(
        ":loop <start> <end>",
        "Set loop region rows and activate",
        theme,
    ));
    lines.push(key(
        ":adsr <A> <D> <S%> <R>",
        "Set ADSR volume envelope on current instrument",
        theme,
    ));
    lines.push(key(":rename <name>", "Rename current track/channel", theme));
    lines.push(key(
        ":pname <name>",
        "Rename current/selected pattern",
        theme,
    ));
    lines.push(key(
        ":dup",
        "Duplicate current/selected pattern to a new slot",
        theme,
    ));
    lines.push(key(
        ":track <add|del>",
        "Add/remove channel in current pattern",
        theme,
    ));
    lines.push(key(
        ":title <name>",
        "Set song title (shown in header)",
        theme,
    ));
    lines.push(key(":artist <name>", "Set song artist name", theme));
    lines.push(key(
        ":mode <native|compat|amiga>",
        "Switch effect interpretation mode",
        theme,
    ));
    lines.push(key(
        ":goto <n>  / :<n>",
        "Jump to row n (1-based; also m key)",
        theme,
    ));
    lines.push(key(":tutor", "Open this help view", theme));
    lines.push(blank());

    // ── Navigation ───────────────────────────────────────────────────────────
    lines.push(section("NAVIGATION", theme));
    lines.push(blank());
    lines.push(key("h j k l  /  arrows", "Move cursor", theme));
    lines.push(key("Tab  /  Shift+Tab", "Next / previous track", theme));
    lines.push(key("PageUp / PageDown", "Page up / down (8 rows)", theme));
    lines.push(key("gg  (chord)", "Go to row 0", theme));
    lines.push(key("( )", "Octave down / up", theme));
    lines.push(key("1 – 6", "Switch view (1=Pattern, 2=Arrange, …)", theme));
    lines.push(key(
        "Ctrl+\\",
        "Toggle split-view (pattern + code editor)",
        theme,
    ));
    lines.push(blank());

    // ── Editing ──────────────────────────────────────────────────────────────
    lines.push(section("EDITING  (Normal mode)", theme));
    lines.push(blank());
    lines.push(key("x  /  Delete", "Delete current cell", theme));
    lines.push(key("Insert", "Insert a blank row at cursor", theme));
    lines.push(key("dd  (chord)", "Delete current row", theme));
    lines.push(key("u  /  Ctrl+R", "Undo / Redo", theme));
    lines.push(key("y  /  p", "Copy / Paste cell", theme));
    lines.push(key(
        "Ctrl+C  /  Ctrl+V",
        "Copy / Paste (alternative)",
        theme,
    ));
    lines.push(key("Ctrl+X", "Cut cell", theme));
    lines.push(key(
        "Shift+Up/Down",
        "Transpose selection +/- semitone",
        theme,
    ));
    lines.push(key(
        "Ctrl+Shift+Up/Down",
        "Transpose selection +/- octave",
        theme,
    ));
    lines.push(blank());

    // ── Tracks ───────────────────────────────────────────────────────────────
    lines.push(section("TRACKS", theme));
    lines.push(blank());
    lines.push(key("T", "Add new track", theme));
    lines.push(key("D", "Delete current track", theme));
    lines.push(key("C", "Clone current track", theme));
    lines.push(key("M", "Mute / unmute track", theme));
    lines.push(key("S", "Solo / unsolo track", theme));
    lines.push(key("Alt+Up  /  Alt+Down", "Track volume +5% / -5%", theme));
    lines.push(key(
        "Alt+Left  /  Alt+Right",
        "Track pan left / right 10%",
        theme,
    ));
    lines.push(key("Q", "Quantize selection", theme));
    lines.push(blank());

    // ── Transport ────────────────────────────────────────────────────────────
    lines.push(section("TRANSPORT", theme));
    lines.push(blank());
    lines.push(key(
        "Space",
        "Play / Pause  (Instrument list: preview note)",
        theme,
    ));
    lines.push(key("Enter  (when stopped)", "Play from cursor row", theme));
    lines.push(key("=  /  -", "BPM +1 / -1", theme));
    lines.push(key("Ctrl+B", "BPM inline prompt", theme));
    lines.push(key(
        "t  (Normal mode)",
        "Tap tempo (≥2 taps average)",
        theme,
    ));
    lines.push(key("[  /  ]", "Previous / next pattern", theme));
    lines.push(key("Shift+P", "Toggle Pattern / Song playback mode", theme));
    lines.push(key(
        "f",
        "Toggle follow mode (cursor tracks playhead)",
        theme,
    ));
    lines.push(key("Alt+[", "Set loop start at cursor row", theme));
    lines.push(key("Alt+]", "Set loop end at cursor row", theme));
    lines.push(key("Ctrl+Shift+L", "Toggle loop region on / off", theme));
    lines.push(blank());

    // ── Views ────────────────────────────────────────────────────────────────
    lines.push(section("VIEWS", theme));
    lines.push(blank());
    lines.push(key("1", "Pattern editor", theme));
    lines.push(key("2", "Arrangement editor", theme));
    lines.push(key("3", "Instrument list", theme));
    lines.push(key("4", "Code / script editor", theme));
    lines.push(key("5", "Pattern list", theme));
    lines.push(key("6", "Sample browser", theme));
    lines.push(key("?", "Toggle keyboard shortcuts overlay", theme));
    lines.push(blank());

    // ── Arrangement editor ───────────────────────────────────────────────────
    lines.push(section("ARRANGEMENT EDITOR  (view 2)", theme));
    lines.push(blank());
    lines.push(key("j  /  k", "Move cursor down / up", theme));
    lines.push(key("Insert  /  o", "Insert pattern at cursor", theme));
    lines.push(key("x  /  Del", "Delete entry at cursor", theme));
    lines.push(key("n", "Create new pattern and append", theme));
    lines.push(key(
        "c",
        "Clone pattern at cursor (deep copy, insert after)",
        theme,
    ));
    lines.push(key(
        "Ctrl+K  /  Ctrl+J",
        "Move entry up / down (reorder)",
        theme,
    ));
    lines.push(key("Enter", "Jump to selected pattern in editor", theme));
    lines.push(blank());

    // ── Project ──────────────────────────────────────────────────────────────
    lines.push(section("PROJECT", theme));
    lines.push(blank());
    lines.push(key("Ctrl+S", "Save project", theme));
    lines.push(key("Ctrl+O", "Load project", theme));
    lines.push(key("Ctrl+F", "Open file / sample browser", theme));
    lines.push(key("Ctrl+E", "Open audio export dialog", theme));
    lines.push(blank());

    // ── Visual mode ──────────────────────────────────────────────────────────
    lines.push(section("VISUAL MODE", theme));
    lines.push(blank());
    lines.push(key("h j k l", "Extend selection", theme));
    lines.push(key("x  /  d", "Delete / cut selection", theme));
    lines.push(key("y  /  p", "Yank (copy) / paste selection", theme));
    lines.push(key("I", "Interpolate selection", theme));
    lines.push(blank());

    // ── Sample browser ───────────────────────────────────────────────────────
    lines.push(section("SAMPLE BROWSER  (view 6)", theme));
    lines.push(blank());
    lines.push(key("j  /  k", "Navigate file list", theme));
    lines.push(key("l  /  Enter", "Enter directory / load sample", theme));
    lines.push(key("h", "Go up one directory", theme));
    lines.push(blank());

    // ── Live / Script ────────────────────────────────────────────────────────
    lines.push(section("LIVE / SCRIPT", theme));
    lines.push(blank());
    lines.push(key(
        "Ctrl+L",
        "Toggle live mode (scripts re-run each loop)",
        theme,
    ));
    lines.push(key(
        "Ctrl+Enter",
        "Execute current script (full pattern)",
        theme,
    ));
    lines.push(key(
        "Ctrl+Enter  (Visual mode)",
        "Execute script on visual selection",
        theme,
    ));
    lines.push(key("Ctrl+T", "Open script templates", theme));
    lines.push(blank());
    lines.push(text(
        "Live mode is shown as  LIVE  in the header bar.",
        theme,
    ));
    lines.push(blank());
    lines.push(text(
        "Full-pattern scope: num_rows, num_channels, get_note(r,c).",
        theme,
    ));
    lines.push(text(
        "DSL functions: set_note, fill_column, generate_beat, transpose,",
        theme,
    ));
    lines.push(text(
        "  reverse, rotate, humanize, shuffle, clear_cell, clear_pattern,",
        theme,
    ));
    lines.push(text(
        "  interpolate_vol(ch, start_row, end_row, start_vol, end_vol).",
        theme,
    ));
    lines.push(text(
        "Scope vars: num_rows, num_channels, bpm, tpl (ticks per line).",
        theme,
    ));
    lines.push(text(
        "Selection scope: sel_rows, sel_channels, get_note(r,c) (relative coords).",
        theme,
    ));
    lines.push(text(
        "zxx_triggers: array of #{channel,param} maps from Zxx effects.",
        theme,
    ));
    lines.push(text(
        "Example: for t in zxx_triggers { if t.channel==0 { transpose(t.param - 64); } }",
        theme,
    ));
    lines.push(blank());

    // ── Hooks ────────────────────────────────────────────────────────────────
    lines.push(section("HOOKS  (hooks.rhai)", theme));
    lines.push(blank());
    lines.push(text(
        "Place ~/.config/riffl/hooks.rhai to customise riffl with Rhai script.",
        theme,
    ));
    lines.push(text(
        "Run  riffl --dump-hooks  to print an annotated template to stdout.",
        theme,
    ));
    lines.push(blank());
    lines.push(subsection("Supported hook functions", theme));
    lines.push(blank());
    lines.push(key(
        "normalize_picker_path(raw)",
        "Transform raw picker output to a clean file path.",
        theme,
    ));
    lines.push(text(
        "Called before riffl opens a file selected by an external picker",
        theme,
    ));
    lines.push(text(
        "(yazi, lf, etc.). Return the corrected path string. If this",
        theme,
    ));
    lines.push(text(
        "function is not defined, built-in yazi search:// stripping runs.",
        theme,
    ));
    lines.push(blank());
    lines.push(key(
        "on_project_loaded(path)",
        "Called after a .rtm project is successfully loaded.",
        theme,
    ));
    lines.push(key(
        "on_sample_loaded(path, idx)",
        "Called after a sample is loaded into instrument slot idx.",
        theme,
    ));
    lines.push(key(
        "on_startup()",
        "Called once when riffl starts, after config is read.",
        theme,
    ));
    lines.push(blank());
    lines.push(subsection(
        "Example: custom picker path normalisation",
        theme,
    ));
    lines.push(blank());
    lines.push(text("fn normalize_picker_path(raw) {", theme));
    lines.push(text(
        "  // strip \"file://\" prefix from a custom picker",
        theme,
    ));
    lines.push(text(
        "  if raw.starts_with(\"file://\") { return raw.sub_string(7); }",
        theme,
    ));
    lines.push(text(
        "  raw  // return unchanged → built-in yazi handling still runs",
        theme,
    ));
    lines.push(text("}", theme));
    lines.push(blank());
    lines.push(text(
        "Undefined hooks are silently skipped. Errors are printed to stderr.",
        theme,
    ));
    lines.push(blank());

    lines
}
