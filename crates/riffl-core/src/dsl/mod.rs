/// Live coding DSL module for algorithmic pattern generation.
///
/// Provides a Rhai-based scripting engine that can generate notes, scales,
/// chords, and rhythms, and manipulate tracker patterns programmatically.
pub mod engine;
pub mod examples;
pub mod hooks;
pub mod pattern_api;

pub use hooks::HooksEngine;
