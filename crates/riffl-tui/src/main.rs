#![allow(dead_code, unused_imports)]
mod app;
mod config;
mod editor;
mod input;
mod registry;
mod ui;

use crate::app::App;
use crate::input::handler::{handle_key_event, handle_mouse_event};
use std::io;
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// Tick rate for the event loop (16ms ≈ 60 FPS for smooth BPM timing)
const TICK_RATE: Duration = Duration::from_millis(16);

fn main() -> Result<()> {
    // Initialize logging as early as possible
    let _ = riffl_core::log::init();

    // Check for --dump-config before doing any terminal setup
    if std::env::args().any(|arg| arg == "--dump-config") {
        let config = crate::config::Config::load();
        if let Ok(toml_str) = toml::to_string_pretty(&config) {
            println!("{}", toml_str);
        } else {
            println!("Failed to serialize config to TOML");
        }
        return Ok(());
    }

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
            // No terminal yet, can use println/eprintln safely here
            eprintln!("riffl: Failed to initialize terminal: {}", e);
            eprintln!("This application requires an interactive terminal (TTY) to run.");
            return Err(e);
        }
    };

    // Resolve sample directories and ensure the default one exists
    let cli_sample_dir = parse_sample_dir_flag();
    let config = crate::config::Config::load();
    let sample_dirs = config.resolve_sample_dirs(cli_sample_dir.as_deref());
    let default_samples = crate::config::Config::default_samples_dir();
    let _ = std::fs::create_dir_all(&default_samples);

    // Resolve module directories and ensure the default one exists
    let _module_dirs = config.resolve_module_dirs();
    let default_modules = crate::config::Config::default_modules_dir();
    let _ = std::fs::create_dir_all(&default_modules);

    // Create and initialize app
    let mut app = App::new();
    app.set_sample_dirs(sample_dirs);

    // Apply config: set theme from config file (or default "mocha")
    let theme_kind = config.theme_kind();
    app.theme = crate::ui::theme::Theme::from_kind(theme_kind.clone());
    app.theme_kind = theme_kind;
    app.config = config;

    // Re-apply roots so persisted bookmarks from config appear at startup.
    // set_sample_dirs (above) ran before app.config was assigned, so bookmarks
    // were not applied on that first call.
    app.refresh_browser_roots();

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
    execute!(stdout, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    execute!(io::stdout(), event::DisableMouseCapture)?;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.should_run() {
        if app.needs_full_redraw {
            app.needs_full_redraw = false;
            terminal.clear()?;
        }
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(app, key);
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(app, mouse);
                }
                Event::Resize(_width, _height) => {}
                _ => {}
            }
        }

        app.refresh_system_stats();
        app.update()?;
    }

    Ok(())
}

/// Parse `--sample-dir <path>` from the process arguments, returning the value if present.
fn parse_sample_dir_flag() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--sample-dir" {
            return iter.next().cloned();
        }
        if let Some(val) = arg.strip_prefix("--sample-dir=") {
            return Some(val.to_string());
        }
    }
    None
}
