use crate::hardware::pantalla::{Pantalla, COLUMNAS};

/// Pantalla de reposo: la bascula esta operativa y espera trabajo.
/// `en_linea` = el servidor respondio al arranque.
pub fn mostrar(pantalla: &mut Pantalla, device_id: &str, en_linea: bool) {
    let [titulo, separador] = super::cabecera();
    let estado = if en_linea { "LISTO" } else { "SIN SERVIDOR" };
    let centrado = format!("{:^ancho$}", estado, ancho = COLUMNAS);

    pantalla.texto(&[&titulo, &separador, "", &centrado, "", device_id]);
}
