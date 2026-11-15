//! Comportamiento tipo Spotlight de la ventana principal: se muestra centrada
//! en el monitor del ratón, se oculta al perder el foco, con Esc (desde la
//! interfaz) o al cerrarla. Cerrarla nunca sale de la app.
//!
//! **Fijada**: si el usuario la arrastra (o pulsa la chincheta), deja de
//! ocultarse al perder el foco, para poder copiar de otras ventanas e ir
//! pegando en ella. Al ocultarla o volver a mostrarla se suelta y vuelve a
//! salir centrada: la posición no se guarda.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow, WindowEvent};

pub const MAIN: &str = "main";

/// Si la ventana se ocultó por perder el foco hace menos de esto, un toggle
/// la deja oculta. Al pulsar el icono de la bandeja (o un lanzador externo),
/// primero se pierde el foco y luego llega el clic: sin esta ventana de
/// gracia, la ventana se ocultaría y volvería a aparecer.
const BLUR_GRACE: Duration = Duration::from_millis(350);

/// Vista que debe mostrar la interfaz al aparecer (evento `show-view`).
#[derive(Clone, Copy, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    #[default]
    List,
    Settings,
}

#[derive(Default)]
pub struct WindowState {
    hidden_on_blur_at: Mutex<Option<Instant>>,
    /// Última vista pedida. El `show-view` del arranque se emite antes de que
    /// la interfaz escuche, así que esta la consulta al cargar (`current_view`).
    view: Mutex<View>,
    /// Pausas activas del ocultado al perder el foco (ver `pause_autohide`).
    autohide_pauses: AtomicUsize,
    /// Fijada: no se oculta al perder el foco (ver `set_pinned`).
    pinned: AtomicBool,
    /// Posición en la que `show` colocó la ventana. Un `Moved` a otra
    /// posición es que la ha arrastrado el usuario.
    placed_at: Mutex<Option<PhysicalPosition<i32>>>,
}

/// Fija o suelta la ventana y avisa a la interfaz (`pinned-changed`).
pub fn set_pinned(app: &AppHandle, pinned: bool) {
    if app.state::<WindowState>().pinned.swap(pinned, Ordering::SeqCst) != pinned {
        let _ = app.emit_to(MAIN, "pinned-changed", pinned);
    }
}

pub fn is_pinned(app: &AppHandle) -> bool {
    app.state::<WindowState>().pinned.load(Ordering::SeqCst)
}

/// Un movimiento que no viene de `show` lo ha hecho el usuario: se fija.
fn on_moved(app: &AppHandle, pos: PhysicalPosition<i32>) {
    let Some(w) = main_window(app) else { return };
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    if *app.state::<WindowState>().placed_at.lock().unwrap() == Some(pos) {
        return;
    }
    set_pinned(app, true);
}

/// Mientras exista, perder el foco no oculta la ventana. Para los diálogos
/// de archivo, que se llevan el foco: sin esto, la ventana desaparecería al
/// abrirlos. Al soltarse (también si hay un error o un pánico) se reactiva.
pub struct AutohidePause(AppHandle);

pub fn pause_autohide(app: &AppHandle) -> AutohidePause {
    app.state::<WindowState>().autohide_pauses.fetch_add(1, Ordering::SeqCst);
    AutohidePause(app.clone())
}

impl Drop for AutohidePause {
    fn drop(&mut self) {
        self.0.state::<WindowState>().autohide_pauses.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn current_view(app: &AppHandle) -> View {
    *app.state::<WindowState>().view.lock().unwrap()
}

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN)
}

pub fn init(app: &AppHandle) {
    app.manage(WindowState::default());
    let Some(w) = main_window(app) else { return };
    let handle = app.clone();
    w.on_window_event(move |event| match event {
        WindowEvent::Focused(false) => hide_on_blur(&handle),
        WindowEvent::Moved(pos) => on_moved(&handle, *pos),
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&handle);
        }
        _ => {}
    });
}

fn hide_on_blur(app: &AppHandle) {
    let Some(w) = main_window(app) else { return };
    // Con las herramientas de desarrollo abiertas, ocultarla haría imposible depurar.
    #[cfg(debug_assertions)]
    if w.is_devtools_open() {
        return;
    }
    let state = app.state::<WindowState>();
    if state.pinned.load(Ordering::SeqCst)
        || state.autohide_pauses.load(Ordering::SeqCst) > 0
        || !w.is_visible().unwrap_or(false)
    {
        return;
    }
    *state.hidden_on_blur_at.lock().unwrap() = Some(Instant::now());
    let _ = w.hide();
}

/// Muestra la ventana centrada y sin fijar.
pub fn show(app: &AppHandle, view: View) {
    let Some(w) = main_window(app) else { return };
    let state = app.state::<WindowState>();
    *state.view.lock().unwrap() = view;
    set_pinned(app, false);
    // Se anota antes de mover para que `on_moved` reconozca el movimiento.
    *state.placed_at.lock().unwrap() = center_on_cursor_monitor(&w);
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to(MAIN, "show-view", view);
}

/// La oculta y la suelta: la próxima vez sale centrada.
pub fn hide(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        let _ = w.hide();
    }
    set_pinned(app, false);
}

/// Atajo global, clic en la bandeja y `--toggle`.
pub fn toggle(app: &AppHandle) {
    let Some(w) = main_window(app) else { return };
    let just_hidden = app
        .state::<WindowState>()
        .hidden_on_blur_at
        .lock()
        .unwrap()
        .take()
        .is_some_and(|t| t.elapsed() < BLUR_GRACE);
    if just_hidden {
        return;
    }
    let visible = w.is_visible().unwrap_or(false);
    let focused = w.is_focused().unwrap_or(false);
    if visible && focused {
        hide(app);
    } else if visible {
        // Visible sin foco solo pasa si está fijada (el usuario está en otra
        // ventana): se le devuelve el foco sin moverla ni soltarla.
        let _ = w.set_focus();
    } else {
        show(app, View::List);
    }
}

/// Centra la ventana en el monitor donde está el ratón (si no se puede, en
/// el actual) y devuelve dónde la ha puesto. En Wayland el compositor decide
/// la posición y esto no hace nada.
fn center_on_cursor_monitor(w: &WebviewWindow) -> Option<PhysicalPosition<i32>> {
    let monitor = w
        .cursor_position()
        .ok()
        .and_then(|p| w.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.current_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, w.outer_size()) else {
        let _ = w.center();
        return w.outer_position().ok();
    };
    let (mp, ms) = (m.position(), m.size());
    let pos = PhysicalPosition::new(
        mp.x + (ms.width as i32 - size.width as i32) / 2,
        mp.y + (ms.height as i32 - size.height as i32) / 2,
    );
    let _ = w.set_position(pos);
    Some(pos)
}
