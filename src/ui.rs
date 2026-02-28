use crate::editor::Editor;
use crate::pattern::Note;
use ratatui::{
    layout::{Constraint, Layout, Direction},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, Cell},
    Frame,
};

/// Render the pattern editor UI
///
/// This function renders the complete TUI interface including:
/// - Pattern grid with row numbers
/// - Note values in each channel
/// - Border and title
pub fn render(frame: &mut Frame, editor: &Editor) {
    let area = frame.area();

    // Create the main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(area);

    // Build the pattern table
    let table = build_pattern_table(editor);

    frame.render_widget(table, chunks[0]);
}

/// Build the pattern table widget
fn build_pattern_table(editor: &Editor) -> Table<'static> {
    let pattern = editor.pattern();
    let num_rows = pattern.num_rows();
    let num_channels = pattern.num_channels();
    let cursor_row = editor.current_row();
    let cursor_col = editor.current_col();

    // Create header row
    let mut header_cells = vec![Cell::from("Row").style(Style::default().add_modifier(Modifier::BOLD))];
    for i in 0..num_channels {
        header_cells.push(
            Cell::from(format!("Ch{}", i + 1))
                .style(Style::default().add_modifier(Modifier::BOLD))
        );
    }
    let header = Row::new(header_cells)
        .style(Style::default().bg(Color::DarkGray))
        .height(1);

    // Create data rows
    let mut rows = Vec::new();
    for row_idx in 0..num_rows {
        let mut cells = vec![
            Cell::from(format!("{:03}", row_idx))
                .style(Style::default().fg(Color::Yellow))
        ];

        // Add cells for each channel
        for channel_idx in 0..num_channels {
            let note_text = if let Some(note_opt) = pattern.get_note(row_idx, channel_idx) {
                if let Some(note) = note_opt {
                    format_note(note)
                } else {
                    "---".to_string()
                }
            } else {
                "---".to_string()
            };

            // Apply cursor highlighting to the cell at cursor position
            let cell_style = if row_idx == cursor_row && channel_idx == cursor_col {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            cells.push(Cell::from(note_text).style(cell_style));
        }

        rows.push(Row::new(cells).height(1));
    }

    // Calculate column widths
    let mut widths = vec![Constraint::Length(5)]; // Row number column
    for _ in 0..num_channels {
        widths.push(Constraint::Length(8)); // Note columns
    }

    // Build and return the table
    Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Pattern Editor ")
                .style(Style::default().fg(Color::White))
        )
        .column_spacing(1)
}

/// Format a note for display in the grid
fn format_note(note: &Note) -> String {
    let base = note.to_string();

    // Add velocity and instrument if present
    let vel = if let Some(v) = note.velocity {
        format!(" v{:02X}", v)
    } else {
        "".to_string()
    };

    let inst = if let Some(i) = note.instrument {
        format!(" i{:02}", i)
    } else {
        "".to_string()
    };

    format!("{}{}{}", base, vel, inst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::{Pitch, Note};

    #[test]
    fn test_format_note_basic() {
        let note = Note::new(Pitch::C, 4);
        assert_eq!(format_note(&note), "C4");
    }

    #[test]
    fn test_format_note_with_velocity() {
        let note = Note {
            pitch: Pitch::A,
            octave: 5,
            velocity: Some(127),
            instrument: None,
        };
        assert_eq!(format_note(&note), "A5 v7F");
    }

    #[test]
    fn test_format_note_with_all_fields() {
        let note = Note::with_all(Pitch::G, 3, 64, 2);
        assert_eq!(format_note(&note), "G3 v40 i02");
    }

    #[test]
    fn test_format_note_different_pitches() {
        assert_eq!(format_note(&Note::new(Pitch::C, 0)), "C0");
        assert_eq!(format_note(&Note::new(Pitch::D, 1)), "D1");
        assert_eq!(format_note(&Note::new(Pitch::E, 2)), "E2");
        assert_eq!(format_note(&Note::new(Pitch::F, 3)), "F3");
        assert_eq!(format_note(&Note::new(Pitch::G, 4)), "G4");
        assert_eq!(format_note(&Note::new(Pitch::A, 5)), "A5");
        assert_eq!(format_note(&Note::new(Pitch::B, 6)), "B6");
    }

    #[test]
    fn test_build_pattern_table() {
        let editor = Editor::new(8, 4);
        let table = build_pattern_table(&editor);

        // Table should be created successfully (no panics)
        // The actual rendering is tested through manual verification
        drop(table);
    }

    #[test]
    fn test_build_pattern_table_with_notes() {
        let mut editor = Editor::new(4, 2);

        // Add some notes
        editor.pattern_mut().set_note(0, 0, Some(Note::new(Pitch::C, 4)));
        editor.pattern_mut().set_note(1, 1, Some(Note::new(Pitch::A, 5)));

        let table = build_pattern_table(&editor);

        // Table should be created successfully with the notes
        drop(table);
    }

    #[test]
    fn test_build_pattern_table_small_pattern() {
        let editor = Editor::new(1, 1);
        let table = build_pattern_table(&editor);

        // Should handle minimal patterns without panic
        drop(table);
    }

    #[test]
    fn test_build_pattern_table_large_pattern() {
        let editor = Editor::new(64, 8);
        let table = build_pattern_table(&editor);

        // Should handle larger patterns without panic
        drop(table);
    }

    #[test]
    fn test_cursor_highlighting() {
        let mut editor = Editor::new(8, 4);

        // Test cursor at default position (0, 0)
        let table = build_pattern_table(&editor);
        drop(table);

        // Move cursor and rebuild table
        editor.move_down();
        editor.move_right();
        let table = build_pattern_table(&editor);
        drop(table);

        // Move cursor to different positions
        editor.move_down();
        editor.move_down();
        editor.move_right();
        editor.move_right();
        let table = build_pattern_table(&editor);
        drop(table);
    }
}
