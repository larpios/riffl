use super::FormatData;

pub fn import_xm(data: &[u8]) -> Result<FormatData, String> {
    let module =
        xmrs::module::Module::load_xm(data).map_err(|e| format!("XM parse error: {:?}", e))?;
    super::convert_xmrs_module(module)
}
