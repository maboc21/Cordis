//! Comandos expuestos a la interfaz. Son finos: cogen la conexión, leen el
//! reloj si hace falta y delegan en `db`. Desde JS los argumentos van en
//! camelCase (`remindAt`), como hace Tauri por defecto.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use crate::db::{self, settings, tasks, tasks::Task, Db};
use crate::error::{Error, Result};
use crate::reminders::Scheduler;
use crate::{backup, popup, shortcut, window};

#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn list_tasks(db: State<'_, Db>) -> Result<Vec<Task>> {
    tasks::list(&db.lock())
}

#[tauri::command]
pub fn create_task(db: State<'_, Db>, title: String) -> Result<Task> {
    tasks::create(&db.lock(), &title, db::now_ms())
}

#[tauri::command]
pub fn rename_task(db: State<'_, Db>, id: i64, title: String) -> Result<Task> {
    tasks::rename(&db.lock(), id, &title)
}

#[tauri::command]
pub fn set_task_notes(db: State<'_, Db>, id: i64, notes: Option<String>) -> Result<Task> {
    tasks::set_notes(&db.lock(), id, notes.as_deref())
}

// Los tres siguientes pueden cambiar qué recordatorio toca después, así que
// despiertan al planificador. El candado de la BD se suelta antes (al acabar
// la expresión), para que el planificador pueda cogerlo.

#[tauri::command]
pub fn set_task_done(
    app: AppHandle,
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
    done: bool,
) -> Result<Task> {
    let task = tasks::set_done(&db.lock(), id, done, db::now_ms())?;
    scheduler.reschedule();
    popup::refresh(&app);
    Ok(task)
}

#[tauri::command]
pub fn set_task_reminder(
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
    remind_at: Option<i64>,
) -> Result<Task> {
    let task = tasks::set_reminder(&db.lock(), id, remind_at)?;
    scheduler.reschedule();
    Ok(task)
}

#[tauri::command]
pub fn delete_task(
    app: AppHandle,
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
) -> Result<()> {
    tasks::delete(&db.lock(), id)?;
    scheduler.reschedule();
    popup::refresh(&app);
    Ok(())
}

// --- Popup de recordatorios ---
//
// Los llama la ventana `reminder`. Cada acción actualiza la BD, despierta al
// planificador, quita la tarea del popup (que se cierra si queda vacío) y
// avisa a la ventana principal con `tasks-changed` para que recargue.

#[tauri::command]
pub fn popup_tasks(app: AppHandle) -> Vec<Task> {
    popup::tasks(&app)
}

/// La interfaz del popup mide su contenido y pide esta altura (px lógicos).
#[tauri::command]
pub fn popup_resize(app: AppHandle, height: f64) {
    popup::resize(&app, height);
}

#[tauri::command]
pub fn popup_done(
    app: AppHandle,
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
) -> Result<()> {
    tasks::set_done(&db.lock(), id, true, db::now_ms())?;
    scheduler.reschedule();
    popup::remove(&app, id);
    let _ = app.emit("tasks-changed", ());
    Ok(())
}

/// Pospone: el recordatorio vuelve a avisar en `until` (ms Unix UTC).
#[tauri::command]
pub fn popup_snooze(
    app: AppHandle,
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
    until: i64,
) -> Result<()> {
    if until <= db::now_ms() {
        return Err(Error::Invalid("Esa hora ya ha pasado".into()));
    }
    tasks::set_reminder(&db.lock(), id, Some(until))?;
    scheduler.reschedule();
    popup::remove(&app, id);
    let _ = app.emit("tasks-changed", ());
    Ok(())
}

/// Cierra el popup y muestra la lista.
#[tauri::command]
pub fn popup_open_list(app: AppHandle) {
    popup::close(&app);
    window::show(&app, window::View::List);
}

/// Cierra el popup; los recordatorios quedan como avisados.
#[tauri::command]
pub fn popup_close(app: AppHandle) {
    popup::close(&app);
}

#[tauri::command]
pub fn reorder_tasks(db: State<'_, Db>, ids: Vec<i64>) -> Result<()> {
    tasks::reorder(&mut db.lock(), &ids)
}

// --- Ventana ---

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    window::hide(&app);
}

/// Fija (no se oculta al perder el foco) o suelta la ventana principal.
#[tauri::command]
pub fn set_pinned(app: AppHandle, pinned: bool) {
    window::set_pinned(&app, pinned);
}

#[tauri::command]
pub fn is_pinned(app: AppHandle) -> bool {
    window::is_pinned(&app)
}

/// Vista con la que se mostró la ventana por última vez.
#[tauri::command]
pub fn current_view(app: AppHandle) -> window::View {
    window::current_view(&app)
}

// --- Ajustes ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    shortcut: String,
    default_shortcut: &'static str,
    /// El atajo está registrado de verdad (puede fallar si otra app lo usa).
    shortcut_active: bool,
    /// Motivo por el que no se pudo registrar, si lo hay.
    shortcut_error: Option<String>,
    /// Sesión Wayland: no hay atajos globales, hay que usar `--toggle`.
    wayland: bool,
    /// Arranca con el sistema (lo guarda el propio sistema, no la BD).
    autostart: bool,
}

fn settings_view(app: &AppHandle, db: &Db) -> Result<SettingsView> {
    let shortcut = settings::shortcut(&db.lock())?;
    Ok(SettingsView {
        shortcut_active: shortcut::is_active(app, &shortcut),
        shortcut_error: app.state::<shortcut::Status>().error(),
        shortcut,
        default_shortcut: settings::DEFAULT_SHORTCUT,
        wayland: shortcut::is_wayland(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    })
}

/// Activa o desactiva el arranque con el sistema (con `--hidden`, en la bandeja).
#[tauri::command]
pub fn set_autostart(app: AppHandle, db: State<'_, Db>, enabled: bool) -> Result<SettingsView> {
    let al = app.autolaunch();
    let result = if enabled { al.enable() } else { al.disable() };
    result.map_err(|e| {
        Error::Invalid(format!("No se pudo cambiar el arranque automático ({e})"))
    })?;
    settings_view(&app, &db)
}

#[tauri::command]
pub fn get_settings(app: AppHandle, db: State<'_, Db>) -> Result<SettingsView> {
    settings_view(&app, &db)
}

/// Registra el nuevo atajo y solo entonces lo guarda. Si no se puede
/// registrar, se vuelve a activar el anterior y se devuelve el error.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, db: State<'_, Db>, shortcut: String) -> Result<SettingsView> {
    let accel = shortcut.trim();
    shortcut::parse(accel)?;
    if let Err(e) = shortcut::register(&app, accel) {
        shortcut::register_saved(&app);
        return Err(e);
    }
    settings::set(&db.lock(), settings::SHORTCUT, accel)?;
    settings_view(&app, &db)
}

/// Desactiva el atajo mientras la interfaz graba uno nuevo, para que pulsar
/// el actual no oculte la ventana.
#[tauri::command]
pub fn suspend_shortcut(app: AppHandle) {
    shortcut::suspend(&app);
}

#[tauri::command]
pub fn resume_shortcut(app: AppHandle) {
    shortcut::register_saved(&app);
}

// --- Copia de seguridad ---
//
// Son `async` a propósito: los diálogos `blocking_*` se quedarían bloqueados
// en el hilo principal, que es donde Tauri ejecuta los comandos síncronos.

/// Diálogo de archivo encima de la ventana y en Documentos.
fn file_dialog(app: &AppHandle) -> FileDialogBuilder<tauri::Wry> {
    let mut d = app.dialog().file().add_filter("Copia de seguridad (JSON)", &["json"]);
    if let Some(w) = app.get_webview_window(window::MAIN) {
        d = d.set_parent(&w);
    }
    if let Ok(docs) = app.path().document_dir() {
        d = d.set_directory(docs);
    }
    d
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSummary {
    path: String,
    /// Tareas exportadas, o que contiene la copia.
    count: usize,
    /// Tareas que hay ahora (solo al previsualizar una importación).
    current: usize,
}

/// Pide dónde guardar y exporta. `None` si el usuario cancela.
#[tauri::command]
pub async fn export_tasks(app: AppHandle, default_name: String) -> Result<Option<FileSummary>> {
    let path = {
        let _pause = window::pause_autohide(&app);
        file_dialog(&app).set_file_name(default_name).blocking_save_file()
    };
    let Some(path) = path.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let (json, count) = backup::export(&app.state::<Db>().lock(), db::now_ms())?;
    std::fs::write(&path, json)?;
    Ok(Some(FileSummary { path: path.display().to_string(), count, current: count }))
}

/// Pide un archivo y lo valida sin importarlo, para que la interfaz pida
/// confirmación con los números delante. `None` si el usuario cancela.
#[tauri::command]
pub async fn preview_import(app: AppHandle) -> Result<Option<FileSummary>> {
    let path = {
        let _pause = window::pause_autohide(&app);
        file_dialog(&app).blocking_pick_file()
    };
    let Some(path) = path.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let backup = backup::parse(&std::fs::read_to_string(&path)?)?;
    let current = tasks::list(&app.state::<Db>().lock())?.len();
    Ok(Some(FileSummary { path: path.display().to_string(), count: backup.tasks.len(), current }))
}

/// Reemplaza todas las tareas por las del archivo (ya confirmado).
#[tauri::command]
pub async fn import_tasks(app: AppHandle, path: String) -> Result<usize> {
    let backup = backup::parse(&std::fs::read_to_string(PathBuf::from(path))?)?;
    let count = backup::import(&mut app.state::<Db>().lock(), &backup)?;
    app.state::<Scheduler>().reschedule();
    Ok(count)
}
