use embedded_graphics::{
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10},
        MonoTextStyle,
    },
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
/// Caracteres por linea con FONT_10X20: 128 / 10.
pub const COLUMNAS_GRANDES: usize = 12;
/// Alto util del display en pixeles.
const ALTO: i32 = 64;
/// Alto de cada fuente, sin aire.
const LINEA: i32 = 10;
const LINEA_GRANDE: i32 = 20;

/// Tamano de letra de una linea. `Grande` ocupa dos filas normales.
#[derive(Clone, Copy)]
pub enum Tamano {
    Normal,
    Grande,
}

impl Tamano {
    fn alto(self) -> i32 {
        match self {
            Tamano::Normal => LINEA,
            Tamano::Grande => LINEA_GRANDE,
        }
    }

    fn estilo(self) -> MonoTextStyle<'static, BinaryColor> {
        match self {
            Tamano::Normal => MonoTextStyle::new(&FONT_6X10, BinaryColor::On),
            Tamano::Grande => MonoTextStyle::new(&FONT_10X20, BinaryColor::On),
        }
    }
}

/// Una linea de la vista: el texto y con que letra va.
#[derive(Clone, Copy)]
pub struct Linea<'a> {
    pub texto: &'a str,
    pub tamano: Tamano,
}

impl<'a> Linea<'a> {
    pub fn normal(texto: &'a str) -> Self {
        Self {
            texto,
            tamano: Tamano::Normal,
        }
    }

    pub fn grande(texto: &'a str) -> Self {
        Self {
            texto,
            tamano: Tamano::Grande,
        }
    }
}

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

    /// Borra todo y escribe las lineas de arriba a abajo, avanzando segun el
    /// alto de cada letra. La linea que no quepa entera se descarta, igual que
    /// lo que se pase de COLUMNAS. Un solo flush al final.
    pub fn dibujar(&mut self, lineas: &[Linea]) {
        let Some(d) = self.display.as_mut() else {
            return;
        };

        d.clear_buffer();
        let mut y = 0;
        for linea in lineas {
            let alto = linea.tamano.alto();
            if y + alto > ALTO {
                break;
            }
            let punto = Point::new(0, y);
            let _ = Text::with_baseline(linea.texto, punto, linea.tamano.estilo(), Baseline::Top)
                .draw(d);
            y += alto;
        }
        let _ = d.flush();
    }

    /// Pantalla de solo texto normal, el caso comun.
    pub fn texto(&mut self, lineas: &[&str]) {
        let mut vista = [Linea::normal(""); FILAS];
        let usadas = lineas.len().min(FILAS);
        for (destino, texto) in vista.iter_mut().zip(lineas) {
            *destino = Linea::normal(texto);
        }
        self.dibujar(&vista[..usadas]);
    }
}
