use super::FormatData;

pub fn import_s3m(_data: &[u8]) -> Result<FormatData, String> {
    // TODO: Implement native S3M parser (similar to IT/XM parsers)
    Err("S3M import is not yet implemented with the native parser".into())
}
