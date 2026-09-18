use std::time::Duration;

pub const WIFI_SSID: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_SSID"));
pub const WIFI_PASSWORD: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_PASSWORD"));
pub const API_KEY: &str = include_str!(concat!(env!("OUT_DIR"), "/API_KEY"));

/// Una URL no lleva espacios al borde, asi que `env!` directo sirve.
pub const SERVER_URL: &str = env!("SERVER_URL");

pub const VERSION_FIRMWARE: &str = env!("CARGO_PKG_VERSION");

/// Peso a partir del cual se considera que hay canasta puesta.
pub const PESO_MINIMO_KG: f32 = 10.0;
/// Tiempo minimo entre dos pesajes de la misma tarjeta.
pub const TARJETA_ESPERA: Duration = Duration::from_secs(10 * 60);
