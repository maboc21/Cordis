//! CRUD de tareas y reordenación.
//!
//! Las funciones reciben la conexión (y `now` cuando lo necesitan) en lugar
//! de leer el reloj, para poder testearlas de forma determinista.

use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Params, Row};
use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub notes: Option<String>,
    pub position: i64,
    pub done: bool,
    /// ms Unix UTC.
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub remind_at: Option<i64>,
    /// Ya se ha mostrado la notificación de `remind_at`.
    pub reminded: bool,
}

const COLUMNS: &str =
    "id, title, notes, position, done, created_at, completed_at, remind_at, reminded";

fn from_row(r: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        title: r.get(1)?,
        notes: r.get(2)?,
        position: r.get(3)?,
        done: r.get(4)?,
        created_at: r.get(5)?,
        completed_at: r.get(6)?,
        remind_at: r.get(7)?,
        reminded: r.get(8)?,
    })
}

fn clean_title(title: &str) -> Result<String> {
    let t = title.trim();
    if t.is_empty() {
        return Err(Error::Invalid("El título no puede estar vacío".into()));
    }
    Ok(t.to_owned())
}

fn clean_notes(notes: Option<&str>) -> Option<String> {
    notes.map(str::trim).filter(|n| !n.is_empty()).map(str::to_owned)
}

/// Ejecuta un UPDATE/DELETE sobre una tarea y falla si no existe.
fn exec_on(conn: &Connection, id: i64, sql: &str, p: impl Params) -> Result<()> {
    match conn.execute(sql, p)? {
        0 => Err(Error::NotFound(id)),
        _ => Ok(()),
    }
}

/// Todas las tareas en el orden de la lista (hechas incluidas).
pub fn list(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt =
        conn.prepare_cached(&format!("SELECT {COLUMNS} FROM tasks ORDER BY position, id"))?;
    let tasks = stmt.query_map([], from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tasks)
}

pub fn get(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(&format!("SELECT {COLUMNS} FROM tasks WHERE id = ?1"), [id], from_row)
        .optional()?
        .ok_or(Error::NotFound(id))
}

/// Crea una tarea al final de la lista.
pub fn create(conn: &Connection, title: &str, now: i64) -> Result<Task> {
    let title = clean_title(title)?;
    conn.execute(
        "INSERT INTO tasks (title, position, created_at)
         VALUES (?1, (SELECT COALESCE(MAX(position), -1) + 1 FROM tasks), ?2)",
        params![title, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, title: &str) -> Result<Task> {
    let title = clean_title(title)?;
    exec_on(conn, id, "UPDATE tasks SET title = ?2 WHERE id = ?1", params![id, title])?;
    get(conn, id)
}

/// Notas vacías o solo con espacios se guardan como `NULL`.
pub fn set_notes(conn: &Connection, id: i64, notes: Option<&str>) -> Result<Task> {
    let notes = clean_notes(notes);
    exec_on(conn, id, "UPDATE tasks SET notes = ?2 WHERE id = ?1", params![id, notes])?;
    get(conn, id)
}

/// Marca o desmarca como hecha. Al marcar se guarda `completed_at`
/// (sin pisarlo si ya estaba hecha); al desmarcar se borra.
pub fn set_done(conn: &Connection, id: i64, done: bool, now: i64) -> Result<Task> {
    exec_on(
        conn,
        id,
        "UPDATE tasks
            SET done = ?2,
                completed_at = CASE WHEN ?2 THEN COALESCE(completed_at, ?3) ELSE NULL END
          WHERE id = ?1",
        params![id, done, now],
    )?;
    get(conn, id)
}

/// Pone o quita el recordatorio. Cambiarlo siempre rearma la notificación.
pub fn set_reminder(conn: &Connection, id: i64, remind_at: Option<i64>) -> Result<Task> {
    exec_on(
        conn,
        id,
        "UPDATE tasks SET remind_at = ?2, reminded = 0 WHERE id = ?1",
        params![id, remind_at],
    )?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    exec_on(conn, id, "DELETE FROM tasks WHERE id = ?1", [id])
}

/// Tareas con un recordatorio aún sin avisar (pendientes y no hechas),
/// por orden de vencimiento. Usa el índice parcial `idx_tasks_pending_reminders`.
pub fn pending_reminders(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare_cached(&format!(
        "SELECT {COLUMNS} FROM tasks
          WHERE remind_at IS NOT NULL AND reminded = 0 AND done = 0
          ORDER BY remind_at, id"
    ))?;
    let tasks = stmt.query_map([], from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(tasks)
}

/// Marca como avisados los recordatorios de `ids`. Los ids que ya no
/// existan (borrados entre medias) se ignoran.
pub fn mark_reminded(conn: &mut Connection, ids: &[i64]) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("UPDATE tasks SET reminded = 1 WHERE id = ?1")?;
        for id in ids {
            stmt.execute([id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Reordena las tareas `ids` para que queden en ese orden.
///
/// Las tareas reciben las mismas posiciones que ya ocupaban entre todas,
/// repartidas en el nuevo orden. Así se puede reordenar solo un subconjunto
/// (por ejemplo, las pendientes) sin mover el resto. Todo o nada: si algún id
/// no existe o está repetido, no se cambia nada.
pub fn reorder(conn: &mut Connection, ids: &[i64]) -> Result<()> {
    let mut seen = HashSet::with_capacity(ids.len());
    if let Some(dup) = ids.iter().find(|id| !seen.insert(**id)) {
        return Err(Error::Invalid(format!("La tarea {dup} aparece repetida al reordenar")));
    }

    let tx = conn.transaction()?;
    let mut slots = Vec::with_capacity(ids.len());
    {
        let mut stmt = tx.prepare("SELECT position FROM tasks WHERE id = ?1")?;
        for &id in ids {
            let pos: i64 = stmt
                .query_row([id], |r| r.get(0))
                .optional()?
                .ok_or(Error::NotFound(id))?;
            slots.push(pos);
        }
    }
    slots.sort_unstable();
    {
        let mut stmt = tx.prepare("UPDATE tasks SET position = ?2 WHERE id = ?1")?;
        for (&id, &pos) in ids.iter().zip(&slots) {
            stmt.execute(params![id, pos])?;
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    const T0: i64 = 1_700_000_000_000;

    fn titles(conn: &Connection) -> Vec<String> {
        list(conn).unwrap().into_iter().map(|t| t.title).collect()
    }

    fn seed(conn: &Connection, names: &[&str]) -> Vec<i64> {
        names.iter().map(|n| create(conn, n, T0).unwrap().id).collect()
    }

    // --- Crear y leer ---

    #[test]
    fn crear_rellena_los_valores_por_defecto() {
        let conn = open_in_memory().unwrap();
        let t = create(&conn, "Comprar pan", T0).unwrap();
        assert_eq!(t.title, "Comprar pan");
        assert_eq!(t.notes, None);
        assert!(!t.done);
        assert_eq!(t.created_at, T0);
        assert_eq!(t.completed_at, None);
        assert_eq!(t.remind_at, None);
        assert!(!t.reminded);
        assert_eq!(get(&conn, t.id).unwrap(), t);
    }

    #[test]
    fn las_nuevas_van_al_final() {
        let conn = open_in_memory().unwrap();
        seed(&conn, &["a", "b", "c"]);
        assert_eq!(titles(&conn), ["a", "b", "c"]);
        let pos: Vec<i64> = list(&conn).unwrap().iter().map(|t| t.position).collect();
        assert_eq!(pos, [0, 1, 2]);
    }

    #[test]
    fn el_titulo_se_recorta_y_no_puede_quedar_vacio() {
        let conn = open_in_memory().unwrap();
        assert_eq!(create(&conn, "  hola \n", T0).unwrap().title, "hola");
        assert!(matches!(create(&conn, "   ", T0), Err(Error::Invalid(_))));
        assert!(matches!(create(&conn, "", T0), Err(Error::Invalid(_))));
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn get_de_una_que_no_existe() {
        let conn = open_in_memory().unwrap();
        assert!(matches!(get(&conn, 42), Err(Error::NotFound(42))));
    }

    // --- Editar ---

    #[test]
    fn renombrar() {
        let conn = open_in_memory().unwrap();
        let id = create(&conn, "viejo", T0).unwrap().id;
        assert_eq!(rename(&conn, id, " nuevo ").unwrap().title, "nuevo");
        assert!(matches!(rename(&conn, id, " "), Err(Error::Invalid(_))));
        assert_eq!(get(&conn, id).unwrap().title, "nuevo");
        assert!(matches!(rename(&conn, 999, "x"), Err(Error::NotFound(999))));
    }

    #[test]
    fn notas_vacias_se_guardan_como_null() {
        let conn = open_in_memory().unwrap();
        let id = create(&conn, "t", T0).unwrap().id;
        assert_eq!(set_notes(&conn, id, Some(" detalle ")).unwrap().notes.as_deref(), Some("detalle"));
        assert_eq!(set_notes(&conn, id, Some("  ")).unwrap().notes, None);
        set_notes(&conn, id, Some("x")).unwrap();
        assert_eq!(set_notes(&conn, id, None).unwrap().notes, None);
    }

    #[test]
    fn marcar_y_desmarcar_como_hecha() {
        let conn = open_in_memory().unwrap();
        let id = create(&conn, "t", T0).unwrap().id;

        let t = set_done(&conn, id, true, T0 + 10).unwrap();
        assert!(t.done);
        assert_eq!(t.completed_at, Some(T0 + 10));

        // Volver a marcarla no cambia cuándo se completó.
        assert_eq!(set_done(&conn, id, true, T0 + 99).unwrap().completed_at, Some(T0 + 10));

        let t = set_done(&conn, id, false, T0 + 20).unwrap();
        assert!(!t.done);
        assert_eq!(t.completed_at, None);

        assert!(matches!(set_done(&conn, 999, true, T0), Err(Error::NotFound(999))));
    }

    #[test]
    fn cambiar_el_recordatorio_lo_rearma() {
        let conn = open_in_memory().unwrap();
        let id = create(&conn, "t", T0).unwrap().id;
        set_reminder(&conn, id, Some(T0 + 1000)).unwrap();
        conn.execute("UPDATE tasks SET reminded = 1 WHERE id = ?1", [id]).unwrap();

        let t = set_reminder(&conn, id, Some(T0 + 5000)).unwrap();
        assert_eq!(t.remind_at, Some(T0 + 5000));
        assert!(!t.reminded);

        let t = set_reminder(&conn, id, None).unwrap();
        assert_eq!(t.remind_at, None);
        assert!(!t.reminded);
    }

    // --- Recordatorios ---

    #[test]
    fn recordatorios_pendientes_excluye_hechas_avisadas_y_sin_fecha() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["sin", "tarde", "pronto", "hecha", "avisada"]);
        set_reminder(&conn, ids[1], Some(T0 + 900)).unwrap();
        set_reminder(&conn, ids[2], Some(T0 + 100)).unwrap();
        set_reminder(&conn, ids[3], Some(T0 + 200)).unwrap();
        set_done(&conn, ids[3], true, T0).unwrap();
        set_reminder(&conn, ids[4], Some(T0 + 300)).unwrap();
        mark_reminded(&mut conn, &[ids[4]]).unwrap();

        let titles: Vec<String> =
            pending_reminders(&conn).unwrap().into_iter().map(|t| t.title).collect();
        assert_eq!(titles, ["pronto", "tarde"]);
    }

    #[test]
    fn marcar_avisados_ignora_ids_borrados() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b"]);
        set_reminder(&conn, ids[0], Some(T0)).unwrap();
        set_reminder(&conn, ids[1], Some(T0)).unwrap();
        delete(&conn, ids[1]).unwrap();
        mark_reminded(&mut conn, &[ids[0], ids[1]]).unwrap();
        assert!(get(&conn, ids[0]).unwrap().reminded);
        assert!(pending_reminders(&conn).unwrap().is_empty());
    }

    #[test]
    fn desmarcar_una_hecha_vuelve_a_poner_su_recordatorio_en_cola() {
        let conn = open_in_memory().unwrap();
        let id = create(&conn, "t", T0).unwrap().id;
        set_reminder(&conn, id, Some(T0 + 100)).unwrap();
        set_done(&conn, id, true, T0).unwrap();
        assert!(pending_reminders(&conn).unwrap().is_empty());
        set_done(&conn, id, false, T0).unwrap();
        assert_eq!(pending_reminders(&conn).unwrap().len(), 1);
    }

    // --- Borrar ---

    #[test]
    fn borrar() {
        let conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b", "c"]);
        delete(&conn, ids[1]).unwrap();
        assert_eq!(titles(&conn), ["a", "c"]);
        assert!(matches!(delete(&conn, ids[1]), Err(Error::NotFound(_))));
        // Una nueva sigue yendo al final aunque queden huecos.
        create(&conn, "d", T0).unwrap();
        assert_eq!(titles(&conn), ["a", "c", "d"]);
    }

    // --- Reordenar ---

    #[test]
    fn reordenar_toda_la_lista() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b", "c", "d"]);
        reorder(&mut conn, &[ids[3], ids[0], ids[2], ids[1]]).unwrap();
        assert_eq!(titles(&conn), ["d", "a", "c", "b"]);
    }

    #[test]
    fn reordenar_un_subconjunto_no_mueve_el_resto() {
        let mut conn = open_in_memory().unwrap();
        // b y d están hechas; solo se reordenan las pendientes a, c, e.
        let ids = seed(&conn, &["a", "b", "c", "d", "e"]);
        reorder(&mut conn, &[ids[4], ids[0], ids[2]]).unwrap();
        assert_eq!(titles(&conn), ["e", "b", "a", "d", "c"]);
    }

    #[test]
    fn reordenar_con_huecos_tras_borrar() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b", "c", "d"]);
        delete(&conn, ids[1]).unwrap();
        reorder(&mut conn, &[ids[3], ids[2], ids[0]]).unwrap();
        assert_eq!(titles(&conn), ["d", "c", "a"]);
        create(&conn, "e", T0).unwrap();
        assert_eq!(titles(&conn), ["d", "c", "a", "e"]);
    }

    #[test]
    fn mover_una_tarea_arriba_del_todo() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b", "c"]);
        reorder(&mut conn, &[ids[2], ids[0], ids[1]]).unwrap();
        assert_eq!(titles(&conn), ["c", "a", "b"]);
    }

    #[test]
    fn reordenar_con_ids_repetidos_o_inexistentes_no_cambia_nada() {
        let mut conn = open_in_memory().unwrap();
        let ids = seed(&conn, &["a", "b", "c"]);

        let err = reorder(&mut conn, &[ids[1], ids[0], ids[1]]);
        assert!(matches!(err, Err(Error::Invalid(_))));
        assert_eq!(titles(&conn), ["a", "b", "c"]);

        let err = reorder(&mut conn, &[ids[2], 999, ids[0]]);
        assert!(matches!(err, Err(Error::NotFound(999))));
        assert_eq!(titles(&conn), ["a", "b", "c"]);
    }

    #[test]
    fn reordenar_una_lista_vacia_no_hace_nada() {
        let mut conn = open_in_memory().unwrap();
        seed(&conn, &["a", "b"]);
        reorder(&mut conn, &[]).unwrap();
        assert_eq!(titles(&conn), ["a", "b"]);
    }

    #[test]
    fn las_posiciones_siguen_siendo_unicas_tras_muchas_operaciones() {
        let mut conn = open_in_memory().unwrap();
        let mut ids = seed(&conn, &["a", "b", "c", "d", "e", "f"]);
        for i in 0..20 {
            ids.rotate_left(i % 4 + 1);
            reorder(&mut conn, &ids[..4]).unwrap();
            if i % 5 == 0 {
                delete(&conn, ids.pop().unwrap()).unwrap();
                ids.push(create(&conn, "n", T0).unwrap().id);
            }
        }
        let mut pos: Vec<i64> = list(&conn).unwrap().iter().map(|t| t.position).collect();
        let n = pos.len();
        pos.dedup();
        assert_eq!(pos.len(), n, "posiciones duplicadas");
    }
}
