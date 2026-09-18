use std::time::Duration;

use crate::arranque::Sistema;
use crate::ui;

pub enum Fase {
    Reposo,
}

pub struct Pesaje {
    fase: Fase,
}

impl Pesaje {
    pub fn new() -> Self {
        Self { fase: Fase::Reposo }
    }

    /// Bucle principal. No retorna.
    pub fn correr(&mut self, sistema: &mut Sistema) -> ! {
        self.entrar(Fase::Reposo, sistema);
        loop {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Unico lugar que toca semaforo y pantalla.
    fn entrar(&mut self, nueva: Fase, sistema: &mut Sistema) {
        match &nueva {
            Fase::Reposo => {
                sistema.semaforo.reposo();
                ui::listo::mostrar(&mut sistema.pantalla, &sistema.device_id, sistema.en_linea);
            }
        }

        self.fase = nueva;
    }
}
