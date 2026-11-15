//! Acceso a SQLite. Una única conexión compartida tras un `Mutex`: las
//! operaciones son de microsegundos y así el planificador de recordatorios
//! y los comandos ven siempre el mismo estado.

pub mod migrations;
pub mod settings;
pub mod tasks;

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::Result;

/// Estado gestionado por Tauri (`app.manage`).
pub struct Db(Mutex<Connection>);

impl Db {
    pub fn new(conn: Connection) -> Self {
        Db(Mutex::new(conn))
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        // Un pánico con el candado cogido no deja la conexión inservible:
        // SQLite deshace la transacción a medias. Seguimos usándola.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Abre (o crea) la base de datos y aplica las migraciones pendientes.
pub fn open(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    // journal_mode devuelve una fila con el modo resultante ("memory" en tests).
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

/// Instante actual en milisegundos Unix (UTC).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
