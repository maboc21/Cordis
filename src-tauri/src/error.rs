//! Error común del backend. Los comandos de Tauri lo devuelven serializado
//! como texto, así que los mensajes están en español y son legibles.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Db(rusqlite::Error),
    Io(std::io::Error),
    /// No existe ninguna tarea con ese id.
    NotFound(i64),
    /// Datos de entrada no válidos; el texto se muestra tal cual al usuario.
    Invalid(String),
    /// La base de datos tiene un esquema más nuevo que el que conoce esta versión.
    DbTooNew { found: usize, supported: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Db(e) => write!(f, "Error de base de datos: {e}"),
            Error::Io(e) => write!(f, "Error de archivo: {e}"),
            Error::NotFound(id) => write!(f, "No existe la tarea {id}"),
            Error::Invalid(msg) => f.write_str(msg),
            Error::DbTooNew { found, supported } => write!(
                f,
                "La base de datos es de una versión más nueva de la app \
                 (esquema {found}; esta versión admite hasta el {supported})"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Db(e) => Some(e),
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Db(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
