//! Comportamiento tipo Spotlight de la ventana principal: se muestra centrada
//! en el monitor del ratón, se oculta al perder el foco, con Esc (desde la
//! interfaz) o al cerrarla. Cerrarla nunca sale de la app.

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
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    *app.state::<WindowState>().hidden_on_blur_at.lock().unwrap() = Some(Instant::now());
    let _ = w.hide();
}

pub fn show(app: &AppHandle, view: View) {
    let Some(w) = main_window(app) else { return };
    *app.state::<WindowState>().view.lock().unwrap() = view;
    center_on_cursor_monitor(&w);
    let _ = w.unminimize();
    let _ = w.show();
    let _ = w.set_focus();
    let _ = app.emit_to(MAIN, "show-view", view);
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        let _ = w.hide();
    }
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
    } else {
        show(app, View::List);
    }
}

/// Centra la ventana en el monitor donde está el ratón (si no se puede, en
/// el actual). En Wayland el compositor decide la posición y esto no hace nada.
fn center_on_cursor_monitor(w: &WebviewWindow) {
    let monitor = w
        .cursor_position()
        .ok()
        .and_then(|p| w.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.current_monitor().ok().flatten());
    let (Some(m), Ok(size)) = (monitor, w.outer_size()) else {
        let _ = w.center();
        return;
    };
    let (mp, ms) = (m.position(), m.size());
    let x = mp.x + (ms.width as i32 - size.width as i32) / 2;
    let y = mp.y + (ms.height as i32 - size.height as i32) / 2;
    let _ = w.set_position(PhysicalPosition::new(x, y));
}
