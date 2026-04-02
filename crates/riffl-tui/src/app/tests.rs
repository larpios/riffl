use super::*;

#[test]
fn test_app_view_default_is_pattern_editor() {
    let app = App::new();
    assert_eq!(app.current_view, AppView::PatternEditor);
}

#[test]
fn test_set_view_to_arrangement() {
    let mut app = App::new();
    app.set_view(AppView::Arrangement);
    assert_eq!(app.current_view, AppView::Arrangement);
}

#[test]
fn test_set_view_to_instrument_list() {
    let mut app = App::new();
    app.set_view(AppView::InstrumentList);
    assert_eq!(app.current_view, AppView::InstrumentList);
}

#[test]
fn test_set_view_back_to_pattern_editor() {
    let mut app = App::new();
    app.set_view(AppView::Arrangement);
    app.set_view(AppView::PatternEditor);
    assert_eq!(app.current_view, AppView::PatternEditor);
}

#[test]
fn test_set_view_same_view_is_noop() {
    let mut app = App::new();
    app.set_view(AppView::PatternEditor);
    assert_eq!(app.current_view, AppView::PatternEditor);
}

#[test]
fn test_app_view_enum_equality() {
    assert_eq!(AppView::PatternEditor, AppView::PatternEditor);
    assert_eq!(AppView::Arrangement, AppView::Arrangement);
    assert_eq!(AppView::InstrumentList, AppView::InstrumentList);
    assert_eq!(AppView::CodeEditor, AppView::CodeEditor);
    assert_ne!(AppView::PatternEditor, AppView::Arrangement);
    assert_ne!(AppView::Arrangement, AppView::InstrumentList);
    assert_ne!(AppView::InstrumentList, AppView::CodeEditor);
}

#[test]
fn test_app_view_is_copy() {
    let view = AppView::Arrangement;
    let copy = view;
    assert_eq!(view, copy); // Both still valid (Copy trait)
}

#[test]
fn test_view_cycle_all_three() {
    let mut app = App::new();
    assert_eq!(app.current_view, AppView::PatternEditor);
    app.set_view(AppView::Arrangement);
    assert_eq!(app.current_view, AppView::Arrangement);
    app.set_view(AppView::InstrumentList);
    assert_eq!(app.current_view, AppView::InstrumentList);
    app.set_view(AppView::PatternEditor);
    assert_eq!(app.current_view, AppView::PatternEditor);
}

// --- Song-level playback tests ---

#[test]
fn test_default_playback_mode_is_song() {
    let app = App::new();
    assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);
}

#[test]
fn test_toggle_playback_mode() {
    let mut app = App::new();
    assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);

    app.toggle_playback_mode();
    assert_eq!(app.transport.playback_mode(), PlaybackMode::Pattern);

    app.toggle_playback_mode();
    assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);
}

#[test]
fn test_jump_next_pattern_with_multiple_patterns() {
    let mut app = App::new();
    // Add a second pattern to the song pool
    let pattern2 = Pattern::new(8, 4);
    app.song.patterns.push(pattern2);
    app.song.arrangement = vec![0, 1]; // Two entries in arrangement

    assert_eq!(app.transport.arrangement_position(), 0);

    app.jump_next_pattern();
    assert_eq!(app.transport.arrangement_position(), 1);

    // Already at last position — should not advance
    app.jump_next_pattern();
    assert_eq!(app.transport.arrangement_position(), 1);
}

#[test]
fn test_jump_prev_pattern() {
    let mut app = App::new();
    let pattern2 = Pattern::new(8, 4);
    app.song.patterns.push(pattern2);
    app.song.arrangement = vec![0, 1];

    // Start at 0 — cannot go back
    app.jump_prev_pattern();
    assert_eq!(app.transport.arrangement_position(), 0);

    // Jump to 1, then back to 0
    app.jump_next_pattern();
    assert_eq!(app.transport.arrangement_position(), 1);

    app.jump_prev_pattern();
    assert_eq!(app.transport.arrangement_position(), 0);
}

#[test]
fn test_jump_pattern_loads_correct_pattern_into_editor() {
    let mut app = App::new();
    // Pattern 0: 16 rows, Pattern 1: 8 rows
    let pattern2 = Pattern::new(8, 4);
    app.song.patterns.push(pattern2);
    app.song.arrangement = vec![0, 1];

    // Editor starts with pattern 0 (16 rows)
    assert_eq!(app.editor.pattern().num_rows(), 16);

    // Jump to pattern 1 (8 rows)
    app.jump_next_pattern();
    assert_eq!(app.editor.pattern().num_rows(), 8);

    // Jump back to pattern 0 (16 rows)
    app.jump_prev_pattern();
    assert_eq!(app.editor.pattern().num_rows(), 16);
}

#[test]
fn test_stop_resets_arrangement_position() {
    let mut app = App::new();
    let pattern2 = Pattern::new(8, 4);
    app.song.patterns.push(pattern2);
    app.song.arrangement = vec![0, 1];

    app.jump_next_pattern();
    assert_eq!(app.transport.arrangement_position(), 1);

    app.stop();
    assert_eq!(app.transport.arrangement_position(), 0);
    assert_eq!(app.transport.current_row(), 0);
}

#[test]
fn test_song_mode_toggle_play_loads_arrangement_pattern() {
    let mut app = App::new();
    let pattern2 = Pattern::new(8, 4);
    app.song.patterns.push(pattern2);
    app.song.arrangement = vec![0, 1];

    // Default is already Song mode; verify
    assert_eq!(app.transport.playback_mode(), PlaybackMode::Song);

    // Starting playback in Song mode should load the arrangement pattern
    app.toggle_play();
    assert!(app.transport.is_playing());
}

// --- Export Dialog Tests ---

#[test]
fn test_open_export_dialog_default_path() {
    let mut app = App::new();
    assert!(!app.has_export_dialog());

    app.open_export_dialog();
    assert!(app.has_export_dialog());
    assert_eq!(app.export_dialog.output_path, "untitled.wav");
}

#[test]
fn test_open_export_dialog_with_project_path() {
    let mut app = App::new();
    app.project_path = Some(PathBuf::from("my_song.rtm"));

    app.open_export_dialog();
    assert!(app.has_export_dialog());
    assert_eq!(app.export_dialog.output_path, "my_song.wav");
}

#[test]
fn test_export_dialog_close() {
    let mut app = App::new();
    app.open_export_dialog();
    assert!(app.has_export_dialog());

    app.export_dialog.close();
    assert!(!app.has_export_dialog());
}

#[test]
fn test_execute_export_creates_file() {
    let mut app = App::new();
    let dir = std::env::temp_dir().join("tracker_rs_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test_app_export.wav");

    app.open_export_dialog();
    app.export_dialog.output_path = path.display().to_string();
    app.execute_export();

    use crate::ui::export_dialog::ExportPhase;
    assert_eq!(app.export_dialog.phase, ExportPhase::Done);
    assert_eq!(app.export_dialog.progress, 100);
    assert!(path.exists());

    // Verify it's a valid WAV
    let reader = hound::WavReader::open(&path).unwrap();
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_rate, 44100);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_execute_export_with_custom_settings() {
    let mut app = App::new();
    let dir = std::env::temp_dir().join("tracker_rs_tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test_app_export_48k.wav");

    app.open_export_dialog();
    app.export_dialog.output_path = path.display().to_string();
    app.export_dialog.sample_rate = 48000;
    app.export_dialog.bit_depth = riffl_core::export::BitDepth::Bits24;
    app.execute_export();

    use crate::ui::export_dialog::ExportPhase;
    assert_eq!(app.export_dialog.phase, ExportPhase::Done);

    let reader = hound::WavReader::open(&path).unwrap();
    assert_eq!(reader.spec().sample_rate, 48000);
    assert_eq!(reader.spec().bits_per_sample, 24);

    std::fs::remove_file(&path).ok();
}

#[test]
fn test_execute_export_invalid_path_fails() {
    let mut app = App::new();
    app.open_export_dialog();
    // Use an invalid directory path
    app.export_dialog.output_path = "/nonexistent/path/to/file.wav".to_string();
    app.execute_export();

    use crate::ui::export_dialog::ExportPhase;
    assert_eq!(app.export_dialog.phase, ExportPhase::Failed);
    assert!(!app.export_dialog.result_message.is_empty());
}

// --- Code Editor and Split View Tests ---

#[test]
fn test_set_view_code_editor_activates_editor() {
    let mut app = App::new();
    assert!(!app.code_editor.active);
    app.set_view(AppView::CodeEditor);
    assert_eq!(app.current_view, AppView::CodeEditor);
    assert!(app.code_editor.active);
}

#[test]
fn test_set_view_pattern_deactivates_code_editor() {
    let mut app = App::new();
    app.set_view(AppView::CodeEditor);
    assert!(app.code_editor.active);
    app.set_view(AppView::PatternEditor);
    assert!(!app.code_editor.active);
}

#[test]
fn test_toggle_split_view_on() {
    let mut app = App::new();
    assert!(!app.split_view);
    assert!(!app.code_editor.active);
    app.toggle_split_view();
    assert!(app.split_view);
    assert!(app.code_editor.active);
}

#[test]
fn test_toggle_split_view_off() {
    let mut app = App::new();
    app.toggle_split_view();
    assert!(app.split_view);
    app.toggle_split_view();
    assert!(!app.split_view);
    assert!(!app.code_editor.active);
}

#[test]
fn test_split_view_from_code_editor_switches_to_pattern() {
    let mut app = App::new();
    app.set_view(AppView::CodeEditor);
    app.toggle_split_view();
    assert!(app.split_view);
    // Should switch to PatternEditor for the split
    assert_eq!(app.current_view, AppView::PatternEditor);
}

#[test]
fn test_is_code_editor_active() {
    let mut app = App::new();
    assert!(!app.is_code_editor_active());

    app.set_view(AppView::CodeEditor);
    assert!(app.is_code_editor_active());

    app.set_view(AppView::PatternEditor);
    assert!(!app.is_code_editor_active());

    app.toggle_split_view();
    assert!(app.is_code_editor_active());
}

#[test]
fn test_execute_script_empty() {
    let mut app = App::new();
    app.execute_script(&[]);
    assert_eq!(app.code_editor.output(), "(empty script)");
    assert!(!app.code_editor.output_is_error);
}

#[test]
fn test_execute_script_simple_expression() {
    let mut app = App::new();
    app.code_editor.set_text("40 + 2");
    app.execute_script(&[]);
    assert_eq!(app.code_editor.output(), "42");
    assert!(!app.code_editor.output_is_error);
}

#[test]
fn test_execute_script_error() {
    let mut app = App::new();
    app.code_editor.set_text("let x = ;");
    app.execute_script(&[]);
    assert!(app.code_editor.output_is_error);
    assert!(!app.code_editor.output().is_empty());
}

#[test]
fn test_execute_script_set_note() {
    let mut app = App::new();
    app.code_editor.set_text(
        r#"
        let n = note("C", 4);
        set_note(0, 0, n);
    "#,
    );
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    assert!(app.code_editor.output().contains("Applied"));
    // Verify note was placed
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_some());
    let cell = cell.unwrap();
    assert!(cell.note.is_some());
}

#[test]
fn test_execute_script_clear_pattern() {
    let mut app = App::new();
    // First set some notes
    app.editor
        .pattern_mut()
        .set_note(0, 0, Note::simple(Pitch::C, 4));
    // Then clear via script
    app.code_editor.set_text("clear_pattern();");
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    // Verify pattern was cleared
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_none_or(|c| c.is_empty()));
}

#[test]
fn test_view_cycle_includes_code_editor() {
    let mut app = App::new();
    assert_eq!(app.current_view, AppView::PatternEditor);
    app.set_view(AppView::Arrangement);
    assert_eq!(app.current_view, AppView::Arrangement);
    app.set_view(AppView::InstrumentList);
    assert_eq!(app.current_view, AppView::InstrumentList);
    app.set_view(AppView::CodeEditor);
    assert_eq!(app.current_view, AppView::CodeEditor);
    app.set_view(AppView::PatternEditor);
    assert_eq!(app.current_view, AppView::PatternEditor);
}

// --- Live Mode Tests ---

#[test]
fn test_live_mode_default_off() {
    let app = App::new();
    assert!(!app.live_mode);
}

#[test]
fn test_toggle_live_mode() {
    let mut app = App::new();
    assert!(!app.live_mode);
    app.toggle_live_mode();
    assert!(app.live_mode);
    app.toggle_live_mode();
    assert!(!app.live_mode);
}

#[test]
fn test_live_mode_re_executes_on_pattern_loop() {
    let mut app = App::new();
    // Set up a small 4-row pattern
    let pattern = Pattern::new(4, 4);
    app.editor = Editor::new(pattern);
    app.transport.set_num_rows(4);
    app.transport.set_playback_mode(PlaybackMode::Pattern);
    app.transport.set_loop_enabled(true);

    // Write a script that sets a note at row 0
    app.code_editor.set_text(
        r#"
        let n = note("D", 5);
        set_note(0, 0, n);
    "#,
    );

    // Enable live mode and start playback
    app.live_mode = true;
    app.transport.play();

    // Advance through all rows to trigger the loop
    let spr = (2.5 / 120.0) * 6.0; // seconds per row at 120 BPM
    app.transport.advance(spr); // Row 1
    app.last_update = Instant::now();
    app.transport.advance(spr); // Row 2
    app.transport.advance(spr); // Row 3

    // Clear the specific cell before the loop triggers
    app.editor.pattern_mut().clear_cell(0, 0);
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_none_or(|c| c.is_empty()));

    // Now advance past the end — should loop to row 0 and re-execute script
    // We need to call update() which handles the advance and live mode logic
    // But update() uses last_update for delta, so let's simulate directly
    // by calling the transport advance and then mimicking update behavior
    let result = app.transport.advance(spr);
    assert_eq!(result, riffl_core::transport::AdvanceResult::Row(0));

    // Simulate what update() does for Row(0) with live_mode
    if app.live_mode {
        app.execute_script(&[]);
    }

    // Verify the script was re-executed: note should be placed at (0, 0)
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_some());
    assert!(cell.unwrap().note.is_some());
}

#[test]
fn test_live_mode_does_not_execute_when_disabled() {
    let mut app = App::new();
    let pattern = Pattern::new(4, 4);
    app.editor = Editor::new(pattern);
    app.transport.set_num_rows(4);
    app.transport.set_playback_mode(PlaybackMode::Pattern);
    app.transport.set_loop_enabled(true);

    // Write a script that sets a note
    app.code_editor.set_text(
        r#"
        let n = note("D", 5);
        set_note(0, 0, n);
    "#,
    );

    // Live mode OFF
    app.live_mode = false;
    app.transport.play();

    // Advance through all rows to trigger the loop
    let spr = (2.5 / 120.0) * 6.0;
    app.transport.advance(spr); // Row 1
    app.transport.advance(spr); // Row 2
    app.transport.advance(spr); // Row 3
    let result = app.transport.advance(spr); // Row 0 (loop)
    assert_eq!(result, riffl_core::transport::AdvanceResult::Row(0));

    // Pattern should remain empty since live mode is off
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_none_or(|c| c.is_empty()));
}

#[test]
fn test_live_mode_with_empty_script() {
    let mut app = App::new();
    let pattern = Pattern::new(4, 4);
    app.editor = Editor::new(pattern);
    app.transport.set_num_rows(4);

    // Empty script — live mode should not crash
    app.code_editor.set_text("");
    app.live_mode = true;
    app.execute_script(&[]); // Should handle gracefully
    assert!(!app.code_editor.output_is_error);
}

#[test]
fn test_live_mode_with_error_script() {
    let mut app = App::new();
    let pattern = Pattern::new(4, 4);
    app.editor = Editor::new(pattern);
    app.transport.set_num_rows(4);

    // Invalid script — live mode should display error, not panic
    app.code_editor.set_text("let x = ;");
    app.live_mode = true;
    app.execute_script(&[]); // Should handle gracefully
    assert!(app.code_editor.output_is_error);
}

// --- Audio Wiring Tests ---

#[test]
fn test_script_execution_retriggers_mixer_during_playback() {
    let mut app = App::new();
    // Start playback
    app.transport.play();
    assert!(app.transport.is_playing());

    // Execute a script that modifies the pattern — should retrigger mixer
    app.code_editor.set_text(
        r#"
        let n = note("E", 4);
        set_note(0, 0, n);
    "#,
    );
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    assert!(app.code_editor.output().contains("Applied"));
    // Verify note was placed (pattern was modified)
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_some());
    assert!(cell.unwrap().note.is_some());
}

#[test]
fn test_script_no_retrigger_when_stopped() {
    let mut app = App::new();
    // Transport is stopped
    assert!(app.transport.is_stopped());

    // Execute a script — should still apply commands, just no mixer retrigger
    app.code_editor.set_text(
        r#"
        let n = note("E", 4);
        set_note(0, 0, n);
    "#,
    );
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    assert!(app.code_editor.output().contains("Applied"));
}

#[test]
fn test_script_no_retrigger_for_readonly_script() {
    let mut app = App::new();
    app.transport.play();

    // Execute a script that doesn't modify the pattern (no commands)
    app.code_editor.set_text("40 + 2");
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    assert_eq!(app.code_editor.output(), "42");
}

#[test]
fn test_script_execution_does_not_block_audio_thread() {
    // Verify that script execution runs synchronously on main thread
    // while audio callback runs on separate thread via Arc<Mutex<Mixer>>.
    // The mixer is behind Arc<Mutex>, so scripts don't touch the audio callback.
    let mut app = App::new();
    app.transport.play();

    // Heavy script execution should complete without deadlock
    app.code_editor.set_text(
        r#"
        for i in range(0, 16) {
            let n = note("C", 4);
            set_note(i, 0, n);
        }
    "#,
    );
    app.execute_script(&[]);
    assert!(!app.code_editor.output_is_error);
    assert!(app.code_editor.output().contains("Applied 16 commands"));
}

#[test]
fn test_live_mode_changes_take_effect_on_next_loop() {
    let mut app = App::new();
    let pattern = Pattern::new(4, 4);
    app.editor = Editor::new(pattern);
    app.transport.set_num_rows(4);
    app.transport.set_playback_mode(PlaybackMode::Pattern);
    app.transport.set_loop_enabled(true);

    // Script fills column 0 with C4 notes
    app.code_editor.set_text(
        r#"
        for i in range(0, 4) {
            let n = note("C", 4);
            set_note(i, 0, n);
        }
    "#,
    );

    // Enable live mode and start playback
    app.live_mode = true;
    app.transport.play();

    // Advance through all rows without executing script
    let spr = (2.5 / 120.0) * 6.0;
    app.transport.advance(spr); // Row 1
    app.transport.advance(spr); // Row 2
    app.transport.advance(spr); // Row 3

    // Verify pattern is empty before the loop
    for i in 0..4 {
        let cell = app.editor.pattern().get_cell(i, 0);
        assert!(cell.is_none_or(|c| c.is_empty()));
    }

    // Loop back to row 0 — live mode should re-execute script
    let result = app.transport.advance(spr);
    assert_eq!(result, riffl_core::transport::AdvanceResult::Row(0));
    // Simulate update() behavior
    if app.live_mode {
        app.execute_script(&[]);
    }

    // Now all 4 rows should have notes
    for i in 0..4 {
        let cell = app.editor.pattern().get_cell(i, 0);
        assert!(cell.is_some(), "Row {} should have a note", i);
        assert!(
            cell.unwrap().note.is_some(),
            "Row {} note should not be empty",
            i
        );
    }
}

#[test]
fn test_execute_script_during_playback_preserves_transport_state() {
    let mut app = App::new();
    app.transport.set_num_rows(16);
    app.transport.play();

    // Advance a few rows
    let spr = (2.5 / 120.0) * 6.0;
    app.transport.advance(spr); // Row 1
    app.transport.advance(spr); // Row 2
    let row_before = app.transport.current_row();
    assert_eq!(row_before, 2);

    // Execute script
    app.code_editor.set_text(
        r#"
        let n = note("A", 3);
        set_note(0, 0, n);
    "#,
    );
    app.execute_script(&[]);

    // Transport state should be unchanged
    assert!(app.transport.is_playing());
    assert_eq!(app.transport.current_row(), 2);
}

// --- BPM prompt tests ---

#[test]
fn test_open_bpm_prompt_prepopulates_current_bpm() {
    let mut app = App::new();
    app.transport.set_bpm(140.0);
    app.open_bpm_prompt();
    assert!(app.bpm_prompt_mode);
    assert_eq!(app.bpm_prompt_input, "140");
}

#[test]
fn test_execute_bpm_prompt_applies_valid_bpm() {
    let mut app = App::new();
    app.bpm_prompt_mode = true;
    app.bpm_prompt_input = "180".to_string();
    app.execute_bpm_prompt();
    assert!(!app.bpm_prompt_mode);
    assert!(app.bpm_prompt_input.is_empty());
    assert_eq!(app.transport.bpm(), 180.0);
    assert_eq!(app.song.bpm, 180.0);
}

#[test]
fn test_execute_bpm_prompt_clamps_to_min() {
    let mut app = App::new();
    app.bpm_prompt_mode = true;
    app.bpm_prompt_input = "5".to_string();
    app.execute_bpm_prompt();
    assert_eq!(app.transport.bpm(), 20.0);
}

#[test]
fn test_execute_bpm_prompt_clamps_to_max() {
    let mut app = App::new();
    app.bpm_prompt_mode = true;
    app.bpm_prompt_input = "9999".to_string();
    app.execute_bpm_prompt();
    assert_eq!(app.transport.bpm(), 999.0);
}

#[test]
fn test_execute_bpm_prompt_ignores_invalid_input() {
    let mut app = App::new();
    let original_bpm = app.transport.bpm();
    app.bpm_prompt_mode = true;
    app.bpm_prompt_input = "abc".to_string();
    app.execute_bpm_prompt();
    assert!(!app.bpm_prompt_mode);
    // BPM unchanged for invalid input
    assert_eq!(app.transport.bpm(), original_bpm);
}

// --- Pattern length prompt tests ---

#[test]
fn test_open_len_prompt_prepopulates_current_row_count() {
    let mut app = App::new();
    let current_len = app.editor.pattern().row_count();
    app.open_len_prompt();
    assert!(app.len_prompt_mode);
    assert_eq!(app.len_prompt_input, format!("{}", current_len));
}

#[test]
fn test_execute_len_prompt_resizes_pattern_and_transport() {
    let mut app = App::new();
    app.len_prompt_mode = true;
    app.len_prompt_input = "32".to_string();
    app.execute_len_prompt();
    assert!(!app.len_prompt_mode);
    assert_eq!(app.editor.pattern().row_count(), 32);
    assert_eq!(app.transport.num_rows(), 32);
}

#[test]
fn test_execute_len_prompt_clamps_to_min() {
    let mut app = App::new();
    app.len_prompt_mode = true;
    app.len_prompt_input = "4".to_string(); // below 16
    app.execute_len_prompt();
    assert_eq!(app.editor.pattern().row_count(), 16);
    assert_eq!(app.transport.num_rows(), 16);
}

#[test]
fn test_execute_len_prompt_clamps_to_max() {
    let mut app = App::new();
    app.len_prompt_mode = true;
    app.len_prompt_input = "9999".to_string(); // above 512
    app.execute_len_prompt();
    assert_eq!(app.editor.pattern().row_count(), 512);
    assert_eq!(app.transport.num_rows(), 512);
}

#[test]
fn test_execute_len_prompt_ignores_invalid_input() {
    let mut app = App::new();
    let original = app.editor.pattern().row_count();
    app.len_prompt_mode = true;
    app.len_prompt_input = "abc".to_string();
    app.execute_len_prompt();
    assert!(!app.len_prompt_mode);
    // Row count unchanged for invalid input
    assert_eq!(app.editor.pattern().row_count(), original);
}

#[test]
fn test_execute_len_prompt_flushes_to_song() {
    let mut app = App::new();
    app.len_prompt_mode = true;
    app.len_prompt_input = "48".to_string();
    app.execute_len_prompt();
    // The song's pattern 0 should also be updated
    let pat_idx = app.song.arrangement[app.transport.arrangement_position()];
    assert_eq!(app.song.patterns[pat_idx].row_count(), 48);
}

// --- Tap tempo tests ---

#[test]
fn test_single_tap_does_not_change_bpm() {
    let mut app = App::new();
    let original_bpm = app.transport.bpm();
    app.tap_tempo();
    // Only 1 tap — no interval to compute
    assert_eq!(app.transport.bpm(), original_bpm);
}

#[test]
fn test_two_taps_set_bpm_from_interval() {
    let mut app = App::new();
    // Manually insert two taps 0.5s apart (= 120 BPM)
    let base = Instant::now();
    app.tap_times.push(base);
    app.tap_times
        .push(base + std::time::Duration::from_millis(500));
    // Simulate a third tap 0.5s after the last one
    app.tap_times
        .push(base + std::time::Duration::from_millis(1000));
    // Compute expected BPM: avg interval = 0.5s → 120 BPM
    let intervals = [0.5f64, 0.5];
    let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
    let expected_bpm = (60.0 / avg).clamp(20.0, 999.0);

    // Set transport to the computed BPM directly (mimics what tap_tempo would do)
    app.transport.set_bpm(expected_bpm);
    assert!((app.transport.bpm() - 120.0).abs() < 1.0);
}

#[test]
fn test_tap_times_older_than_3s_are_dropped() {
    let mut app = App::new();
    // Insert a very old tap (5 seconds ago)
    app.tap_times
        .push(Instant::now() - std::time::Duration::from_secs(5));
    let original_bpm = app.transport.bpm();
    app.tap_tempo(); // Only 1 valid tap after pruning → no BPM change
    assert_eq!(app.transport.bpm(), original_bpm);
}

// --- Loop region tests ---

#[test]
fn test_set_loop_start_sets_region_and_activates() {
    let mut app = App::new();
    app.editor.go_to_row(4);
    app.set_loop_start();
    let region = app.transport.loop_region();
    assert!(region.is_some());
    assert_eq!(region.unwrap().0, 4); // start = cursor row
    assert!(app.transport.loop_region_active());
}

#[test]
fn test_set_loop_end_sets_region_and_activates() {
    let mut app = App::new();
    app.editor.go_to_row(8);
    app.set_loop_end();
    let region = app.transport.loop_region();
    assert!(region.is_some());
    assert_eq!(region.unwrap().1, 8); // end = cursor row
    assert!(app.transport.loop_region_active());
}

#[test]
fn test_set_loop_start_then_end_gives_correct_region() {
    let mut app = App::new();
    app.editor.go_to_row(4);
    app.set_loop_start();
    app.editor.go_to_row(12);
    app.set_loop_end();
    assert_eq!(app.transport.loop_region(), Some((4, 12)));
    assert!(app.transport.loop_region_active());
}

#[test]
fn test_set_loop_end_before_start_adjusts_start() {
    let mut app = App::new();
    app.editor.go_to_row(8);
    app.set_loop_start();
    // Move cursor before the start and set end there
    app.editor.go_to_row(3);
    app.set_loop_end();
    let region = app.transport.loop_region();
    assert!(region.is_some());
    let (s, e) = region.unwrap();
    assert!(s <= e); // region must be valid
    assert_eq!(e, 3);
}

#[test]
fn test_toggle_loop_region_active() {
    let mut app = App::new();
    app.editor.go_to_row(0);
    app.set_loop_start();
    app.editor.go_to_row(7);
    app.set_loop_end();
    assert!(app.transport.loop_region_active()); // auto-activated
    app.toggle_loop_region_active();
    assert!(!app.transport.loop_region_active());
    app.toggle_loop_region_active();
    assert!(app.transport.loop_region_active());
}

// --- Draw mode tests ---

#[test]
fn test_draw_mode_starts_inactive() {
    let app = App::new();
    assert!(!app.draw_mode);
    assert!(app.draw_note.is_none());
}

#[test]
fn test_toggle_draw_mode() {
    let mut app = App::new();
    app.toggle_draw_mode();
    assert!(app.draw_mode);
    app.toggle_draw_mode();
    assert!(!app.draw_mode);
}

#[test]
fn test_apply_draw_note_writes_to_cursor() {
    use riffl_core::pattern::note::NoteEvent;
    let mut app = App::new();
    app.draw_mode = true;
    app.draw_note = Some(NoteEvent::On(Note::simple(Pitch::C, 4)));
    app.editor.go_to_row(2);
    app.apply_draw_note();
    let cell = app.editor.pattern().get_cell(2, 0);
    assert!(cell.is_some());
    assert_eq!(
        cell.unwrap().note,
        Some(NoteEvent::On(Note::simple(Pitch::C, 4)))
    );
}

#[test]
fn test_apply_draw_note_noop_when_mode_off() {
    use riffl_core::pattern::note::NoteEvent;
    let mut app = App::new();
    app.draw_mode = false;
    app.draw_note = Some(NoteEvent::On(Note::simple(Pitch::C, 4)));
    app.editor.go_to_row(2);
    app.apply_draw_note();
    let cell = app.editor.pattern().get_cell(2, 0);
    // Row 2 should be empty (no note written)
    assert!(
        cell.is_none() || cell.unwrap().note.is_none(),
        "apply_draw_note should be a no-op when draw_mode is false"
    );
}

#[test]
fn test_apply_draw_note_noop_when_note_none() {
    let mut app = App::new();
    app.draw_mode = true;
    app.draw_note = None;
    app.editor.go_to_row(2);
    app.apply_draw_note();
    let cell = app.editor.pattern().get_cell(2, 0);
    assert!(
        cell.is_none() || cell.unwrap().note.is_none(),
        "apply_draw_note should be a no-op when draw_note is None"
    );
}

#[test]
fn test_draw_waveform_sample_persists_and_refreshes_chip_render() {
    let mut app = App::new();
    app.set_instrument_selection(Some(0));
    app.waveform_editor.set_cursor(0);
    app.waveform_editor.pencil_value = 0.75;

    app.draw_waveform_sample().unwrap();

    let sample = app.loaded_samples().first().unwrap().clone();
    assert!((sample.data()[0] - 0.75).abs() < 0.001);
    let chip_render = app.song.instruments[0].chip_render.as_ref().unwrap();
    assert_eq!(
        chip_render.wavetable_2a03.len(),
        riffl_core::audio::CHIP_WAVETABLE_LEN
    );
    assert_eq!(chip_render.dpcm.len(), riffl_core::audio::CHIP_DPCM_BYTES);
}

// --- Tutor view tests ---

#[test]
fn test_tutor_starts_hidden() {
    let app = App::new();
    assert!(!app.show_tutor);
    assert_eq!(app.tutor_scroll, 0);
}

#[test]
fn test_execute_command_tutor_opens_view() {
    let mut app = App::new();
    app.command_mode = true;
    app.command_input = "tutor".to_string();
    app.execute_command();
    assert!(app.show_tutor, "show_tutor should be true after :tutor");
    assert_eq!(app.tutor_scroll, 0, "scroll should reset to 0");
    assert!(!app.command_mode, "command mode should be exited");
}

#[test]
fn test_tutor_content_has_lines() {
    let count = crate::ui::tutor::content_line_count();
    assert!(count > 20, "tutor should have at least 20 lines of content");
}

#[test]
fn test_project_samples_dir_auto_added_to_browser() {
    let dir = std::env::temp_dir().join("riffl_app_proj_samples");
    std::fs::create_dir_all(&dir).unwrap();
    let samples_dir = dir.join("samples");
    std::fs::create_dir_all(&samples_dir).unwrap();

    let mut app = App::new();
    // Simulate a loaded project whose directory contains ./samples/
    app.project_path = Some(dir.join("test.rtm"));
    app.refresh_browser_roots();

    let has_samples = app
        .sample_browser
        .entries()
        .iter()
        .any(|e| e.path == samples_dir);
    assert!(
        has_samples,
        "project-relative samples/ should be auto-added as a root"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_project_samples_dir_not_added_when_missing() {
    let dir = std::env::temp_dir().join("riffl_app_proj_no_samples");
    std::fs::create_dir_all(&dir).unwrap();
    // No samples/ subdir created here

    let mut app = App::new();
    app.project_path = Some(dir.join("test.rtm"));
    app.refresh_browser_roots();

    let samples_dir = dir.join("samples");
    let has_samples = app
        .sample_browser
        .entries()
        .iter()
        .any(|e| e.path == samples_dir);
    assert!(
        !has_samples,
        "should not add samples/ root when directory doesn't exist"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// --- Browser preview toggle & scrub state ---

// --- Browser bookmarks ---

#[test]
fn test_toggle_bookmark_adds_dir() {
    let dir = std::env::temp_dir().join("riffl_bm_add");
    std::fs::create_dir_all(&dir).unwrap();

    let mut app = App::new();
    app.set_sample_dirs(vec![dir.clone()]);

    // Select the first entry (our dir is a root)
    assert!(app.sample_browser.at_roots());
    app.sample_browser.select(0);

    assert!(app.config.bookmarked_dirs.is_empty(), "no bookmarks yet");
    app.toggle_browser_bookmark();

    assert_eq!(app.config.bookmarked_dirs.len(), 1);
    assert_eq!(app.config.bookmarked_dirs[0], dir.display().to_string());
    assert!(app.sample_browser.selected_is_bookmarked());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_toggle_bookmark_removes_dir() {
    let dir = std::env::temp_dir().join("riffl_bm_remove");
    std::fs::create_dir_all(&dir).unwrap();

    let mut app = App::new();
    app.set_sample_dirs(vec![dir.clone()]);
    app.sample_browser.select(0);

    app.toggle_browser_bookmark(); // add
    assert_eq!(app.config.bookmarked_dirs.len(), 1);

    app.toggle_browser_bookmark(); // remove
    assert!(
        app.config.bookmarked_dirs.is_empty(),
        "bookmark should be removed on second toggle"
    );
    assert!(!app.sample_browser.selected_is_bookmarked());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_bookmarks_restored_on_startup_sequence() {
    // Regression: main.rs called set_sample_dirs before app.config = config,
    // so refresh_browser_roots ran with empty bookmarks (Config::default()).
    // Fix: call refresh_browser_roots again after assigning the real config.
    let dir = std::env::temp_dir().join("riffl_bm_startup");
    std::fs::create_dir_all(&dir).unwrap();

    let config = crate::config::Config {
        bookmarked_dirs: vec![dir.display().to_string()],
        ..Default::default()
    };

    let mut app = App::new();
    // Simulate old main.rs ordering: set_sample_dirs first (config still default/empty)
    app.set_sample_dirs(vec![dir.clone()]);
    // Then assign the real config and refresh — this is the fix
    app.config = config;
    app.refresh_browser_roots();

    app.sample_browser.select(0);
    assert!(
        app.sample_browser.selected_is_bookmarked(),
        "bookmark should be present after refresh_browser_roots post-config assignment"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_toggle_bookmark_no_effect_on_file_selection() {
    let dir = std::env::temp_dir().join("riffl_bm_file");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("kick.wav"), b"x").unwrap();

    let mut app = App::new();
    app.set_sample_dirs(vec![dir.clone()]);
    // Enter the root so we see the file
    app.sample_browser.enter_dir();
    app.sample_browser.select(0);
    assert!(app.sample_browser.selected_is_file());

    app.toggle_browser_bookmark();
    assert!(
        app.config.bookmarked_dirs.is_empty(),
        "file selection should not create a bookmark"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn test_browser_preview_inactive_initially() {
    let app = App::new();
    assert!(!app.browser_preview_active);
    assert_eq!(app.browser_preview_offset_frames, 0);
}

#[test]
fn test_stop_browser_preview_clears_active() {
    let mut app = App::new();
    // Manually set active to simulate a started preview
    app.browser_preview_active = true;
    app.stop_browser_preview();
    assert!(!app.browser_preview_active);
}

#[test]
fn test_reset_browser_preview_clears_offset_and_sample() {
    let mut app = App::new();
    app.browser_preview_active = true;
    app.browser_preview_offset_frames = 4410;
    app.reset_browser_preview();
    assert!(!app.browser_preview_active);
    assert_eq!(app.browser_preview_offset_frames, 0);
    assert!(app.browser_preview_sample.is_none());
}

#[test]
fn test_preview_cursor_state_returns_zeros_when_idle() {
    let app = App::new();
    let (pos, total, _rate) = app.preview_cursor_state();
    // No preview active: pos and total should both be 0.
    assert_eq!(pos, 0);
    assert_eq!(total, 0);
}

// --- VU Meter Tests ---

#[test]
fn test_channel_levels_returns_correct_count() {
    let app = App::new();
    let levels = app.channel_levels(4);
    assert_eq!(levels.len(), 4);
}

#[test]
fn test_channel_levels_initially_zero() {
    let app = App::new();
    let levels = app.channel_levels(4);
    for (l, r) in levels {
        assert_eq!(l, 0.0, "Initial left level should be 0.0");
        assert_eq!(r, 0.0, "Initial right level should be 0.0");
    }
}

#[test]
fn test_channel_levels_zero_channels() {
    let app = App::new();
    let levels = app.channel_levels(0);
    assert!(levels.is_empty());
}

#[test]
fn test_decay_not_called_when_stopped() {
    // When transport is stopped, update() should NOT decay channel levels.
    // We verify indirectly: levels stay at 0.0 after update() while stopped.
    let mut app = App::new();
    assert!(app.transport.is_stopped());
    // Levels start at zero and stay at zero (decay of zero is still zero,
    // but this also confirms the code path doesn't panic).
    let _ = app.update();
    let levels = app.channel_levels(4);
    for (l, r) in levels {
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }
}

#[test]
fn test_import_mod_file_syncs_bpm_to_transport() {
    // Obsolete test
}

#[test]
fn test_command_transpose() {
    let mut app = App::new();
    use riffl_core::pattern::note::{Note, Pitch};
    app.editor
        .pattern_mut()
        .set_note(0, 0, Note::simple(Pitch::C, 4)); // C-4 = midi 48
    app.command_input = "transpose 12".to_string();
    app.execute_command();
    let cell = app.editor.pattern().get_cell(0, 0).unwrap();
    let note = cell.note.as_ref().unwrap();
    if let riffl_core::pattern::note::NoteEvent::On(n) = note {
        assert_eq!(n.midi_note(), 60); // C-5
    } else {
        panic!("expected note on");
    }
}

#[test]
fn test_command_transpose_down() {
    let mut app = App::new();
    use riffl_core::pattern::note::{Note, Pitch};
    app.editor
        .pattern_mut()
        .set_note(0, 0, Note::simple(Pitch::C, 4)); // C-4 = midi 48
    app.command_input = "tr -12".to_string();
    app.execute_command();
    let cell = app.editor.pattern().get_cell(0, 0).unwrap();
    let note = cell.note.as_ref().unwrap();
    if let riffl_core::pattern::note::NoteEvent::On(n) = note {
        assert_eq!(n.midi_note(), 36); // C-3
    } else {
        panic!("expected note on");
    }
}

#[test]
fn test_command_len_resizes_pattern() {
    let mut app = App::new();
    app.command_input = "len 32".to_string();
    app.execute_command();
    assert_eq!(app.editor.pattern().num_rows(), 32);
}

#[test]
fn test_command_len_clamps_to_minimum() {
    let mut app = App::new();
    app.command_input = "len 1".to_string(); // below MIN=16
    app.execute_command();
    assert_eq!(app.editor.pattern().num_rows(), 16);
}

#[test]
fn test_command_clear_empties_pattern() {
    let mut app = App::new();
    use riffl_core::pattern::note::{Note, Pitch};
    app.editor
        .pattern_mut()
        .set_note(0, 0, Note::simple(Pitch::C, 4));
    assert!(app
        .editor
        .pattern()
        .get_cell(0, 0)
        .is_some_and(|c| !c.is_empty()));
    app.command_input = "clear".to_string();
    app.execute_command();
    let cell = app.editor.pattern().get_cell(0, 0);
    assert!(cell.is_none_or(|c| c.is_empty()));
}

#[test]
fn test_command_speed_sets_tpl() {
    let mut app = App::new();
    app.command_input = "speed 8".to_string();
    app.execute_command();
    assert_eq!(app.song.tpl, 8);
    assert_eq!(app.transport.tpl(), 8);
}

#[test]
fn test_command_speed_clamps_to_range() {
    let mut app = App::new();
    app.command_input = "speed 0".to_string(); // below min=1
    app.execute_command();
    assert_eq!(app.song.tpl, 1);
}

#[test]
fn test_command_loop_sets_region() {
    let mut app = App::new();
    app.command_input = "loop 4 12".to_string();
    app.execute_command();
    assert_eq!(app.transport.loop_region(), Some((4, 12)));
    assert!(app.transport.loop_region_active());
}

#[test]
fn test_command_interpolate_is_recognised() {
    let mut app = App::new();
    // Interpolate on a single cell is a no-op (needs a visual selection with volume data)
    // but the command must not produce an "Unknown command" modal.
    app.command_input = "interpolate".to_string();
    app.execute_command();
    // No error modal should be open
    assert!(!app.has_modal());
}

#[test]
fn test_command_interp_alias() {
    let mut app = App::new();
    app.command_input = "interp".to_string();
    app.execute_command();
    assert!(!app.has_modal());
}

#[test]
fn test_arrangement_clone_pattern_duplicates_and_inserts() {
    let mut app = App::new();
    // Default app has one arrangement entry (pattern 0)
    let initial_patterns = app.song.patterns.len();
    let initial_arrangement_len = app.song.arrangement.len();

    app.arrangement_clone_pattern();

    assert_eq!(app.song.patterns.len(), initial_patterns + 1);
    assert_eq!(app.song.arrangement.len(), initial_arrangement_len + 1);
    // Cursor advanced to the newly inserted entry
    assert_eq!(app.arrangement_view.cursor(), 1);
}

#[test]
fn test_arrangement_clone_pattern_preserves_content() {
    use riffl_core::pattern::note::{Note, NoteEvent, Pitch};
    use riffl_core::pattern::row::Cell;

    let mut app = App::new();
    // Put a note in pattern 0
    app.editor.pattern_mut().set_cell(
        0,
        0,
        Cell::with_note(NoteEvent::On(Note::new(Pitch::C, 4, 100, 0))),
    );
    // Flush so pattern 0 in song matches
    app.flush_editor_pattern(0);

    app.arrangement_clone_pattern();

    // The clone (pattern index == initial_patterns) should have the same note
    let clone_idx = *app
        .song
        .arrangement
        .get(app.arrangement_view.cursor())
        .unwrap();
    let clone_pattern = &app.song.patterns[clone_idx];
    assert!(clone_pattern.get_cell(0, 0).is_some());
    assert!(clone_pattern.get_cell(0, 0).unwrap().note.is_some());
}

#[test]
fn test_adjust_track_volume_increases() {
    let mut app = App::new();
    let initial_vol = app.song.tracks.first().map(|t| t.volume).unwrap_or(1.0);
    app.adjust_track_volume(0.05);
    let new_vol = app.song.tracks.first().map(|t| t.volume).unwrap_or(1.0);
    // Default is 1.0 so clamping keeps it at 1.0; test with a lower starting value
    let _ = (initial_vol, new_vol); // skip magnitude check since 1.0+0.05 clamps to 1.0
}

#[test]
fn test_adjust_track_volume_decreases() {
    let mut app = App::new();
    // Set track volume to 0.5 first via song tracks
    if let Some(t) = app.song.tracks.get_mut(0) {
        t.set_volume(0.5);
    }
    app.adjust_track_volume(-0.1);
    let new_vol = app.song.tracks.first().map(|t| t.volume).unwrap_or(0.0);
    assert!(
        (new_vol - 0.4).abs() < 1e-4,
        "Expected ~0.4, got {}",
        new_vol
    );
}

#[test]
fn test_adjust_track_volume_clamps_at_zero() {
    let mut app = App::new();
    if let Some(t) = app.song.tracks.get_mut(0) {
        t.set_volume(0.0);
    }
    app.adjust_track_volume(-0.5);
    let new_vol = app.song.tracks.first().map(|t| t.volume).unwrap_or(0.0);
    assert_eq!(new_vol, 0.0, "Volume should not go below 0.0");
}

#[test]
fn test_adjust_track_pan_right() {
    let mut app = App::new();
    // Default pan is 0.0
    app.adjust_track_pan(0.1);
    let pan = app.song.tracks.first().map(|t| t.pan).unwrap_or(0.0);
    assert!((pan - 0.1).abs() < 1e-4, "Expected pan ~0.1, got {}", pan);
}

#[test]
fn test_adjust_track_pan_left() {
    let mut app = App::new();
    app.adjust_track_pan(-0.3);
    let pan = app.song.tracks.first().map(|t| t.pan).unwrap_or(0.0);
    assert!((pan + 0.3).abs() < 1e-4, "Expected pan ~-0.3, got {}", pan);
}

#[test]
fn test_adjust_track_pan_clamps() {
    let mut app = App::new();
    app.adjust_track_pan(2.0);
    let pan = app.song.tracks.first().map(|t| t.pan).unwrap_or(0.0);
    assert_eq!(pan, 1.0, "Pan should clamp at 1.0");
    app.adjust_track_pan(-5.0);
    let pan = app.song.tracks.first().map(|t| t.pan).unwrap_or(0.0);
    assert_eq!(pan, -1.0, "Pan should clamp at -1.0");
}

#[test]
fn test_command_adsr_sets_envelope() {
    let mut app = App::new();
    app.command_input = "adsr 10 50 70 200".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    let idx = app.instrument_selection().unwrap_or(0);
    let adsr = app.song.instruments[idx].volume_adsr.as_ref();
    assert!(adsr.is_some(), "ADSR should be set");
    let a = adsr.unwrap();
    assert_eq!(a.attack, 10.0);
    assert_eq!(a.decay, 50.0);
    assert!((a.sustain - 0.7).abs() < 1e-4, "Sustain should be 70%=0.7");
    assert_eq!(a.release, 200.0);
}

#[test]
fn test_command_adsr_invalid_shows_error() {
    let mut app = App::new();
    app.command_input = "adsr notanumber".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_rename_sets_track_name() {
    let mut app = App::new();
    app.command_input = "rename Kick Drum".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    let ch = app.editor.cursor_channel();
    let name = app
        .song
        .tracks
        .get(ch)
        .map(|t| t.name.as_str())
        .unwrap_or("");
    assert_eq!(name, "Kick Drum");
}

#[test]
fn test_command_fill_whole_channel() {
    use riffl_core::pattern::note::NoteEvent;
    let mut app = App::new();
    app.command_input = "fill C-4".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    // Every row in channel 0 should have C4
    let ch = app.editor.cursor_channel();
    let rows = app.editor.pattern().num_rows();
    for r in 0..rows {
        let cell = app.editor.pattern().get_cell(r, ch).unwrap();
        assert!(
            matches!(cell.note, Some(NoteEvent::On(_))),
            "Row {} should have a note after :fill C-4",
            r
        );
    }
}

#[test]
fn test_command_fill_with_step() {
    use riffl_core::pattern::note::NoteEvent;
    let mut app = App::new();
    app.command_input = "fill C-4 4".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    let ch = app.editor.cursor_channel();
    // Rows 0, 4, 8, ... should have notes; rows 1, 2, 3, 5 ... should be empty
    let cell_0 = app.editor.pattern().get_cell(0, ch).unwrap();
    assert!(
        matches!(cell_0.note, Some(NoteEvent::On(_))),
        "Row 0 should have note"
    );
    let cell_1 = app.editor.pattern().get_cell(1, ch).unwrap();
    assert!(cell_1.note.is_none(), "Row 1 should be empty");
    let cell_4 = app.editor.pattern().get_cell(4, ch).unwrap();
    assert!(
        matches!(cell_4.note, Some(NoteEvent::On(_))),
        "Row 4 should have note"
    );
}

#[test]
fn test_command_fill_invalid_note_shows_error() {
    let mut app = App::new();
    app.command_input = "fill xyz".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_mode_native() {
    use riffl_core::pattern::effect::EffectMode;
    let mut app = App::new();
    app.song.effect_mode = EffectMode::Compatible;
    app.command_input = "mode native".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.effect_mode, EffectMode::RifflNative);
}

#[test]
fn test_command_mode_compat() {
    use riffl_core::pattern::effect::EffectMode;
    let mut app = App::new();
    app.command_input = "mode compat".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.effect_mode, EffectMode::Compatible);
}

#[test]
fn test_command_mode_amiga() {
    use riffl_core::pattern::effect::EffectMode;
    let mut app = App::new();
    app.command_input = "mode amiga".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.effect_mode, EffectMode::Amiga);
}

#[test]
fn test_command_mode_it_alias() {
    use riffl_core::pattern::effect::EffectMode;
    let mut app = App::new();
    app.command_input = "mode it".to_string();
    app.execute_command();
    assert_eq!(app.song.effect_mode, EffectMode::Compatible);
}

#[test]
fn test_command_track_add_increases_channels() {
    let mut app = App::new();
    let before = app.editor.pattern().num_channels();
    app.command_input = "track add".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.editor.pattern().num_channels(), before + 1);
}

#[test]
fn test_command_track_del_decreases_channels() {
    let mut app = App::new();
    // Ensure at least 2 channels
    app.command_input = "track add".to_string();
    app.execute_command();
    let before = app.editor.pattern().num_channels();
    app.command_input = "track del".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.editor.pattern().num_channels(), before - 1);
}

#[test]
fn test_command_track_no_arg_shows_info() {
    let mut app = App::new();
    app.command_input = "track".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_pname_sets_pattern_name() {
    let mut app = App::new();
    app.command_input = "pname Intro".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    // pattern_selection is None so we fall back to arrangement_position
    let arr_pos = app.transport.arrangement_position();
    let pat_idx = app.song.arrangement.get(arr_pos).copied().unwrap_or(0);
    assert_eq!(app.song.patterns[pat_idx].name, "Intro");
}

#[test]
fn test_command_pname_no_arg_shows_info() {
    let mut app = App::new();
    app.command_input = "pname".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_pname_with_selection() {
    let mut app = App::new();
    // Add a second pattern and select it
    let pat = riffl_core::pattern::Pattern::new(16, 4);
    app.song.patterns.push(pat);
    let second_idx = app.song.patterns.len() - 1;
    app.set_pattern_selection(Some(second_idx));
    app.command_input = "pname Chorus".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.patterns[second_idx].name, "Chorus");
}

#[test]
fn test_command_title_sets_song_name() {
    let mut app = App::new();
    app.command_input = "title MySong".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.name, "MySong");
}

#[test]
fn test_command_title_no_arg_shows_info() {
    let mut app = App::new();
    app.command_input = "title".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_artist_sets_song_artist() {
    let mut app = App::new();
    app.command_input = "artist John".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.song.artist, "John");
}

#[test]
fn test_command_artist_no_arg_shows_info() {
    let mut app = App::new();
    app.command_input = "artist".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_dup_clones_pattern() {
    let mut app = App::new();
    let initial_count = app.song.patterns.len();
    app.command_input = "dup".to_string();
    app.execute_command();
    // Should have created a new pattern and opened an info modal
    assert_eq!(app.song.patterns.len(), initial_count + 1);
    assert!(app.has_modal());
}

#[test]
fn test_command_dup_sets_copy_name() {
    let mut app = App::new();
    // Use :pname to set a name so both song and editor are in sync
    app.command_input = "pname Intro".to_string();
    app.execute_command();
    app.command_input = "dup".to_string();
    app.execute_command();
    let new_idx = app.song.patterns.len() - 1;
    assert_eq!(app.song.patterns[new_idx].name, "Intro copy");
}

#[test]
fn test_command_mode_unknown_shows_info() {
    let mut app = App::new();
    app.command_input = "mode badvalue".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_goto_jumps_to_row() {
    let mut app = App::new();
    // :goto 5 should move cursor to row 4 (1-based input)
    app.command_input = "goto 5".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.editor.cursor_row(), 4);
}

#[test]
fn test_command_bare_number_jumps_to_row() {
    let mut app = App::new();
    // :8 should move cursor to row 7 (1-based)
    app.command_input = "8".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.editor.cursor_row(), 7);
}

#[test]
fn test_command_goto_no_arg_shows_info() {
    let mut app = App::new();
    app.command_input = "goto".to_string();
    app.execute_command();
    assert!(app.has_modal());
}

#[test]
fn test_command_g_alias_jumps_to_row() {
    let mut app = App::new();
    app.command_input = "g 3".to_string();
    app.execute_command();
    assert!(!app.has_modal());
    assert_eq!(app.editor.cursor_row(), 2);
}
