use super::FormatData;

pub fn import_s3m(data: &[u8]) -> Result<FormatData, String> {
    let module =
        xmrs::module::Module::load_s3m(data).map_err(|e| format!("S3M parse error: {:?}", e))?;
    super::convert_xmrs_module(module)
}
