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

/// Trait for module loaders.
pub trait ModuleLoader {
    /// Human-readable name of the format (e.g. "FastTracker II").
    fn name(&self) -> &'static str;

    /// File extensions associated with this format (e.g. ["xm"]).
    fn extensions(&self) -> &[&str];

    /// Check if the data looks like this format (e.g. magic bytes).
    fn detect(&self, data: &[u8]) -> bool;

    /// Parse the data into a FormatData structure.
    fn load(&self, data: &[u8]) -> Result<FormatData, String>;
}

/// List of all supported loaders.
pub fn get_loaders() -> Vec<Box<dyn ModuleLoader>> {
    vec![
        Box::new(xm::XmLoader),
        Box::new(it::ItLoader),
        Box::new(s3m::S3mLoader),
        Box::new(protracker::ModLoader),
    ]
}

/// Attempt to load a module from raw data.
///
/// This tries to detect the format by magic bytes first.
/// If that fails, it tries all loaders.
pub fn load(data: &[u8]) -> Result<FormatData, String> {
    let loaders = get_loaders();

    // 1. Try to detect by content
    for loader in &loaders {
        if loader.detect(data) {
            return loader.load(data);
        }
    }

    Err("Unknown or unsupported format".to_string())
}
