mod app;
mod audio;
mod editor;
mod input;
mod pattern;
mod ui;

use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use app::App;
use editor::Editor;
use input::keybindings::{map_key_to_action, Action};

/// Tick rate for the event loop (16ms ≈ 60 FPS for smooth BPM timing)
const TICK_RATE: Duration = Duration::from_millis(16);

fn main() -> Result<()> {
    // Set up panic hook to restore terminal before panicking
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Initialize terminal — requires a TTY (won't work in CI/headless environments)
    let mut terminal = match init_terminal() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("tracker-rs: Failed to initialize terminal: {}", e);
            eprintln!("This application requires an interactive terminal (TTY) to run.");
            return Err(e);
        }
    };

    // Create and initialize app
    let mut app = App::new();
    app.init()?;

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    restore_terminal()?;

    // Propagate any errors from the app
    result
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    while app.should_run() {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key);
                    }
                }
                Event::Resize(_width, _height) => {}
                _ => {}
            }
        }

        app.update()?;
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    // If a modal is open, handle modal-specific input first
    if app.has_modal() {
        let action = map_key_to_action(key, app.editor_mode());
        match action {
            Action::Cancel | Action::Confirm | Action::EnterNormalMode => {
                app.close_modal();
            }
            _ => {}
        }
        return;
    }

    let action = map_key_to_action(key, app.editor_mode());

    match action {
        // Navigation — delegate to editor
        Action::MoveLeft => app.editor.move_left(),
        Action::MoveDown => app.editor.move_down(),
        Action::MoveUp => app.editor.move_up(),
        Action::MoveRight => app.editor.move_right(),
        Action::PageUp => app.editor.page_up(),
        Action::PageDown => app.editor.page_down(),

        // Mode transitions
        Action::EnterInsertMode => app.editor.enter_insert_mode(),
        Action::EnterNormalMode => app.editor.enter_normal_mode(),
        Action::EnterVisualMode => app.editor.enter_visual_mode(),

        // Note entry (Insert mode)
        Action::EnterNote(c) => {
            if let Some(pitch) = Editor::char_to_pitch(c) {
                app.editor.enter_note(pitch);
            }
        }
        Action::SetOctave(oct) => app.editor.set_octave(oct),

        // Editing
        Action::DeleteCell => app.editor.delete_cell(),
        Action::InsertRow => app.editor.insert_row(),
        Action::DeleteRow => app.editor.delete_row(),
        Action::Undo => { app.editor.undo(); }

        // Application
        Action::Quit => app.quit(),
        Action::TogglePlay => app.toggle_play(),
        Action::OpenModal => app.open_test_modal(),
        Action::Cancel => { app.close_modal(); }
        Action::Confirm => {
            if app.has_modal() {
                app.close_modal();
            }
        }
        Action::None => {}
    }
}
