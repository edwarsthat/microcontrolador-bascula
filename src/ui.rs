pub mod pantallas;
pub mod semaforo;

use crate::config::VERSION_FIRMWARE as VERSION;

/// Ancho de la etiqueta en una linea "etiqueta    valor": deja el valor
/// siempre en la misma columna, sin que cada vista lo cuente a mano.
const ETIQUETA: usize = 12;

/// Primera y segunda linea de toda pantalla.
pub fn cabecera() -> [String; 2] {
    [
        format!("AgroFenix {:>10}", format!("v{VERSION}")),
        "--------------------".to_string(),
    ]
}

/// Centra el texto. `ancho` depende de la letra: COLUMNAS o COLUMNAS_GRANDES.
pub fn centrar(texto: &str, ancho: usize) -> String {
    format!("{:^ancho$}", texto, ancho = ancho)
}

/// Linea de "etiqueta    valor".
pub fn campo(etiqueta: &str, valor: &str) -> String {
    format!("{:<ancho$}{}", etiqueta, valor, ancho = ETIQUETA)
}
