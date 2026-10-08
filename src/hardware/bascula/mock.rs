//! Bascula falsa para correr el flujo sin hardware.

use std::time::{Duration, Instant};

use super::{Acumulador, FuenteBascula, Lectura};

struct Tramo {
    hasta_ms: u64,
    peso_kg: f32,
    estable: bool,
}

/// Un pesaje: vacia, ponen la canasta, queda estable 20 s (tiempo para pasar
/// la tarjeta) y la retiran de golpe. Ciclo de 30 s.
const GUION: &[Tramo] = &[
    Tramo {
        hasta_ms: 5_000,
        peso_kg: 0.00,
        estable: true,
    }, // vacia
    Tramo {
        hasta_ms: 6_000,
        peso_kg: 14.50,
        estable: false,
    }, // poniendo la canasta
    Tramo {
        hasta_ms: 7_000,
        peso_kg: 21.85,
        estable: false,
    },
    Tramo {
        hasta_ms: 27_000,
        peso_kg: 22.00,
        estable: true,
    }, // estable 20 s: pasar la tarjeta
    Tramo {
        hasta_ms: 30_000,
        peso_kg: 0.00,
        estable: true,
    }, // retirada
];

pub struct BasculaMock {
    /// Se fija en el primer `leer()`, no al crearlo: asi el guion empieza
    /// cuando `Pesaje` empieza a pesar y no durante el arranque.
    inicio: Option<Instant>,
    acc: Acumulador,
}

impl BasculaMock {
    pub fn new() -> Self {
        Self {
            inicio: None,
            acc: Acumulador::new(),
        }
    }
}

impl FuenteBascula for BasculaMock {
    fn leer(&mut self) -> Option<Lectura> {
        // 1. Esperar como la bascula real
        std::thread::sleep(Duration::from_millis(200));

        // 2. En que punto del ciclo vamos
        let total_ms = GUION[GUION.len() - 1].hasta_ms;
        let inicio = *self.inicio.get_or_insert_with(Instant::now);
        let t = inicio.elapsed().as_millis() as u64 % total_ms;

        // 3. Buscar el tramo actual
        let tramo = GUION.iter().find(|tr| tr.hasta_ms > t)?;

        // 4. Armar la trama como la manda la bascula
        let trama = format!(
            ">{}G{:7.2}kg\r\n",
            if tramo.estable { 'S' } else { 'M' },
            tramo.peso_kg
        );

        // 5. Pasarla por el mismo camino que la real
        self.acc.empujar_bytes(trama.as_bytes())
    }

    fn descartar(&mut self) {
        self.acc.reiniciar();
    }
}
