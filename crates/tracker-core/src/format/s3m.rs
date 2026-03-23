use super::{FormatData, ModuleLoader};

pub struct S3mLoader;

impl ModuleLoader for S3mLoader {
    fn name(&self) -> &'static str {
        "ScreamTracker III"
    }

    fn extensions(&self) -> &[&str] {
        &["s3m"]
    }

    fn detect(&self, data: &[u8]) -> bool {
        if data.len() < 0x30 {
            return false;
        }
        &data[0x2C..0x30] == b"SCRM"
    }

    fn load(&self, _data: &[u8]) -> Result<FormatData, String> {
        Err("S3M loading not yet implemented".to_string())
    }
}
