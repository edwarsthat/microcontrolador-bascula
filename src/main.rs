use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod arranque;
mod config;
mod envio;
mod hardware;
mod mensajes;
mod nvs;
mod pesaje;
mod red;
mod registros;
mod ui;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando Aplicacion");

    let peripherals = Peripherals::take().expect("Fallo al obtener los perifericos");

    let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");
    let mut sistema = arranque::iniciar(peripherals, sysloop, nvs);
    pesaje::Pesaje::new().correr(&mut sistema);
}
