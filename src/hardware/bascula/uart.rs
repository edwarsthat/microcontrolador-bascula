//! La bascula real: bytes de la UART, armados en tramas por el acumulador.

use std::time::Instant;

use crate::config;
use crate::hardware::bus_uart::BusUart;

use super::{Acumulador, FuenteBascula, Lectura};

pub struct BasculaUart {
    uart: BusUart,
    acc: Acumulador,
}

impl BasculaUart {
    pub fn new(uart: BusUart) -> Self {
        Self {
            uart,
            acc: Acumulador::new(),
        }
    }
}

impl FuenteBascula for BasculaUart {
    fn leer(&mut self) -> Option<Lectura> {
        let limite = Instant::now() + config::TIMEOUT_BASCULA;
        let mut buf = [0u8; 32];

        while Instant::now() < limite {
            let n = self.uart.leer(&mut buf, 50);
            if let Some(l) = self.acc.empujar_bytes(&buf[..n]) {
                return Some(l);
            }
        }
        None
    }

    fn descartar(&mut self) {
        self.uart.descartar_rx();
        self.acc.reiniciar();
    }
}
