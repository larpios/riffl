/// Song and arrangement data model for the tracker.
///
/// A Song is the top-level container that holds a pool of patterns,
/// an arrangement (ordered sequence of pattern indices), global track
/// metadata, and instrument definitions.
use serde::{Deserialize, Serialize};

use crate::pattern::{Note, Pattern, Pitch, Track};

/// A point in an envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopePoint {
    /// Frame or tick number where this point occurs.
    pub frame: u16,
    /// Envelope value at this point. Typically 0.0 to 1.0 for volume/panning, or -1.0 to 1.0 for pitch.
    pub value: f32,
}

/// An envelope describing how a parameter changes over time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Envelope {
    /// Ordered list of points forming the envelope.
    pub points: Vec<EnvelopePoint>,
    /// Whether the envelope is enabled.
    pub enabled: bool,
    /// Sustain point index (loop while key is held).
    pub sustain_enabled: bool,
    pub sustain_start_point: usize,
    pub sustain_end_point: usize,
    /// Loop segment (loop while key is held or after if no sustain exists).
    pub loop_enabled: bool,
    pub loop_start_point: usize,
    pub loop_end_point: usize,
}

/// ADSR (Attack, Decay, Sustain, Release) envelope parameters.
/// Times are in milliseconds, sustain level is 0.0 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Adsr {
    /// Attack time in milliseconds (0 to max).
    pub attack: f32,
    /// Decay time in milliseconds.
    pub decay: f32,
    /// Sustain level (0.0 to 1.0).
    pub sustain: f32,
    /// Release time in milliseconds.
    pub release: f32,
}

impl Adsr {
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack: attack.max(0.0),
            decay: decay.max(0.0),
            sustain: sustain.clamp(0.0, 1.0),
            release: release.max(0.0),
        }
    }
}

/// LFO (Low Frequency Oscillator) waveform types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LfoWaveform {
    #[default]
    Sine,
    Triangle,
    Square,
    Sawtooth,
    ReverseSaw,
    Random,
}

/// An LFO (Low Frequency Oscillator) for modulating parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lfo {
    /// Waveform shape of the LFO.
    pub waveform: LfoWaveform,
    /// Rate in Hz (cycles per second).
    pub rate: f32,
    /// Depth/intensity of the modulation (0.0 to 1.0).
    pub depth: f32,
    /// DC offset added to the LFO output (-1.0 to 1.0).
    pub offset: f32,
    /// Whether the LFO is enabled.
    pub enabled: bool,
    /// Phase offset for synchronization (0.0 to 1.0).
    pub phase: f32,
}

impl Default for Lfo {
    fn default() -> Self {
        Self {
            waveform: LfoWaveform::Sine,
            rate: 0.0,
            depth: 0.0,
            offset: 0.0,
            enabled: true,
            phase: 0.0,
        }
    }
}

impl Lfo {
    pub fn new(waveform: LfoWaveform, rate: f32, depth: f32, offset: f32) -> Self {
        Self {
            waveform,
            rate: rate.max(0.0),
            depth: depth.clamp(0.0, 1.0),
            offset: offset.clamp(-1.0, 1.0),
            enabled: true,
            phase: 0.0,
        }
    }

    pub fn sine(rate: f32, depth: f32) -> Self {
        Self::new(LfoWaveform::Sine, rate, depth, 0.0)
    }

    pub fn triangle(rate: f32, depth: f32) -> Self {
        Self::new(LfoWaveform::Triangle, rate, depth, 0.0)
    }

    pub fn square(rate: f32, depth: f32) -> Self {
        Self::new(LfoWaveform::Square, rate, depth, 0.0)
    }

    pub fn sawtooth(rate: f32, depth: f32) -> Self {
        Self::new(LfoWaveform::Sawtooth, rate, depth, 0.0)
    }
}

impl Envelope {
    /// Evaluates the envelope value at the given tick, handling sustain and loop points.
    /// Returns a tuple of `(value, next_tick)`.
    pub fn evaluate(&self, tick: usize, key_on: bool) -> (f32, usize) {
        if !self.enabled || self.points.is_empty() {
            return (1.0, tick + 1);
        }

        // Calculate next tick based on loop points
        let mut next_tick = tick + 1;

        if self.sustain_enabled && key_on {
            let sus_start = self
                .points
                .get(self.sustain_start_point)
                .map(|p| p.frame)
                .unwrap_or(0);
            let sus_end = self
                .points
                .get(self.sustain_end_point)
                .map(|p| p.frame)
                .unwrap_or(0);
            if next_tick > sus_end as usize {
                next_tick = sus_start as usize;
            }
        } else if self.loop_enabled {
            let loop_start = self
                .points
                .get(self.loop_start_point)
                .map(|p| p.frame)
                .unwrap_or(0);
            let loop_end = self
                .points
                .get(self.loop_end_point)
                .map(|p| p.frame)
                .unwrap_or(0);
            if next_tick > loop_end as usize {
                next_tick = loop_start as usize;
            }
        }

        // Clamp to end
        let last_point = self.points.last().unwrap();
        if tick >= last_point.frame as usize {
            return (last_point.value, next_tick.min(tick)); // stick to the last frame
        }

        // Find the segment we are in
        let mut value = 0.0;
        for i in 0..self.points.len() - 1 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];
            if tick >= p1.frame as usize && tick < p2.frame as usize {
                let range = (p2.frame - p1.frame) as f32;
                let fraction = (tick - p1.frame as usize) as f32 / range;
                value = p1.value + (p2.value - p1.value) * fraction;
                break;
            }
        }

        (value, next_tick)
    }
}

/// A keyzone maps a MIDI note and velocity range to a specific sample.
///
/// Multi-sample instruments use multiple keyzones to select different samples
/// based on the incoming note pitch and velocity. Keyzones may overlap;
/// the first matching zone (sorted by note_min) wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyzone {
    /// Minimum MIDI note number for this zone (inclusive, 0-119).
    pub note_min: u8,
    /// Maximum MIDI note number for this zone (inclusive, 0-119).
    pub note_max: u8,
    /// Minimum velocity for this zone (inclusive, 0-127).
    pub velocity_min: u8,
    /// Maximum velocity for this zone (inclusive, 0-127).
    pub velocity_max: u8,
    /// Index into the sample pool for this zone.
    pub sample_index: usize,
    /// Base note override for pitch calculation in this zone (MIDI note number).
    /// If `None`, uses the sample's own base_note.
    pub base_note_override: Option<u8>,
}

impl Keyzone {
    /// Create a new keyzone spanning the full note and velocity range.
    pub fn new(sample_index: usize) -> Self {
        Self {
            note_min: 0,
            note_max: 119,
            velocity_min: 0,
            velocity_max: 127,
            sample_index,
            base_note_override: None,
        }
    }

    /// Create a keyzone with a specific note range.
    pub fn with_note_range(mut self, min: u8, max: u8) -> Self {
        self.note_min = min.min(119);
        self.note_max = max.min(119);
        self
    }

    /// Create a keyzone with a specific velocity range.
    pub fn with_velocity_range(mut self, min: u8, max: u8) -> Self {
        self.velocity_min = min.min(127);
        self.velocity_max = max.min(127);
        self
    }

    /// Set a base note override for this zone.
    pub fn with_base_note(mut self, base_note: u8) -> Self {
        self.base_note_override = Some(base_note);
        self
    }

    /// Check whether a given MIDI note and velocity fall within this keyzone.
    pub fn matches(&self, midi_note: u8, velocity: u8) -> bool {
        midi_note >= self.note_min
            && midi_note <= self.note_max
            && velocity >= self.velocity_min
            && velocity <= self.velocity_max
    }
}

/// An instrument definition linking a name to a sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    /// Display name for this instrument.
    pub name: String,
    /// Index into the sample pool, if a sample is assigned.
    pub sample_index: Option<usize>,
    /// File path to the audio sample, relative to the project or absolute.
    pub sample_path: Option<String>,
    /// Base note for sample pitch mapping (default C-4).
    pub base_note: Note,
    /// Volume multiplier for this instrument (0.0 to 1.0).
    pub volume: f32,
    /// Default panning for the instrument (-1.0 to 1.0), if any.
    pub panning: Option<f32>,
    /// Finetune adjustment (-8 to +7).
    /// Each unit is 1/8th of a semitone (12.5 cents).
    pub finetune: i8,
    /// Volume envelope, if any.
    pub volume_envelope: Option<Envelope>,
    /// Panning envelope, if any.
    pub panning_envelope: Option<Envelope>,
    /// Pitch envelope, if any.
    pub pitch_envelope: Option<Envelope>,
    /// ADSR volume envelope parameters.
    pub volume_adsr: Option<Adsr>,
    /// ADSR panning envelope parameters.
    pub panning_adsr: Option<Adsr>,
    /// ADSR pitch envelope parameters.
    pub pitch_adsr: Option<Adsr>,
    /// LFO for volume modulation.
    pub volume_lfo: Option<Lfo>,
    /// LFO for panning modulation.
    pub panning_lfo: Option<Lfo>,
    /// LFO for pitch modulation.
    pub pitch_lfo: Option<Lfo>,
    /// Fadeout speed for the instrument (0-65535).
    /// Subtracted from the fadeout multiplier every tick after Note Off.
    pub fadeout: u16,
    /// Multi-sample keyzones. When non-empty, sample selection uses keyzone
    /// matching instead of `sample_index`. Sorted by `note_min` for lookup.
    #[serde(default)]
    pub keyzones: Vec<Keyzone>,
    /// NSF-specific data for NSF-loaded instruments.
    #[serde(default, skip)]
    #[doc(hidden)]
    pub nsf_data: Option<crate::format::nsf::NsfInstrumentData>,
}

impl Instrument {
    /// Create a new instrument with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sample_index: None,
            sample_path: None,
            base_note: Note::simple(Pitch::C, 4),
            volume: 1.0,
            panning: None,
            finetune: 0,
            volume_envelope: None,
            panning_envelope: None,
            pitch_envelope: None,
            volume_adsr: None,
            panning_adsr: None,
            pitch_adsr: None,
            volume_lfo: None,
            panning_lfo: None,
            pitch_lfo: None,
            fadeout: 0,
            keyzones: Vec::new(),
            nsf_data: None,
        }
    }

    /// Set the finetune value for the instrument.
    pub fn with_finetune(mut self, finetune: i8) -> Self {
        self.finetune = finetune.clamp(-8, 7);
        self
    }

    /// Set the default volume for the instrument (0.0 to 1.0).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Find the matching keyzone for a MIDI note and velocity.
    /// Returns the keyzone reference, or `None` if no zone matches.
    pub fn find_keyzone(&self, midi_note: u8, velocity: u8) -> Option<&Keyzone> {
        self.keyzones
            .iter()
            .find(|kz| kz.matches(midi_note, velocity))
    }

    /// Resolve the sample index for a given note and velocity.
    /// If keyzones are defined and a match is found, returns the keyzone's sample index.
    /// Otherwise falls back to `self.sample_index`.
    pub fn resolve_sample_index(&self, midi_note: u8, velocity: u8) -> Option<usize> {
        if !self.keyzones.is_empty() {
            self.find_keyzone(midi_note, velocity)
                .map(|kz| kz.sample_index)
        } else {
            self.sample_index
        }
    }
}

impl Default for Instrument {
    fn default() -> Self {
        Self::new("Instrument")
    }
}

/// A song containing patterns, arrangement, tracks, and instruments.
///
/// The song is the top-level data structure for a tracker project.
/// Patterns are stored in a pool (up to 256) and referenced by index
/// in the arrangement sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    /// Song title.
    pub name: String,
    /// Artist name.
    pub artist: String,
    /// Tempo in beats per minute.
    pub bpm: f64,
    /// Lines per beat.
    pub lpb: u32,
    /// Ticks per row (line), defines sub-row resolution.
    pub tpl: u32,
    /// Pattern pool (up to 256 patterns).
    pub patterns: Vec<Pattern>,
    /// Arrangement: ordered list of pattern indices forming the song sequence.
    pub arrangement: Vec<usize>,
    /// Global track metadata (volume, pan, mute, solo, instrument).
    pub tracks: Vec<Track>,
    /// Global volume multiplier (0.0 - 1.0).
    pub global_volume: f32,
    /// Instrument definitions linking to samples.
    pub instruments: Vec<Instrument>,
}

/// Maximum number of patterns in the pool.
pub const MAX_PATTERNS: usize = 256;

impl Song {
    /// Create a new empty song with one default pattern.
    pub fn new(name: impl Into<String>, bpm: f64) -> Self {
        let default_pattern = Pattern::default();
        let tracks: Vec<Track> = (1..=default_pattern.num_channels())
            .map(Track::with_number)
            .collect();
        Self {
            name: name.into(),
            artist: String::new(),
            bpm,
            lpb: 4,
            tpl: 6,
            patterns: vec![default_pattern],
            arrangement: vec![0],
            tracks,
            global_volume: 1.0,
            instruments: Vec::new(),
        }
    }

    /// Add a new pattern to the pool.
    ///
    /// Returns the index of the new pattern, or None if the pool is full (256 patterns).
    pub fn add_pattern(&mut self, pattern: Pattern) -> Option<usize> {
        if self.patterns.len() >= MAX_PATTERNS {
            return None;
        }
        let index = self.patterns.len();
        self.patterns.push(pattern);
        Some(index)
    }

    /// Remove a pattern from the pool by index.
    ///
    /// Updates the arrangement to remove references to the deleted pattern
    /// and adjusts indices of patterns that come after it.
    /// Returns false if the index is out of bounds or it's the last pattern.
    pub fn remove_pattern(&mut self, index: usize) -> bool {
        if index >= self.patterns.len() || self.patterns.len() <= 1 {
            return false;
        }
        self.patterns.remove(index);

        // Remove arrangement entries that referenced the deleted pattern
        // and adjust indices for patterns after the removed one
        self.arrangement.retain(|&i| i != index);
        for entry in &mut self.arrangement {
            if *entry > index {
                *entry -= 1;
            }
        }

        // If arrangement is now empty, point to pattern 0
        if self.arrangement.is_empty() {
            self.arrangement.push(0);
        }

        true
    }

    /// Duplicate a pattern and add the copy to the pool.
    ///
    /// Returns the index of the new pattern, or None if the pool is full
    /// or the source index is out of bounds.
    pub fn duplicate_pattern(&mut self, index: usize) -> Option<usize> {
        if index >= self.patterns.len() || self.patterns.len() >= MAX_PATTERNS {
            return None;
        }
        let cloned = self.patterns[index].clone();
        self.add_pattern(cloned)
    }

    /// Reorder an arrangement entry from one position to another.
    ///
    /// Returns false if either position is out of bounds.
    pub fn reorder_arrangement(&mut self, from: usize, to: usize) -> bool {
        if from >= self.arrangement.len() || to >= self.arrangement.len() {
            return false;
        }
        let entry = self.arrangement.remove(from);
        self.arrangement.insert(to, entry);
        true
    }

    /// Insert a pattern reference into the arrangement at the given position.
    ///
    /// Returns false if the pattern_index is out of bounds or position > arrangement length.
    pub fn insert_in_arrangement(&mut self, position: usize, pattern_index: usize) -> bool {
        if pattern_index >= self.patterns.len() || position > self.arrangement.len() {
            return false;
        }
        self.arrangement.insert(position, pattern_index);
        true
    }

    /// Remove an entry from the arrangement at the given position.
    ///
    /// Returns the pattern index that was removed, or None if out of bounds.
    pub fn remove_from_arrangement(&mut self, position: usize) -> Option<usize> {
        if position >= self.arrangement.len() {
            return None;
        }
        Some(self.arrangement.remove(position))
    }
}

impl Default for Song {
    fn default() -> Self {
        Self::new("Untitled", 120.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_song_new() {
        let song = Song::new("Test Song", 140.0);
        assert_eq!(song.name, "Test Song");
        assert_eq!(song.bpm, 140.0);
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.arrangement, vec![0]);
        assert!(!song.tracks.is_empty());
    }

    #[test]
    fn test_song_default() {
        let song = Song::default();
        assert_eq!(song.name, "Untitled");
        assert_eq!(song.bpm, 120.0);
    }

    #[test]
    fn test_instrument_new() {
        let inst = Instrument::new("Kick");
        assert_eq!(inst.name, "Kick");
        assert_eq!(inst.sample_index, None);
        assert_eq!(inst.base_note.pitch, Pitch::C);
        assert_eq!(inst.base_note.octave, 4);
        assert_eq!(inst.volume, 1.0);
    }

    #[test]
    fn test_instrument_default() {
        let inst = Instrument::default();
        assert_eq!(inst.name, "Instrument");
    }

    #[test]
    fn test_add_pattern() {
        let mut song = Song::default();
        assert_eq!(song.patterns.len(), 1);

        let idx = song.add_pattern(Pattern::default());
        assert_eq!(idx, Some(1));
        assert_eq!(song.patterns.len(), 2);
    }

    #[test]
    fn test_add_pattern_pool_full() {
        let mut song = Song::default();
        // Fill up to 256
        for _ in 1..MAX_PATTERNS {
            song.add_pattern(Pattern::default());
        }
        assert_eq!(song.patterns.len(), MAX_PATTERNS);
        assert_eq!(song.add_pattern(Pattern::default()), None);
    }

    #[test]
    fn test_remove_pattern() {
        let mut song = Song::default();
        song.add_pattern(Pattern::new(32, 4));
        song.add_pattern(Pattern::new(16, 4));
        assert_eq!(song.patterns.len(), 3);

        // Arrangement references pattern 0, 1, 2
        song.arrangement = vec![0, 1, 2, 1];

        // Remove pattern 1
        assert!(song.remove_pattern(1));
        assert_eq!(song.patterns.len(), 2);

        // Arrangement should have removed refs to 1 and decremented refs > 1
        assert_eq!(song.arrangement, vec![0, 1]);
    }

    #[test]
    fn test_remove_pattern_last_remaining() {
        let mut song = Song::default();
        // Cannot remove the last pattern
        assert!(!song.remove_pattern(0));
        assert_eq!(song.patterns.len(), 1);
    }

    #[test]
    fn test_remove_pattern_out_of_bounds() {
        let mut song = Song::default();
        assert!(!song.remove_pattern(5));
    }

    #[test]
    fn test_remove_pattern_arrangement_becomes_empty() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        // Arrangement only references pattern 0
        song.arrangement = vec![0];

        // Remove pattern 0 → arrangement loses its only entry → should default to [0]
        assert!(song.remove_pattern(0));
        assert_eq!(song.arrangement, vec![0]);
    }

    #[test]
    fn test_duplicate_pattern() {
        let mut song = Song::default();
        let idx = song.duplicate_pattern(0);
        assert_eq!(idx, Some(1));
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.patterns[1].num_rows(), song.patterns[0].num_rows());
        assert_eq!(
            song.patterns[1].num_channels(),
            song.patterns[0].num_channels()
        );
    }

    #[test]
    fn test_duplicate_pattern_out_of_bounds() {
        let mut song = Song::default();
        assert_eq!(song.duplicate_pattern(5), None);
    }

    #[test]
    fn test_reorder_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0, 1, 2];

        assert!(song.reorder_arrangement(0, 2));
        assert_eq!(song.arrangement, vec![1, 2, 0]);
    }

    #[test]
    fn test_reorder_arrangement_out_of_bounds() {
        let mut song = Song::default();
        song.arrangement = vec![0];
        assert!(!song.reorder_arrangement(0, 5));
        assert!(!song.reorder_arrangement(5, 0));
    }

    #[test]
    fn test_insert_in_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0];

        assert!(song.insert_in_arrangement(1, 1));
        assert_eq!(song.arrangement, vec![0, 1]);

        assert!(song.insert_in_arrangement(0, 1));
        assert_eq!(song.arrangement, vec![1, 0, 1]);
    }

    #[test]
    fn test_insert_in_arrangement_invalid_pattern() {
        let mut song = Song::default();
        assert!(!song.insert_in_arrangement(0, 5)); // pattern 5 doesn't exist
    }

    #[test]
    fn test_insert_in_arrangement_invalid_position() {
        let mut song = Song::default();
        assert!(!song.insert_in_arrangement(10, 0)); // position beyond length
    }

    #[test]
    fn test_remove_from_arrangement() {
        let mut song = Song::default();
        song.add_pattern(Pattern::default());
        song.arrangement = vec![0, 1, 0];

        assert_eq!(song.remove_from_arrangement(1), Some(1));
        assert_eq!(song.arrangement, vec![0, 0]);
    }

    #[test]
    fn test_remove_from_arrangement_out_of_bounds() {
        let mut song = Song::default();
        assert_eq!(song.remove_from_arrangement(5), None);
    }

    #[test]
    fn test_instrument_assignment() {
        let mut song = Song::default();
        let inst = Instrument::new("Bass");
        song.instruments.push(inst);

        assert_eq!(song.instruments.len(), 1);
        assert_eq!(song.instruments[0].name, "Bass");

        song.instruments[0].sample_index = Some(0);
        assert_eq!(song.instruments[0].sample_index, Some(0));
    }

    #[test]
    fn test_song_tracks_match_pattern_channels() {
        let song = Song::default();
        assert_eq!(song.tracks.len(), song.patterns[0].num_channels());
    }

    #[test]
    fn test_adsr_new() {
        let adsr = Adsr::new(10.0, 100.0, 0.7, 200.0);
        assert_eq!(adsr.attack, 10.0);
        assert_eq!(adsr.decay, 100.0);
        assert_eq!(adsr.sustain, 0.7);
        assert_eq!(adsr.release, 200.0);
    }

    #[test]
    fn test_adsr_clamping() {
        let adsr = Adsr::new(-5.0, -10.0, 1.5, -1.0);
        assert_eq!(adsr.attack, 0.0);
        assert_eq!(adsr.decay, 0.0);
        assert_eq!(adsr.sustain, 1.0);
        assert_eq!(adsr.release, 0.0);
    }

    #[test]
    fn test_lfo_default() {
        let lfo = Lfo::default();
        assert_eq!(lfo.waveform, LfoWaveform::Sine);
        assert_eq!(lfo.rate, 0.0);
        assert_eq!(lfo.depth, 0.0);
        assert_eq!(lfo.offset, 0.0);
        assert!(lfo.enabled);
    }

    #[test]
    fn test_lfo_new() {
        let lfo = Lfo::new(LfoWaveform::Triangle, 4.0, 0.5, 0.2);
        assert_eq!(lfo.waveform, LfoWaveform::Triangle);
        assert_eq!(lfo.rate, 4.0);
        assert_eq!(lfo.depth, 0.5);
        assert_eq!(lfo.offset, 0.2);
        assert!(lfo.enabled);
    }

    #[test]
    fn test_lfo_factory_methods() {
        let sine_lfo = Lfo::sine(2.0, 0.8);
        assert_eq!(sine_lfo.waveform, LfoWaveform::Sine);
        assert_eq!(sine_lfo.rate, 2.0);
        assert_eq!(sine_lfo.depth, 0.8);

        let tri_lfo = Lfo::triangle(1.0, 0.5);
        assert_eq!(tri_lfo.waveform, LfoWaveform::Triangle);

        let sq_lfo = Lfo::square(3.0, 0.3);
        assert_eq!(sq_lfo.waveform, LfoWaveform::Square);

        let saw_lfo = Lfo::sawtooth(0.5, 1.0);
        assert_eq!(saw_lfo.waveform, LfoWaveform::Sawtooth);
    }

    #[test]
    fn test_lfo_clamping() {
        let lfo = Lfo::new(LfoWaveform::Sine, -5.0, 1.5, 2.0);
        assert_eq!(lfo.rate, 0.0);
        assert_eq!(lfo.depth, 1.0);
        assert_eq!(lfo.offset, 1.0);
    }

    #[test]
    fn test_instrument_with_lfo() {
        let mut inst = Instrument::new("Lead");
        inst.volume_lfo = Some(Lfo::sine(4.0, 0.5));
        inst.pitch_lfo = Some(Lfo::triangle(2.0, 0.3));
        inst.panning_lfo = Some(Lfo::square(1.0, 0.2));

        assert!(inst.volume_lfo.is_some());
        assert!(inst.pitch_lfo.is_some());
        assert!(inst.panning_lfo.is_some());

        let vol_lfo = inst.volume_lfo.unwrap();
        assert_eq!(vol_lfo.waveform, LfoWaveform::Sine);
        assert_eq!(vol_lfo.rate, 4.0);
        assert_eq!(vol_lfo.depth, 0.5);
    }

    #[test]
    fn test_instrument_with_adsr() {
        let mut inst = Instrument::new("Pad");
        inst.volume_adsr = Some(Adsr::new(5.0, 100.0, 0.8, 300.0));
        inst.pitch_adsr = Some(Adsr::new(10.0, 50.0, 1.0, 100.0));

        assert!(inst.volume_adsr.is_some());
        assert!(inst.pitch_adsr.is_some());

        let vol_adsr = inst.volume_adsr.unwrap();
        assert_eq!(vol_adsr.attack, 5.0);
        assert_eq!(vol_adsr.decay, 100.0);
        assert_eq!(vol_adsr.sustain, 0.8);
        assert_eq!(vol_adsr.release, 300.0);
    }

    #[test]
    fn test_keyzone_new() {
        let kz = Keyzone::new(0);
        assert_eq!(kz.note_min, 0);
        assert_eq!(kz.note_max, 119);
        assert_eq!(kz.velocity_min, 0);
        assert_eq!(kz.velocity_max, 127);
        assert_eq!(kz.sample_index, 0);
        assert_eq!(kz.base_note_override, None);
    }

    #[test]
    fn test_keyzone_with_note_range() {
        let kz = Keyzone::new(1).with_note_range(36, 59);
        assert_eq!(kz.note_min, 36);
        assert_eq!(kz.note_max, 59);
        assert_eq!(kz.sample_index, 1);
    }

    #[test]
    fn test_keyzone_with_velocity_range() {
        let kz = Keyzone::new(2).with_velocity_range(64, 127);
        assert_eq!(kz.velocity_min, 64);
        assert_eq!(kz.velocity_max, 127);
    }

    #[test]
    fn test_keyzone_matches() {
        let kz = Keyzone::new(0)
            .with_note_range(36, 59)
            .with_velocity_range(1, 100);

        assert!(kz.matches(48, 64));
        assert!(kz.matches(36, 1));
        assert!(kz.matches(59, 100));
        assert!(!kz.matches(35, 64)); // below note range
        assert!(!kz.matches(60, 64)); // above note range
        assert!(!kz.matches(48, 0)); // below velocity range
        assert!(!kz.matches(48, 101)); // above velocity range
    }

    #[test]
    fn test_keyzone_with_base_note() {
        let kz = Keyzone::new(0).with_base_note(60);
        assert_eq!(kz.base_note_override, Some(60));
    }

    #[test]
    fn test_instrument_find_keyzone() {
        let mut inst = Instrument::new("Piano");
        inst.keyzones = vec![
            Keyzone::new(0).with_note_range(0, 47),
            Keyzone::new(1).with_note_range(48, 71),
            Keyzone::new(2).with_note_range(72, 119),
        ];

        let kz = inst.find_keyzone(48, 100);
        assert!(kz.is_some());
        assert_eq!(kz.unwrap().sample_index, 1);

        let kz = inst.find_keyzone(30, 100);
        assert_eq!(kz.unwrap().sample_index, 0);

        let kz = inst.find_keyzone(90, 100);
        assert_eq!(kz.unwrap().sample_index, 2);
    }

    #[test]
    fn test_instrument_find_keyzone_velocity_layers() {
        let mut inst = Instrument::new("Drums");
        inst.keyzones = vec![
            Keyzone::new(0)
                .with_note_range(36, 36)
                .with_velocity_range(0, 63),
            Keyzone::new(1)
                .with_note_range(36, 36)
                .with_velocity_range(64, 127),
        ];

        assert_eq!(inst.find_keyzone(36, 32).unwrap().sample_index, 0);
        assert_eq!(inst.find_keyzone(36, 100).unwrap().sample_index, 1);
        assert!(inst.find_keyzone(37, 100).is_none());
    }

    #[test]
    fn test_instrument_resolve_sample_index_with_keyzones() {
        let mut inst = Instrument::new("Multi");
        inst.sample_index = Some(99);
        inst.keyzones = vec![
            Keyzone::new(5).with_note_range(0, 59),
            Keyzone::new(6).with_note_range(60, 119),
        ];

        assert_eq!(inst.resolve_sample_index(48, 100), Some(5));
        assert_eq!(inst.resolve_sample_index(72, 100), Some(6));
    }

    #[test]
    fn test_instrument_resolve_sample_index_fallback() {
        let mut inst = Instrument::new("Single");
        inst.sample_index = Some(3);

        assert_eq!(inst.resolve_sample_index(48, 100), Some(3));
    }

    #[test]
    fn test_instrument_resolve_sample_index_no_match() {
        let mut inst = Instrument::new("Sparse");
        inst.sample_index = Some(3);
        inst.keyzones = vec![Keyzone::new(0).with_note_range(60, 72)];

        // Note 48 doesn't match any keyzone
        assert_eq!(inst.resolve_sample_index(48, 100), None);
    }

    #[test]
    fn test_keyzone_serde_roundtrip() {
        let kz = Keyzone::new(5)
            .with_note_range(36, 59)
            .with_velocity_range(32, 96)
            .with_base_note(48);

        let json = serde_json::to_string(&kz).unwrap();
        let restored: Keyzone = serde_json::from_str(&json).unwrap();
        assert_eq!(kz, restored);
    }

    #[test]
    fn test_instrument_keyzones_serde_roundtrip() {
        let mut inst = Instrument::new("Multi");
        inst.keyzones = vec![
            Keyzone::new(0).with_note_range(0, 59),
            Keyzone::new(1).with_note_range(60, 119),
        ];

        let json = serde_json::to_string(&inst).unwrap();
        let restored: Instrument = serde_json::from_str(&json).unwrap();
        assert_eq!(inst.keyzones, restored.keyzones);
    }

    #[test]
    fn test_instrument_empty_keyzones_serde() {
        let inst = Instrument::new("NoZones");
        let json = serde_json::to_string(&inst).unwrap();
        let restored: Instrument = serde_json::from_str(&json).unwrap();
        assert!(restored.keyzones.is_empty());
    }
}
