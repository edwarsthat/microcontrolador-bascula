use std::time::Duration;

pub const WIFI_SSID: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_SSID"));
pub const WIFI_PASSWORD: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_PASSWORD"));
#[expect(dead_code, reason = "autenticacion con el servidor, pendiente de definir")]
pub const API_KEY: &str = include_str!(concat!(env!("OUT_DIR"), "/API_KEY"));

/// Una URL no lleva espacios al borde, asi que `env!` directo sirve.
pub const SERVER_URL: &str = env!("SERVER_URL");

pub const VERSION_FIRMWARE: &str = env!("CARGO_PKG_VERSION");

/// Peso a partir del cual se considera que hay canasta puesta.
pub const PESO_MINIMO_KG: f32 = 5.0;
/// Tiempo minimo entre dos pesajes de la misma tarjeta.
pub const TARJETA_ESPERA: Duration = Duration::from_secs(10 * 60);

/// Sin ninguna trama valida en este tiempo, la bascula se da por desconectada.
/// Manda una cada ~200 ms, asi que 1 s son ~5 tramas perdidas.
#[cfg(not(feature = "bascula-mock"))]
pub const TIMEOUT_BASCULA: Duration = Duration::from_secs(1);
/// Con canasta estable, cuanto se espera la tarjeta antes de volver a esperar peso.
pub const TIMEOUT_TARJETA: Duration = Duration::from_secs(30);
/// Diferencia maxima entre el peso antes y despues de la tarjeta para darlo por igual.
/// La bascula oscila de a 0.05 kg (capturas/04_peso_alto), asi que dos pasos.
pub const TOLERANCIA_KG: f32 = 0.5;
/// Despues de la tarjeta, cuanto se espera una lectura estable para comparar.
pub const TIMEOUT_VERIFICACION: Duration = Duration::from_secs(3);
/// Debajo de este peso, estable, se da la canasta por retirada. No es 0.00 exacto
/// porque tierra u hojas en el plato dejan la bascula marcando algo.
pub const PESO_CERO_KG: f32 = 1.0;
