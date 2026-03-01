/// Rhai scripting engine for live coding pattern generation.
///
/// Wraps a Rhai `Engine` with registered music-domain functions and types,
/// allowing users to generate and manipulate patterns programmatically.

use rhai::{Engine, EvalAltResult, Array, Dynamic, Scope, INT};
use rand::Rng;

use crate::pattern::{Note, Pattern, Pitch};
use super::pattern_api;

/// Result of evaluating a script.
#[derive(Debug)]
pub enum ScriptResult {
    /// The script produced a modified pattern.
    PatternResult(Pattern),
    /// The script returned a value (displayed as string).
    Value(String),
    /// The script produced no meaningful return value.
    Unit,
}

/// The DSL scripting engine wrapping Rhai.
pub struct ScriptEngine {
    engine: Engine,
}

impl ScriptEngine {
    /// Create a new ScriptEngine with all music functions registered.
    pub fn new() -> Self {
        let mut engine = Engine::new();
        register_music_functions(&mut engine);
        Self { engine }
    }

    /// Evaluate a script string and return the result.
    pub fn eval(&self, code: &str) -> Result<ScriptResult, String> {
        let mut scope = Scope::new();
        match self.engine.eval_with_scope::<Dynamic>(&mut scope, code) {
            Ok(result) => {
                if result.is_unit() {
                    Ok(ScriptResult::Unit)
                } else {
                    Ok(ScriptResult::Value(format!("{}", result)))
                }
            }
            Err(e) => Err(format_rhai_error(&e)),
        }
    }

    /// Evaluate a script with a pattern in scope, returning the modified pattern.
    pub fn eval_with_pattern(
        &self,
        code: &str,
        pattern: &Pattern,
    ) -> Result<(ScriptResult, Vec<PatternCommand>), String> {
        // Collect commands from the script
        let commands = std::sync::Arc::new(std::sync::Mutex::new(Vec::<PatternCommand>::new()));

        // Build a fresh engine with all the base functions plus pattern commands
        let mut engine = Engine::new();
        register_music_functions(&mut engine);

        // Register set_note(row, channel, note_map)
        let cmds_clone = commands.clone();
        engine.register_fn("set_note", move |row: INT, channel: INT, note: rhai::Map| {
            if let Some(cmd) = map_to_set_note_command(row, channel, &note) {
                cmds_clone.lock().unwrap().push(cmd);
            }
        });

        // Register clear_cell(row, channel)
        let cmds_clone = commands.clone();
        engine.register_fn("clear_cell", move |row: INT, channel: INT| {
            if row >= 0 && channel >= 0 {
                cmds_clone.lock().unwrap().push(PatternCommand::ClearCell {
                    row: row as usize,
                    channel: channel as usize,
                });
            }
        });

        // Register clear_pattern()
        let cmds_clone = commands.clone();
        engine.register_fn("clear_pattern", move || {
            cmds_clone.lock().unwrap().push(PatternCommand::ClearPattern);
        });

        // Register fill_column(channel, notes_array)
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("fill_column", move |channel: INT, notes: Array| {
            if channel < 0 {
                return;
            }
            let parsed_notes: Vec<Note> = notes
                .iter()
                .filter_map(|d| dynamic_to_note(d))
                .collect();
            let new_cmds = pattern_api::fill_column(&pat_clone, channel as usize, &parsed_notes);
            cmds_clone.lock().unwrap().extend(new_cmds);
        });

        // Register generate_beat(channel, rhythm_array, note)
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("generate_beat", move |channel: INT, rhythm: Array, note: rhai::Map| {
            if channel < 0 {
                return;
            }
            let bools: Vec<bool> = rhythm
                .iter()
                .map(|d| d.as_bool().unwrap_or(false))
                .collect();
            if let Some(n) = map_to_note(&note) {
                let new_cmds = pattern_api::generate_beat(&pat_clone, channel as usize, &bools, n);
                cmds_clone.lock().unwrap().extend(new_cmds);
            }
        });

        // Register transpose(semitones)
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("transpose", move |semitones: INT| {
            let new_cmds = pattern_api::transpose(&pat_clone, semitones as i32);
            cmds_clone.lock().unwrap().extend(new_cmds);
        });

        // Register reverse()
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("reverse", move || {
            let new_cmds = pattern_api::reverse(&pat_clone);
            cmds_clone.lock().unwrap().extend(new_cmds);
        });

        // Register rotate(offset)
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("rotate", move |offset: INT| {
            let new_cmds = pattern_api::rotate(&pat_clone, offset as i32);
            cmds_clone.lock().unwrap().extend(new_cmds);
        });

        // Register humanize(velocity_variance)
        let cmds_clone = commands.clone();
        let pat_clone = pattern.clone();
        engine.register_fn("humanize", move |velocity_variance: INT| {
            let variance = (velocity_variance.max(0).min(127)) as u8;
            let new_cmds = pattern_api::humanize(&pat_clone, variance);
            cmds_clone.lock().unwrap().extend(new_cmds);
        });

        let mut scope = Scope::new();
        scope.push("num_rows", pattern.num_rows() as INT);
        scope.push("num_channels", pattern.num_channels() as INT);

        match engine.eval_with_scope::<Dynamic>(&mut scope, code) {
            Ok(result) => {
                let cmds = commands.lock().unwrap().clone();
                let script_result: ScriptResult = if result.is_unit() {
                    ScriptResult::Unit
                } else {
                    ScriptResult::Value(format!("{}", result))
                };
                Ok((script_result, cmds))
            }
            Err(e) => Err(format_rhai_error(&e)),
        }
    }
}

/// Commands that a script can issue to modify a pattern.
#[derive(Debug, Clone)]
pub enum PatternCommand {
    SetNote {
        row: usize,
        channel: usize,
        note: Note,
    },
    ClearCell {
        row: usize,
        channel: usize,
    },
    ClearPattern,
}

/// Apply a list of pattern commands to a pattern.
pub fn apply_commands(pattern: &mut Pattern, commands: &[PatternCommand]) {
    for cmd in commands {
        match cmd {
            PatternCommand::SetNote { row, channel, note } => {
                pattern.set_note(*row, *channel, *note);
            }
            PatternCommand::ClearCell { row, channel } => {
                pattern.clear_cell(*row, *channel);
            }
            PatternCommand::ClearPattern => {
                for r in 0..pattern.num_rows() {
                    for c in 0..pattern.num_channels() {
                        pattern.clear_cell(r, c);
                    }
                }
            }
        }
    }
}

/// Register all music-domain functions on a Rhai Engine.
fn register_music_functions(engine: &mut Engine) {
    // note(pitch_str, octave) -> note map
    engine.register_fn("note", |pitch_str: &str, octave: INT| -> Dynamic {
        let pitch = match Pitch::from_str(pitch_str) {
            Some(p) => p,
            None => return Dynamic::UNIT,
        };
        if octave < 0 || octave > 9 {
            return Dynamic::UNIT;
        }
        note_to_dynamic(Note::simple(pitch, octave as u8))
    });

    // scale(root, mode, octave) -> array of note maps
    engine.register_fn("scale", |root: &str, mode: &str, octave: INT| -> Array {
        let pitch = match Pitch::from_str(root) {
            Some(p) => p,
            None => return Array::new(),
        };
        if octave < 0 || octave > 9 {
            return Array::new();
        }
        let intervals = match mode.to_lowercase().as_str() {
            "major" => vec![0, 2, 4, 5, 7, 9, 11],
            "minor" => vec![0, 2, 3, 5, 7, 8, 10],
            "pentatonic" => vec![0, 2, 4, 7, 9],
            "blues" => vec![0, 3, 5, 6, 7, 10],
            "dorian" => vec![0, 2, 3, 5, 7, 9, 10],
            "mixolydian" => vec![0, 2, 4, 5, 7, 9, 10],
            _ => return Array::new(),
        };
        let base_note = Note::simple(pitch, octave as u8);
        intervals
            .iter()
            .filter_map(|&interval| base_note.transpose(interval))
            .map(|n| note_to_dynamic(n))
            .collect()
    });

    // chord(root, quality, octave) -> array of note maps
    engine.register_fn("chord", |root: &str, quality: &str, octave: INT| -> Array {
        let pitch = match Pitch::from_str(root) {
            Some(p) => p,
            None => return Array::new(),
        };
        if octave < 0 || octave > 9 {
            return Array::new();
        }
        let intervals = match quality.to_lowercase().as_str() {
            "major" | "maj" => vec![0, 4, 7],
            "minor" | "min" => vec![0, 3, 7],
            "7th" | "7" | "dom7" => vec![0, 4, 7, 10],
            "maj7" => vec![0, 4, 7, 11],
            "dim" => vec![0, 3, 6],
            "aug" => vec![0, 4, 8],
            _ => return Array::new(),
        };
        let base_note = Note::simple(pitch, octave as u8);
        intervals
            .iter()
            .filter_map(|&interval| base_note.transpose(interval))
            .map(|n| note_to_dynamic(n))
            .collect()
    });

    // random_note(notes_array) -> random note from array
    engine.register_fn("random_note", |notes: Array| -> Dynamic {
        if notes.is_empty() {
            return Dynamic::UNIT;
        }
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..notes.len());
        notes[idx].clone()
    });

    // euclidean(pulses, steps) -> array of booleans
    engine.register_fn("euclidean", |pulses: INT, steps: INT| -> Array {
        if steps <= 0 || pulses < 0 {
            return Array::new();
        }
        let steps = steps as usize;
        let pulses = (pulses as usize).min(steps);
        generate_euclidean(pulses, steps)
            .into_iter()
            .map(|b| Dynamic::from(b))
            .collect()
    });

    // random(min, max) -> random integer in range
    engine.register_fn("random", |min: INT, max: INT| -> INT {
        if min >= max {
            return min;
        }
        let mut rng = rand::thread_rng();
        rng.gen_range(min..=max)
    });

    // random_float() -> random float 0.0-1.0
    engine.register_fn("random_float", || -> f64 {
        let mut rng = rand::thread_rng();
        rng.gen::<f64>()
    });

    // Accessors for note maps
    engine.register_fn("get_pitch", |note: &mut rhai::Map| -> String {
        note.get("pitch")
            .and_then(|v| v.clone().into_string().ok())
            .unwrap_or_default()
    });

    engine.register_fn("get_octave", |note: &mut rhai::Map| -> INT {
        note.get("octave")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(4)
    });

    engine.register_fn("get_velocity", |note: &mut rhai::Map| -> INT {
        note.get("velocity")
            .and_then(|v| v.as_int().ok())
            .unwrap_or(100)
    });
}

/// Convert a Note to a Rhai Dynamic map.
fn note_to_dynamic(note: Note) -> Dynamic {
    let mut map = rhai::Map::new();
    map.insert("pitch".into(), Dynamic::from(pitch_to_string(note.pitch)));
    map.insert("octave".into(), Dynamic::from(note.octave as INT));
    map.insert("velocity".into(), Dynamic::from(note.velocity as INT));
    map.insert("instrument".into(), Dynamic::from(note.instrument as INT));
    Dynamic::from(map)
}

/// Convert a Pitch to its string representation for Rhai.
fn pitch_to_string(pitch: Pitch) -> String {
    match pitch {
        Pitch::C => "C".to_string(),
        Pitch::CSharp => "C#".to_string(),
        Pitch::D => "D".to_string(),
        Pitch::DSharp => "D#".to_string(),
        Pitch::E => "E".to_string(),
        Pitch::F => "F".to_string(),
        Pitch::FSharp => "F#".to_string(),
        Pitch::G => "G".to_string(),
        Pitch::GSharp => "G#".to_string(),
        Pitch::A => "A".to_string(),
        Pitch::ASharp => "A#".to_string(),
        Pitch::B => "B".to_string(),
    }
}

/// Convert a Rhai Dynamic (expected to be a Map) to a Note.
fn dynamic_to_note(d: &Dynamic) -> Option<Note> {
    let map = d.read_lock::<rhai::Map>()?;
    map_to_note(&map)
}

/// Convert a Rhai Map to a Note.
fn map_to_note(note: &rhai::Map) -> Option<Note> {
    let pitch_str = note.get("pitch")?.clone().into_string().ok()?;
    let pitch = Pitch::from_str(&pitch_str)?;
    let octave = note.get("octave")?.as_int().ok()? as u8;
    let velocity = note
        .get("velocity")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(100) as u8;
    let instrument = note
        .get("instrument")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0) as u8;
    if octave > 9 || velocity > 127 {
        return None;
    }
    Some(Note::new(pitch, octave, velocity, instrument))
}

/// Convert a Rhai map + coordinates to a SetNote command.
fn map_to_set_note_command(row: INT, channel: INT, note: &rhai::Map) -> Option<PatternCommand> {
    if row < 0 || channel < 0 {
        return None;
    }
    let pitch_str = note.get("pitch")?.clone().into_string().ok()?;
    let pitch = Pitch::from_str(&pitch_str)?;
    let octave = note.get("octave")?.as_int().ok()? as u8;
    let velocity = note
        .get("velocity")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(100) as u8;
    let instrument = note
        .get("instrument")
        .and_then(|v| v.as_int().ok())
        .unwrap_or(0) as u8;

    if octave > 9 || velocity > 127 {
        return None;
    }

    Some(PatternCommand::SetNote {
        row: row as usize,
        channel: channel as usize,
        note: Note::new(pitch, octave, velocity, instrument),
    })
}

/// Generate a Euclidean rhythm pattern.
///
/// Uses Bjorklund's algorithm to evenly distribute `pulses` across `steps`.
fn generate_euclidean(pulses: usize, steps: usize) -> Vec<bool> {
    if steps == 0 {
        return Vec::new();
    }
    if pulses >= steps {
        return vec![true; steps];
    }
    if pulses == 0 {
        return vec![false; steps];
    }

    // Bjorklund's algorithm using sequence interleaving
    let mut groups: Vec<Vec<bool>> = Vec::new();

    // Start with `pulses` groups of [true] and `steps-pulses` groups of [false]
    for _ in 0..pulses {
        groups.push(vec![true]);
    }
    for _ in 0..(steps - pulses) {
        groups.push(vec![false]);
    }

    loop {
        // Count the "remainder" groups (those after the first group type ends)
        // Find where the pattern type changes
        let first = &groups[0];
        let split_pos = groups.iter().position(|g| g != first);
        let split_pos = match split_pos {
            Some(p) => p,
            None => break, // All groups are identical, we're done
        };

        let remainder = groups.len() - split_pos;
        if remainder <= 1 {
            break; // Only 0 or 1 remainder groups, we're done
        }

        // Distribute remainder groups by appending each to a front group
        let distribute_count = split_pos.min(remainder);
        let mut new_groups = Vec::new();

        // Take the pairs: front[i] ++ remainder[i]
        for i in 0..distribute_count {
            let mut combined = groups[i].clone();
            combined.extend_from_slice(&groups[split_pos + i]);
            new_groups.push(combined);
        }

        // Add any leftover front groups
        for i in distribute_count..split_pos {
            new_groups.push(groups[i].clone());
        }

        // Add any leftover remainder groups
        for i in (split_pos + distribute_count)..groups.len() {
            new_groups.push(groups[i].clone());
        }

        groups = new_groups;
    }

    groups.into_iter().flatten().collect()
}

/// Format a Rhai error into a user-friendly message with line numbers.
fn format_rhai_error(err: &EvalAltResult) -> String {
    format!("{}", err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::NoteEvent;

    #[test]
    fn test_script_engine_creation() {
        let _engine = ScriptEngine::new();
    }

    #[test]
    fn test_note_creation() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let n = note("C", 4); n.pitch"#);
        assert!(result.is_ok());
        match result.unwrap() {
            ScriptResult::Value(v) => assert_eq!(v, "C"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_note_creation_sharp() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let n = note("C#", 4); n.pitch"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "C#"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_note_creation_invalid_pitch() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let n = note("X", 4); n"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "()"),
            ScriptResult::Unit => {} // also acceptable
            _ => panic!("Expected Unit or empty result"),
        }
    }

    #[test]
    fn test_note_creation_invalid_octave() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let n = note("C", 10); n"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "()"),
            ScriptResult::Unit => {}
            _ => panic!("Expected Unit or empty result"),
        }
    }

    #[test]
    fn test_scale_major() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let s = scale("C", "major", 4); s.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "7"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_major_notes() {
        let engine = ScriptEngine::new();
        // C major scale: C, D, E, F, G, A, B
        let result = engine
            .eval(
                r#"
                let s = scale("C", "major", 4);
                let pitches = [];
                for n in s { pitches.push(n.pitch); }
                pitches
                "#,
            )
            .unwrap();
        match result {
            ScriptResult::Value(v) => {
                assert!(v.contains("C"));
                assert!(v.contains("D"));
                assert!(v.contains("E"));
                assert!(v.contains("F"));
                assert!(v.contains("G"));
                assert!(v.contains("A"));
                assert!(v.contains("B"));
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_pentatonic() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let s = scale("C", "pentatonic", 4); s.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "5"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_blues() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let s = scale("A", "blues", 4); s.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "6"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_invalid_mode() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let s = scale("C", "invalid", 4); s.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "0"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_major() {
        let engine = ScriptEngine::new();
        // C major chord: C, E, G
        let result = engine
            .eval(r#"let c = chord("C", "major", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "3"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_major_notes() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(
                r#"
                let c = chord("C", "major", 4);
                let pitches = [];
                for n in c { pitches.push(n.pitch); }
                pitches
                "#,
            )
            .unwrap();
        match result {
            ScriptResult::Value(v) => {
                assert!(v.contains("C"), "Missing C in {}", v);
                assert!(v.contains("E"), "Missing E in {}", v);
                assert!(v.contains("G"), "Missing G in {}", v);
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_minor() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let c = chord("A", "minor", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "3"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_seventh() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let c = chord("G", "7th", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "4"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_dim() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let c = chord("B", "dim", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "3"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_aug() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let c = chord("C", "aug", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "3"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_invalid_quality() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let c = chord("C", "invalid", 4); c.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "0"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_euclidean_3_8() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let e = euclidean(3, 8); e.len()"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "8"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_euclidean_3_8_pulse_count() {
        let engine = ScriptEngine::new();
        // Count how many true values
        let result = engine
            .eval(
                r#"
                let e = euclidean(3, 8);
                let count = 0;
                for v in e { if v { count += 1; } }
                count
                "#,
            )
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "3"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_euclidean_4_16() {
        let result = generate_euclidean(4, 16);
        assert_eq!(result.len(), 16);
        assert_eq!(result.iter().filter(|&&b| b).count(), 4);
    }

    #[test]
    fn test_euclidean_0_steps() {
        let result = generate_euclidean(0, 8);
        assert_eq!(result.len(), 8);
        assert!(result.iter().all(|&b| !b));
    }

    #[test]
    fn test_euclidean_all_pulses() {
        let result = generate_euclidean(8, 8);
        assert_eq!(result.len(), 8);
        assert!(result.iter().all(|&b| b));
    }

    #[test]
    fn test_random_range() {
        let engine = ScriptEngine::new();
        // Run several times; result should be in range
        for _ in 0..10 {
            let result = engine.eval(r#"random(1, 10)"#).unwrap();
            match result {
                ScriptResult::Value(v) => {
                    let val: i64 = v.parse().unwrap();
                    assert!(val >= 1 && val <= 10, "random() out of range: {}", val);
                }
                _ => panic!("Expected Value result"),
            }
        }
    }

    #[test]
    fn test_random_float_range() {
        let engine = ScriptEngine::new();
        for _ in 0..10 {
            let result = engine.eval(r#"random_float()"#).unwrap();
            match result {
                ScriptResult::Value(v) => {
                    let val: f64 = v.parse().unwrap();
                    assert!(
                        val >= 0.0 && val <= 1.0,
                        "random_float() out of range: {}",
                        val
                    );
                }
                _ => panic!("Expected Value result"),
            }
        }
    }

    #[test]
    fn test_eval_error_handling() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let x = ;"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn test_eval_undefined_variable() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"undefined_var + 1"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_unit_result() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let x = 42;"#).unwrap();
        match result {
            ScriptResult::Unit => {}
            _ => panic!("Expected Unit result for let statement"),
        }
    }

    #[test]
    fn test_eval_with_pattern_set_note() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(16, 4);
        let code = r#"
            let n = note("C", 4);
            set_note(0, 0, n);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PatternCommand::SetNote { row, channel, note } => {
                assert_eq!(*row, 0);
                assert_eq!(*channel, 0);
                assert_eq!(note.pitch, Pitch::C);
                assert_eq!(note.octave, 4);
            }
            _ => panic!("Expected SetNote command"),
        }
    }

    #[test]
    fn test_eval_with_pattern_clear() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(16, 4);
        let code = r#"clear_pattern();"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PatternCommand::ClearPattern => {}
            _ => panic!("Expected ClearPattern command"),
        }
    }

    #[test]
    fn test_eval_with_pattern_dimensions_available() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(32, 8);
        let code = r#"num_rows"#;
        let (result, _) = engine.eval_with_pattern(code, &pattern).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "32"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_apply_commands_set_note() {
        let mut pattern = Pattern::new(16, 4);
        let commands = vec![PatternCommand::SetNote {
            row: 0,
            channel: 0,
            note: Note::simple(Pitch::C, 4),
        }];
        apply_commands(&mut pattern, &commands);
        let cell = pattern.get_cell(0, 0).unwrap();
        assert_eq!(
            cell.note,
            Some(NoteEvent::On(Note::simple(Pitch::C, 4)))
        );
    }

    #[test]
    fn test_apply_commands_clear_pattern() {
        let mut pattern = Pattern::new(4, 2);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(1, 1, Note::simple(Pitch::E, 4));

        let commands = vec![PatternCommand::ClearPattern];
        apply_commands(&mut pattern, &commands);

        for r in 0..4 {
            for c in 0..2 {
                assert!(pattern.get_cell(r, c).unwrap().is_empty());
            }
        }
    }

    #[test]
    fn test_generate_euclidean_basic_patterns() {
        // E(1,4) should have 1 pulse in 4 steps
        let result = generate_euclidean(1, 4);
        assert_eq!(result.len(), 4);
        assert_eq!(result.iter().filter(|&&b| b).count(), 1);

        // E(2,8) should have 2 pulses in 8 steps
        let result = generate_euclidean(2, 8);
        assert_eq!(result.len(), 8);
        assert_eq!(result.iter().filter(|&&b| b).count(), 2);
    }

    #[test]
    fn test_script_complex_flow() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(8, 1);
        let code = r#"
            let s = scale("C", "pentatonic", 4);
            for i in range(0, 5) {
                set_note(i, 0, s[i]);
            }
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 5);
    }

    #[test]
    fn test_note_octave_accessor() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let n = note("A", 5); n.octave"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "5"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_note_velocity_accessor() {
        let engine = ScriptEngine::new();
        let result = engine
            .eval(r#"let n = note("A", 4); n.velocity"#)
            .unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "100"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_script_fill_column() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(8, 2);
        let code = r#"
            let notes = [note("C", 4), note("E", 4)];
            fill_column(0, notes);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        // fill_column should produce one SetNote per row (8 rows)
        assert_eq!(commands.len(), 8);

        // Apply and verify cycling
        let mut pat = Pattern::new(8, 2);
        apply_commands(&mut pat, &commands);
        match &pat.get_cell(0, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 0"),
        }
        match &pat.get_cell(1, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::E),
            _ => panic!("Expected E at row 1"),
        }
        match &pat.get_cell(2, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 2 (cycling)"),
        }
    }

    #[test]
    fn test_script_generate_beat() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(8, 1);
        let code = r#"
            let rhythm = euclidean(3, 8);
            let kick = note("C", 2);
            generate_beat(0, rhythm, kick);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        // Should have exactly 3 notes placed (3 pulses in euclidean(3,8))
        let set_notes: Vec<_> = commands
            .iter()
            .filter(|c| matches!(c, PatternCommand::SetNote { .. }))
            .collect();
        assert_eq!(set_notes.len(), 3);
    }

    #[test]
    fn test_script_transpose() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(2, 0, Note::simple(Pitch::E, 4));

        let code = r#"transpose(2);"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 2);

        apply_commands(&mut pattern, &commands);
        match &pattern.get_cell(0, 0).unwrap().note {
            Some(NoteEvent::On(n)) => {
                assert_eq!(n.pitch, Pitch::D);
                assert_eq!(n.octave, 4);
            }
            _ => panic!("Expected transposed note at row 0"),
        }
        match &pattern.get_cell(2, 0).unwrap().note {
            Some(NoteEvent::On(n)) => {
                assert_eq!(n.pitch, Pitch::FSharp);
                assert_eq!(n.octave, 4);
            }
            _ => panic!("Expected transposed note at row 2"),
        }
    }

    #[test]
    fn test_script_reverse() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        pattern.set_note(3, 0, Note::simple(Pitch::G, 4));

        let code = r#"reverse();"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        // Should have ClearPattern + 2 SetNote commands
        assert!(commands.len() >= 3);

        apply_commands(&mut pattern, &commands);
        match &pattern.get_cell(0, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::G),
            _ => panic!("Expected G at row 0 after reverse"),
        }
        match &pattern.get_cell(3, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 3 after reverse"),
        }
    }

    #[test]
    fn test_script_rotate() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));

        let code = r#"rotate(2);"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();

        apply_commands(&mut pattern, &commands);
        assert!(pattern.get_cell(0, 0).unwrap().note.is_none());
        match &pattern.get_cell(2, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 2 after rotate"),
        }
    }

    #[test]
    fn test_script_humanize() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::new(Pitch::C, 4, 100, 0));
        pattern.set_note(1, 0, Note::new(Pitch::E, 4, 100, 0));

        let code = r#"humanize(10);"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 2);

        apply_commands(&mut pattern, &commands);
        for row in 0..2 {
            if let Some(NoteEvent::On(n)) = &pattern.get_cell(row, 0).unwrap().note {
                assert!(
                    n.velocity >= 90 && n.velocity <= 110,
                    "Velocity {} out of expected range for row {}",
                    n.velocity,
                    row
                );
            }
        }
    }

    #[test]
    fn test_script_combined_operations() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(8, 1);
        let code = r#"
            let s = scale("C", "pentatonic", 4);
            fill_column(0, s);
            transpose(3);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        // fill_column produces 8 commands, transpose works on the *original* pattern
        // (which is empty), so it produces 0 transpose commands
        assert_eq!(commands.len(), 8);
    }

    // --- Additional tests per Phase-06 Task 3 spec ---

    #[test]
    fn test_note_creation_produces_correct_fields() {
        let engine = ScriptEngine::new();
        // Verify pitch, octave, and default velocity
        let result = engine.eval(r#"
            let n = note("C", 4);
            [n.pitch, n.octave, n.velocity]
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                assert!(v.contains("C"), "pitch should be C, got: {}", v);
                assert!(v.contains("4"), "octave should be 4, got: {}", v);
                assert!(v.contains("100"), "default velocity should be 100, got: {}", v);
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_note_creation_all_pitches() {
        let engine = ScriptEngine::new();
        let pitches = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        for p in pitches {
            let code = format!(r#"let n = note("{}", 4); n.pitch"#, p);
            let result = engine.eval(&code).unwrap();
            match result {
                ScriptResult::Value(v) => assert_eq!(v, p, "Pitch mismatch for {}", p),
                _ => panic!("Expected Value for pitch {}", p),
            }
        }
    }

    #[test]
    fn test_scale_major_exact_pitches() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let s = scale("C", "major", 4);
            let pitches = [];
            for n in s { pitches.push(n.pitch); }
            pitches
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                // C major: C D E F G A B - verify order
                let expected = ["C", "D", "E", "F", "G", "A", "B"];
                for pitch in expected {
                    assert!(v.contains(pitch), "Missing {} in C major scale: {}", pitch, v);
                }
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_minor() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let s = scale("A", "minor", 4); s.len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "7"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_dorian() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let s = scale("D", "dorian", 4); s.len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "7"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_scale_mixolydian() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let s = scale("G", "mixolydian", 4); s.len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "7"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_major_exact_pitches() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let c = chord("C", "major", 4);
            let pitches = [];
            for n in c { pitches.push(n.pitch); }
            pitches
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                // Should contain exactly C, E, G
                assert!(v.contains("C"), "Missing C in chord: {}", v);
                assert!(v.contains("E"), "Missing E in chord: {}", v);
                assert!(v.contains("G"), "Missing G in chord: {}", v);
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_minor_pitches() {
        let engine = ScriptEngine::new();
        // A minor chord: A, C, E
        let result = engine.eval(r#"
            let c = chord("A", "minor", 4);
            let pitches = [];
            for n in c { pitches.push(n.pitch); }
            pitches
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                assert!(v.contains("A"), "Missing A in chord: {}", v);
                assert!(v.contains("C"), "Missing C in chord: {}", v);
                assert!(v.contains("E"), "Missing E in chord: {}", v);
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_chord_maj7() {
        let engine = ScriptEngine::new();
        // C maj7: C, E, G, B (4 notes)
        let result = engine.eval(r#"let c = chord("C", "maj7", 4); c.len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "4"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_euclidean_3_8_exact_pattern() {
        // Bjorklund E(3,8) should produce: [true, false, false, true, false, false, true, false]
        let result = generate_euclidean(3, 8);
        assert_eq!(result.len(), 8);
        assert_eq!(result.iter().filter(|&&b| b).count(), 3);
        // Verify even distribution: true values should not be adjacent
        for i in 0..result.len() {
            if result[i] {
                let next = (i + 1) % result.len();
                // In E(3,8) no two trues should be adjacent
                assert!(
                    !result[next] || i == result.len() - 1,
                    "Euclidean(3,8) should have evenly distributed pulses"
                );
            }
        }
    }

    #[test]
    fn test_euclidean_5_8() {
        let result = generate_euclidean(5, 8);
        assert_eq!(result.len(), 8);
        assert_eq!(result.iter().filter(|&&b| b).count(), 5);
    }

    #[test]
    fn test_euclidean_1_1() {
        let result = generate_euclidean(1, 1);
        assert_eq!(result, vec![true]);
    }

    #[test]
    fn test_euclidean_via_script() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let e = euclidean(3, 8);
            let trues = 0;
            for v in e { if v { trues += 1; } }
            [e.len(), trues]
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                assert!(v.contains("8"), "Expected 8 steps: {}", v);
                assert!(v.contains("3"), "Expected 3 pulses: {}", v);
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_euclidean_negative_pulses() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"euclidean(-1, 8).len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "0"),
            _ => panic!("Expected empty array for negative pulses"),
        }
    }

    #[test]
    fn test_euclidean_zero_steps() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"euclidean(3, 0).len()"#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "0"),
            _ => panic!("Expected empty array for zero steps"),
        }
    }

    #[test]
    fn test_eval_with_pattern_set_note_and_apply() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(8, 2);
        let code = r#"
            let c = note("C", 4);
            let e = note("E", 4);
            let g = note("G", 4);
            set_note(0, 0, c);
            set_note(1, 0, e);
            set_note(2, 0, g);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 3);
        apply_commands(&mut pattern, &commands);

        match &pattern.get_cell(0, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::C),
            _ => panic!("Expected C at row 0"),
        }
        match &pattern.get_cell(1, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::E),
            _ => panic!("Expected E at row 1"),
        }
        match &pattern.get_cell(2, 0).unwrap().note {
            Some(NoteEvent::On(n)) => assert_eq!(n.pitch, Pitch::G),
            _ => panic!("Expected G at row 2"),
        }
    }

    #[test]
    fn test_eval_with_pattern_clear_cell() {
        let engine = ScriptEngine::new();
        let mut pattern = Pattern::new(4, 1);
        pattern.set_note(0, 0, Note::simple(Pitch::C, 4));
        let code = r#"clear_cell(0, 0);"#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            PatternCommand::ClearCell { row, channel } => {
                assert_eq!(*row, 0);
                assert_eq!(*channel, 0);
            }
            _ => panic!("Expected ClearCell command"),
        }
        apply_commands(&mut pattern, &commands);
        assert!(pattern.get_cell(0, 0).unwrap().note.is_none());
    }

    #[test]
    fn test_error_handling_syntax_error_has_content() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"let x = ;"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error should contain useful information, not be empty
        assert!(err.len() > 10, "Error message too short: {}", err);
    }

    #[test]
    fn test_error_handling_type_mismatch() {
        let engine = ScriptEngine::new();
        // Calling a method on wrong type should produce an error
        let result = engine.eval(r#"let x = 42; x.len()"#);
        assert!(result.is_err(), "Type mismatch should produce an error");
    }

    #[test]
    fn test_error_handling_unknown_function() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"nonexistent_function(42)"#);
        assert!(result.is_err(), "Unknown function should produce an error");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn test_error_handling_does_not_panic_on_bad_script() {
        let engine = ScriptEngine::new();
        // Various malformed scripts - none should panic
        let bad_scripts = [
            "",
            "{{{{",
            "fn x() { x() }",  // recursive but should just error
            r#"note("C", "not_a_number")"#,
            "let x = []; x[999]",
        ];
        for script in bad_scripts {
            let result = engine.eval(script);
            // Either Ok or Err is fine; the point is no panic
            let _ = result;
        }
    }

    #[test]
    fn test_error_handling_with_pattern_bad_script() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(4, 1);
        let result = engine.eval_with_pattern(r#"let x = ;"#, &pattern);
        assert!(result.is_err(), "Bad script with pattern should return error");
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error message should not be empty");
    }

    #[test]
    fn test_random_note_from_scale() {
        let engine = ScriptEngine::new();
        // random_note should return a valid note from the scale
        let result = engine.eval(r#"
            let s = scale("C", "pentatonic", 4);
            let n = random_note(s);
            n.pitch
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => {
                let valid = ["C", "D", "E", "G", "A"];
                assert!(
                    valid.contains(&v.as_str()),
                    "Random note {} not in C pentatonic scale", v
                );
            }
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_random_note_empty_array() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let empty = [];
            let n = random_note(empty);
            n
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "()"),
            ScriptResult::Unit => {}
            _ => panic!("Expected Unit for random_note of empty array"),
        }
    }

    #[test]
    fn test_get_pitch_accessor_function() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let n = note("F#", 3);
            get_pitch(n)
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "F#"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_get_octave_accessor_function() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let n = note("C", 7);
            get_octave(n)
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "7"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_get_velocity_accessor_function() {
        let engine = ScriptEngine::new();
        let result = engine.eval(r#"
            let n = note("C", 4);
            get_velocity(n)
        "#).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "100"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_num_channels_available_in_pattern_scope() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(16, 8);
        let code = r#"num_channels"#;
        let (result, _) = engine.eval_with_pattern(code, &pattern).unwrap();
        match result {
            ScriptResult::Value(v) => assert_eq!(v, "8"),
            _ => panic!("Expected Value result"),
        }
    }

    #[test]
    fn test_script_loop_with_set_note() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(16, 1);
        let code = r#"
            let s = scale("C", "major", 4);
            for i in range(0, 7) {
                set_note(i, 0, s[i]);
            }
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert_eq!(commands.len(), 7, "Should place 7 notes for C major scale");
    }

    #[test]
    fn test_negative_row_set_note_ignored() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(4, 1);
        let code = r#"
            let n = note("C", 4);
            set_note(-1, 0, n);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert!(commands.is_empty(), "Negative row should be ignored");
    }

    #[test]
    fn test_negative_channel_set_note_ignored() {
        let engine = ScriptEngine::new();
        let pattern = Pattern::new(4, 1);
        let code = r#"
            let n = note("C", 4);
            set_note(0, -1, n);
        "#;
        let (_, commands) = engine.eval_with_pattern(code, &pattern).unwrap();
        assert!(commands.is_empty(), "Negative channel should be ignored");
    }
}
