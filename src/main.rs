use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::WifiDeviceId;
use std::time::Duration;

mod estado;
mod hardware;
mod http;
mod mensajes;
mod ui;
mod wifi;

use hardware::{bus_i2c::BusI2c, pantalla::Pantalla};
use ui::arranque::{Arranque, Estado};

fn main() {
    // It is necessary to call this function once. Otherwise, some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando Aplicacion");

    let peripherals = Peripherals::take().expect("Fallo al obtener los perifericos");
    let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");

    // Antes del WiFi: `peripherals.modem` se mueve al conectar.
    let mut pantalla = match BusI2c::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    ) {
        Ok(mut bus) => {
            bus.escanear();
            Pantalla::new(Some(bus.into_driver()))
        }
        Err(e) => {
            log::error!("I2C0 no inicializo: {e}. Se continua sin bus I2C");
            Pantalla::new(None)
        }
    };

    let mut arranque = Arranque {
        wifi: Estado::EnCurso,
        ..Default::default()
    };
    arranque.mostrar(&mut pantalla);

    let mut wifi = match wifi::connect_with_retry(peripherals.modem, sysloop, nvs.clone()) {
        Ok(w) => w,
        Err(err) => {
            log::error!("No se pudo establecer conexion WiFi: {:?}", err);
            arranque.wifi = Estado::Fallo;
            arranque.mostrar(&mut pantalla);
            std::thread::sleep(Duration::from_secs(3));
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    arranque.wifi = Estado::Ok;
    arranque.servidor = Estado::EnCurso;
    arranque.mostrar(&mut pantalla);

    let mut cliente = match http::ClienteServidor::new() {
        Ok(c) => c,
        Err(e) => {
            log::error!("No se pudo crear el cliente HTTP: {}", e);
            unsafe { esp_idf_svc::sys::esp_restart() };
        }
    };

    let sesion = estado::siguiente_sesion(nvs.clone()).unwrap_or(0);
    let mac = wifi.wifi().get_mac(WifiDeviceId::Sta).unwrap_or([0; 6]);

    let msg = mensajes::Arranque {
        device_id: format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        ),
        sesion_arranque: sesion,
        monotonico_ancla_us: unsafe { esp_idf_svc::sys::esp_timer_get_time() },
        version_firmware: env!("CARGO_PKG_VERSION"),
    };
    let body = serde_json::to_string(&msg).expect("serializar arranque");
    arranque.servidor = match cliente.post_json(http::RUTA_ARRANQUE, &body) {
        Ok(status) => {
            log::info!("Arranque reportado ({})", status);
            Estado::Ok
        }
        Err(e) => {
            log::error!("No se pudo reportar el arranque: {}", e);
            Estado::Fallo
        }
    };
    arranque.mostrar(&mut pantalla);
    std::thread::sleep(Duration::from_secs(2)); // que el resultado se alcance a leer
    ui::listo::mostrar(
        &mut pantalla,
        &msg.device_id,
        arranque.servidor == Estado::Ok,
    );

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
