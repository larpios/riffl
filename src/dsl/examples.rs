/// Built-in example scripts for the code editor template menu.
///
/// Each template demonstrates a different DSL capability and includes
/// comments explaining what it does. Accessible via `Ctrl+T` in the
/// code editor.

/// A named template with its title and source code.
#[derive(Debug, Clone)]
pub struct Template {
    /// Display name shown in the template menu.
    pub name: &'static str,
    /// Description shown below the template name.
    pub description: &'static str,
    /// The Rhai source code for this template.
    pub code: &'static str,
}

/// All built-in templates, in menu display order.
pub const TEMPLATES: &[Template] = &[
    Template {
        name: "Simple Beat",
        description: "4/4 kick-snare pattern using euclidean rhythms",
        code: SIMPLE_BEAT,
    },
    Template {
        name: "Random Melody",
        description: "Random notes from a pentatonic scale",
        code: RANDOM_MELODY,
    },
    Template {
        name: "Arpeggiator",
        description: "Cycle through chord tones across rows",
        code: ARPEGGIATOR,
    },
    Template {
        name: "Probability Beat",
        description: "Notes placed with random probability",
        code: PROBABILITY_BEAT,
    },
];

/// "Simple Beat" — 4/4 kick-snare pattern using euclidean rhythms.
const SIMPLE_BEAT: &str = r#"// Simple Beat — 4/4 kick-snare using euclidean rhythms
//
// Uses euclidean() to generate evenly-spaced rhythms:
//   - Kick:  4 hits across 16 steps (classic four-on-the-floor)
//   - Snare: 2 hits across 16 steps (backbeat on 4 and 12)
//   - Hat:   6 hits across 16 steps (syncopated hi-hat)

// Generate euclidean rhythms
let kick_rhythm  = euclidean(4, num_rows);
let snare_rhythm = euclidean(2, num_rows);
let hat_rhythm   = euclidean(6, num_rows);

// Create notes for each drum
let kick  = note("C", 2);
let snare = note("D", 2);
let hat   = note("F#", 2);

// Place the beats into channels 0, 1, 2
generate_beat(0, kick_rhythm, kick);
generate_beat(1, snare_rhythm, snare);
generate_beat(2, hat_rhythm, hat);
"#;

/// "Random Melody" — random notes from a pentatonic scale.
const RANDOM_MELODY: &str = r#"// Random Melody — pentatonic scale randomness
//
// Generates a random melody by picking notes from
// the C minor pentatonic scale. Each row gets a
// random note with a 70% probability of being filled.

let penta = scale("C", "pentatonic", 4);

for row in 0..num_rows {
  // 70% chance of placing a note
  if random(0, 100) < 70 {
    let n = random_note(penta);
    set_note(row, 0, n);
  }
}
"#;

/// "Arpeggiator" — cycle through chord tones across rows.
const ARPEGGIATOR: &str = r#"// Arpeggiator — cycle through chord tones
//
// Creates an arpeggiated pattern by cycling through
// the notes of a chord. Try changing the root, quality,
// or octave to hear different arpeggios.

let root = "C";
let quality = "maj7";  // try: major, minor, 7th, dim, aug
let octave = 4;

let tones = chord(root, quality, octave);
let len = tones.len();

for row in 0..num_rows {
  let idx = row % len;
  set_note(row, 0, tones[idx]);
}
"#;

/// "Probability Beat" — notes placed with random probability.
const PROBABILITY_BEAT: &str = r#"// Probability Beat — stochastic rhythm generator
//
// Each channel has a different probability of triggering
// a note on any given row. This creates organic, evolving
// rhythms that change every time you run the script.

let kick  = note("C", 2);
let snare = note("D", 2);
let hat   = note("F#", 3);
let perc  = note("A", 3);

for row in 0..num_rows {
  // Kick: 30% probability
  if random(0, 100) < 30 {
    set_note(row, 0, kick);
  }
  // Snare: 20% probability
  if random(0, 100) < 20 {
    set_note(row, 1, snare);
  }
  // Hi-hat: 50% probability
  if random(0, 100) < 50 {
    set_note(row, 2, hat);
  }
  // Percussion: 15% probability
  if random(0, 100) < 15 {
    set_note(row, 3, perc);
  }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_not_empty() {
        assert!(!TEMPLATES.is_empty());
    }

    #[test]
    fn test_template_count() {
        assert_eq!(TEMPLATES.len(), 4);
    }

    #[test]
    fn test_all_templates_have_names() {
        for t in TEMPLATES {
            assert!(!t.name.is_empty(), "Template has empty name");
        }
    }

    #[test]
    fn test_all_templates_have_descriptions() {
        for t in TEMPLATES {
            assert!(
                !t.description.is_empty(),
                "Template '{}' has empty description",
                t.name
            );
        }
    }

    #[test]
    fn test_all_templates_have_code() {
        for t in TEMPLATES {
            assert!(!t.code.is_empty(), "Template '{}' has empty code", t.name);
            assert!(
                t.code.contains("//"),
                "Template '{}' should have comments",
                t.name
            );
        }
    }

    #[test]
    fn test_template_names_are_unique() {
        let names: Vec<&str> = TEMPLATES.iter().map(|t| t.name).collect();
        for (i, name) in names.iter().enumerate() {
            for (j, other) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(name, other, "Duplicate template name: {}", name);
                }
            }
        }
    }

    #[test]
    fn test_simple_beat_template() {
        let t = &TEMPLATES[0];
        assert_eq!(t.name, "Simple Beat");
        assert!(t.code.contains("euclidean"));
        assert!(t.code.contains("generate_beat"));
    }

    #[test]
    fn test_random_melody_template() {
        let t = &TEMPLATES[1];
        assert_eq!(t.name, "Random Melody");
        assert!(t.code.contains("scale"));
        assert!(t.code.contains("random_note"));
    }

    #[test]
    fn test_arpeggiator_template() {
        let t = &TEMPLATES[2];
        assert_eq!(t.name, "Arpeggiator");
        assert!(t.code.contains("chord"));
        assert!(t.code.contains("set_note"));
    }

    #[test]
    fn test_probability_beat_template() {
        let t = &TEMPLATES[3];
        assert_eq!(t.name, "Probability Beat");
        assert!(t.code.contains("random"));
        assert!(t.code.contains("set_note"));
    }
}
