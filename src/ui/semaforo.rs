use crate::hardware::leds::Leds;

const AZUL: usize = 0;
const AMARILLO: usize = 1;
const VERDE: usize = 2;
const ROJO: usize = 3;

pub struct Semaforo {
    leds: Option<Leds>,
}

impl Semaforo {
    pub fn new(leds: Option<Leds>) -> Self {
        let mut s = Self { leds };
        s.reposo();
        s
    }

    /// Apaga todo y enciende solo los indicados. Es la UNICA funcion que toca
    /// los pines: asi ningun estado deja un LED olvidado del estado anterior.
    fn solo(&mut self, encendidos: &[usize]) {
        let Some(leds) = &mut self.leds else { return };
        for i in 0..4 {
            let _ = leds.poner(i, encendidos.contains(&i));
        }
    }

    pub fn reposo(&mut self) {
        self.solo(&[])
    }
    pub fn canasta(&mut self) {
        self.solo(&[AZUL])
    }
    pub fn enviando(&mut self) {
        self.solo(&[AMARILLO])
    }
    pub fn confirmado(&mut self) {
        self.solo(&[VERDE])
    }
    pub fn pendiente(&mut self) {
        self.solo(&[VERDE, AMARILLO])
    }
    pub fn error(&mut self) {
        self.solo(&[ROJO])
    }
}
