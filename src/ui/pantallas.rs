//! Catalogo de pantallas: una funcion por vista, todas con la misma cabecera.
//!
//! Es dueño de `Pantalla` igual que `Semaforo` lo es de `Leds`, y no guarda
//! estado de negocio: dibuja lo que le pasan. El estado vive en quien maneja
//! el flujo (`arranque::iniciar` o `Pesaje`).

use crate::arranque::{Arranque, Estado};
use crate::hardware::pantalla::{Linea, Pantalla, COLUMNAS, COLUMNAS_GRANDES, FILAS};
use crate::ui::{cabecera, campo, centrar};

pub struct Pantallas {
    pantalla: Pantalla,
}

impl Pantallas {
    pub fn new(pantalla: Pantalla) -> Self {
        Self { pantalla }
    }

    /// Unica funcion que dibuja: siempre antepone la cabecera, asi ninguna
    /// vista se olvida de ella. El cuerpo son los 44 px que quedan; la linea
    /// que no quepa entera se descarta.
    fn pintar(&mut self, cuerpo: &[Linea]) {
        let [titulo, separador] = cabecera();
        let mut vista = [Linea::normal(""); FILAS];
        vista[0] = Linea::normal(&titulo);
        vista[1] = Linea::normal(&separador);

        let usadas = 2 + cuerpo.len().min(FILAS - 2);
        for (destino, linea) in vista[2..].iter_mut().zip(cuerpo) {
            *destino = *linea;
        }
        self.pantalla.dibujar(&vista[..usadas]);
    }

    /// Progreso del arranque, se redibuja en cada paso:
    ///
    ///   AgroFenix     v0.1.0
    ///   --------------------
    ///   Lector NFC  OK
    ///   WiFi        ...
    pub fn arranque(&mut self, progreso: &Arranque) {
        let nfc = campo("Lector NFC", marca(progreso.nfc));
        let wifi = campo("WiFi", marca(progreso.wifi));
        let servidor = campo("Servidor", marca(progreso.servidor));

        self.pintar(&[
            Linea::normal(&nfc),
            Linea::normal(&wifi),
            Linea::normal(&servidor),
        ]);
    }

    /// Operacion normal: la bascula espera carga y muestra el peso en vivo,
    /// pase o no el umbral. `kg` en `None` es la bascula sin responder, que no
    /// es lo mismo que cero. La ultima linea queda vacia cuando todo esta bien:
    ///
    ///   AgroFenix     v0.1.0
    ///   --------------------
    ///      0.0 kg
    ///    ponga la canasta
    ///    SIN SERVIDOR
    pub fn espera(&mut self, kg: Option<f32>, en_linea: bool) {
        // Ancho fijo antes de centrar: asi el numero no se corre una columna
        // al pasar de 9.8 a 10.2.
        let peso = match kg {
            Some(kg) => format!("{kg:>6.1} kg"),
            None => "  ---- kg".to_string(),
        };
        let peso = centrar(&peso, COLUMNAS_GRANDES);
        let aviso = centrar("ponga la canasta", COLUMNAS);
        let estado = if en_linea {
            String::new()
        } else {
            centrar("SIN SERVIDOR", COLUMNAS)
        };

        self.pintar(&[
            Linea::grande(&peso),
            Linea::normal(&aviso),
            Linea::normal(&estado),
        ]);
    }
}

/// Como se ve cada estado del arranque al lado de su etiqueta.
fn marca(estado: Estado) -> &'static str {
    match estado {
        Estado::Pendiente => "",
        Estado::EnCurso => "...",
        Estado::Ok => "OK",
        Estado::Fallo => "FALLO",
    }
}
