mod commands;
mod db;
mod error;
mod reminders;
mod shortcut;
mod tray;
mod window;

use tauri::Manager;

/// Nombre del fichero SQLite dentro de la carpeta de datos de la app.
const DB_FILE: &str = "tareas.db";

/// Argumentos de línea de comandos.
/// `--toggle`: muestra u oculta la ventana de la instancia que ya está abierta
///   (para asignarlo a una tecla en Wayland, donde no hay atajos globales).
/// `--hidden`: arranca en la bandeja sin mostrar la ventana (autoarranque).
const ARG_TOGGLE: &str = "--toggle";
const ARG_HIDDEN: &str = "--hidden";

pub fn run() {
    tauri::Builder::default()
        // Debe ser el primer plugin: si ya hay una instancia, este proceso le
        // pasa sus argumentos y termina antes de crear nada.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a == ARG_TOGGLE) {
                window::toggle(app);
            } else {
                window::show(app, window::View::List);
            }
        }))
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join(DB_FILE))?;
            app.manage(db::Db::new(conn));

            let handle = app.handle();
            window::init(handle);
            tray::init(handle)?;
            app.manage(shortcut::Status::default());
            let shortcut_ok = shortcut::register_saved(handle);
            // Primera vuelta inmediata: avisa lo que venció con la app cerrada.
            reminders::start(handle);

            if !shortcut_ok && !shortcut::is_wayland() {
                // Sin atajo, la única vía para abrir la ventana sería la bandeja,
                // que en Windows suele quedar escondida. Se abren los Ajustes
                // (aunque sea con --hidden) para que el usuario elija otro.
                window::show(handle, window::View::Settings);
            } else if !std::env::args().any(|a| a == ARG_HIDDEN) {
                window::show(handle, window::View::List);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_version,
            commands::list_tasks,
            commands::create_task,
            commands::rename_task,
            commands::set_task_notes,
            commands::set_task_done,
            commands::set_task_reminder,
            commands::delete_task,
            commands::reorder_tasks,
            commands::hide_window,
            commands::current_view,
            commands::get_settings,
            commands::set_shortcut,
            commands::suspend_shortcut,
            commands::resume_shortcut,
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación");
}
