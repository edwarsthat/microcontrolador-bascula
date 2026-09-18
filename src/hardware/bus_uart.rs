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
    /// TX=GPIO17, RX=GPIO16. 9600 8N1: lo que traen casi todas las basculas
    /// de fabrica. Sin control de flujo: el MAX3232 solo tiene TX/RX.
    pub fn new<UART: Uart + 'static>(
        uart: UART,
        tx: impl OutputPin + 'static,
        rx: impl InputPin + 'static,
    ) -> Result<Self, EspError> {
        let cfg = Config::new()
            .baudrate(9600.Hz())
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

    /// Prueba de lazo: manda `msg` y espera recibirlo de vuelta.
    /// Solo tiene sentido con TX puenteado a RX (etapas 1 y 2).
    pub fn loopback(&self, msg: &[u8]) -> bool {
        let _ = self.driver.clear_rx();
        if let Err(e) = self.driver.write(msg) {
            log::error!("UART: fallo al escribir: {e}");
            return false;
        }
        let mut buf = [0u8; 64];
        let timeout = TickType::new_millis(200).ticks();
        match self.driver.read(&mut buf, timeout) {
            Ok(n) if &buf[..n] == msg => {
                log::info!("UART: loopback OK ({n} bytes)");
                true
            }
            Ok(n) => {
                log::warn!(
                    "UART: loopback recibio {:?} (esperaba {:?})",
                    &buf[..n],
                    msg
                );
                false
            }
            Err(e) => {
                log::warn!("UART: loopback sin respuesta: {e}");
                false
            }
        }
    }

    /// Vuelca crudo lo que llegue durante `segundos`, en hex y ASCII.
    /// Para ver que manda la bascula antes de escribir el parser (etapa 3).
    pub fn espiar(&self, segundos: u64) {
        let mut buf = [0u8; 128];
        let timeout = TickType::new_millis(500).ticks();
        let fin = std::time::Instant::now() + std::time::Duration::from_secs(segundos);
        let mut total = 0usize;
        while std::time::Instant::now() < fin {
            match self.driver.read(&mut buf, timeout) {
                Ok(n) if n > 0 => {
                    total += n;
                    let ascii: String = buf[..n]
                        .iter()
                        .map(|&b| {
                            if b.is_ascii_graphic() || b == b' ' {
                                b as char
                            } else {
                                '.'
                            }
                        })
                        .collect();
                    log::info!("UART rx {n:3} | {} | {ascii}", hex::encode(&buf[..n]));
                }
                Ok(_) => {}
                Err(e) => log::warn!("UART: error leyendo: {e}"),
            }
        }
        if total == 0 {
            log::warn!("UART: la bascula no mando nada en {segundos}s");
        }
    }
}
