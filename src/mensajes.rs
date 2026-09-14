use serde::Serialize;

#[derive(Serialize)]
pub struct Arranque {
    pub device_id: String,
    pub sesion_arranque: i64,
    pub monotonico_ancla_us: i64,
    pub version_firmware: &'static str,
}
