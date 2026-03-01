/// Pattern data model for the tracker grid.
///
/// This module provides the core data structures for representing music
/// in a tracker format: notes, cells, rows, and patterns.

pub mod note;
pub mod pattern;
pub mod row;
pub mod track;

// Re-export commonly used types
pub use note::{Note, NoteEvent, NoteOff, Pitch};
pub use pattern::Pattern;
pub use row::{Cell, Effect, Row};
pub use track::Track;
