//! Built-in example scripts for the code editor template menu.
//!
//! Each template demonstrates a different DSL capability and includes
//! comments explaining what it does. Accessible via `Ctrl+T` in the
//! code editor.

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
    Template {
        name: "Shuffle & Humanize",
        description: "Randomise note order and add velocity variation",
        code: SHUFFLE_HUMANIZE,
    },
    Template {
        name: "Walking Bass",
        description: "Ascending/descending scale run across channels",
        code: WALKING_BASS,
    },
    Template {
        name: "Zxx Live Trigger",
        description: "React to Zxx effects during live playback",
        code: ZXX_LIVE_TRIGGER,
    },
    Template {
        name: "Volume Ramp",
        description: "Fade volume in/out across pattern channels",
        code: VOLUME_RAMP,
    },
    Template {
        name: "Tempo Beat",
        description: "Place beat notes aligned to current BPM and ticks-per-line",
        code: TEMPO_BEAT,
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

/// "Shuffle & Humanize" — randomise note order then add velocity variation.
const SHUFFLE_HUMANIZE: &str = r#"// Shuffle & Humanize — randomise rows, then add feel
//
// 1. First builds a simple repeating melody using scale notes.
// 2. shuffle() reorders the notes within each channel while
//    keeping the same rows occupied — instant variation.
// 3. humanize() adds ±12 velocity deviation to make it feel
//    less mechanical.

let mel = scale("C", "minor", 4);
let len = mel.len();

// Fill channel 0 with a simple ascending scale loop
for row in 0..num_rows {
  set_note(row, 0, mel[row % len]);
}

// Now shuffle the note order and add humanisation
shuffle();
humanize(12);
"#;

/// "Walking Bass" — ascending/descending scale run across the pattern.
const WALKING_BASS: &str = r#"// Walking Bass — ascending then descending scale
//
// Creates a classic walking bass-line by stepping through
// a scale, rising and falling across the pattern length.
// Change root/mode/octave to transpose.

let root = "C";
let mode = "dorian";  // try: major, minor, dorian, mixolydian
let oct  = 2;

let tones = scale(root, mode, oct);
let half  = num_rows / 2;

// Ascend for the first half
for row in 0..half {
  let n = tones[row % tones.len()];
  set_note(row, 0, n);
}

// Descend for the second half (reverse the scale index)
for row in half..num_rows {
  let idx = (num_rows - 1 - row) % tones.len();
  let n = tones[idx];
  set_note(row, 0, n);
}
"#;

/// "Zxx Live Trigger" — react to Zxx effects during live playback.
const ZXX_LIVE_TRIGGER: &str = r#"// Zxx Live Trigger — live pattern generation from Z effects
//
// During live mode this script is re-run every time a Zxx
// effect fires in the pattern. zxx_triggers is an array of
// maps: #{ channel: int, param: int }.
//
// Convention:
//   Z00–Z3F  (param  0– 63) → pentatonic, octave 3
//   Z40–Z7F  (param 64–127) → major,      octave 4
//   Z80–ZFF  (param 128–255) → minor,      octave 5
//
// Enable live mode with Ctrl+L, then add a Zxx effect to
// any channel and press play.

for t in zxx_triggers {
  let oct = if t.param < 64 { 3 }
            else if t.param < 128 { 4 }
            else { 5 };

  let mode = if t.param < 64 { "pentatonic" }
             else if t.param < 128 { "major" }
             else { "minor" };

  let tones = scale("C", mode, oct);
  let len   = tones.len();

  for row in 0..num_rows {
    set_note(row, t.channel, tones[row % len]);
  }
}
"#;

/// "Volume Ramp" — fade volume in/out across pattern channels.
const VOLUME_RAMP: &str = r#"// Volume Ramp — fade volume in and out across channels
//
// Uses interpolate_vol() to create smooth volume ramps:
//   - Channel 0: fade in  (0 → 127) over the full pattern
//   - Channel 1: fade out (127 → 0) over the full pattern
//   - Channel 2: swell    (0 → 127 → 0) using two half-ramps
//
// Tip: combine with a beat or melody on the same channels,
// then run this script to add a smooth volume shape on top.

let last = num_rows - 1;
let mid  = num_rows / 2;

// Channel 0: fade in
interpolate_vol(0, 0, last, 0, 127);

// Channel 1: fade out
interpolate_vol(1, 0, last, 127, 0);

// Channel 2: swell (rise then fall)
interpolate_vol(2, 0, mid,  0, 127);
interpolate_vol(2, mid, last, 127, 0);
"#;

/// "Tempo Beat" — place kick/snare/hihat aligned to the live bpm and tpl.
const TEMPO_BEAT: &str = r#"// Tempo Beat — place kick/snare/hi-hat aligned to current song tempo.
//
// `bpm`  — beats per minute (read-only f64)
// `tpl`  — ticks per line, i.e. rows per beat (read-only int)
//
// Channel 0: kick on every beat
// Channel 1: snare on beats 2 and 4 of each 4/4 bar
// Channel 2: closed hi-hat on every half-beat (if tpl is even)

// Kick: every tpl rows
let mut row = 0;
while row < num_rows {
    fill_column(row, 0, "C-5", 64);
    row += tpl;
}

// Snare: beats 2 and 4 per 4-beat bar
let bar_rows = tpl * 4;
let beat2 = tpl;
let beat4 = tpl * 3;
row = 0;
while row < num_rows {
    if row + beat2 < num_rows { fill_column(row + beat2, 1, "D-5", 80); }
    if row + beat4 < num_rows { fill_column(row + beat4, 1, "D-5", 80); }
    row += bar_rows;
}

// Hi-hat: every half-beat (only if tpl >= 2)
let half = tpl / 2;
if half > 0 {
    let mut r = 0;
    while r < num_rows {
        fill_column(r, 2, "F#5", 50);
        r += half;
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
        assert_eq!(TEMPLATES.len(), 9);
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
