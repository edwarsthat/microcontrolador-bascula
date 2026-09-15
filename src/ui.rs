pub mod arranque;
pub mod listo;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Primera y segunda linea de toda pantalla.
pub fn cabecera() -> [String; 2] {
    [
        format!("AgroFenix {:>10}", format!("v{VERSION}")),
        "--------------------".to_string(),
    ]
}
