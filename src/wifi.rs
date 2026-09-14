use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::WifiModemPeripheral,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use std::time::Duration;

#[derive(Debug)]
#[allow(dead_code)]
pub enum WifiError {
    InitFailed(esp_idf_svc::sys::EspError),
    ConfigFailed(esp_idf_svc::sys::EspError),
    ConnectionFailed(esp_idf_svc::sys::EspError),
    /// El SSID o el password no caben en los buffers del driver.
    CredencialesInvalidas(&'static str),
    NetworkInterfaceTimeout,
}

impl std::fmt::Display for WifiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WifiError::InitFailed(e) => write!(f, "fallo al inicializar el WiFi: {}", e),
            WifiError::ConfigFailed(e) => write!(f, "fallo al configurar el WiFi: {}", e),
            WifiError::ConnectionFailed(e) => write!(f, "fallo al conectar el WiFi: {}", e),
            WifiError::CredencialesInvalidas(campo) => write!(f, "credencial inválida: {}", campo),
            WifiError::NetworkInterfaceTimeout => {
                write!(f, "timeout esperando que levante la interfaz de red")
            }
        }
    }
}

impl std::error::Error for WifiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WifiError::InitFailed(e)
            | WifiError::ConfigFailed(e)
            | WifiError::ConnectionFailed(e) => Some(e),
            _ => None,
        }
    }
}

const SSID: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_SSID"));
const PASSWORD: &str = include_str!(concat!(env!("OUT_DIR"), "/WIFI_PASSWORD"));

/// Espera del primer reintento; se duplica en cada fallo hasta `ESPERA_MAX`.
const ESPERA_INICIAL: Duration = Duration::from_secs(1);
const ESPERA_MAX: Duration = Duration::from_secs(60);
/// Pausa entre el `stop()` y el `start()` al reiniciar el driver.
const ESPERA_REINICIO: Duration = Duration::from_secs(1);
/// Cada cuántos intentos fallidos se reinicia el driver completo.
const INTENTOS_POR_REINICIO: u32 = 10;
/// Reintentos suaves de `reconectar` antes de escalar a reiniciar el driver.
const INTENTOS_RECONEXION: u32 = 3;

type Wifi = BlockingWifi<EspWifi<'static>>;

/// Carga el SSID/password en el driver. Se vuelve a llamar tras cada reinicio
/// del driver para no depender de que la configuración sobreviva al `stop()`.
fn configurar(wifi: &mut Wifi) -> Result<(), WifiError> {
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().map_err(|_| {
            log::error!("SSID inválido (máximo 32 caracteres)");
            WifiError::CredencialesInvalidas("SSID")
        })?,
        password: PASSWORD.try_into().map_err(|_| {
            log::error!("Password inválido (máximo 64 caracteres)");
            WifiError::CredencialesInvalidas("password")
        })?,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))
    .map_err(WifiError::ConfigFailed)
}

/// Un intento de asociarse al AP y esperar a que la interfaz tenga IP.
fn intentar_conectar(wifi: &mut Wifi) -> Result<(), WifiError> {
    wifi.connect().map_err(WifiError::ConnectionFailed)?;
    wifi.wait_netif_up().map_err(|e| {
        log::warn!("La interfaz de red no levantó: {:?}", e);
        WifiError::NetworkInterfaceTimeout
    })
}

/// Reinicio duro del driver: última carta cuando los reintentos suaves no bastan.
fn reiniciar_driver(wifi: &mut Wifi) -> Result<(), WifiError> {
    log::warn!("Reiniciando el driver WiFi...");
    let _ = wifi.stop();
    std::thread::sleep(ESPERA_REINICIO);
    configurar(wifi)?;
    wifi.start().map_err(|e| {
        log::error!("No se pudo reiniciar el driver WiFi: {:?}", e);
        WifiError::ConnectionFailed(e)
    })
}

/// Inicializa el WiFi y reintenta indefinidamente con backoff exponencial
/// (1s → 60s), reiniciando el driver cada `INTENTOS_POR_REINICIO` fallos.
/// Sólo retorna `Err` si la configuración o el reinicio del driver fallan.
pub fn connect_with_retry(
    modem: impl WifiModemPeripheral + 'static,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Result<Wifi, WifiError> {
    let esp_wifi =
        EspWifi::new(modem, sysloop.clone(), Some(nvs)).map_err(WifiError::InitFailed)?;

    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop).map_err(WifiError::InitFailed)?;

    configurar(&mut wifi)?;

    wifi.start().map_err(WifiError::ConnectionFailed)?;
    log::info!("WiFi iniciado, conectando a '{}'...", SSID);

    match wifi.scan() {
        Ok(aps) => {
            log::info!("--- {} APs encontrados ---", aps.len());
            for ap in aps {
                log::info!(
                    "SSID={:?} ch={} rssi={} auth={:?}",
                    ap.ssid,
                    ap.channel,
                    ap.signal_strength,
                    ap.auth_method
                );
            }
        }
        Err(e) => log::error!("Scan fallido: {:?}", e),
    }

    let mut intento: u32 = 0;
    let mut espera = ESPERA_INICIAL;

    loop {
        intento += 1;

        match intentar_conectar(&mut wifi) {
            Ok(()) => {
                log::info!("WiFi conectado en el intento {}", intento);
                return Ok(wifi);
            }
            Err(e) => log::warn!("Intento {} fallido: {}", intento, e),
        }

        let _ = wifi.disconnect();

        if intento % INTENTOS_POR_REINICIO == 0 {
            log::warn!("{} intentos fallidos seguidos", intento);
            reiniciar_driver(&mut wifi)?;
        }

        log::info!("Reintentando en {:?}", espera);
        std::thread::sleep(espera);
        espera = (espera * 2).min(ESPERA_MAX);
    }
}

/// Reconecta un WiFi ya inicializado. Hace `INTENTOS_RECONEXION` intentos
/// suaves con backoff y, si no basta, reinicia el driver y prueba una vez más.
pub fn reconectar(wifi: &mut Wifi) -> Result<(), WifiError> {
    log::warn!("Intentando reconectar WiFi...");
    let _ = wifi.disconnect();

    let mut espera = ESPERA_INICIAL;

    for intento in 1..=INTENTOS_RECONEXION {
        match intentar_conectar(wifi) {
            Ok(()) => {
                log::info!("WiFi reconectado en el intento {}", intento);
                return Ok(());
            }
            Err(e) => log::warn!(
                "Reconexión {}/{} fallida: {}",
                intento,
                INTENTOS_RECONEXION,
                e
            ),
        }

        let _ = wifi.disconnect();
        std::thread::sleep(espera);
        espera = (espera * 2).min(ESPERA_MAX);
    }

    reiniciar_driver(wifi)?;

    match intentar_conectar(wifi) {
        Ok(()) => {
            log::info!("WiFi reconectado tras reiniciar el driver");
            Ok(())
        }
        Err(e) => {
            log::error!("Reconexión fallida tras reiniciar el driver: {}", e);
            Err(e)
        }
    }
}
