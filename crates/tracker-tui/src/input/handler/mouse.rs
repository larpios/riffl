use crate::app::{App, AppView};
use crate::editor::EditorMode;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    use crate::ui::layout;

    if app.has_modal() || app.has_export_dialog() || app.has_file_browser() {
        return;
    }

    if app.show_help || app.show_tutor {
        match mouse.kind {
            MouseEventKind::ScrollDown => {
                if app.show_help {
                    app.help_scroll = app.help_scroll.saturating_add(3);
                } else if app.show_tutor {
                    app.tutor_scroll = app.tutor_scroll.saturating_add(3);
                }
            }
            MouseEventKind::ScrollUp => {
                if app.show_help {
                    app.help_scroll = app.help_scroll.saturating_sub(3);
                } else if app.show_tutor {
                    app.tutor_scroll = app.tutor_scroll.saturating_sub(3);
                }
            }
            _ => {}
        }
        return;
    }

    let full_area = ratatui::layout::Rect::new(0, 0, 80, 24);
    let (_header_area, content_area, _footer_area) = layout::create_main_layout(full_area, 3, 1);

    if app.current_view == AppView::InstrumentList {
        handle_instrument_view_mouse(app, mouse, content_area);
        return;
    }

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            if app.current_view == AppView::PatternEditor {
                app.editor.page_down();
            } else if app.current_view == AppView::Arrangement {
                let len = app.song.arrangement.len();
                app.arrangement_view.move_down(len);
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_down();
            }
        }
        MouseEventKind::ScrollUp => {
            if app.current_view == AppView::PatternEditor {
                app.editor.page_up();
            } else if app.current_view == AppView::Arrangement {
                app.arrangement_view.move_up();
            } else if app.current_view == AppView::PatternList {
                app.pattern_selection_up();
            }
        }
        MouseEventKind::Down(btn) | MouseEventKind::Drag(btn) | MouseEventKind::Up(btn) => {
            if btn != MouseButton::Left && btn != MouseButton::Right {
                return;
            }

            // Check if click is in header area - reset horizontal view
            if mouse.row < content_area.y && app.channel_scroll > 0 {
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    app.reset_horizontal_view();
                }
                return;
            }

            if mouse.column < content_area.x
                || mouse.column >= content_area.x + content_area.width
                || mouse.row < content_area.y
                || mouse.row >= content_area.y + content_area.height
            {
                return;
            }

            let mouse_x = mouse.column;
            let mouse_y = mouse.row;

            let pattern_width = content_area.width.saturating_sub(2);
            let pattern_height = content_area.height.saturating_sub(2);
            let pattern_x = content_area.x + 1;
            let pattern_y = content_area.y + 1;

            let _pattern_area =
                ratatui::layout::Rect::new(pattern_x, pattern_y, pattern_width, pattern_height);

            let ch_scroll = calculate_channel_scroll_for_mouse(
                app.editor.cursor_channel(),
                pattern_width,
                app.editor.pattern().num_channels(),
            );

            let header_height = 1u16;
            let visible_rows = pattern_height.saturating_sub(header_height) as usize;
            let scroll_offset = calculate_scroll_offset_for_mouse(
                app.editor.cursor_row(),
                visible_rows,
                app.editor.pattern().num_rows(),
            );

            match btn {
                MouseButton::Left => match mouse.kind {
                    MouseEventKind::Down(_) => {
                        if app.current_view == AppView::PatternEditor {
                            let local_x = mouse_x.saturating_sub(pattern_x);
                            let local_y = mouse_y.saturating_sub(pattern_y);

                            if local_y < header_height {
                                return;
                            }

                            let (row, ch) = app.editor.set_cursor_from_mouse(
                                local_y.saturating_sub(header_height),
                                local_x,
                                scroll_offset,
                                ch_scroll,
                            );

                            if app.editor.mode() != EditorMode::Visual {
                                if app.editor.mode() == EditorMode::Insert {
                                    app.editor.enter_normal_mode();
                                }
                                app.editor.enter_visual_mode();
                                app.editor.set_visual_anchor(row, ch);
                            } else {
                                app.editor.set_visual_anchor(row, ch);
                            }
                        }
                    }
                    MouseEventKind::Drag(_) => {
                        if app.current_view == AppView::PatternEditor
                            && app.editor.mode() == EditorMode::Visual
                        {
                            let local_x = mouse_x.saturating_sub(pattern_x);
                            let local_y = mouse_y.saturating_sub(pattern_y);

                            if local_y < header_height {
                                return;
                            }

                            let (row, ch) = app.editor.set_cursor_from_mouse(
                                local_y.saturating_sub(header_height),
                                local_x,
                                scroll_offset,
                                ch_scroll,
                            );
                            app.editor.set_cursor(row, ch);
                        }
                    }
                    MouseEventKind::Up(_) => {}
                    _ => {}
                },
                MouseButton::Right => {
                    if app.current_view == AppView::PatternEditor {
                        let local_x = mouse_x.saturating_sub(pattern_x);
                        let local_y = mouse_y.saturating_sub(pattern_y);

                        if local_y < header_height {
                            return;
                        }

                        let _ = app.editor.set_cursor_from_mouse(
                            local_y.saturating_sub(header_height),
                            local_x,
                            scroll_offset,
                            ch_scroll,
                        );
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub fn handle_instrument_editor_mouse(app: &mut App, mouse: MouseEvent, area: ratatui::layout::Rect) {
    use crate::ui::instrument_editor::{field_at_row, InstrumentField};

    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !point_in_rect(area, mouse.column, mouse.row) {
                return;
            }

            app.inst_editor.focus();
            if mouse.row < inner.y || mouse.row >= inner.y + inner.height {
                return;
            }

            let row_offset = mouse.row.saturating_sub(inner.y);
            let Some(field) = field_at_row(row_offset) else {
                return;
            };
            app.inst_editor.field = field;

            match field {
                InstrumentField::LoopMode => app.cycle_instrument_loop_mode(),
                InstrumentField::Name => {}
                _ if field.is_draggable() => {
                    app.inst_editor.start_drag(field, mouse.column, mouse.row);
                }
                _ => {}
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            let Some(field) = app.inst_editor.dragging() else {
                return;
            };
            let Some((dx, dy)) = app
                .inst_editor
                .update_drag_position(mouse.column, mouse.row)
            else {
                return;
            };
            apply_instrument_field_drag(app, field, dx - dy);
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.inst_editor.end_drag();
        }
        _ => {}
    }
}

pub fn handle_envelope_mouse(app: &mut App, mouse: MouseEvent, area: ratatui::layout::Rect) {
    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);
    if inner.width == 0 || inner.height < 4 {
        return;
    }

    let graph_height = inner.height.saturating_sub(3);
    if graph_height == 0 {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if !point_in_rect(area, mouse.column, mouse.row) {
                return;
            }

            app.env_editor.focus();
            if mouse.column < inner.x
                || mouse.column >= inner.x + inner.width
                || mouse.row < inner.y
                || mouse.row >= inner.y + graph_height
            {
                return;
            }

            let Some(idx) = app.instrument_selection() else {
                return;
            };
            let env_type = app.env_editor.envelope_type;
            let envelope = app.env_editor.get_envelope(&app.song.instruments[idx]);
            let local_x = mouse.column.saturating_sub(inner.x);
            let local_y = mouse.row.saturating_sub(inner.y);

            let selected = crate::ui::envelope_editor::point_at_position(
                envelope,
                env_type,
                inner.width as usize,
                graph_height as usize,
                local_x,
                local_y,
            );
            app.env_editor.select_point(selected);
            if selected.is_some() {
                app.env_editor.start_drag(0.0);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if !app.env_editor.is_dragging() {
                return;
            }
            let Some(idx) = app.instrument_selection() else {
                return;
            };
            if mouse.column < inner.x || mouse.row < inner.y {
                return;
            }

            let local_x = mouse.column.saturating_sub(inner.x);
            let local_y = mouse.row.saturating_sub(inner.y);
            let env_type = app.env_editor.envelope_type;
            if let Some(selected_point) = app.env_editor.selected_point {
                let envelope = app
                    .env_editor
                    .get_envelope_mut(&mut app.song.instruments[idx]);
                crate::ui::envelope_editor::update_point_from_position(
                    envelope,
                    env_type,
                    selected_point,
                    inner.width as usize,
                    graph_height as usize,
                    local_x,
                    local_y.min(graph_height.saturating_sub(1)),
                );
                app.mark_dirty();
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.env_editor.end_drag();
        }
        _ => {}
    }
}

pub fn handle_instrument_view_mouse(
    app: &mut App,
    mouse: MouseEvent,
    content_area: ratatui::layout::Rect,
) {
    let [list_area, editor_area, envelope_area, waveform_area] =
        instrument_view_chunks(content_area);

    if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
        app.inst_editor.end_drag();
        app.env_editor.end_drag();
        app.waveform_editor.end_loop_marker_drag();
    }

    if matches!(mouse.kind, MouseEventKind::ScrollDown) {
        app.instrument_selection_down();
        return;
    }
    if matches!(mouse.kind, MouseEventKind::ScrollUp) {
        app.instrument_selection_up();
        return;
    }

    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
        && point_in_rect(list_area, mouse.column, mouse.row)
    {
        let inner = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .inner(list_area);
        let row = mouse.row.saturating_sub(inner.y);
        let instrument_idx = row.saturating_sub(1) as usize;
        if row >= 1 && instrument_idx < app.song.instruments.len() {
            app.set_instrument_selection(Some(instrument_idx));
        }
    }

    if app.instrument_selection().is_none() {
        return;
    }

    let inst_dragging = app.inst_editor.dragging().is_some();
    let env_dragging = app.env_editor.is_dragging();
    let wf_dragging = app.waveform_editor.is_loop_marker_dragging();

    if point_in_rect(editor_area, mouse.column, mouse.row) || inst_dragging {
        handle_instrument_editor_mouse(app, mouse, editor_area);
    }
    if point_in_rect(envelope_area, mouse.column, mouse.row) || env_dragging {
        handle_envelope_mouse(app, mouse, envelope_area);
    }
    if point_in_rect(waveform_area, mouse.column, mouse.row) || wf_dragging {
        handle_waveform_mouse(app, mouse, waveform_area);
    }
}

pub fn handle_waveform_mouse(app: &mut App, mouse: MouseEvent, waveform_area: ratatui::layout::Rect) {
    // Check if click is within waveform area
    if mouse.column < waveform_area.x
        || mouse.column >= waveform_area.x + waveform_area.width
        || mouse.row < waveform_area.y
        || mouse.row >= waveform_area.y + waveform_area.height
    {
        return;
    }

    let idx = match app.instrument_selection() {
        Some(i) if i < app.song.instruments.len() => i,
        _ => return,
    };

    let sample_idx = match app.song.instruments[idx].sample_index {
        Some(si) => si,
        None => return,
    };

    let samples = app.loaded_samples();
    let sample = match samples.get(sample_idx) {
        Some(s) => s.as_ref(),
        None => return,
    };

    let frame_count = sample.frame_count();
    if frame_count == 0 {
        return;
    }

    // Calculate which sample frame the mouse is pointing to
    let local_x = mouse.column.saturating_sub(waveform_area.x + 2); // Account for left padding
    let grid_width = (waveform_area.width.saturating_sub(4)).max(1) as usize;
    let frame_at_cursor = if local_x < grid_width as u16 {
        ((local_x as usize * frame_count) / grid_width).min(frame_count.saturating_sub(1))
    } else {
        frame_count.saturating_sub(1)
    };

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check for Shift key to set loop start
            if mouse
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT)
            {
                app.waveform_editor
                    .start_loop_marker_drag(crate::ui::waveform_editor::LoopMarkerDrag::Start);
                app.set_sample_loop_settings(
                    idx,
                    sample_idx,
                    tracker_core::audio::sample::LoopMode::Forward,
                    frame_at_cursor,
                    sample.loop_end,
                );
            }
            // Check for Ctrl key to set loop end
            else if mouse
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                app.set_sample_loop_settings(
                    idx,
                    sample_idx,
                    tracker_core::audio::sample::LoopMode::Forward,
                    sample.loop_start,
                    frame_at_cursor,
                );
            }
            // Normal click: move cursor to position
            else {
                app.waveform_editor.set_cursor(frame_at_cursor);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Drag while holding left button: update loop marker being dragged
            match app.waveform_editor.dragging_loop_marker() {
                crate::ui::waveform_editor::LoopMarkerDrag::Start => {
                    app.set_sample_loop_settings(
                        idx,
                        sample_idx,
                        tracker_core::audio::sample::LoopMode::Forward,
                        frame_at_cursor.min(sample.loop_end.saturating_sub(1)),
                        sample.loop_end,
                    );
                }
                crate::ui::waveform_editor::LoopMarkerDrag::End => {
                    app.set_sample_loop_settings(
                        idx,
                        sample_idx,
                        tracker_core::audio::sample::LoopMode::Forward,
                        sample.loop_start,
                        frame_at_cursor.max(sample.loop_start + 1),
                    );
                }
                crate::ui::waveform_editor::LoopMarkerDrag::None => {}
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.waveform_editor.end_loop_marker_drag();
        }
        _ => {}
    }
}

pub fn point_in_rect(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

pub fn instrument_view_chunks(content_area: ratatui::layout::Rect) -> [ratatui::layout::Rect; 4] {
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage(30),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(25),
            ratatui::layout::Constraint::Percentage(20),
        ])
        .split(content_area);

    [chunks[0], chunks[1], chunks[2], chunks[3]]
}

pub fn apply_instrument_field_drag(
    app: &mut App,
    field: crate::ui::instrument_editor::InstrumentField,
    delta: i16,
) {
    if delta == 0 {
        return;
    }

    match field {
        crate::ui::instrument_editor::InstrumentField::BaseNote => {
            app.adjust_instrument_base_note(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::Volume => {
            app.adjust_instrument_volume(delta as i32 * 5);
        }
        crate::ui::instrument_editor::InstrumentField::Finetune => {
            app.adjust_instrument_finetune(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::LoopStart => {
            app.adjust_instrument_loop_start(delta as i32 * 128);
        }
        crate::ui::instrument_editor::InstrumentField::LoopEnd => {
            app.adjust_instrument_loop_end(delta as i32 * 128);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneNoteMin => {
            app.adjust_keyzone_note_min(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneNoteMax => {
            app.adjust_keyzone_note_max(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneVelMin => {
            app.adjust_keyzone_velocity_min(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::KeyzoneVelMax => {
            app.adjust_keyzone_velocity_max(delta as i32);
        }
        crate::ui::instrument_editor::InstrumentField::Name
        | crate::ui::instrument_editor::InstrumentField::LoopMode
        | crate::ui::instrument_editor::InstrumentField::KeyzoneList
        | crate::ui::instrument_editor::InstrumentField::KeyzoneSample
        | crate::ui::instrument_editor::InstrumentField::KeyzoneBaseNote => {}
    }
}

pub fn calculate_channel_scroll_for_mouse(
    cursor_channel: usize,
    available_width: u16,
    num_channels: usize,
) -> usize {
    const CHANNEL_COL_WIDTH: u16 = 17;
    const ROW_NUM_WIDTH: u16 = 6;

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

pub fn calculate_scroll_offset_for_mouse(
    cursor_row: usize,
    visible_rows: usize,
    total_rows: usize,
) -> usize {
    if visible_rows >= total_rows {
        return 0;
    }
    if cursor_row < visible_rows / 2 {
        0
    } else if cursor_row + visible_rows / 2 >= total_rows {
        total_rows.saturating_sub(visible_rows)
    } else {
        cursor_row.saturating_sub(visible_rows / 2)
    }
}
