use esp_idf_svc::hal::{
    cpu::{self, Core},
    task::thread::ThreadSpawnConfiguration,
};
use std::time::Duration;

use crate::red::http::{ClienteServidor, HttpError, RUTA_PESAJES};
use crate::red::wifi::Wifi;
use crate::registros::Compartidos;

/// Pila del hilo. HTTP + JSON necesitan mas que los 4 KB por defecto.
const PILA: usize = 8 * 1024;
/// Cada cuanto se revisa la cola cuando no hay nada que enviar.
const ESPERA_SIN_PENDIENTES: Duration = Duration::from_secs(5);
/// Si no se puede crear el cliente HTTP, cuanto esperar para reintentar.
const ESPERA_REINTENTO: Duration = Duration::from_secs(10);
/// Espera despues de un fallo; se duplica en cada fallo seguido hasta ESPERA_MAX.
const ESPERA_INICIAL: Duration = Duration::from_secs(5);
const ESPERA_MAX: Duration = Duration::from_secs(60);

pub fn lanzar(registros: Compartidos, wifi: Wifi, device_id: String) {
    let config = ThreadSpawnConfiguration {
        name: Some(c"envio"),
        pin_to_core: Some(Core::Core1),
        priority: 5,
        ..Default::default()
    };
    if let Err(e) = config.set() {
        log::warn!("Envio: no se pudo fijar el nucleo 1 ({e}); el hilo corre donde caiga");
    }
    let resultado = std::thread::Builder::new()
        .stack_size(PILA)
        .spawn(move || correr(registros, wifi, device_id));

    // Restaurar: que los hilos que se creen despues no terminen en el nucleo 1.
    if let Err(e) = ThreadSpawnConfiguration::default().set() {
        log::warn!("Envio: no se pudo restaurar la config de hilos: {e}");
    }

    if let Err(e) = resultado {
        log::error!("Envio: no se pudo crear el hilo: {e}. Los registros quedan en la cola");
    }
}

/// El ciclo del hilo: mantiene el WiFi arriba y envia los pendientes del mas
/// viejo al mas nuevo, borrando cada uno cuando el servidor lo confirma.
fn correr(registros: Compartidos, mut wifi: Wifi, device_id: String) -> ! {
    log::info!(
        "Envio: hilo corriendo en {:?}, dispositivo {device_id}",
        cpu::core()
    );

    // El cliente se crea aqui: no se puede mover entre hilos.
    let cliente = loop {
        match ClienteServidor::new() {
            Ok(c) => break c,
            Err(e) => {
                log::error!("Envio: no se pudo crear el cliente HTTP: {e}; reintento");
                std::thread::sleep(ESPERA_REINTENTO);
            }
        }
    };

    let mut cliente = cliente;
    let mut espera = ESPERA_INICIAL;

    loop {
        if !asegurar_wifi(&mut wifi) {
            esperar(&mut espera);
            continue;
        }

        let Some((id, registro)) = registros.con(|r| r.siguiente_pendiente()) else {
            std::thread::sleep(ESPERA_SIN_PENDIENTES);
            continue;
        };

        let recibido = match enviar(&mut cliente, &device_id, &registro, false) {
            Respuesta::Recibido => true,
            Respuesta::Rechazado => {
                log::warn!("Envio: r{id} rechazado (422), se reenvia marcado");
                matches!(
                    enviar(&mut cliente, &device_id, &registro, true),
                    Respuesta::Recibido
                )
            }
            Respuesta::Reintentar => false,
        };

        if recibido {
            if let Err(e) = registros.con(|r| r.confirmar(id)) {
                log::error!("Envio: r{id} recibido pero no se pudo borrar: {e}");
            }
            espera = ESPERA_INICIAL;
        } else {
            log::warn!(
                "Envio: r{id} no se pudo enviar; reintento en {} s",
                espera.as_secs()
            );
            esperar(&mut espera);
        }
    }
}

/// Lo que la bascula hace segun lo que respondio el servidor (ver el contrato).
enum Respuesta {
    /// 2xx o 409: el servidor lo tiene. Se borra.
    Recibido,
    /// 422: el dato es invalido. Se reenvia marcado para la tabla de errores.
    Rechazado,
    /// Cualquier otra cosa: el dato puede estar bien, el problema es la red o el servidor.
    Reintentar,
}

/// Arma el cuerpo y hace el POST. `rechazado` marca un reenvio despues de un 422.
fn enviar(
    cliente: &mut ClienteServidor,
    device_id: &str,
    registro: &str,
    rechazado: bool,
) -> Respuesta {
    let cuerpo = if rechazado {
        format!(r#"{{"device_id":"{device_id}","rechazado":true,"registro":{registro}}}"#)
    } else {
        format!(r#"{{"device_id":"{device_id}","registro":{registro}}}"#)
    };

    match cliente.post_json(RUTA_PESAJES, &cuerpo) {
        Ok(_) => Respuesta::Recibido,
        Err(HttpError::Servidor(409)) => Respuesta::Recibido,
        Err(HttpError::Servidor(422)) => Respuesta::Rechazado,
        Err(e) => {
            log::warn!("Envio: {e}");
            Respuesta::Reintentar
        }
    }
}

fn wifi_arriba(wifi: &Wifi) -> bool {
    matches!(wifi.is_up(), Ok(true))
}

fn asegurar_wifi(wifi: &mut Wifi) -> bool {
    if wifi_arriba(wifi) {
        return true;
    }
    log::warn!("Envio: el Wifi se cayo, reconectando");
    match crate::red::wifi::reconectar(wifi) {
        Ok(()) => true,
        Err(e) => {
            log::error!("Envio: no se pudo reconectar el Wifi: {e}");
            false
        }
    }
}

fn esperar(espera: &mut Duration) {
    std::thread::sleep(*espera);
    *espera = (*espera * 2).min(ESPERA_MAX);
}
