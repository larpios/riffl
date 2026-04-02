mod editing;
mod instrument;
mod navigation;
mod project;
mod track;
mod transport;
mod view;

use crate::app::App;
use crate::input::keybindings::Action;
use crossterm::event::KeyEvent;

pub(super) fn handle_action(app: &mut App, action: Action, key: KeyEvent) {
    let term_width = crossterm::terminal::size().map(|(w, _)| w).unwrap_or(80);

    if navigation::handle(app, &action, term_width) {
        return;
    }
    if editing::handle(app, &action) {
        return;
    }
    if transport::handle(app, &action) {
        return;
    }
    if track::handle(app, &action) {
        return;
    }
    if instrument::handle(app, &action, key) {
        return;
    }
    if view::handle(app, &action) {
        return;
    }
    project::handle(app, &action);
}
