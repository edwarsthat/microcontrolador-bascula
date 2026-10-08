use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{arranque::Sistema, config};

#[derive(Clone, Copy)]
pub enum Fase {
    EsperandoPeso,
    EsperandoTarjeta { peso_kg: f32, desde: Instant },
    EsperandoCero,
}

enum Verificacion {
    Igual { peso_kg: f32 },
    Cambio { antes: f32, ahora: f32 },
    SinLectura,
}

pub struct Pesaje {
    fase: Fase,
    ultimos: HashMap<String, Instant>,
}

impl Pesaje {
    pub fn new() -> Self {
        Self {
            fase: Fase::EsperandoPeso,
            ultimos: HashMap::new(),
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
                Fase::EsperandoCero => self.esperando_cero(sistema),
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
            if let Some(falta) = self.falta_para(&uid) {
                let minutos = falta.as_secs().div_ceil(60);
                log::warn!("Tarjeta {uid} rechazada: registro hace poco, faltan {minutos} min");
                sistema.semaforo.error();
                sistema
                    .pantallas
                    .aviso("ESPERE", &format!("faltan {minutos} min"));
                std::thread::sleep(Duration::from_secs(3));
                self.entrar(Fase::EsperandoCero, sistema);
                return;
            }
            log::info!("Tarjeta {uid} con {peso_kg:.2} kg -> verificar peso");
            let resultado = verificar_peso(peso_kg, sistema);
            if mostrar_resultado(&uid, resultado, sistema) {
                self.ultimos.insert(uid, Instant::now());
            }
            self.entrar(Fase::EsperandoCero, sistema);
            return;
        }

        if desde.elapsed() > config::TIMEOUT_TARJETA {
            log::warn!(
                "Sin tarjeta en {} s, se vuelve a esperar peso",
                config::TIMEOUT_TARJETA.as_secs()
            );
            self.entrar(Fase::EsperandoCero, sistema);
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
            Fase::EsperandoCero => {
                sistema.bascula.descartar();
                sistema.pantallas.retiro(None);
            }
        }

        self.fase = nueva;
    }

    /// Una vuelta de la espera de retiro: muestra el peso hasta que baje de PESO_CERO_KG.
    fn esperando_cero(&mut self, sistema: &mut Sistema) {
        let lectura = sistema.bascula.leer();
        sistema.pantallas.retiro(lectura.map(|l| l.peso_kg));

        if let Some(l) = lectura {
            if l.estable && l.peso_kg < config::PESO_CERO_KG {
                log::info!("Canasta retirada ({:.2} kg) -> esperar peso", l.peso_kg);
                self.entrar(Fase::EsperandoPeso, sistema);
            }
        }
    }

    /// Si la tarjeta registro hace menos de TARJETA_ESPERA, cuanto le falta.
    /// `None` si puede registrar.
    fn falta_para(&mut self, uid: &str) -> Option<Duration> {
        // De paso se olvidan las tarjetas que ya cumplieron la espera.
        self.ultimos
            .retain(|_, cuando| cuando.elapsed() < config::TARJETA_ESPERA);

        let cuando = self.ultimos.get(uid)?;
        Some(config::TARJETA_ESPERA - cuando.elapsed())
    }
}

fn verificar_peso(peso_1: f32, sistema: &mut Sistema) -> Verificacion {
    sistema.bascula.descartar();

    let limite = Instant::now() + config::TIMEOUT_VERIFICACION;
    while Instant::now() < limite {
        let Some(l) = sistema.bascula.leer() else {
            continue;
        };
        if !l.estable {
            continue;
        }

        let peso_2 = l.peso_kg;
        let cota_s = peso_1 + config::TOLERANCIA_KG;
        let cota_i = peso_1 - config::TOLERANCIA_KG;

        if peso_2 <= cota_s && peso_2 >= cota_i {
            return Verificacion::Igual { peso_kg: peso_2 };
        } else {
            return Verificacion::Cambio {
                antes: peso_1,
                ahora: peso_2,
            };
        }
    }

    Verificacion::SinLectura
}

/// Muestra el resultado unos segundos (LED + pantalla + log).
fn mostrar_resultado(uid: &str, resultado: Verificacion, sistema: &mut Sistema) -> bool {
    let guardado = match resultado {
        Verificacion::Igual { peso_kg } => {
            match sistema.registros.registrar(uid, peso_kg, sistema.sesion) {
                Ok(r) => {
                    log::info!("Pesaje #{}: tarjeta {uid}, {peso_kg:.2} kg", r.transaccion);
                    sistema.semaforo.confirmado();
                    sistema.pantallas.aviso(
                        &format!("{peso_kg:.1} kg"),
                        &format!("registro #{}", r.transaccion),
                    );
                    true
                }
                Err(e) => {
                    log::error!("Pesaje NO guardado: tarjeta {uid}, {peso_kg:.2} kg: {e}");
                    sistema.semaforo.error();
                    sistema.pantallas.aviso("ERROR", "no se guardo");
                    false
                }
            }
        }

        Verificacion::Cambio { antes, ahora } => {
            log::warn!("Pesaje RECHAZADO: tarjeta {uid}, {antes:.2} kg -> {ahora:.2} kg");
            sistema.semaforo.error();
            sistema.pantallas.aviso("ERROR", "el peso cambio");
            false
        }
        Verificacion::SinLectura => {
            log::warn!("Pesaje RECHAZADO: tarjeta {uid}, sin lectura estable");
            sistema.semaforo.error();
            sistema.pantallas.aviso("ERROR", "peso inestable");
            false
        }
    };
    std::thread::sleep(Duration::from_secs(3));
    guardado
}
