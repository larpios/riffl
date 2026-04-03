use super::*;
use crate::ui::pattern_renderer::{
    calculate_scroll_offset, format_cell_display, format_cell_parts,
};

#[test]
fn test_scroll_offset_small_pattern() {
    // Pattern fits in view
    assert_eq!(calculate_scroll_offset(0, 20, 16), 0);
    assert_eq!(calculate_scroll_offset(15, 20, 16), 0);
}

#[test]
fn test_scroll_offset_at_top() {
    assert_eq!(calculate_scroll_offset(0, 10, 64), 0);
    assert_eq!(calculate_scroll_offset(3, 10, 64), 0);
}

#[test]
fn test_scroll_offset_middle() {
    assert_eq!(calculate_scroll_offset(30, 10, 64), 25);
}

#[test]
fn test_scroll_offset_at_bottom() {
    assert_eq!(calculate_scroll_offset(63, 10, 64), 54);
}

#[test]
fn test_format_cell_empty() {
    let cell = riffl_core::pattern::row::Cell::empty();
    assert_eq!(format_cell_display(&cell), "--- .. .. .... ....");
}

#[test]
fn test_format_cell_with_note() {
    use riffl_core::pattern::note::{Note, Pitch};
    let cell = riffl_core::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
    assert_eq!(format_cell_display(&cell), "C-4 .. .. .... ....");
}

#[test]
fn test_format_cell_note_off() {
    let cell = riffl_core::pattern::row::Cell::with_note(NoteEvent::Off);
    assert_eq!(format_cell_display(&cell), "=== .. .. .... ....");
}

#[test]
fn test_format_cell_note_cut() {
    let cell = riffl_core::pattern::row::Cell::with_note(NoteEvent::Cut);
    assert_eq!(format_cell_display(&cell), "^^^ .. .. .... ....");
}

#[test]
fn test_format_cell_full() {
    use riffl_core::pattern::effect::Effect;
    use riffl_core::pattern::note::{Note, Pitch};
    let cell = riffl_core::pattern::row::Cell {
        note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
        instrument: Some(1),
        volume: Some(0x40),
        effects: vec![Effect::new(0xC, 0x20)],
    };
    assert_eq!(format_cell_display(&cell), "C#4 01 40 0C20 ....");
}

// --- format_cell_parts tests ---

#[test]
fn test_format_cell_parts_none() {
    let (n, i, v, e, _) = format_cell_parts(None);
    assert_eq!(n, "---");
    assert_eq!(i, "..");
    assert_eq!(v, "..");
    assert_eq!(e, "....");
}

#[test]
fn test_format_cell_parts_empty() {
    let cell = riffl_core::pattern::row::Cell::empty();
    let (n, i, v, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(n, "---");
    assert_eq!(i, "..");
    assert_eq!(v, "..");
    assert_eq!(e, "....");
}

#[test]
fn test_format_cell_parts_with_note() {
    use riffl_core::pattern::note::{Note, Pitch};
    let cell = riffl_core::pattern::row::Cell::with_note(NoteEvent::On(Note::simple(Pitch::C, 4)));
    let (n, i, v, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(n, "C-4");
    assert_eq!(i, "..");
    assert_eq!(v, "..");
    assert_eq!(e, "....");
}

#[test]
fn test_format_cell_parts_full() {
    use riffl_core::pattern::effect::Effect;
    use riffl_core::pattern::note::{Note, Pitch};
    let cell = riffl_core::pattern::row::Cell {
        note: Some(NoteEvent::On(Note::new(Pitch::CSharp, 4, 100, 1))),
        instrument: Some(1),
        volume: Some(0x40),
        effects: vec![Effect::new(0xC, 0x20)],
    };
    let (n, i, v, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(n, "C#4");
    assert_eq!(i, "01");
    assert_eq!(v, "40");
    assert_eq!(e, "0C20");
}

// --- ProTracker effect rendering tests (Phase 2 effects) ---

#[test]
fn test_format_cell_parts_effect_5xy_tone_porta_vol_slide() {
    use riffl_core::pattern::effect::Effect;
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0x5, 0x34)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0534");
}

#[test]
fn test_format_cell_parts_effect_6xy_vibrato_vol_slide() {
    use riffl_core::pattern::effect::Effect;
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0x6, 0x12)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0612");
}

#[test]
fn test_format_cell_parts_effect_7xy_tremolo() {
    use riffl_core::pattern::effect::Effect;
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0x7, 0x44)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0744");
}

#[test]
fn test_format_cell_parts_effect_9xx_sample_offset() {
    use riffl_core::pattern::effect::Effect;
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0x9, 0x80)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0980");
}

#[test]
fn test_format_cell_parts_effect_exy_extended() {
    use riffl_core::pattern::effect::Effect;
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0xE, 0x10)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0E10");
}

#[test]
fn test_format_cell_parts_effect_zero_param() {
    use riffl_core::pattern::effect::Effect;
    // Effect with zero param renders as "X00"
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0xA, 0x00)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0A00");
}

#[test]
fn test_format_cell_parts_effect_ff_param() {
    use riffl_core::pattern::effect::Effect;
    // Full-range param renders correctly
    let cell = riffl_core::pattern::row::Cell {
        note: None,
        instrument: None,
        volume: None,
        effects: vec![Effect::new(0xF, 0xFF)],
    };
    let (_, _, _, e, _) = format_cell_parts(Some(&cell));
    assert_eq!(e, "0FFF");
}

#[test]
fn test_format_cell_parts_note_off() {
    let cell = riffl_core::pattern::row::Cell::with_note(NoteEvent::Off);
    let (n, _i, _v, _e, _) = format_cell_parts(Some(&cell));
    assert_eq!(n, "===");
}

// --- Scroll target selection tests ---

#[test]
fn test_scroll_offset_follows_playback_position() {
    // When playing, scroll should follow the playback row, not the cursor
    // The scroll_target logic in render_content selects transport.current_row()
    // during playback. Here we verify the scroll offset calculation works
    // correctly for a playback row deep in a large pattern.
    let playback_row = 50;
    let visible = 20;
    let total = 64;
    let offset = calculate_scroll_offset(playback_row, visible, total);
    // Playback row should be centered: 50 - 10 = 40
    assert_eq!(offset, 40);
}

#[test]
fn test_scroll_offset_playback_at_start() {
    // When playback is at the start, offset should be 0
    assert_eq!(calculate_scroll_offset(0, 20, 64), 0);
    assert_eq!(calculate_scroll_offset(5, 20, 64), 0);
}

// --- Channel scroll tests ---

#[test]
fn test_channel_scroll_all_fit() {
    // 4 channels, wide terminal — no scrolling needed
    assert_eq!(calculate_channel_scroll(0, 200, 4), 0);
    assert_eq!(calculate_channel_scroll(3, 200, 4), 0);
}

#[test]
fn test_channel_scroll_narrow_terminal() {
    // 8 channels, only room for 4 (6 + 4*17 = 74 needed, width=74)
    let width = ROW_NUM_WIDTH + CHANNEL_COL_WIDTH * 4; // 74
    assert_eq!(calculate_channel_scroll(0, width, 8), 0);
    assert_eq!(calculate_channel_scroll(1, width, 8), 0);
    // Cursor at ch 6 should scroll
    assert_eq!(calculate_channel_scroll(6, width, 8), 4);
    // Cursor at ch 7 (last) should show last 4
    assert_eq!(calculate_channel_scroll(7, width, 8), 4);
}

#[test]
fn test_channel_scroll_center_cursor() {
    // 8 channels, room for 4
    let width = ROW_NUM_WIDTH + CHANNEL_COL_WIDTH * 4;
    // Cursor at ch 4 should center: 4 - 2 = 2
    assert_eq!(calculate_channel_scroll(4, width, 8), 2);
}

#[test]
fn test_scroll_offset_playback_at_end() {
    // When playback is near the end, should not scroll past the bottom
    let offset = calculate_scroll_offset(63, 20, 64);
    assert_eq!(offset, 44); // 64 - 20 = 44
}
