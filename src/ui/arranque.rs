use crate::hardware::pantalla::Pantalla;

/// Estado de un paso del arranque, como se ve en pantalla.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Estado {
    #[default]
    Pendiente,
    EnCurso,
    Ok,
    Fallo,
}

impl Estado {
    fn texto(self) -> &'static str {
        match self {
            Estado::Pendiente => "",
            Estado::EnCurso => "...",
            Estado::Ok => "OK",
            Estado::Fallo => "FALLO",
        }
    }
}

/// Progreso del arranque. `main` lo actualiza paso a paso y llama a `mostrar`.
#[derive(Clone, Copy, Default)]
pub struct Arranque {
    pub wifi: Estado,
    pub servidor: Estado,
}

impl Arranque {
    /// Progreso del arranque, se redibuja en cada paso:
    ///
    ///   AgroFenix     v0.1.0
    ///   --------------------
    ///   WiFi        OK
    ///   Servidor    ...
    pub fn mostrar(&self, pantalla: &mut Pantalla) {
        let [titulo, separador] = super::cabecera();
        let wifi = format!("WiFi        {}", self.wifi.texto());
        let servidor = format!("Servidor    {}", self.servidor.texto());

        pantalla.texto(&[&titulo, &separador, &wifi, &servidor]);
    }
}
