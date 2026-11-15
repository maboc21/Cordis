//! Comandos expuestos a la interfaz. Son finos: cogen la conexión, leen el
//! reloj si hace falta y delegan en `db`. Desde JS los argumentos van en
//! camelCase (`remindAt`), como hace Tauri por defecto.

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::db::{self, settings, tasks, tasks::Task, Db};
use crate::error::Result;
use crate::reminders::Scheduler;
use crate::{shortcut, window};

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
    db: State<'_, Db>,
    scheduler: State<'_, Scheduler>,
    id: i64,
    done: bool,
) -> Result<Task> {
    let task = tasks::set_done(&db.lock(), id, done, db::now_ms())?;
    scheduler.reschedule();
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
pub fn delete_task(db: State<'_, Db>, scheduler: State<'_, Scheduler>, id: i64) -> Result<()> {
    tasks::delete(&db.lock(), id)?;
    scheduler.reschedule();
    Ok(())
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
}

fn settings_view(app: &AppHandle, db: &Db) -> Result<SettingsView> {
    let shortcut = settings::shortcut(&db.lock())?;
    Ok(SettingsView {
        shortcut_active: shortcut::is_active(app, &shortcut),
        shortcut_error: app.state::<shortcut::Status>().error(),
        shortcut,
        default_shortcut: settings::DEFAULT_SHORTCUT,
        wayland: shortcut::is_wayland(),
    })
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
