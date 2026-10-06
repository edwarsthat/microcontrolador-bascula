use std::time::{Duration, Instant};

use crate::{arranque::Sistema, config};

#[derive(Clone, Copy)]
pub enum Fase {
    EsperandoPeso,
    EsperandoTarjeta { peso_kg: f32, desde: Instant },
}

pub struct Pesaje {
    fase: Fase,
}

impl Pesaje {
    pub fn new() -> Self {
        Self {
            fase: Fase::EsperandoPeso,
        }
    }

    /// Bucle principal. No retorna.
    pub fn correr(&mut self, sistema: &mut Sistema) -> ! {
        self.entrar(Fase::EsperandoPeso, sistema);
        loop {
            match self.fase {
                Fase::EsperandoPeso => self.esperando_peso(sistema),
                Fase::EsperandoTarjeta { peso_kg, desde } => {
                    self.esperando_tarjeta(peso_kg, desde, sistema)
                }
            }
        }
    }

    /// Una vuelta de la fase de espera: lee, muestra y revisa si ya hay canasta.
    fn esperando_peso(&mut self, sistema: &mut Sistema) {
        let lectura = sistema.bascula.leer();
        sistema
            .pantallas
            .espera(lectura.map(|l| l.peso_kg), sistema.en_linea);

        if let Some(l) = lectura {
            if l.estable && l.peso_kg > config::PESO_MINIMO_KG {
                log::info!("Peso estable: {:.2} kg -> esperar tarjeta", l.peso_kg);
                self.entrar(
                    Fase::EsperandoTarjeta {
                        peso_kg: l.peso_kg,
                        desde: Instant::now(),
                    },
                    sistema,
                );
            }

        }
    }

        /// Una vuelta de la espera de tarjeta: revisa el lector y el tiempo limite.
    fn esperando_tarjeta(&mut self, peso_kg: f32, desde: Instant, sistema: &mut Sistema) {
        if let Some(uid) = sistema.lector.leer_uid() {
            log::info!("Tarjeta {uid} con {peso_kg:.2} kg -> verificar peso");
            // Parte 3: aqui se pasa a Verificando. Por ahora, volver a esperar.
            self.entrar(Fase::EsperandoPeso, sistema);
            return;
        }

        if desde.elapsed() > config::TIMEOUT_TARJETA {
            log::warn!("Sin tarjeta en {} s, se vuelve a esperar peso", config::TIMEOUT_TARJETA.as_secs());
            self.entrar(Fase::EsperandoPeso, sistema);
            return;
        }

        std::thread::sleep(Duration::from_millis(100));
    }


    /// Lo que se hace una sola vez al llegar a una fase: LEDs y pantalla inicial.
    fn entrar(&mut self, nueva: Fase, sistema: &mut Sistema) {
        match nueva {
            Fase::EsperandoPeso => {
                // Lo que llego mientras no se miraba la bascula ya es viejo.
                sistema.bascula.descartar();
                sistema.semaforo.reposo();
                sistema.pantallas.espera(None, sistema.en_linea);
            }
            Fase::EsperandoTarjeta { peso_kg, .. } => {
                sistema.semaforo.canasta();
                sistema.pantallas.tarjeta(peso_kg);
            }
        }

        self.fase = nueva;
    }

}
