//! Ajustes como pares clave/valor en la tabla `settings`.
//! Cada ajuste tiene su accesor tipado con su valor por defecto.

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::Result;

pub const SHORTCUT: &str = "shortcut";
/// No es Ctrl+Alt+Space porque lo usa Claude Desktop, y las combinaciones
/// Ctrl+Alt chocan con AltGr en teclados europeos.
pub const DEFAULT_SHORTCUT: &str = "Ctrl+Shift+Space";

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Atajo global para mostrar u ocultar la ventana.
pub fn shortcut(conn: &Connection) -> Result<String> {
    Ok(get(conn, SHORTCUT)?.unwrap_or_else(|| DEFAULT_SHORTCUT.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn leer_una_clave_que_no_existe() {
        let conn = open_in_memory().unwrap();
        assert_eq!(get(&conn, "nada").unwrap(), None);
    }

    #[test]
    fn guardar_y_sobrescribir() {
        let conn = open_in_memory().unwrap();
        set(&conn, "k", "1").unwrap();
        assert_eq!(get(&conn, "k").unwrap().as_deref(), Some("1"));
        set(&conn, "k", "2").unwrap();
        assert_eq!(get(&conn, "k").unwrap().as_deref(), Some("2"));
    }

    #[test]
    fn atajo_por_defecto_y_personalizado() {
        let conn = open_in_memory().unwrap();
        assert_eq!(shortcut(&conn).unwrap(), DEFAULT_SHORTCUT);
        set(&conn, SHORTCUT, "Ctrl+Shift+T").unwrap();
        assert_eq!(shortcut(&conn).unwrap(), "Ctrl+Shift+T");
    }
}
