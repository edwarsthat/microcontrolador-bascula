use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_svc::sys::EspError;

const NS: &str = "bascula";
const KEY_ARRANQUES: &str = "arranques";

pub fn siguiente_sesion(nvs: EspDefaultNvsPartition) -> Result<i64, EspError> {
    let mut store = EspNvs::<NvsDefault>::new(nvs, NS, true)?;
    let actual = store.get_i64(KEY_ARRANQUES)?.unwrap_or(0);
    let siguiente = actual + 1;
    store.set_i64(KEY_ARRANQUES, siguiente)?;
    Ok(siguiente)
}
