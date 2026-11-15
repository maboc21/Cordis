//! Atajo global para mostrar u ocultar la ventana.
//!
//! En Wayland las apps no pueden registrar atajos globales: el registro falla
//! (o no hace nada) y el usuario debe asignar `cordis --toggle` a
//! una tecla en su escritorio. Por eso un fallo aquí nunca impide arrancar.

use std::str::FromStr;
use std::sync::Mutex;

use tauri::{plugin::TauriPlugin, AppHandle, Manager, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::db::{settings, Db};
use crate::error::{Error, Result};
use crate::window;

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                window::toggle(app);
            }
        })
        .build()
}

/// Valida un atajo como «Ctrl+Alt+Space». Exige al menos Ctrl, Alt o Super:
/// una tecla sola o con solo Shift capturaría la escritura normal.
pub fn parse(accel: &str) -> Result<Shortcut> {
    let accel = accel.trim();
    let sc = Shortcut::from_str(accel)
        .map_err(|_| Error::Invalid(format!("«{accel}» no es un atajo válido")))?;
    if (sc.mods & !Modifiers::SHIFT).is_empty() {
        return Err(Error::Invalid(
            "El atajo debe incluir Ctrl, Alt o Super".into(),
        ));
    }
    Ok(sc)
}

/// Último error al registrar el atajo, para enseñarlo en Ajustes (al
/// arrancar no hay a quién mostrárselo).
#[derive(Default)]
pub struct Status(Mutex<Option<String>>);

impl Status {
    pub fn error(&self) -> Option<String> {
        self.0.lock().unwrap().clone()
    }
}

/// Sustituye el atajo registrado por `accel`.
pub fn register(app: &AppHandle, accel: &str) -> Result<()> {
    let result = try_register(app, accel);
    *app.state::<Status>().0.lock().unwrap() = result.as_ref().err().map(ToString::to_string);
    result
}

fn try_register(app: &AppHandle, accel: &str) -> Result<()> {
    let sc = parse(accel)?;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.register(sc).map_err(|e| {
        Error::Invalid(format!(
            "No se pudo activar {accel}: probablemente lo usa otra aplicación ({e})"
        ))
    })
}

/// Registra el atajo guardado. Si falla, lo anota en `Status` y devuelve
/// `false`: la app sigue funcionando con la bandeja y `--toggle`.
pub fn register_saved(app: &AppHandle) -> bool {
    let accel = settings::shortcut(&app.state::<Db>().lock())
        .unwrap_or_else(|_| settings::DEFAULT_SHORTCUT.to_owned());
    match register(app, &accel) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("Atajo global no disponible: {e}");
            false
        }
    }
}

pub fn suspend(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}

pub fn is_active(app: &AppHandle, accel: &str) -> bool {
    parse(accel).is_ok_and(|sc| app.global_shortcut().is_registered(sc))
}

/// Sesión Wayland en Linux (sin atajos globales para las apps).
pub fn is_wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_atajo_por_defecto_es_valido() {
        assert!(parse(settings::DEFAULT_SHORTCUT).is_ok());
    }

    #[test]
    fn acepta_combinaciones_con_modificador() {
        for ok in ["Ctrl+Alt+Space", "Ctrl+Shift+T", "Alt+F2", "Super+Space", "Ctrl+1", " Ctrl+Alt+K "] {
            assert!(parse(ok).is_ok(), "{ok} debería valer");
        }
    }

    #[test]
    fn rechaza_teclas_sin_modificador_o_solo_con_shift() {
        for bad in ["Space", "A", "Shift+A"] {
            assert!(matches!(parse(bad), Err(Error::Invalid(_))), "{bad} no debería valer");
        }
    }

    #[test]
    fn rechaza_texto_que_no_es_un_atajo() {
        for bad in ["", "Ctrl+", "Ctrl+Alt+NoExiste", "hola"] {
            assert!(matches!(parse(bad), Err(Error::Invalid(_))), "{bad} no debería valer");
        }
    }
}
