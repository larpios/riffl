/// Waveform editor for manual sample data drawing/editing.
///
/// Provides point-by-point waveform editing within a dedicated editor panel.
/// "Pencil mode" allows drawing sample values using keyboard controls.
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use ratatui_image::protocol::StatefulProtocol;
use riffl_core::audio::sample::{LoopMode, Sample};
use riffl_core::song::Instrument;

use crate::ui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformEditMode {
    Navigate,
    Pencil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMarkerDrag {
    None,
    Start,
    End,
}

pub struct WaveformEditorState {
    pub focused: bool,
    pub edit_mode: WaveformEditMode,
    pub cursor_sample: usize,
    pub pencil_value: f32,
    pub dragging_loop_marker: LoopMarkerDrag,
    pub loop_mode_toggle: bool,
    /// Cached pixel-image render state. `None` when no picker is available or
    /// no sample is loaded yet.
    pub image_state: Option<StatefulProtocol>,
    /// When `true` the pixel image must be rebuilt before the next render.
    pub image_dirty: bool,
    /// Pixel dimensions used when `image_state` was last built; used to detect
    /// terminal resize and force a rebuild.
    pub image_last_size: (u32, u32),
}

impl std::fmt::Debug for WaveformEditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaveformEditorState")
            .field("focused", &self.focused)
            .field("edit_mode", &self.edit_mode)
            .field("cursor_sample", &self.cursor_sample)
            .field("pencil_value", &self.pencil_value)
            .field("image_dirty", &self.image_dirty)
            .finish_non_exhaustive()
    }
}

impl Default for WaveformEditorState {
    fn default() -> Self {
        Self {
            focused: false,
            edit_mode: WaveformEditMode::Navigate,
            cursor_sample: 0,
            pencil_value: 0.0,
            dragging_loop_marker: LoopMarkerDrag::None,
            loop_mode_toggle: false,
            image_state: None,
            image_dirty: true,
            image_last_size: (0, 0),
        }
    }
}

impl WaveformEditorState {
    pub fn focus(&mut self) {
        self.focused = true;
    }

    pub fn unfocus(&mut self) {
        self.focused = false;
        self.edit_mode = WaveformEditMode::Navigate;
    }

    pub fn enter_pencil_mode(&mut self) {
        self.edit_mode = WaveformEditMode::Pencil;
    }

    pub fn exit_pencil_mode(&mut self) {
        self.edit_mode = WaveformEditMode::Navigate;
    }

    pub fn toggle_pencil_mode(&mut self) {
        self.edit_mode = match self.edit_mode {
            WaveformEditMode::Navigate => WaveformEditMode::Pencil,
            WaveformEditMode::Pencil => WaveformEditMode::Navigate,
        };
    }

    pub fn move_cursor_left(&mut self, max_samples: usize) {
        self.cursor_sample = self
            .cursor_sample
            .saturating_sub(1)
            .min(max_samples.saturating_sub(1));
    }

    pub fn move_cursor_right(&mut self, max_samples: usize) {
        self.cursor_sample = self
            .cursor_sample
            .saturating_add(1)
            .min(max_samples.saturating_sub(1));
    }

    pub fn set_cursor(&mut self, sample: usize) {
        self.cursor_sample = sample;
    }

    pub fn pencil_value_up(&mut self) {
        self.pencil_value = (self.pencil_value + 0.1).min(1.0);
    }

    pub fn pencil_value_down(&mut self) {
        self.pencil_value = (self.pencil_value - 0.1).max(-1.0);
    }

    pub fn draw_at_cursor(&self, sample: &mut Sample) {
        if self.cursor_sample < sample.frame_count() {
            sample.set_sample(self.cursor_sample, self.pencil_value);
        }
    }

    pub fn start_loop_marker_drag(&mut self, marker: LoopMarkerDrag) {
        self.dragging_loop_marker = marker;
    }

    pub fn end_loop_marker_drag(&mut self) {
        self.dragging_loop_marker = LoopMarkerDrag::None;
    }

    pub fn is_loop_marker_dragging(&self) -> bool {
        self.dragging_loop_marker != LoopMarkerDrag::None
    }

    pub fn dragging_loop_marker(&self) -> LoopMarkerDrag {
        self.dragging_loop_marker
    }

    pub fn is_loop_mode_enabled(&self) -> bool {
        self.loop_mode_toggle
    }

    pub fn toggle_loop_mode(&mut self) {
        self.loop_mode_toggle = !self.loop_mode_toggle;
    }
}

/// Convert a ratatui Color to an RGBA pixel value with the given alpha.
pub(crate) fn color_to_rgba(color: ratatui::style::Color, alpha: u8) -> Rgba<u8> {
    use ratatui::style::Color::*;
    match color {
        Rgb(r, g, b) => Rgba([r, g, b, alpha]),
        Cyan => Rgba([0, 255, 255, alpha]),
        LightCyan => Rgba([150, 255, 255, alpha]),
        Blue => Rgba([0, 0, 255, alpha]),
        LightBlue => Rgba([100, 149, 237, alpha]),
        Green => Rgba([0, 200, 0, alpha]),
        LightGreen => Rgba([100, 220, 100, alpha]),
        Yellow => Rgba([255, 220, 0, alpha]),
        LightYellow => Rgba([255, 240, 100, alpha]),
        Red => Rgba([220, 0, 0, alpha]),
        LightRed => Rgba([255, 100, 100, alpha]),
        Magenta => Rgba([200, 0, 200, alpha]),
        LightMagenta => Rgba([255, 100, 255, alpha]),
        White => Rgba([255, 255, 255, alpha]),
        Gray => Rgba([170, 170, 170, alpha]),
        DarkGray => Rgba([85, 85, 85, alpha]),
        Black => Rgba([0, 0, 0, alpha]),
        Reset => Rgba([20, 20, 20, alpha]),
        _ => Rgba([200, 200, 200, alpha]),
    }
}

/// Build a pixel-accurate waveform image from sample data.
///
/// `px_w` × `px_h` are the pixel dimensions of the target area.
/// Returns a `DynamicImage` to pass to `picker.new_resize_protocol()`.
pub(crate) fn build_waveform_image(
    sample: &Sample,
    px_w: u32,
    px_h: u32,
    theme: &Theme,
) -> DynamicImage {
    let px_w = px_w.max(1);
    let px_h = px_h.max(1);
    let mut img: RgbaImage = ImageBuffer::new(px_w, px_h);

    // Transparent background — composites against terminal bg for Kitty/Sixel;
    // halfblock blends against the cell background color.
    for p in img.pixels_mut() {
        *p = Rgba([0, 0, 0, 0]);
    }

    let center_y = px_h / 2;
    let dim_color = color_to_rgba(theme.text_dimmed, 120);

    // Center line (drawn first, waveform bars overdraw it if tall enough).
    for x in 0..px_w {
        img.put_pixel(x, center_y, dim_color);
    }

    let frame_count = sample.frame_count();
    if frame_count == 0 {
        return DynamicImage::ImageRgba8(img);
    }

    let wf_color = color_to_rgba(theme.primary, 220);
    let channels = sample.channels() as usize;

    for px_x in 0..px_w {
        let start = (px_x as usize * frame_count) / px_w as usize;
        let end = (((px_x as usize + 1) * frame_count) / px_w as usize)
            .max(start + 1)
            .min(frame_count);

        let mut peak_pos: f32 = 0.0;
        let mut peak_neg: f32 = 0.0;
        for frame_idx in start..end {
            let idx = frame_idx * channels;
            if idx < sample.data().len() {
                let v = sample.data()[idx];
                if v > peak_pos {
                    peak_pos = v;
                }
                if -v > peak_neg {
                    peak_neg = -v;
                }
            }
        }

        if peak_pos > 0.001 {
            let bar = (peak_pos * center_y as f32) as u32;
            let top_y = center_y.saturating_sub(bar);
            for py in top_y..=center_y {
                img.put_pixel(px_x, py, wf_color);
            }
        }
        if peak_neg > 0.001 {
            let bar = (peak_neg * (px_h - center_y - 1) as f32) as u32;
            let bot_y = (center_y + bar).min(px_h - 1);
            for py in center_y..=bot_y {
                img.put_pixel(px_x, py, wf_color);
            }
        }
    }

    DynamicImage::ImageRgba8(img)
}

const MAX_WAVEFORM_DISPLAY_HEIGHT: usize = 8;

#[allow(clippy::needless_range_loop)]
fn draw_waveform_graphic(
    sample: &Sample,
    width: usize,
    height: usize,
    theme: &Theme,
    cursor_sample: Option<usize>,
    edit_mode: WaveformEditMode,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(height.min(MAX_WAVEFORM_DISPLAY_HEIGHT));

    let frame_count = sample.frame_count();
    if frame_count == 0 || width < 4 {
        let dash = "─".repeat(width.max(4));
        let s = Style::default().fg(theme.text_dimmed);
        for _ in 0..height.min(MAX_WAVEFORM_DISPLAY_HEIGHT) {
            lines.push(Line::from(Span::styled(dash.clone(), s)));
        }
        return lines;
    }

    let grid_width = width.saturating_sub(2);
    let grid_height = height.saturating_sub(1).min(MAX_WAVEFORM_DISPLAY_HEIGHT);

    if grid_height < 2 {
        let dash = "─".repeat(width.max(4));
        let s = Style::default().fg(theme.text_dimmed);
        lines.push(Line::from(Span::styled(dash, s)));
        return lines;
    }

    let center_row = grid_height / 2;
    let mut grid: Vec<Vec<char>> = vec![vec![' '; grid_width]; grid_height];

    let mut cursor_col: Option<usize> = None;
    let cursor_char = match edit_mode {
        WaveformEditMode::Pencil => '◆',
        WaveformEditMode::Navigate => '│',
    };

    let has_loop = sample.loop_mode != riffl_core::audio::sample::LoopMode::NoLoop;
    let loop_start_col = if has_loop {
        Some(sample.loop_start * grid_width / frame_count.max(1))
    } else {
        None
    };
    let loop_end_col = if has_loop {
        Some(sample.loop_end * grid_width / frame_count.max(1))
    } else {
        None
    };

    for col in 0..grid_width {
        let start_frame = (col * frame_count) / grid_width;
        let end_frame = ((col + 1) * frame_count / grid_width)
            .max(start_frame + 1)
            .min(frame_count);

        let mut peak_pos: f32 = 0.0;
        let mut peak_neg: f32 = 0.0;
        for frame_idx in start_frame..end_frame {
            let channels = sample.channels() as usize;
            if frame_idx * channels < sample.data().len() {
                let v = sample.data()[frame_idx * channels];
                if v > peak_pos {
                    peak_pos = v;
                }
                if -v > peak_neg {
                    peak_neg = -v;
                }
            }
        }

        let top_row = center_row
            .saturating_sub((peak_pos * (center_row as f32)) as usize)
            .min(center_row);
        let bot_row =
            (center_row + 1 + (peak_neg * ((grid_height - center_row - 1) as f32)) as usize)
                .min(grid_height - 1);

        if peak_pos > 0.01 {
            for row in top_row..=center_row {
                if row < grid_height {
                    grid[row][col] = '█';
                }
            }
        }
        if peak_neg > 0.01 {
            for row in center_row..=bot_row {
                if row < grid_height {
                    grid[row][col] = '█';
                }
            }
        }

        if peak_pos > 0.01 && peak_neg > 0.01 && center_row < grid_height {
            grid[center_row][col] = '█';
        }

        if let Some(cur) = cursor_sample {
            let cur_col = (cur * grid_width) / frame_count;
            if cur_col == col {
                cursor_col = Some(col);
                if let Some(row) = grid.get_mut(center_row) {
                    row[col] = cursor_char;
                }
            }
        }
    }

    let wf_style = Style::default().fg(theme.primary);
    let cursor_style = if edit_mode == WaveformEditMode::Pencil {
        Style::default()
            .fg(theme.warning_color())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.cursor_fg)
    };
    let center_style = Style::default().fg(theme.text_dimmed);

    for (row_idx, row) in grid.iter().enumerate() {
        let row_str: String = row.iter().collect();
        let row_style = if row_idx == center_row {
            center_style
        } else {
            wf_style
        };
        lines.push(Line::from(vec![Span::styled(row_str, row_style)]));
    }

    if let Some(col) = cursor_col {
        let line_idx = center_row.min(lines.len().saturating_sub(1));
        if let Some(line) = lines.get_mut(line_idx) {
            let mut spans = line.spans.clone();
            let char_at_cursor = if edit_mode == WaveformEditMode::Pencil {
                '◆'
            } else {
                '│'
            };
            if col < spans.len() {
                spans[col] = Span::styled(char_at_cursor.to_string(), cursor_style);
            }
            lines[line_idx] = Line::from(spans);
        }
    }

    let loop_start_style = Style::default()
        .fg(theme.status_success)
        .add_modifier(Modifier::BOLD);
    let loop_end_style = Style::default()
        .fg(theme.status_error)
        .add_modifier(Modifier::BOLD);

    if let Some(col) = loop_start_col {
        let line_idx = center_row.min(lines.len().saturating_sub(1));
        if let Some(line) = lines.get_mut(line_idx) {
            let mut spans = line.spans.clone();
            if col < spans.len() {
                spans[col] = Span::styled("◁", loop_start_style);
            }
            lines[line_idx] = Line::from(spans);
        }
    }

    if let Some(col) = loop_end_col {
        let line_idx = center_row.min(lines.len().saturating_sub(1));
        if let Some(line) = lines.get_mut(line_idx) {
            let mut spans = line.spans.clone();
            if col < spans.len() {
                spans[col] = Span::styled("▷", loop_end_style);
            }
            lines[line_idx] = Line::from(spans);
        }
    }

    lines
}

fn format_sample_value(value: f32) -> String {
    if value.abs() < 0.001 {
        "  0.00".to_string()
    } else if value >= 0.0 {
        format!("+{:.2}", value)
    } else {
        format!("{:.2}", value)
    }
}

fn format_chip_preview(bytes: &[u8], count: usize) -> String {
    bytes
        .iter()
        .take(count)
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render cursor and loop markers as a ratatui text overlay on top of the
/// Write cursor and loop-marker characters directly into specific buffer cells,
/// without touching any other cells so the pixel image underneath shows through.
fn render_waveform_overlays(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    sample: &Sample,
    state: &WaveformEditorState,
    theme: &Theme,
) {
    let width = area.width as usize;
    let height = area.height as usize;
    let frame_count = sample.frame_count();
    if width == 0 || height == 0 || frame_count == 0 {
        return;
    }

    let center_row = (height / 2).min(height.saturating_sub(1)) as u16;
    let cursor_col = (state.cursor_sample * width / frame_count.max(1)) as u16;

    // Collect (col, char, style) triples — lower index = lower priority.
    let mut marks: Vec<(u16, &str, Style)> = Vec::new();

    if (cursor_col as usize) < width {
        let (ch, s) = match state.edit_mode {
            WaveformEditMode::Pencil => (
                "◆",
                Style::default()
                    .fg(theme.warning_color())
                    .add_modifier(Modifier::BOLD),
            ),
            WaveformEditMode::Navigate => ("│", Style::default().fg(theme.cursor_fg)),
        };
        marks.push((cursor_col, ch, s));
    }

    if sample.loop_mode != LoopMode::NoLoop {
        let ls = (sample.loop_start * width / frame_count.max(1)) as u16;
        let le = (sample.loop_end * width / frame_count.max(1)) as u16;
        if (ls as usize) < width {
            marks.push((
                ls,
                "◁",
                Style::default()
                    .fg(theme.status_success)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if (le as usize) < width {
            marks.push((
                le,
                "▷",
                Style::default()
                    .fg(theme.status_error)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    // Write only the marked cells — leaves all image-widget cells untouched.
    let buf = frame.buffer_mut();
    let row_y = area.top() + center_row;
    for (col, ch, style) in marks {
        let x = area.left() + col;
        if x < area.right() {
            if let Some(cell) = buf.cell_mut((x, row_y)) {
                cell.set_symbol(ch);
                cell.set_style(style);
                cell.set_skip(false);
            }
        }
    }
}

pub fn render_waveform_editor(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    instrument: &Instrument,
    sample: Option<&Sample>,
    state: &mut WaveformEditorState,
    theme: &Theme,
    picker: Option<&mut ratatui_image::picker::Picker>,
) {
    let border_style = if state.focused {
        theme.focused_border_style()
    } else {
        theme.border_style()
    };

    let proto_tag = match &picker {
        Some(p) => match p.protocol_type() {
            ratatui_image::picker::ProtocolType::Kitty => "[kitty]",
            ratatui_image::picker::ProtocolType::Sixel => "[sixel]",
            ratatui_image::picker::ProtocolType::Iterm2 => "[iterm2]",
            ratatui_image::picker::ProtocolType::Halfblocks => "[half]",
        },
        None => "[chr]",
    };
    let title = format!(
        " Waveform Editor {} {} ",
        match state.edit_mode {
            WaveformEditMode::Navigate => "[NAV]",
            WaveformEditMode::Pencil => "[PENCIL]",
        },
        proto_tag,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_alignment(Alignment::Left);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let width = inner.width as usize;
    let height = inner.height as usize;

    if width < 10 || height < 4 {
        return;
    }

    // ── Header (sample metadata + chip info) ──────────────────────────────────
    let sample_info = if let Some(s) = sample {
        if s.is_empty() {
            "No sample loaded".to_string()
        } else {
            let ch = s.channels();
            let sr = s.sample_rate();
            let frames = s.frame_count();
            let dur = s.duration();
            let loop_info = if s.loop_mode != LoopMode::NoLoop {
                format!(
                    " · ◁{:05} ▷{:05}",
                    s.loop_start.min(frames.saturating_sub(1)),
                    s.loop_end.min(frames.saturating_sub(1))
                )
            } else {
                String::new()
            };
            format!(
                "{:.2}s · {}Hz · {}ch · {} frames{}",
                dur, sr, ch, frames, loop_info
            )
        }
    } else {
        "No sample".to_string()
    };

    let help_text = if state.focused {
        match state.edit_mode {
            WaveformEditMode::Navigate => {
                "h/←→: cursor  [/]: loop pts  l: cycle loop  p: pencil  Esc: exit"
            }
            WaveformEditMode::Pencil => "↑↓: draw value  h/←→: move  Enter: draw  p: exit",
        }
    } else {
        "Enter to edit"
    };

    // Number of header rows (info + optional chip line + blank separator).
    let has_chip = instrument.chip_render.is_some();
    let header_rows: u16 = if has_chip { 3 } else { 2 };

    // Number of footer rows (blank + help, plus pencil status when active).
    let pencil_active = state.focused && state.edit_mode == WaveformEditMode::Pencil;
    let footer_rows: u16 = if pencil_active { 4 } else { 2 };

    // Waveform area sits between header and footer.
    let waveform_rows = inner
        .height
        .saturating_sub(header_rows)
        .saturating_sub(footer_rows);

    let header_rect = ratatui::layout::Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: header_rows,
    };
    let waveform_rect = ratatui::layout::Rect {
        x: inner.x,
        y: inner.y + header_rows,
        width: inner.width,
        height: waveform_rows,
    };
    let footer_rect = ratatui::layout::Rect {
        x: inner.x,
        y: inner.y + header_rows + waveform_rows,
        width: inner.width,
        height: footer_rows,
    };

    // ── Render header ─────────────────────────────────────────────────────────
    {
        let mut header_lines: Vec<Line> = Vec::new();
        header_lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(sample_info, Style::default().fg(theme.text_secondary)),
        ]));
        if let Some(chip) = instrument.chip_render.as_ref() {
            header_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "Chip WT32 [{}] e={:.03} · DPCM{} [{}] e={:.03}",
                        format_chip_preview(&chip.wavetable_2a03, 8),
                        chip.wavetable_error,
                        chip.dpcm.len(),
                        format_chip_preview(&chip.dpcm, 6),
                        chip.dpcm_error
                    ),
                    Style::default().fg(theme.text_dimmed),
                ),
            ]));
        }
        header_lines.push(Line::from("")); // blank separator
        frame.render_widget(
            Paragraph::new(header_lines).alignment(Alignment::Left),
            header_rect,
        );
    }

    // ── Render waveform ───────────────────────────────────────────────────────
    if let Some(s) = sample {
        if waveform_rows >= 2 {
            // Try pixel rendering when a picker is available.
            if let Some(picker) = picker {
                let font_size = picker.font_size();
                let px_w = waveform_rect.width as u32 * font_size.0 as u32;
                let px_h = waveform_rect.height as u32 * font_size.1 as u32;
                let cur_size = (px_w, px_h);

                if state.image_dirty || state.image_last_size != cur_size {
                    let img = build_waveform_image(s, px_w, px_h, theme);
                    state.image_state = Some(picker.new_resize_protocol(img));
                    state.image_dirty = false;
                    state.image_last_size = cur_size;
                }

                if let Some(img_state) = state.image_state.as_mut() {
                    frame.render_stateful_widget(
                        ratatui_image::StatefulImage::default(),
                        waveform_rect,
                        img_state,
                    );
                    render_waveform_overlays(frame, waveform_rect, s, state, theme);
                }
            } else {
                // Character fallback.
                let graphic = draw_waveform_graphic(
                    s,
                    width,
                    waveform_rows as usize,
                    theme,
                    Some(state.cursor_sample),
                    state.edit_mode,
                );
                let char_lines: Vec<Line> = graphic
                    .into_iter()
                    .map(|line| {
                        let mut spans = vec![Span::raw("  ")];
                        spans.extend(line.spans);
                        Line::from(spans)
                    })
                    .collect();
                frame.render_widget(
                    Paragraph::new(char_lines).alignment(Alignment::Left),
                    waveform_rect,
                );
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new(vec![Line::from(vec![Span::styled(
                "  No sample to edit. Load a sample first.",
                Style::default().fg(theme.text_dimmed),
            )])])
            .alignment(Alignment::Left),
            waveform_rect,
        );
    }

    // ── Render footer ─────────────────────────────────────────────────────────
    {
        let mut footer_lines: Vec<Line> = Vec::new();
        footer_lines.push(Line::from("")); // blank separator
        if pencil_active {
            if let Some(s) = sample {
                let current_value = if state.cursor_sample < s.frame_count() {
                    s.data()[state.cursor_sample * s.channels() as usize]
                } else {
                    0.0
                };
                let pencil_val = state.pencil_value;
                footer_lines.push(Line::from(vec![Span::styled(
                    format!(
                        "  Cursor: {:4}  Pencil: {}  ",
                        state.cursor_sample,
                        format_sample_value(pencil_val)
                    ),
                    Style::default().fg(theme.text),
                )]));
                footer_lines.push(Line::from(vec![Span::styled(
                    format!("  Current: {}  ", format_sample_value(current_value)),
                    Style::default().fg(theme.text_dimmed),
                )]));
            }
        }
        footer_lines.push(Line::from(vec![Span::styled(
            format!("  {}", help_text),
            Style::default().fg(theme.text_dimmed),
        )]));
        frame.render_widget(
            Paragraph::new(footer_lines).alignment(Alignment::Left),
            footer_rect,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sample() -> Sample {
        let data: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        Sample::new(data, 44100, 1, Some("test".to_string()))
    }

    #[test]
    fn test_waveform_editor_state_defaults() {
        let state = WaveformEditorState::default();
        assert!(!state.focused);
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
        assert_eq!(state.cursor_sample, 0);
    }

    #[test]
    fn test_focus_unfocus() {
        let mut state = WaveformEditorState::default();
        state.focus();
        assert!(state.focused);
        state.unfocus();
        assert!(!state.focused);
    }

    #[test]
    fn test_pencil_mode_toggle() {
        let mut state = WaveformEditorState::default();
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
        state.enter_pencil_mode();
        assert_eq!(state.edit_mode, WaveformEditMode::Pencil);
        state.exit_pencil_mode();
        assert_eq!(state.edit_mode, WaveformEditMode::Navigate);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = WaveformEditorState::default();
        state.move_cursor_right(100);
        assert_eq!(state.cursor_sample, 1);
        state.move_cursor_left(100);
        assert_eq!(state.cursor_sample, 0);
        state.move_cursor_left(100);
        assert_eq!(state.cursor_sample, 0);
    }

    #[test]
    fn test_pencil_value() {
        let mut state = WaveformEditorState::default();
        state.pencil_value_up();
        assert!((state.pencil_value - 0.1).abs() < 0.001);
        state.pencil_value_down();
        state.pencil_value_down();
        assert!((state.pencil_value - (-0.1)).abs() < 0.001);
    }

    #[test]
    fn test_sample_set_sample() {
        let mut sample = make_sample();
        let original = sample.data()[0];
        sample.set_sample(0, 0.5);
        assert_eq!(sample.data()[0], 0.5);
        sample.set_sample(0, original);
    }

    #[test]
    fn test_sample_set_sample_clamp() {
        let mut sample = make_sample();
        sample.set_sample(0, 2.0);
        assert_eq!(sample.data()[0], 1.0);
        sample.set_sample(0, -2.0);
        assert_eq!(sample.data()[0], -1.0);
    }

    #[test]
    fn test_render_no_panic() {
        let inst = Instrument::new("TestInst");
        let mut state = WaveformEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(frame, frame.area(), &inst, None, &mut state, &theme, None);
            })
            .unwrap();
    }

    #[test]
    fn test_render_with_sample_no_panic() {
        let inst = Instrument::new("TestInst");
        let sample = make_sample();
        let mut state = WaveformEditorState::default();
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(
                    frame,
                    frame.area(),
                    &inst,
                    Some(&sample),
                    &mut state,
                    &theme,
                    None,
                );
            })
            .unwrap();
    }

    #[test]
    fn test_render_focused_pencil_no_panic() {
        let inst = Instrument::new("TestInst");
        let sample = make_sample();
        let mut state = WaveformEditorState::default();
        state.focus();
        state.enter_pencil_mode();
        state.cursor_sample = 50;
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_waveform_editor(
                    frame,
                    frame.area(),
                    &inst,
                    Some(&sample),
                    &mut state,
                    &theme,
                    None,
                );
            })
            .unwrap();
    }
}
