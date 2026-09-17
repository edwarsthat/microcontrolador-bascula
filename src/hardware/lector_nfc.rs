use super::bus_spi::DispositivoSpi;
use esp_idf_svc::hal::gpio::{Output, OutputPin, PinDriver};
use mfrc522::comm::blocking::spi::{DummyDelay, SpiInterface};
use mfrc522::{Initialized, Mfrc522};
use std::time::{Duration, Instant};

type Lector = Mfrc522<SpiInterface<DispositivoSpi, DummyDelay>, Initialized>;

/// La misma tarjeta quieta sobre la antena no se reporta de nuevo dentro de este plazo.
const REBOTE: Duration = Duration::from_secs(3);

pub struct LectorNfc {
    lector: Option<Lector>,
    /// El RST debe seguir vivo: si se libera, el modulo se apaga.
    _rst: Option<PinDriver<'static, Output>>,
    ultimo: Option<(String, Instant)>,
}

impl LectorNfc {
    /// Nunca falla: sin lector, `leer_uid` devuelve siempre None y la bascula sigue.
    pub fn new(device: Option<DispositivoSpi>, rst: impl OutputPin + 'static) -> Self {
        let sin_lector = |motivo: String| {
            log::error!("{motivo}. Se continua sin NFC");
            Self {
                lector: None,
                _rst: None,
                ultimo: None,
            }
        };

        let Some(device) = device else {
            return sin_lector("NFC: no hay bus SPI".into());
        };

        let mut rst = match PinDriver::output(rst) {
            Ok(p) => p,
            Err(e) => return sin_lector(format!("NFC: RST no inicializo: {e}")),
        };
        // Pulso de reset: algunos modulos quedan colgados tras un reset del ESP32.
        let _ = rst.set_low();
        std::thread::sleep(Duration::from_millis(50));
        let _ = rst.set_high();
        std::thread::sleep(Duration::from_millis(50));

        match Mfrc522::new(SpiInterface::new(device)).init() {
            Ok(mut l) => {
                match l.version() {
                    Ok(v) => log::info!("RC522 inicializado, VersionReg = 0x{v:02X}"),
                    Err(e) => log::warn!("RC522 init OK pero version fallo: {e:?}"),
                }
                Self {
                    lector: Some(l),
                    _rst: Some(rst),
                    ultimo: None,
                }
            }
            Err(e) => sin_lector(format!("RC522 no inicializo: {e:?}")),
        }
    }

    /// `Some(uid)` solo cuando hay una tarjeta nueva. El resto del tiempo, `None`.
    pub fn leer_uid(&mut self) -> Option<String> {
        let lector = self.lector.as_mut()?;

        // Err aqui es lo normal: significa "no hay tarjeta". No se loguea.
        let atqa = lector.new_card_present().ok()?;

        let uid = match lector.select(&atqa) {
            Ok(uid) => hex::encode_upper(uid.as_bytes()),
            Err(e) => {
                log::warn!("NFC: tarjeta detectada pero el select fallo: {e:?}");
                return None;
            }
        };

        // HALT: la tarjeta deja de responder a reqa hasta que se retire.
        let _ = lector.hlta();
        let _ = lector.stop_crypto1();

        let ahora = Instant::now();
        if let Some((anterior, cuando)) = &self.ultimo {
            if *anterior == uid && ahora.duration_since(*cuando) < REBOTE {
                self.ultimo = Some((uid, ahora)); // refresca mientras siga encima
                return None;
            }
        }
        self.ultimo = Some((uid.clone(), ahora));
        Some(uid)
    }

    pub fn disponible(&self) -> bool {
        self.lector.is_some()
    }
}
