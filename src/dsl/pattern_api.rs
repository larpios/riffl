/// Pattern manipulation functions for the DSL.
///
/// These functions operate on Pattern data and are callable from scripts
/// via the command system, providing algorithmic composition tools.

use crate::pattern::{Note, NoteEvent, Pattern};
use super::engine::PatternCommand;
use rand::Rng;

/// Place a note in the pattern at the given row and channel.
pub fn set_note(row: usize, channel: usize, note: Note) -> PatternCommand {
    PatternCommand::SetNote { row, channel, note }
}

/// Generate commands to clear all cells in a pattern.
pub fn clear_pattern(pattern: &Pattern) -> Vec<PatternCommand> {
    vec![PatternCommand::ClearPattern]
}

/// Fill a channel with a repeating note sequence.
///
/// The `notes` array is cycled across all rows in the channel.
pub fn fill_column(
    pattern: &Pattern,
    channel: usize,
    notes: &[Note],
) -> Vec<PatternCommand> {
    if notes.is_empty() {
        return Vec::new();
    }
    (0..pattern.num_rows())
        .map(|row| PatternCommand::SetNote {
            row,
            channel,
            note: notes[row % notes.len()],
        })
        .collect()
}

/// Place notes where a Euclidean rhythm pattern is true.
///
/// For each step in `rhythm` that is `true`, a note is placed in the pattern.
pub fn generate_beat(
    pattern: &Pattern,
    channel: usize,
    rhythm: &[bool],
    note: Note,
) -> Vec<PatternCommand> {
    if rhythm.is_empty() {
        return Vec::new();
    }
    let mut commands = Vec::new();
    for row in 0..pattern.num_rows() {
        let step = row % rhythm.len();
        if rhythm[step] {
            commands.push(PatternCommand::SetNote {
                row,
                channel,
                note,
            });
        }
    }
    commands
}

/// Transpose all notes in a pattern by N semitones.
///
/// Notes that would go out of range (below C-0 or above B-9) are left unchanged.
pub fn transpose(pattern: &Pattern, semitones: i32) -> Vec<PatternCommand> {
    let mut commands = Vec::new();
    for row in 0..pattern.num_rows() {
        for channel in 0..pattern.num_channels() {
            if let Some(cell) = pattern.get_cell(row, channel) {
                if let Some(NoteEvent::On(note)) = &cell.note {
                    if let Some(transposed) = note.transpose(semitones) {
                        commands.push(PatternCommand::SetNote {
                            row,
                            channel,
                            note: transposed,
                        });
                    }
                }
            }
        }
    }
    commands
}

/// Reverse the row order of a pattern.
///
/// Returns commands that reconstruct the pattern in reversed row order.
pub fn reverse(pattern: &Pattern) -> Vec<PatternCommand> {
    let mut commands = vec![PatternCommand::ClearPattern];
    let num_rows = pattern.num_rows();
    for row in 0..num_rows {
        for channel in 0..pattern.num_channels() {
            if let Some(cell) = pattern.get_cell(row, channel) {
                if let Some(NoteEvent::On(note)) = &cell.note {
                    commands.push(PatternCommand::SetNote {
                        row: num_rows - 1 - row,
                        channel,
                        note: *note,
                    });
                }
            }
        }
    }
    commands
}

/// Circular shift rows by an offset.
///
/// Positive offset shifts down; negative shifts up.
pub fn rotate(pattern: &Pattern, offset: i32) -> Vec<PatternCommand> {
    let mut commands = vec![PatternCommand::ClearPattern];
    let num_rows = pattern.num_rows() as i32;
    for row in 0..pattern.num_rows() {
        for channel in 0..pattern.num_channels() {
            if let Some(cell) = pattern.get_cell(row, channel) {
                if let Some(NoteEvent::On(note)) = &cell.note {
                    let new_row = ((row as i32 + offset) % num_rows + num_rows) % num_rows;
                    commands.push(PatternCommand::SetNote {
                        row: new_row as usize,
                        channel,
                        note: *note,
                    });
                }
            }
        }
    }
    commands
}

/// Add subtle randomness to note velocities.
///
/// `velocity_variance` is the maximum deviation from the original velocity (0-127).
pub fn humanize(pattern: &Pattern, velocity_variance: u8) -> Vec<PatternCommand> {
    let mut commands = Vec::new();
    let mut rng = rand::thread_rng();

    for row in 0..pattern.num_rows() {
        for channel in 0..pattern.num_channels() {
            if let Some(cell) = pattern.get_cell(row, channel) {
                if let Some(NoteEvent::On(note)) = &cell.note {
                    let vel_delta =
                        rng.gen_range(-(velocity_variance as i16)..=(velocity_variance as i16));
                    let new_vel =
                        (note.velocity as i16 + vel_delta).clamp(0, 127) as u8;
                    let humanized = Note::new(note.pitch, note.octave, new_vel, note.instrument);
                    commands.push(PatternCommand::SetNote {
                        row,
                        channel,
                        note: humanized,
                    });
                }
            }
        }
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::engine::apply_commands;
    use crate::pattern::Pitch;

    #[test]
    fn test_set_note_command() {
        let cmd = set_note(0, 0, Note::simple(Pitch::C, 4));
        match cmd {
            PatternCommand::SetNote { row, channel, note } => {
                assert_eq!(row, 0);
                assert_eq!(channel, 0);
                assert_eq!(note.pitch, Pitch::C);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_clear_pattern_command() {
        let pattern = Pattern::new(4, 2);
        let cmds = clear_pattern(&pattern);
        assert_eq!(cmds.len(), 1);
        match &cmds[0] {
            PatternCommand::ClearPattern => {}
            _ => panic!("Expected ClearPattern"),
        }
    }

    #[test]
    fn test_fill_column() {
        let pattern = Pattern::new(8, 2);
        let notes = vec![
            Note::simple(Pitch::C, 4),
            Note::simple(Pitch::E, 4),
        ];
        let cmds = fill_column(&pattern, 0, &notes);
        assert_eq!(cmds.len(), 8);

        // Apply and verify cycling
        let mut pat = Pattern::new(8, 2);
        apply_commands(&mut pat, &cmds);
        assert_eq!(
            pat.get_cell(0, 0).unwrap().note,
            Some(NoteEvent::On(Note::simple(Pitch::C, 4)))
        );
        assert_eq!(
            pat.get_cell(1, 0).unwrap().note,
            Some(NoteEvent::On(Note::simple(Pitch::E, 4)))
        );
        assert_eq!(
            pat.get_cell(2, 0).unwrap().note,
            Some(NoteEvent::On(Note::simple(Pitch::C, 4)))
        );
    }

    #[test]
    fn test_fill_column_empty_notes() {
        let pattern = Pattern::new(4, 1);
        let cmds = fill_column(&pattern, 0, &[]);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_generate_beat() {
        let pattern = Pattern::new(8, 1);
        let rhythm = vec![true, false, false, true, false, false, true, false];
        let note = Note::simple(Pitch::C, 4);
        let cmds = generate_beat(&pattern, 0, &rhythm, note);

        let mut pat = Pattern::new(8, 1);
        apply_commands(&mut pat, &cmds);

        assert!(pat.get_cell(0, 0).unwrap().note.is_some());
        assert!(pat.get_cell(1, 0).unwrap().note.is_none());
        assert!(pat.get_cell(3, 0).unwrap().note.is_some());
        assert!(pat.get_cell(6, 0).unwrap().note.is_some());
    }

    #[test]
    fn test_transpose() {
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(2, 0, Note::simple(Pitch::E, 4));

        let cmds = transpose(&pattern, 2);
        apply_commands(&mut pattern, &cmds);

        // C+2 = D, E+2 = F#
        let cell0 = pattern.get_cell(0, 0).unwrap();
        match &cell0.note {
            Some(NoteEvent::On(n)) => {
                assert_eq!(n.pitch, Pitch::D);
                assert_eq!(n.octave, 4);
            }
            _ => panic!("Expected note at row 0"),
        }

        let cell2 = pattern.get_cell(2, 0).unwrap();
        match &cell2.note {
            Some(NoteEvent::On(n)) => {
                assert_eq!(n.pitch, Pitch::FSharp);
                assert_eq!(n.octave, 4);
            }
            _ => panic!("Expected note at row 2"),
        }
    }

    #[test]
    fn test_reverse() {
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(3, 0, Note::simple(Pitch::G, 4));

        let cmds = reverse(&pattern);
        apply_commands(&mut pattern, &cmds);

        // Row 0 had C -> now at row 3
        // Row 3 had G -> now at row 0
        let cell0 = pattern.get_cell(0, 0).unwrap();
        match &cell0.note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::G),
            _ => panic!("Expected G at row 0"),
        }

        let cell3 = pattern.get_cell(3, 0).unwrap();
        match &cell3.note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 3"),
        }
    }

    #[test]
    fn test_rotate() {
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));

        let cmds = rotate(&pattern, 2);
        apply_commands(&mut pattern, &cmds);

        // Row 0 note should now be at row 2
        assert!(pattern.get_cell(0, 0).unwrap().note.is_none());
        let cell2 = pattern.get_cell(2, 0).unwrap();
        match &cell2.note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 2"),
        }
    }

    #[test]
    fn test_rotate_negative() {
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(2, 0, Note::simple(Pitch::E, 4));

        let cmds = rotate(&pattern, -1);
        apply_commands(&mut pattern, &cmds);

        // Row 2 note should now be at row 1
        let cell1 = pattern.get_cell(1, 0).unwrap();
        match &cell1.note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::E),
            _ => panic!("Expected E at row 1"),
        }
    }

    #[test]
    fn test_humanize() {
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_note(1, 0, Note::new(Pitch::E, 4, 100, 0));

        let cmds = humanize(&pattern, 10);
        assert_eq!(cmds.len(), 2);

        // Apply and verify velocity is within range
        apply_commands(&mut pattern, &cmds);
        for row in 0..2 {
            if let Some(NoteEvent::On(n)) = &pattern.get_cell(row, 0).unwrap().note {
                assert!(n.velocity >= 90 && n.velocity <= 110,
                    "Velocity {} out of expected range", n.velocity);
            }
        }
    }

    #[test]
    fn test_humanize_zero_variance() {
        let mut pattern = Pattern::new(2, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));

        let cmds = humanize(&pattern, 0);
        apply_commands(&mut pattern, &cmds);

        if let Some(NoteEvent::On(n)) = &pattern.get_cell(0, 0).unwrap().note {
            assert_eq!(n.velocity, 100);
        }
    }

    #[test]
    fn test_humanize_clamps_velocity() {
        let mut pattern = Pattern::new(2, 1);
        // Very low velocity + high variance should clamp to 0
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 5, 0));
        // Very high velocity + high variance should clamp to 127
        pattern.set_note(1, 0, Note::new(Pitch::E, 4, 125, 0));

        let cmds = humanize(&pattern, 50);
        apply_commands(&mut pattern, &cmds);

        for row in 0..2 {
            if let Some(NoteEvent::On(n)) = &pattern.get_cell(row, 0).unwrap().note {
                assert!(n.velocity <= 127, "Velocity exceeds max: {}", n.velocity);
            }
        }
    }
}
