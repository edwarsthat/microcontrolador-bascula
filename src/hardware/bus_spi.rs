use esp_idf_svc::hal::gpio::{InputPin, OutputPin};
use esp_idf_svc::hal::spi::config::{Config, DriverConfig, MODE_0};
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiDeviceDriver, SpiDriver};
use esp_idf_svc::hal::units::FromValueType;
use esp_idf_svc::sys::EspError;

pub type DispositivoSpi = SpiDeviceDriver<'static, SpiDriver<'static>>;

pub struct BusSpi {
    device: DispositivoSpi,
}

impl BusSpi {
    /// SCK=18, MOSI=23, MISO=19, CS=5. 1 MHz y modo 0: lo que pide el RC522.
    pub fn new<SPI: SpiAnyPins + 'static>(
        spi: SPI,
        sclk: impl OutputPin + 'static,
        mosi: impl OutputPin + 'static,
        miso: impl InputPin + 'static,
        cs: impl OutputPin + 'static,
    ) -> Result<Self, EspError> {
        let device = SpiDeviceDriver::new_single(
            spi,
            sclk,
            mosi,
            Some(miso),
            Some(cs),
            &DriverConfig::default(),
            &Config::new().baudrate(1.MHz().into()).data_mode(MODE_0),
        )?;
        Ok(Self { device })
    }

    pub fn into_device(self) -> DispositivoSpi {
        self.device
    }
}
