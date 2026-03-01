/// Pattern data model for the tracker grid.
///
/// This module provides the core data structures for representing music
/// in a tracker format: notes, cells, rows, effects, and patterns.

pub mod effect;
pub mod note;
pub mod pattern;
pub mod row;
pub mod track;

// Re-export commonly used types
pub use effect::{Effect, EffectType, MAX_EFFECTS_PER_CELL};
pub use note::{Note, NoteEvent, NoteOff, Pitch};
pub use pattern::Pattern;
pub use row::{Cell, Row};
pub use track::Track;
