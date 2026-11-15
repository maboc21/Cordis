//! Copia de seguridad: exportar e importar todas las tareas en JSON.
//!
//! El formato es propio y estable, independiente de la tabla: no lleva ids
//! (se regeneran) y el orden de la lista es el orden del array. Lleva
//! `version` para poder leer copias antiguas si el formato cambia.
//! Importar **reemplaza** todas las tareas, en una sola transacción.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::tasks;
use crate::error::{Error, Result};

/// Identifica los ficheros de esta app.
const APP: &str = "organizador-tareas";
/// Versión del formato. Súbela (y adapta `parse`) si cambia.
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub app: String,
    pub version: u32,
    /// ms Unix UTC.
    pub exported_at: i64,
    pub tasks: Vec<BackupTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupTask {
    pub title: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub done: bool,
    pub created_at: i64,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub remind_at: Option<i64>,
    #[serde(default)]
    pub reminded: bool,
}

/// Todas las tareas, en el orden de la lista, como JSON legible.
pub fn export(conn: &Connection, now: i64) -> Result<(String, usize)> {
    let tasks: Vec<BackupTask> = tasks::list(conn)?
        .into_iter()
        .map(|t| BackupTask {
            title: t.title,
            notes: t.notes,
            done: t.done,
            created_at: t.created_at,
            completed_at: t.completed_at,
            remind_at: t.remind_at,
            reminded: t.reminded,
        })
        .collect();
    let count = tasks.len();
    let backup = Backup { app: APP.into(), version: FORMAT_VERSION, exported_at: now, tasks };
    let json = serde_json::to_string_pretty(&backup)
        .map_err(|e| Error::Invalid(format!("No se pudo generar el JSON: {e}")))?;
    Ok((json, count))
}

/// Lee y valida una copia. No toca la base de datos.
pub fn parse(json: &str) -> Result<Backup> {
    let mut backup: Backup = serde_json::from_str(json).map_err(|e| {
        Error::Invalid(format!("El archivo no es una copia de seguridad válida ({e})"))
    })?;
    if backup.app != APP {
        return Err(Error::Invalid(
            "El archivo no es una copia de seguridad de esta aplicación".into(),
        ));
    }
    if backup.version > FORMAT_VERSION {
        return Err(Error::Invalid(format!(
            "La copia es de una versión más nueva de la app (formato {}; esta admite hasta el {FORMAT_VERSION})",
            backup.version
        )));
    }
    for (i, t) in backup.tasks.iter_mut().enumerate() {
        t.title = t.title.trim().to_owned();
        if t.title.is_empty() {
            return Err(Error::Invalid(format!("La tarea {} de la copia no tiene título", i + 1)));
        }
        t.notes = t.notes.take().map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
        // Coherencia: solo las hechas tienen fecha de completado.
        if !t.done {
            t.completed_at = None;
        }
    }
    Ok(backup)
}

/// Sustituye todas las tareas por las de la copia. Todo o nada.
pub fn import(conn: &mut Connection, backup: &Backup) -> Result<usize> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM tasks", [])?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO tasks
                (title, notes, position, done, created_at, completed_at, remind_at, reminded)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;
        for (i, t) in backup.tasks.iter().enumerate() {
            stmt.execute(params![
                t.title,
                t.notes,
                i as i64,
                t.done,
                t.created_at,
                t.completed_at,
                t.remind_at,
                t.reminded
            ])?;
        }
    }
    tx.commit()?;
    Ok(backup.tasks.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_in_memory, tasks};

    const T0: i64 = 1_700_000_000_000;

    fn sample(conn: &mut Connection) {
        let a = tasks::create(conn, "Comprar pan", T0).unwrap().id;
        let b = tasks::create(conn, "Llamar al dentista", T0 + 1).unwrap().id;
        let c = tasks::create(conn, "Sacar la basura", T0 + 2).unwrap().id;
        tasks::set_notes(conn, b, Some("Pedir cita")).unwrap();
        tasks::set_reminder(conn, b, Some(T0 + 5_000)).unwrap();
        tasks::set_done(conn, c, true, T0 + 10).unwrap();
        tasks::reorder(conn, &[c, a, b]).unwrap();
    }

    /// Lo comparable de una tarea, sin id ni posición absoluta.
    fn snapshot(conn: &Connection) -> Vec<BackupTask> {
        let (json, _) = export(conn, T0).unwrap();
        parse(&json).unwrap().tasks
    }

    #[test]
    fn exportar_e_importar_conserva_todo_y_el_orden() {
        let mut src = open_in_memory().unwrap();
        sample(&mut src);
        let (json, count) = export(&src, T0 + 99).unwrap();
        assert_eq!(count, 3);

        let mut dst = open_in_memory().unwrap();
        let backup = parse(&json).unwrap();
        assert_eq!(backup.exported_at, T0 + 99);
        assert_eq!(import(&mut dst, &backup).unwrap(), 3);

        assert_eq!(snapshot(&dst), snapshot(&src));
        let titles: Vec<String> = tasks::list(&dst).unwrap().into_iter().map(|t| t.title).collect();
        assert_eq!(titles, ["Sacar la basura", "Comprar pan", "Llamar al dentista"]);
    }

    #[test]
    fn importar_reemplaza_las_tareas_existentes() {
        let mut src = open_in_memory().unwrap();
        sample(&mut src);
        let (json, _) = export(&src, T0).unwrap();

        let mut dst = open_in_memory().unwrap();
        tasks::create(&dst, "Vieja", T0).unwrap();
        import(&mut dst, &parse(&json).unwrap()).unwrap();
        let titles: Vec<String> = tasks::list(&dst).unwrap().into_iter().map(|t| t.title).collect();
        assert!(!titles.contains(&"Vieja".to_string()));
        assert_eq!(titles.len(), 3);
        // Las nuevas siguen yendo al final.
        tasks::create(&dst, "Nueva", T0).unwrap();
        assert_eq!(tasks::list(&dst).unwrap().last().unwrap().title, "Nueva");
    }

    #[test]
    fn los_recordatorios_importados_vuelven_a_la_cola() {
        let mut src = open_in_memory().unwrap();
        sample(&mut src);
        let (json, _) = export(&src, T0).unwrap();
        let mut dst = open_in_memory().unwrap();
        import(&mut dst, &parse(&json).unwrap()).unwrap();
        let pending = tasks::pending_reminders(&dst).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].remind_at, Some(T0 + 5_000));
    }

    #[test]
    fn exportar_una_lista_vacia() {
        let conn = open_in_memory().unwrap();
        let (json, count) = export(&conn, T0).unwrap();
        assert_eq!(count, 0);
        assert!(parse(&json).unwrap().tasks.is_empty());
    }

    #[test]
    fn rechaza_json_invalido_o_de_otra_app() {
        assert!(matches!(parse("no es json"), Err(Error::Invalid(_))));
        assert!(matches!(parse("{}"), Err(Error::Invalid(_))));
        let otra = r#"{"app":"otra","version":1,"exportedAt":0,"tasks":[]}"#;
        assert!(matches!(parse(otra), Err(Error::Invalid(_))));
    }

    #[test]
    fn rechaza_una_version_mas_nueva() {
        let json = r#"{"app":"organizador-tareas","version":99,"exportedAt":0,"tasks":[]}"#;
        assert!(matches!(parse(json), Err(Error::Invalid(m)) if m.contains("más nueva")));
    }

    #[test]
    fn rechaza_tareas_sin_titulo_sin_tocar_la_base() {
        let json = r#"{"app":"organizador-tareas","version":1,"exportedAt":0,
            "tasks":[{"title":"ok","createdAt":1},{"title":"  ","createdAt":2}]}"#;
        assert!(matches!(parse(json), Err(Error::Invalid(m)) if m.contains("tarea 2")));
    }

    #[test]
    fn normaliza_campos_opcionales_y_notas_vacias() {
        let json = r#"{"app":"organizador-tareas","version":1,"exportedAt":0,
            "tasks":[{"title":" a ","notes":"  ","createdAt":1,"completedAt":5}]}"#;
        let t = &parse(json).unwrap().tasks[0];
        assert_eq!(t.title, "a");
        assert_eq!(t.notes, None);
        assert!(!t.done);
        assert_eq!(t.completed_at, None, "una pendiente no tiene fecha de completado");
        assert_eq!(t.remind_at, None);
    }
}
