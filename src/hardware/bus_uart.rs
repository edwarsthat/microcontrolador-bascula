use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::uart::config::{Config, DataBits, StopBits};
use esp_idf_svc::hal::uart::{Uart, UartDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

pub struct BusUart {
    driver: UartDriver<'static>,
}

impl BusUart {
    /// TX=GPIO17, RX=GPIO16. 1200 8N1: lo que traen casi todas las basculas
    /// de fabrica. Sin control de flujo: el MAX3232 solo tiene TX/RX.
    pub fn new<UART: Uart + 'static>(
        uart: UART,
        tx: impl OutputPin + 'static,
        rx: impl InputPin + 'static,
    ) -> Result<Self, EspError> {
        let cfg = Config::new()
            .baudrate(1200.Hz())
            .data_bits(DataBits::DataBits8)
            .parity_none()
            .stop_bits(StopBits::STOP1);
        let driver = UartDriver::new(
            uart,
            tx,
            rx,
            Option::<esp_idf_svc::hal::gpio::AnyIOPin>::None, // CTS
            Option::<esp_idf_svc::hal::gpio::AnyIOPin>::None, // RTS
            &cfg,
        )?;
        Ok(Self { driver })
    }

    /// Tira todo lo recibido que no se haya leido.
    pub fn descartar_rx(&self) {
        let _ = self.driver.clear_rx();
    }

    /// Lee lo que llegue en hasta `timeout_ms`. Devuelve cuantos bytes quedaron en `buf`
    /// (0 si no llego nada).
    pub fn leer(&self, buf: &mut [u8], timeout_ms: u64) -> usize {
        let ticks = TickType::new_millis(timeout_ms).ticks();
        self.driver.read(buf, ticks).unwrap_or(0)
    }
}
