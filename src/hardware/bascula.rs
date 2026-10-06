#[cfg(feature = "bascula-mock")]
mod mock;
mod uart;

#[cfg(feature = "bascula-mock")]
pub use mock::BasculaMock;
pub use uart::BasculaUart;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lectura {
    pub peso_kg: f32,
    pub estable: bool,
}

pub fn parsear_trama(trama: &[u8]) -> Option<Lectura> {
    if trama.len() != 12 {
        return None;
    }

    if trama[0] != b'>' || &trama[10..12] != b"kg" {
        return None;
    }

    let estable = match trama[1] {
        b'S' => true,
        b'M' => false,
        _ => return None,
    };

    if trama[2] != b'G' {
        return None;
    }

    let texto = core::str::from_utf8(&trama[3..10]).ok()?;
    let texto = texto.trim();
    let peso_kg = texto.parse::<f32>().ok()?;

    Some(Lectura { peso_kg, estable })
}

pub struct Acumulador {
    buf: [u8; 16],
    largo: usize,
    dentro: bool,
}

impl Acumulador {
    pub fn new() -> Self {
        Self {
            buf: [0; 16],
            largo: 0,
            dentro: false,
        }
    }

    pub fn empujar(&mut self, b: u8) -> Option<Lectura> {
        match b {
            b'>' => {
                self.buf[0] = b;
                self.largo = 1;
                self.dentro = true;
                None
            }
            _ if !self.dentro => None, // basura antes de un `>`
            b'\r' => None,             // se ignora
            b'\n' => {
                self.dentro = false;
                parsear_trama(&self.buf[..self.largo])
            }
            _ if self.largo == self.buf.len() => {
                self.dentro = false;
                None
            }
            _ => {
                self.buf[self.largo] = b;
                self.largo += 1;
                None
            }
        }
    }

    pub fn empujar_bytes(&mut self, bytes: &[u8]) -> Option<Lectura> {
        let mut ultima = None;

        for &b in bytes {
            if let Some(l) = self.empujar(b) {
                ultima = Some(l);
            }
        }
        ultima
    }

    pub fn reiniciar(&mut self) {
        self.dentro = false;
        self.largo = 0;
    }
}

/// De donde sale el peso: la bascula real por UART o un mock para probar sin ella.
pub trait FuenteBascula {
    /// Espera la siguiente trama valida, hasta un tiempo maximo.
    /// `None` si no llego nada en ese tiempo (bascula apagada o desconectada).
    fn leer(&mut self) -> Option<Lectura>;

    /// Tira lo que se haya acumulado sin leer, para que la siguiente
    /// lectura sea posterior a este momento.
    fn descartar(&mut self);
}
