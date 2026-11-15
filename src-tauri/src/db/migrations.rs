//! Migraciones versionadas con `PRAGMA user_version`.
//!
//! Cada elemento de `MIGRATIONS` lleva el esquema de la versión `i` a la `i + 1`.
//! **Nunca edites una migración ya publicada**: añade otra al final.

use rusqlite::Connection;

use crate::error::{Error, Result};

const MIGRATIONS: &[&str] = &[
    // 1: esquema inicial.
    "CREATE TABLE tasks (
        id           INTEGER PRIMARY KEY,
        title        TEXT    NOT NULL,
        notes        TEXT,
        position     INTEGER NOT NULL,
        done         INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
        created_at   INTEGER NOT NULL,
        completed_at INTEGER,
        remind_at    INTEGER,
        reminded     INTEGER NOT NULL DEFAULT 0 CHECK (reminded IN (0, 1))
    );
    CREATE INDEX idx_tasks_position ON tasks (position);
    CREATE INDEX idx_tasks_pending_reminders ON tasks (remind_at)
        WHERE remind_at IS NOT NULL AND reminded = 0 AND done = 0;

    CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
    // 2: tareas urgentes (van arriba del todo en la lista).
    "ALTER TABLE tasks ADD COLUMN urgent INTEGER NOT NULL DEFAULT 0 CHECK (urgent IN (0, 1));",
];

/// Versión de esquema que conoce esta build.
pub const LATEST: usize = MIGRATIONS.len();

pub fn current_version(conn: &Connection) -> Result<usize> {
    let v: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    Ok(v as usize)
}

/// Aplica en orden las migraciones pendientes, cada una en su transacción.
pub fn run(conn: &mut Connection) -> Result<()> {
    let current = current_version(conn)?;
    if current > LATEST {
        return Err(Error::DbTooNew { found: current, supported: LATEST });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_nueva_queda_en_la_ultima_version() {
        let conn = crate::db::open_in_memory().unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST);
    }

    #[test]
    fn ejecutar_dos_veces_no_hace_nada() {
        let mut conn = crate::db::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST);
    }

    #[test]
    fn rechaza_una_base_mas_nueva() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", (LATEST + 1) as i64).unwrap();
        assert!(matches!(run(&mut conn), Err(Error::DbTooNew { .. })));
    }

    #[test]
    fn una_base_v1_con_datos_se_actualiza_sin_perderlos() {
        let mut conn = Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute_batch(MIGRATIONS[0]).unwrap();
        tx.pragma_update(None, "user_version", 1).unwrap();
        tx.execute("INSERT INTO tasks (title, position, created_at) VALUES ('vieja', 0, 1)", [])
            .unwrap();
        tx.commit().unwrap();

        run(&mut conn).unwrap();
        assert_eq!(current_version(&conn).unwrap(), LATEST);
        let (title, urgent): (String, bool) = conn
            .query_row("SELECT title, urgent FROM tasks", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((title.as_str(), urgent), ("vieja", false));
    }

    #[test]
    fn una_migracion_fallida_no_cambia_la_version() {
        // Simula una base a medio migrar: la tabla ya existe pero la versión es 0.
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE tasks (id INTEGER)").unwrap();
        assert!(run(&mut conn).is_err());
        assert_eq!(current_version(&conn).unwrap(), 0);
    }
}
