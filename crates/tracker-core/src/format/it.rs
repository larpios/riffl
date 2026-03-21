use super::FormatData;

pub fn import_it(data: &[u8]) -> Result<FormatData, String> {
    let module =
        xmrs::module::Module::load_it(data).map_err(|e| format!("IT parse error: {:?}", e))?;
    super::convert_xmrs_module(module)
}
