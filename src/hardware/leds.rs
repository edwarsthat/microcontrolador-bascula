use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use esp_idf_svc::sys::EspError;

/// Si el modulo es anodo comun (enciende con LOW), pon esto en `true`.
const ANODO_COMUN: bool = true;

pub struct Leds {
    pines: [PinDriver<'static, Output>; 4],
}

impl Leds {
    pub fn new(
        d1: impl OutputPin + 'static,
        d2: impl OutputPin + 'static,
        d3: impl OutputPin + 'static,
        d4: impl OutputPin + 'static,
    ) -> Result<Self, EspError> {
        let pines = [
            PinDriver::output(d1)?,
            PinDriver::output(d2)?,
            PinDriver::output(d3)?,
            PinDriver::output(d4)?,
        ];
        let mut leds = Self { pines };
        leds.todos(false)?;
        Ok(leds)
    }

    /// `i` va de 0 a 3.
    pub fn poner(&mut self, i: usize, encendido: bool) -> Result<(), EspError> {
        let nivel = encendido != ANODO_COMUN;
        if nivel {
            self.pines[i].set_high()
        } else {
            self.pines[i].set_low()
        }
    }

    pub fn todos(&mut self, encendido: bool) -> Result<(), EspError> {
        for i in 0..4 {
            self.poner(i, encendido)?;
        }
        Ok(())
    }

    /// Prueba: enciende uno a uno, luego apaga uno a uno.
    pub fn secuencia(&mut self, ms: u32) {
        use esp_idf_svc::hal::delay::FreeRtos;
        for i in 0..4 {
            let _ = self.poner(i, true);
            FreeRtos::delay_ms(ms);
        }
        for i in 0..4 {
            let _ = self.poner(i, false);
            FreeRtos::delay_ms(ms);
        }
    }
}
