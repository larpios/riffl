use crate::audio::sample::Sample;

/// Result of a successful format import.
pub struct FormatData {
    /// Song structure: patterns, arrangement, instrument definitions.
    pub song: crate::song::Song,
    /// Raw audio data for each instrument slot.
    pub samples: Vec<Sample>,
}

pub mod it;
pub mod protracker;
pub mod s3m;
pub mod xm;
