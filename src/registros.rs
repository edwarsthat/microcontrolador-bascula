use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};
use esp_idf_svc::sys::EspError;
use serde::Serialize;
use std::sync::{Arc, Mutex};

/// Nombre en partitions.csv.
const PARTICION: &str = "registros";
const NS: &str = "bascula";
const KEY_TRANSACCION: &str = "transaccion";
/// El registro mas viejo que el servidor todavia no confirmo.
const KEY_PENDIENTE: &str = "pendiente";

#[derive(Serialize, Debug)]
pub struct Registro {
    /// Consecutivo por bascula, sin huecos.
    pub transaccion: u32,
    pub uid_tarjeta: String,
    pub peso_kg: f32,
    /// Con `monotonico_us`, el servidor reconstruye la hora (igual que en el arranque).
    pub sesion_arranque: i64,
    pub monotonico_us: i64,
}

fn clave(transaccion: u32) -> String {
    format!("r{transaccion}")
}

pub struct Registros {
    store: EspNvs<NvsCustom>,
}

impl Registros {
    pub fn abrir() -> Result<Self, EspError> {
        let particion = EspCustomNvsPartition::take(PARTICION)?;
        let store = EspNvs::new(particion, NS, true)?;
        let mut registros = Self { store };
        registros.reparar()?;
        Ok(registros)
    }

    pub fn transaccion_actual(&self) -> u32 {
        self.store
            .get_u32(KEY_TRANSACCION)
            .ok()
            .flatten()
            .unwrap_or(1)
    }

    pub fn registrar(
        &mut self,
        uid_tarjeta: &str,
        peso_kg: f32,
        sesion_arranque: i64,
    ) -> Result<Registro, EspError> {
        self.reparar()?;

        let registro = Registro {
            transaccion: self.transaccion_actual(),
            uid_tarjeta: uid_tarjeta.to_string(),
            peso_kg,
            sesion_arranque,
            monotonico_us: unsafe { esp_idf_svc::sys::esp_timer_get_time() },
        };
        let json = serde_json::to_string(&registro).expect("serializar_registro");

        // Primero el registro, despues el contador (ver `reparar`).
        self.store.set_str(&clave(registro.transaccion), &json)?;
        self.avanzar_transaccion()?;

        log::info!(
            "Registros: guardado {} = {json}",
            clave(registro.transaccion)
        );
        Ok(registro)
    }

    fn avanzar_transaccion(&mut self) -> Result<u32, EspError> {
        let siguiente = self.transaccion_actual() + 1;
        self.store.set_u32(KEY_TRANSACCION, siguiente)?;
        Ok(siguiente)
    }

    /// Si ya hay un registro con el id actual, el contador quedo atras (se fue la
    /// luz entre guardar el registro y avanzarlo): se avanza para no sobrescribirlo.
    fn reparar(&mut self) -> Result<(), EspError> {
        let actual = self.transaccion_actual();
        if self.store.str_len(&clave(actual))?.is_some() {
            log::warn!("Registros: el {actual} ya existia; se avanza el contador");
            self.avanzar_transaccion()?;
        }
        Ok(())
    }

    fn primer_pendiente(&self) -> u32 {
        self.store
            .get_u32(KEY_PENDIENTE)
            .ok()
            .flatten()
            .unwrap_or(1)
    }

    /// Cuantos registros faltan por enviar.
    #[expect(dead_code, reason = "para mostrar los pendientes en pantalla")]
    pub fn cantidad_pendientes(&self) -> u32 {
        self.transaccion_actual() - self.primer_pendiente()
    }

    pub fn siguiente_pendiente(&mut self) -> Option<(u32, String)> {
        let hasta = self.transaccion_actual();
        let mut id = self.primer_pendiente();

        while id < hasta {
            let clave = clave(id);
            match self.store.str_len(&clave) {
                Ok(Some(largo)) => {
                    let mut buf = vec![0u8; largo];
                    match self.store.get_str(&clave, &mut buf) {
                        Ok(Some(json)) => return Some((id, json.to_string())),
                        _ => {
                            log::error!("Registros: {clave} ilegible, se saca de la cola");
                            let _ = self.store.remove(&clave);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log::error!("Registros: no se pudo leer {clave}: {e}");
                    return None;
                }
            }
            id += 1;
            let _ = self.store.set_u32(KEY_PENDIENTE, id);
        }
        None
    }

    pub fn confirmar(&mut self, transaccion: u32) -> Result<(), EspError> {
        self.store.remove(&clave(transaccion))?;
        self.store.set_u32(KEY_PENDIENTE, transaccion + 1)?;
        log::info!("Registros: r{transaccion} confirmado y borrado");
        Ok(())
    }
}

#[derive(Clone)]
pub struct Compartidos(Arc<Mutex<Registros>>);

impl Compartidos {
    pub fn new(registros: Registros) -> Self {
        Self(Arc::new(Mutex::new(registros)))
    }

    /// Presta los registros mientras corre `f` y suelta el candado al terminar.
    /// Si otro hilo entro en panico con el candado puesto, se recupera igual:
    /// el estado real esta en la flash y `reparar` corrige lo que haya quedado a medias.
    pub fn con<T>(&self, f: impl FnOnce(&mut Registros) -> T) -> T {
        let mut guardia = match self.0.lock() {
            Ok(guardia) => guardia,
            Err(envenenado) => {
                log::error!("Registros: un hilo fallo con el candado puesto; se recupera");
                envenenado.into_inner()
            }
        };
        f(&mut guardia)
    }
}
