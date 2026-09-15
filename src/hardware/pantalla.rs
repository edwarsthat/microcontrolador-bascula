use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use esp_idf_svc::hal::i2c::I2cDriver;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

/// Direccion 7 bits. El silkscreen del modulo dice 0x78: es la misma en 8 bits.
const DIRECCION: u8 = 0x3C;
/// Caracteres por linea y lineas por pantalla con FONT_6X10 en 128x64.
pub const COLUMNAS: usize = 21;
pub const FILAS: usize = 6;
/// Alto de FONT_6X10, sin aire: 6 lineas x 10 px = 60 px, entran en los 64.
const LINEA: i32 = 10;

type Display = Ssd1306<
    I2CInterface<I2cDriver<'static>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>,
>;

/// OLED de la bascula. Si no hay bus o el display no responde queda en `None`
/// y todos los metodos son no-op: una pantalla muerta no detiene el pesaje.
pub struct Pantalla {
    display: Option<Display>,
}

impl Pantalla {
    /// Nunca falla: sin bus o sin respuesta del display, loguea y sigue sin el.
    pub fn new(i2c: Option<I2cDriver<'static>>) -> Self {
        let Some(i2c) = i2c else {
            return Self { display: None };
        };

        let interface = I2CDisplayInterface::new_custom_address(i2c, DIRECCION);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        match display.init() {
            Ok(()) => {
                log::info!("OLED inicializado en 0x{DIRECCION:02X}");
                Self {
                    display: Some(display),
                }
            }
            Err(e) => {
                log::error!("OLED no inicializo: {e:?}. Se continua sin pantalla");
                Self { display: None }
            }
        }
    }

    /// Pantalla de texto: borra todo y escribe `lineas` de arriba a abajo.
    /// Lo que pase de FILAS o de COLUMNAS se corta. Un solo flush al final.
    pub fn texto(&mut self, lineas: &[&str]) {
        let Some(d) = self.display.as_mut() else {
            return;
        };
        let estilo = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

        d.clear_buffer();
        for (i, linea) in lineas.iter().take(FILAS).enumerate() {
            let punto = Point::new(0, i as i32 * LINEA);
            let _ = Text::with_baseline(linea, punto, estilo, Baseline::Top).draw(d);
        }
        let _ = d.flush();
    }
}
