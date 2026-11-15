//! Popup de recordatorios: una ventana pequeña, sin bordes y siempre encima,
//! en la esquina inferior derecha, que no se cierra sola.
//!
//! - Se crea cuando vence un recordatorio y se destruye al cerrarla: no ocupa
//!   memoria el resto del tiempo.
//! - Se crea **sin foco**, para no robarle el teclado a lo que esté haciendo
//!   el usuario.
//! - Si ya está abierta, los nuevos avisos se añaden a su cola (`push`) y la
//!   interfaz se actualiza con el evento `popup-updated`.
//! - Mide su contenido y pide su altura con `resize`; la ventana crece hacia
//!   arriba manteniéndose pegada a la esquina.

use std::sync::Mutex;

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Monitor, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

use crate::db::{tasks, tasks::Task, Db};

pub const LABEL: &str = "reminder";

/// Ancho y márgenes en píxeles lógicos.
const WIDTH: f64 = 340.0;
const MARGIN: f64 = 12.0;
const MIN_HEIGHT: f64 = 120.0;
const MAX_HEIGHT: f64 = 480.0;

/// Ids de las tareas que muestra el popup, en orden de llegada.
#[derive(Default)]
pub struct PopupState(Mutex<Vec<i64>>);

/// Añade avisos al popup, creándolo si no está abierto.
pub fn push(app: &AppHandle, ids: &[i64]) {
    if ids.is_empty() {
        return;
    }
    {
        let state = app.state::<PopupState>();
        let mut queue = state.0.lock().unwrap();
        for id in ids {
            if !queue.contains(id) {
                queue.push(*id);
            }
        }
    }
    match app.get_webview_window(LABEL) {
        Some(w) => {
            let _ = w.emit("popup-updated", ());
        }
        None => create(app),
    }
}

fn create(app: &AppHandle) {
    let height = 180.0;
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Recordatorio")
        .inner_size(WIDTH, height)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false);
    if let Some((x, y)) = app.primary_monitor().ok().flatten().map(|m| corner(&m, height)) {
        builder = builder.position(x, y);
    }
    match builder.build() {
        Ok(w) => {
            let handle = app.clone();
            w.on_window_event(move |e| {
                // Cerrado por cualquier vía (también Alt+F4): se vacía la cola.
                if let WindowEvent::Destroyed = e {
                    handle.state::<PopupState>().0.lock().unwrap().clear();
                }
            });
        }
        Err(e) => eprintln!("No se pudo abrir el popup de recordatorios: {e}"),
    }
}

/// Esquina superior izquierda (lógica) para que una ventana de `height`
/// quede pegada a la esquina inferior derecha del área de trabajo (sin la
/// barra de tareas).
fn corner(m: &Monitor, height: f64) -> (f64, f64) {
    let scale = m.scale_factor();
    let area = m.work_area();
    let right = (area.position.x as f64 + area.size.width as f64) / scale;
    let bottom = (area.position.y as f64 + area.size.height as f64) / scale;
    (right - WIDTH - MARGIN, bottom - height - MARGIN)
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Ajusta la altura al contenido (con límites) y recoloca en la esquina.
pub fn resize(app: &AppHandle, content_height: f64) {
    let Some(w) = window(app) else { return };
    let height = content_height.clamp(MIN_HEIGHT, MAX_HEIGHT);
    let _ = w.set_size(LogicalSize::new(WIDTH, height));
    let monitor = w.current_monitor().ok().flatten().or_else(|| app.primary_monitor().ok().flatten());
    if let Some(m) = monitor {
        let (x, y) = corner(&m, height);
        let _ = w.set_position(LogicalPosition::new(x, y));
    }
}

/// Tareas de la cola que siguen existiendo y pendientes.
pub fn tasks(app: &AppHandle) -> Vec<Task> {
    let ids = app.state::<PopupState>().0.lock().unwrap().clone();
    let db = app.state::<Db>();
    let conn = db.lock();
    ids.into_iter()
        .filter_map(|id| tasks::get(&conn, id).ok())
        .filter(|t| !t.done)
        .collect()
}

/// Tras cambiar tareas desde la ventana principal (hecha, borrada…): si el
/// popup ya no tiene nada que mostrar se cierra; si no, se actualiza.
pub fn refresh(app: &AppHandle) {
    if window(app).is_none() {
        return;
    }
    if tasks(app).is_empty() {
        close(app);
    } else if let Some(w) = window(app) {
        let _ = w.emit("popup-updated", ());
    }
}

/// Quita una tarea del popup; si no queda ninguna, lo cierra.
pub fn remove(app: &AppHandle, id: i64) {
    let empty = {
        let state = app.state::<PopupState>();
        let mut queue = state.0.lock().unwrap();
        queue.retain(|&x| x != id);
        queue.is_empty()
    };
    if empty {
        close(app);
    } else if let Some(w) = window(app) {
        let _ = w.emit("popup-updated", ());
    }
}

pub fn close(app: &AppHandle) {
    app.state::<PopupState>().0.lock().unwrap().clear();
    if let Some(w) = window(app) {
        let _ = w.destroy();
    }
}
