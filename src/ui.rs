pub mod arranque;
pub mod listo;
pub mod semaforo;

use crate::config::VERSION_FIRMWARE as VERSION;

/// Primera y segunda linea de toda pantalla.
pub fn cabecera() -> [String; 2] {
    [
        format!("AgroFenix {:>10}", format!("v{VERSION}")),
        "--------------------".to_string(),
    ]
}
