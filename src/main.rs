mod pattern;
mod editor;
mod event;
mod ui;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use editor::Editor;
use pattern::{Note, Pitch};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

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

        // Read raw keyboard events with a short timeout
        if crossterm::event::poll(Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key_event) = crossterm::event::read()? {
                // Handle the key event
                if !handle_key_input(key_event, editor, pending_pitch) {
                    // Quit signal received
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Handle keyboard input and update editor state
/// Returns false if the application should quit, true otherwise
fn handle_key_input(
    key_event: crossterm::event::KeyEvent,
    editor: &mut Editor,
    pending_pitch: &mut Option<Pitch>,
) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};

    // Handle Ctrl+C for quit
    if key_event.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') = key_event.code {
            return false;
        }
    }

    match key_event.code {
        // Navigation - Arrow keys
        KeyCode::Up => {
            editor.move_up();
            *pending_pitch = None;
        }
        KeyCode::Down => {
            editor.move_down();
            *pending_pitch = None;
        }
        KeyCode::Left => {
            editor.move_left();
            *pending_pitch = None;
        }
        KeyCode::Right => {
            editor.move_right();
            *pending_pitch = None;
        }

        // Navigation - vim-style (but only if not entering a note)
        KeyCode::Char('h') if pending_pitch.is_none() => {
            editor.move_left();
        }
        KeyCode::Char('j') if pending_pitch.is_none() => {
            editor.move_down();
        }
        KeyCode::Char('k') if pending_pitch.is_none() => {
            editor.move_up();
        }
        KeyCode::Char('l') if pending_pitch.is_none() => {
            editor.move_right();
        }

        // Edit operations
        KeyCode::Delete | KeyCode::Backspace => {
            editor.delete_note();
            *pending_pitch = None;
        }
        KeyCode::Insert => {
            editor.insert_row();
            *pending_pitch = None;
        }

        // Quit
        KeyCode::Char('q') if pending_pitch.is_none() => {
            return false;
        }
        KeyCode::Esc => {
            return false;
        }

        // Character input for note entry
        KeyCode::Char(c) => {
            // Check if it's a pitch letter (A-G)
            if let Some(pitch) = Pitch::from_char(c) {
                // Store pending pitch (waiting for octave digit)
                *pending_pitch = Some(pitch);
            } else if c.is_ascii_digit() {
                // Check if we have a pending pitch
                if let Some(pitch) = pending_pitch.take() {
                    if let Some(octave) = c.to_digit(10) {
                        let note = Note::new(pitch, octave as u8);
                        editor.enter_note(note);
                    }
                }
            } else {
                // Unknown character, cancel pending input
                *pending_pitch = None;
            }
        }

        _ => {
            // Other keys - ignore
        }
    }

    true
}
