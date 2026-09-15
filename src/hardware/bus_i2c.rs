use esp_idf_svc::hal::delay::TickType;
use esp_idf_svc::hal::gpio::{Gpio21, Gpio22};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver, I2C0};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

pub struct BusI2c {
    driver: I2cDriver<'static>,
}

impl BusI2c {
    /// SDA = GPIO21, SCL = GPIO22, 400 kHz, pull-ups internos activados.
    pub fn new(
        i2c0: I2C0<'static>,
        sda: Gpio21<'static>,
        scl: Gpio22<'static>,
    ) -> Result<Self, EspError> {
        let cfg = I2cConfig::new()
            .baudrate(400.kHz().into())
            .sda_enable_pullup(true)
            .scl_enable_pullup(true);
        let driver = I2cDriver::new(i2c0, sda, scl, &cfg)?;
        Ok(Self { driver })
    }

    /// Equivalente a `i2cdetect`: un write vacio es START + direccion + STOP.
    /// Timeout finito para que un bus trabado no cuelgue el arranque.
    /// Devuelve las direcciones que contestaron ACK.
    pub fn escanear(&mut self) -> Vec<u8> {
        let timeout = TickType::new_millis(50).ticks();
        let encontrados: Vec<u8> = (0x08u8..=0x77)
            .filter(|&addr| self.driver.write(addr, &[], timeout).is_ok())
            .collect();
        for addr in &encontrados {
            log::info!("I2C: dispositivo en 0x{addr:02X}");
        }
        if encontrados.is_empty() {
            log::warn!("I2C: ningun dispositivo respondio");
        }
        encontrados
    }

    /// Entrega el driver a quien lo va a usar. Cuando el bus se comparta,
    /// esto pasa a devolver un device compartido y nadie mas cambia.
    pub fn into_driver(self) -> I2cDriver<'static> {
        self.driver
    }
}
