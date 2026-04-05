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
    event::{
        self, Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
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

    // Print an annotated hooks.rhai template to stdout.
    if std::env::args().any(|arg| arg == "--dump-hooks") {
        print!("{}", HOOKS_TEMPLATE);
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
    // Enable kitty keyboard protocol so Ctrl+Enter (and similar combos) are
    // distinguishable from plain Enter. Terminals that don't support it ignore
    // the sequence silently, so this is safe to use unconditionally.
    let _ = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    );
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    execute!(io::stdout(), event::DisableMouseCapture)?;
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while app.should_run() {
        // While an external file picker (e.g. yazi) owns the terminal, skip
        // rendering and event reading — they would corrupt the picker's UI.
        // The sequencer update still runs so audio playback continues normally.
        if app.has_external_picker_running() {
            std::thread::sleep(TICK_RATE);
            app.refresh_system_stats();
            app.update()?;
            app.poll_picker();
            continue;
        }

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
                    let area = terminal.size().unwrap_or_default().into();
                    handle_mouse_event(app, mouse, area);
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

/// Annotated hooks.rhai template printed by `--dump-hooks`.
const HOOKS_TEMPLATE: &str = r#"// ~/.config/riffl/hooks.rhai
//
// Riffl hook script — place this file at ~/.config/riffl/hooks.rhai.
// Define any subset of the functions below; undefined hooks are silently skipped.
// Errors (syntax or runtime) are printed to stderr; riffl keeps running normally.
//
// Run  riffl --dump-hooks  to print this template again.

// ---------------------------------------------------------------------------
// normalize_picker_path(raw: string) -> string
//
// Transform the raw string written by an external file picker (yazi, lf, etc.)
// to a clean, usable file path before riffl tries to open it.
//
// Called for: Ctrl-F (sample picker), Ctrl-I (module picker), Ctrl-O (project picker).
//
// If this function is NOT defined, riffl's built-in normalisation runs:
//   • Strips yazi's search-mode URI prefix  search://<query>//<path>
//   • Resolves relative paths against the picker's start directory
//
// Returning `raw` unchanged lets the built-in normalisation run afterwards.
// ---------------------------------------------------------------------------
// fn normalize_picker_path(raw) {
//     // Example: strip a "file://" prefix produced by a custom picker
//     if raw.starts_with("file://") {
//         return raw.sub_string(7);
//     }
//     raw
// }

// ---------------------------------------------------------------------------
// on_project_loaded(path: string)
//
// Called after a .rtm project file is successfully loaded from disk.
// `path` is the absolute path of the loaded file.
// ---------------------------------------------------------------------------
// fn on_project_loaded(path) {
//     // print(`loaded project: ${path}`);
// }

// ---------------------------------------------------------------------------
// on_sample_loaded(path: string, inst_idx: int)
//
// Called after a sample file is loaded into instrument slot `inst_idx`.
// `path` is the absolute path of the loaded sample file.
// ---------------------------------------------------------------------------
// fn on_sample_loaded(path, inst_idx) {
//     // print(`loaded sample: ${path} → slot ${inst_idx}`);
// }

// ---------------------------------------------------------------------------
// on_startup()
//
// Called once when riffl starts, after configuration is loaded.
// ---------------------------------------------------------------------------
// fn on_startup() {
// }
"#;

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
