mod pattern;
mod editor;
mod event;
mod ui;

use editor::Editor;
use event::{read_event, Event};
use pattern::{Note, Pitch};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create editor with 16 rows and 4 channels
    let mut editor = Editor::new(16, 4);

    // State for two-character note input (pitch + octave)
    let mut pending_pitch: Option<Pitch> = None;

    // Main loop
    let result = run_event_loop(&mut terminal, &mut editor, &mut pending_pitch);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    editor: &mut Editor,
    pending_pitch: &mut Option<Pitch>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Render UI
        terminal.draw(|frame| {
            ui::render(frame, editor);
        })?;

        // Read keyboard events via the event module with a short timeout
        match read_event(Duration::from_millis(100))? {
            Event::MoveUp => {
                editor.move_up();
                *pending_pitch = None;
            }
            Event::MoveDown => {
                editor.move_down();
                *pending_pitch = None;
            }
            Event::MoveLeft => {
                editor.move_left();
                *pending_pitch = None;
            }
            Event::MoveRight => {
                editor.move_right();
                *pending_pitch = None;
            }
            Event::Delete => {
                editor.delete_note();
                *pending_pitch = None;
            }
            Event::Insert => {
                editor.insert_row();
                *pending_pitch = None;
            }
            Event::Quit => {
                // If a note is being entered, cancel it rather than quitting
                if pending_pitch.is_some() {
                    *pending_pitch = None;
                } else {
                    break;
                }
            }
            Event::NoteChar(c) => {
                if let Some(pitch) = Pitch::from_char(c) {
                    // First character: pitch letter — wait for octave digit
                    *pending_pitch = Some(pitch);
                } else if c.is_ascii_digit() {
                    // Second character: octave digit — complete the note
                    if let Some(pitch) = pending_pitch.take() {
                        if let Some(octave) = c.to_digit(10) {
                            let note = Note::new(pitch, octave as u8);
                            editor.enter_note(note);
                        }
                    }
                }
            }
            Event::Resize | Event::None => {}
        }
    }

    Ok(())
}
