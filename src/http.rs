use esp_idf_svc::http::client::{Configuration as HttpConfiguration, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::sys::EspError;
use std::time::Duration;

const SERVER_URL: &str = env!("SERVER_URL");
pub const RUTA_ARRANQUE: &str = "/dispositivos/arranque";
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub enum HttpError {
    /// No se pudo crear la conexión (memoria, config inválida).
    Init(EspError),
    /// Falló al abrir el socket o escribir: casi siempre problema de red.
    Envio(EspError),
    /// Falló leyendo la respuesta.
    Respuesta(EspError),
    /// El servidor contestó pero con un status fuera de 2xx.
    Servidor(u16),
}

pub struct ClienteServidor {
    conn: EspHttpConnection,
}

impl ClienteServidor {
    pub fn new() -> Result<Self, HttpError> {
        let conn = EspHttpConnection::new(&HttpConfiguration {
            timeout: Some(TIMEOUT),
            ..Default::default()
        })
        .map_err(HttpError::Init)?;

        log::info!("Cliente HTTP listo, servidor: {}", SERVER_URL);
        Ok(Self { conn })
    }

    pub fn post_json(&mut self, ruta: &str, body: &str) -> Result<u16, HttpError> {
        let url = format!("{}{}", SERVER_URL, ruta);
        let content_length = body.len().to_string();
        let headers = [
            ("Content-Type", "application/json"),
            ("Content-Length", &content_length),
        ];

        log::info!("POST {} ({} bytes)", url, body.len());

        // Initial -> Request: abre socket y manda línea + headers
        self.conn
            .initiate_request(Method::Post, &url, &headers)
            .map_err(HttpError::Envio)?;

        // Cuerpo
        self.conn
            .write_all(body.as_bytes())
            .map_err(HttpError::Envio)?;

        // Request -> Response: flush, espera status + headers
        self.conn
            .initiate_response()
            .map_err(HttpError::Respuesta)?;

        let status = self.conn.status();
        self.descartar_cuerpo();

        if (200..300).contains(&status) {
            log::info!("Servidor respondió {}", status);
            Ok(status)
        } else {
            log::error!(
                "Servidor respondió {} {:?}",
                status,
                self.conn.status_message()
            );
            Err(HttpError::Servidor(status))
        }
    }

    /// Lee y descarta el cuerpo para dejar la conexión limpia para el siguiente request.
    fn descartar_cuerpo(&mut self) {
        let mut buf = [0u8; 128];
        loop {
            match self.conn.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => log::debug!("resp: {}", String::from_utf8_lossy(&buf[..n])),
                Err(e) => {
                    log::warn!("Error leyendo respuesta: {:?}", e);
                    break;
                }
            }
        }
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpError::Init(e) => write!(f, "fallo al crear el cliente HTTP: {}", e),
            HttpError::Envio(e) => write!(f, "fallo al enviar la petición: {}", e),
            HttpError::Respuesta(e) => write!(f, "fallo al leer la respuesta: {}", e),
            HttpError::Servidor(s) => write!(f, "el servidor respondió {}", s),
        }
    }
}
