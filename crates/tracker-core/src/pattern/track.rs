/// Track metadata for multi-track support.
///
/// Each track corresponds to a channel in the pattern and holds
/// metadata such as name, volume, pan, mute/solo state, and
/// an optional instrument assignment.
use serde::{Deserialize, Serialize};

/// Default track volume (full volume).
pub const DEFAULT_VOLUME: f32 = 1.0;

/// Default track pan (center).
pub const DEFAULT_PAN: f32 = 0.0;

/// Track metadata for a single channel in a pattern.
///
/// Tracks provide per-channel mixing controls (volume, pan, mute, solo)
/// and an optional instrument assignment. The actual note data lives in
/// the pattern grid; tracks hold the metadata that controls how each
/// channel is mixed into the final output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    /// Display name for this track (e.g., "Kick", "Bass", "Lead").
    pub name: String,
    /// Volume level, 0.0 (silent) to 1.0 (full volume).
    pub volume: f32,
    /// Stereo pan position, -1.0 (full left) to 1.0 (full right).
    pub pan: f32,
    /// Whether this track is muted (produces no audio output).
    pub muted: bool,
    /// Whether this track is soloed (when any track is soloed, only soloed tracks produce audio).
    pub solo: bool,
    /// Optional instrument index assigned to this track.
    pub instrument_index: Option<usize>,
    /// Per-bus send levels (0.0 = no send, 1.0 = full send). Post-fader.
    /// Length may differ from actual bus count; missing entries default to 0.0.
    pub send_levels: Vec<f32>,
}

impl Track {
    /// Create a new track with the given name and default settings.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            volume: DEFAULT_VOLUME,
            pan: DEFAULT_PAN,
            muted: false,
            solo: false,
            instrument_index: None,
            send_levels: Vec::new(),
        }
    }

    /// Create a default track with a numbered name (e.g., "Track 1").
    pub fn with_number(number: usize) -> Self {
        Self::new(format!("Track {}", number))
    }

    /// Set the volume level, clamped to 0.0..=1.0.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    /// Set the pan position, clamped to -1.0..=1.0.
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
    }

    /// Toggle mute state.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Toggle solo state.
    pub fn toggle_solo(&mut self) {
        self.solo = !self.solo;
    }

    /// Get the send level for a specific bus, defaulting to 0.0 if not set.
    pub fn send_level(&self, bus_index: usize) -> f32 {
        self.send_levels.get(bus_index).copied().unwrap_or(0.0)
    }

    /// Set the send level for a specific bus, clamped to 0.0..=1.0.
    /// Extends the send_levels vector if needed.
    pub fn set_send_level(&mut self, bus_index: usize, level: f32) {
        if bus_index >= self.send_levels.len() {
            self.send_levels.resize(bus_index + 1, 0.0);
        }
        self.send_levels[bus_index] = level.clamp(0.0, 1.0);
    }

    /// Check if this track should produce audio given the solo state of all tracks.
    ///
    /// Solo logic: if any track in the set is soloed, only soloed tracks produce audio.
    /// A muted track never produces audio, even if soloed.
    pub fn is_audible(&self, any_soloed: bool) -> bool {
        if self.muted {
            return false;
        }
        if any_soloed {
            return self.solo;
        }
        true
    }
}

impl Default for Track {
    fn default() -> Self {
        Self::new("Track")
    }
}

/// Check if any track in a slice is soloed.
pub fn any_track_soloed(tracks: &[Track]) -> bool {
    tracks.iter().any(|t| t.solo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_new() {
        let track = Track::new("Kick");
        assert_eq!(track.name, "Kick");
        assert_eq!(track.volume, 1.0);
        assert_eq!(track.pan, 0.0);
        assert!(!track.muted);
        assert!(!track.solo);
        assert_eq!(track.instrument_index, None);
        assert!(track.send_levels.is_empty());
    }

    #[test]
    fn test_track_with_number() {
        let track = Track::with_number(3);
        assert_eq!(track.name, "Track 3");
    }

    #[test]
    fn test_track_default() {
        let track = Track::default();
        assert_eq!(track.name, "Track");
        assert_eq!(track.volume, 1.0);
        assert_eq!(track.pan, 0.0);
        assert!(track.send_levels.is_empty());
    }

    #[test]
    fn test_set_volume_clamping() {
        let mut track = Track::new("Test");
        track.set_volume(0.5);
        assert_eq!(track.volume, 0.5);

        track.set_volume(-0.5);
        assert_eq!(track.volume, 0.0);

        track.set_volume(1.5);
        assert_eq!(track.volume, 1.0);

        track.set_volume(0.0);
        assert_eq!(track.volume, 0.0);

        track.set_volume(1.0);
        assert_eq!(track.volume, 1.0);
    }

    #[test]
    fn test_set_pan_clamping() {
        let mut track = Track::new("Test");
        track.set_pan(0.5);
        assert_eq!(track.pan, 0.5);

        track.set_pan(-1.5);
        assert_eq!(track.pan, -1.0);

        track.set_pan(1.5);
        assert_eq!(track.pan, 1.0);

        track.set_pan(-1.0);
        assert_eq!(track.pan, -1.0);

        track.set_pan(1.0);
        assert_eq!(track.pan, 1.0);
    }

    #[test]
    fn test_toggle_mute() {
        let mut track = Track::new("Test");
        assert!(!track.muted);
        track.toggle_mute();
        assert!(track.muted);
        track.toggle_mute();
        assert!(!track.muted);
    }

    #[test]
    fn test_toggle_solo() {
        let mut track = Track::new("Test");
        assert!(!track.solo);
        track.toggle_solo();
        assert!(track.solo);
        track.toggle_solo();
        assert!(!track.solo);
    }

    #[test]
    fn test_is_audible_no_solo() {
        let track = Track::new("Test");
        // No tracks soloed, not muted → audible
        assert!(track.is_audible(false));
    }

    #[test]
    fn test_is_audible_muted() {
        let mut track = Track::new("Test");
        track.muted = true;
        // Muted → not audible regardless of solo state
        assert!(!track.is_audible(false));
        assert!(!track.is_audible(true));
    }

    #[test]
    fn test_is_audible_solo_active_not_soloed() {
        let track = Track::new("Test");
        // Some other track is soloed, this one is not → not audible
        assert!(!track.is_audible(true));
    }

    #[test]
    fn test_is_audible_solo_active_and_soloed() {
        let mut track = Track::new("Test");
        track.solo = true;
        // This track is soloed → audible
        assert!(track.is_audible(true));
    }

    #[test]
    fn test_is_audible_muted_and_soloed() {
        let mut track = Track::new("Test");
        track.solo = true;
        track.muted = true;
        // Muted takes priority over solo → not audible
        assert!(!track.is_audible(true));
    }

    #[test]
    fn test_any_track_soloed() {
        let tracks = vec![Track::new("A"), Track::new("B"), Track::new("C")];
        assert!(!any_track_soloed(&tracks));

        let mut tracks = tracks;
        tracks[1].solo = true;
        assert!(any_track_soloed(&tracks));
    }

    #[test]
    fn test_any_track_soloed_empty() {
        let tracks: Vec<Track> = vec![];
        assert!(!any_track_soloed(&tracks));
    }

    #[test]
    fn test_instrument_index() {
        let mut track = Track::new("Bass");
        assert_eq!(track.instrument_index, None);
        track.instrument_index = Some(3);
        assert_eq!(track.instrument_index, Some(3));
    }

    #[test]
    fn test_multi_track_solo_scenario() {
        // Simulate a real multi-track scenario
        let mut tracks = vec![
            Track::new("Kick"),
            Track::new("Snare"),
            Track::new("Hi-Hat"),
            Track::new("Bass"),
        ];

        // Initially all audible
        let any_solo = any_track_soloed(&tracks);
        assert!(tracks.iter().all(|t| t.is_audible(any_solo)));

        // Solo the bass
        tracks[3].toggle_solo();
        let any_solo = any_track_soloed(&tracks);

        // Only bass should be audible
        assert!(!tracks[0].is_audible(any_solo));
        assert!(!tracks[1].is_audible(any_solo));
        assert!(!tracks[2].is_audible(any_solo));
        assert!(tracks[3].is_audible(any_solo));

        // Also solo the kick
        tracks[0].toggle_solo();
        let any_solo = any_track_soloed(&tracks);

        // Kick and bass should be audible
        assert!(tracks[0].is_audible(any_solo));
        assert!(!tracks[1].is_audible(any_solo));
        assert!(!tracks[2].is_audible(any_solo));
        assert!(tracks[3].is_audible(any_solo));

        // Mute the bass (even though soloed)
        tracks[3].toggle_mute();
        let any_solo = any_track_soloed(&tracks);

        // Only kick audible (bass is muted even though soloed)
        assert!(tracks[0].is_audible(any_solo));
        assert!(!tracks[1].is_audible(any_solo));
        assert!(!tracks[2].is_audible(any_solo));
        assert!(!tracks[3].is_audible(any_solo));
    }

    #[test]
    fn test_track_send_level_default() {
        let track = Track::new("Send");
        assert_eq!(track.send_level(0), 0.0);
        assert!(track.send_levels.is_empty());
    }

    #[test]
    fn test_track_set_send_level() {
        let mut track = Track::new("Send");
        track.set_send_level(0, 0.75);
        assert_eq!(track.send_level(0), 0.75);
    }

    #[test]
    fn test_track_set_send_level_extends() {
        let mut track = Track::new("Send");
        track.set_send_level(3, 0.25);
        assert_eq!(track.send_levels.len(), 4);
        assert_eq!(track.send_level(0), 0.0);
        assert_eq!(track.send_level(3), 0.25);
    }

    #[test]
    fn test_track_set_send_level_clamping() {
        let mut track = Track::new("Send");
        track.set_send_level(0, -0.5);
        assert_eq!(track.send_level(0), 0.0);

        track.set_send_level(0, 1.5);
        assert_eq!(track.send_level(0), 1.0);
    }
}
