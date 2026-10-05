use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod arranque;
mod config;
mod hardware;
mod mensajes;
mod nvs;
mod pesaje;
mod red;
mod ui;


fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Iniciando Aplicacion");

    let peripherals = Peripherals::take().expect("Fallo al obtener los perifericos");

    // ---- PRUEBA EN PLANTA: solo UART de la bascula ----
    let uart = hardware::bus_uart::BusUart::new(
        peripherals.uart2,
        peripherals.pins.gpio17, // TX
        peripherals.pins.gpio16, // RX
    )
    .expect("UART2 no inicializo");
loop {
    //uart.loopback(b"hola bascula\r\n");
    //uart.sondear();
    uart.espiar(10);
}
    // ---- fin prueba ----

    // let sysloop = EspSystemEventLoop::take().expect("Fallo al obtener el sistema de eventos");
    // let nvs = EspDefaultNvsPartition::take().expect("Fallo al obtener la particion NVS");
    // let mut sistema = arranque::iniciar(peripherals, sysloop, nvs);
    // pesaje::Pesaje::new().correr(&mut sistema);
}



