use esp_idf_svc::nvs::{EspCustomNvsPartition, EspNvs, NvsCustom};
use esp_idf_svc::sys::EspError;
use serde::Serialize;

/// Nombre en partitions.csv.
const PARTICION: &str = "registros";
const NS: &str = "bascula";
const KEY_TRANSACCION: &str = "transaccion";

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
}
