//! Encendido de la bascula: levanta cada pieza en orden, muestra el progreso
//! en pantalla y devuelve todo listo para el ciclo de pesaje.

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::WifiDeviceId;
use std::time::Duration;

use crate::hardware::bascula::FuenteBascula;
use crate::hardware::{bus_i2c::BusI2c, lector_nfc::LectorNfc, pantalla::Pantalla};
use crate::ui::pantallas::Pantallas;
use crate::ui::semaforo::Semaforo;
use crate::{config, hardware, mensajes, nvs, red};

/// Estado de un paso del arranque, como se ve en pantalla.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Estado {
    #[default]
    Pendiente,
    EnCurso,
    Ok,
    Fallo,
}

/// Progreso del arranque. `iniciar` lo actualiza paso a paso y lo manda a
/// dibujar; como se ve cada estado lo decide `ui::pantallas`.
#[derive(Clone, Copy, Default)]
pub struct Arranque {
    pub nfc: Estado,
    pub wifi: Estado,
    pub servidor: Estado,
}

/// Todo lo que el ciclo principal necesita, ya inicializado.
pub struct Sistema {
    pub pantallas: Pantallas,
    pub lector: LectorNfc,
    /// Real o mock segun el feature `bascula-mock`; `Pesaje` no distingue.
    pub bascula: Box<dyn FuenteBascula>,
    pub semaforo: Semaforo,
    /// El servidor respondio al arranque.
    pub en_linea: bool,
    /// Compartido con el hilo de envio: hay que pedir el candado con `.lock()`.
    pub registros: crate::registros::Compartidos,
    /// Numero de arranque; con el tiempo monotonico fecha los pesajes.
    pub sesion: i64,
}

pub fn iniciar(
    peripherals: Peripherals,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> Sistema {
    let mut pantallas = match BusI2c::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    ) {
        Ok(mut bus) => {
            bus.escanear();
            Pantallas::new(Pantalla::new(Some(bus.into_driver())))
        }
        Err(e) => {
            log::error!("I2C0 no inicializo: {e}. Se continua sin bus I2C");
            Pantallas::new(Pantalla::new(None))
        }
    };

    let mut arranque = Arranque {
        nfc: Estado::EnCurso,
        ..Default::default()
    };
    pantallas.arranque(&arranque);

    let bus_spi = match hardware::bus_spi::BusSpi::new(
        peripherals.spi3,
        peripherals.pins.gpio18,
        peripherals.pins.gpio23,
        peripherals.pins.gpio19,
        peripherals.pins.gpio5,
    ) {
        Ok(bus) => Some(bus.into_device()),
        Err(e) => {
            log::error!("SPI3 no inicializo: {e}");
            None
        }
    };
    let lector = hardware::lector_nfc::LectorNfc::new(bus_spi, peripherals.pins.gpio27);

    arranque.nfc = if lector.disponible() {
        Estado::Ok
    } else {
        Estado::Fallo
    };

    #[cfg(feature = "bascula-mock")]
    let bascula: Box<dyn FuenteBascula> = {
        log::warn!("Bascula: usando MOCK, no se lee la UART");
        Box::new(hardware::bascula::BasculaMock::new())
    };

    #[cfg(not(feature = "bascula-mock"))]
    let bascula: Box<dyn FuenteBascula> = match hardware::bus_uart::BusUart::new(
        peripherals.uart2,
        peripherals.pins.gpio17,
        peripherals.pins.gpio16,
    ) {
        Ok(u) => Box::new(hardware::bascula::BasculaUart::new(u)),
        Err(e) => {
            log::error!("UART2 no inicializo: {e}. Reiniciando");
            std::thread::sleep(Duration::from_secs(3));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    let mut leds = match hardware::leds::Leds::new(
        peripherals.pins.gpio25,
        peripherals.pins.gpio26,
        peripherals.pins.gpio32,
        peripherals.pins.gpio33,
    ) {
        Ok(l) => Some(l),
        Err(e) => {
            log::error!("LEDs no inicializaron: {e}");
            None
        }
    };
    if let Some(l) = &mut leds {
        l.secuencia(300); // prueba: uno a uno enciende, uno a uno apaga
    }
    let semaforo = Semaforo::new(leds);

    arranque.wifi = Estado::EnCurso;
    pantallas.arranque(&arranque);

    let wifi = match red::wifi::connect_with_retry(peripherals.modem, sysloop, nvs.clone()) {
        Ok(w) => w,
        Err(err) => {
            log::error!("No se pudo establecer conexion WiFi: {:?}", err);
            arranque.wifi = Estado::Fallo;
            pantallas.arranque(&arranque);
            std::thread::sleep(Duration::from_secs(3));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    arranque.wifi = Estado::Ok;
    arranque.servidor = Estado::EnCurso;
    pantallas.arranque(&arranque);

    let mut cliente = match red::http::ClienteServidor::new() {
        Ok(c) => c,
        Err(e) => {
            log::error!("No se pudo crear el cliente HTTP: {}", e);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    let sesion = nvs::siguiente_sesion(nvs.clone()).unwrap_or(0);
    let registros = match crate::registros::Registros::abrir() {
        Ok(r) => r,
        Err(e) => {
            log::error!("Particion de registros no abrio: {e}. Reiniciando");
            std::thread::sleep(Duration::from_secs(3));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };
    log::info!(
        "Sesion de arranque {sesion}, proxima transaccion {}",
        registros.transaccion_actual()
    );
    let registros = crate::registros::Compartidos::new(registros);

    let mac = wifi.wifi().get_mac(WifiDeviceId::Sta).unwrap_or([0; 6]);

    let msg = mensajes::Arranque {
        device_id: format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        ),
        sesion_arranque: sesion,
        monotonico_ancla_us: unsafe { esp_idf_svc::sys::esp_timer_get_time() },
        version_firmware: config::VERSION_FIRMWARE,
    };
    let body = serde_json::to_string(&msg).expect("serializar arranque");
    arranque.servidor = match cliente.post_json(red::http::RUTA_ARRANQUE, &body) {
        Ok(status) => {
            log::info!("Arranque reportado ({})", status);
            Estado::Ok
        }
        Err(e) => {
            log::error!("No se pudo reportar el arranque: {}", e);
            Estado::Fallo
        }
    };
    pantallas.arranque(&arranque);
    std::thread::sleep(Duration::from_secs(2)); // que el resultado se alcance a leer

    crate::envio::lanzar(registros.clone(), wifi, msg.device_id);

    Sistema {
        pantallas,
        lector,
        bascula,
        semaforo,
        en_linea: arranque.servidor == Estado::Ok,
        registros,
        sesion,
    }
}
