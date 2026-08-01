//! The error type returned by fallible [`Engine`](super::Engine) operations.

use std::fmt;

/// Everything that can go wrong while loading or running an [`Engine`](super::Engine).
#[derive(Debug)]
pub enum EcsError {
    /// The YAML definition file could not be read.
    Io(std::io::Error),
    /// The YAML definition file's content did not match the expected shape.
    Yaml(serde_yaml_ng::Error),
    /// A SQLite operation outside of a named system (schema creation or an
    /// entity helper) failed.
    Sql(rusqlite::Error),
    /// A named system's SQL failed while running.
    System {
        /// The failing system's name, as declared in the YAML definition.
        name: String,
        /// The underlying SQLite error.
        source: rusqlite::Error,
    },
}

impl fmt::Display for EcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EcsError::Io(err) => write!(f, "failed to read the game definition file: {err}"),
            EcsError::Yaml(err) => write!(f, "failed to parse the game definition YAML: {err}"),
            EcsError::Sql(err) => write!(f, "a SQLite operation failed: {err}"),
            EcsError::System { name, source } => {
                write!(f, "system {name:?} failed: {source}")
            }
        }
    }
}

impl std::error::Error for EcsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EcsError::Io(err) => Some(err),
            EcsError::Yaml(err) => Some(err),
            EcsError::Sql(err) => Some(err),
            EcsError::System { source, .. } => Some(source),
        }
    }
}

impl From<std::io::Error> for EcsError {
    fn from(err: std::io::Error) -> Self {
        EcsError::Io(err)
    }
}

impl From<serde_yaml_ng::Error> for EcsError {
    fn from(err: serde_yaml_ng::Error) -> Self {
        EcsError::Yaml(err)
    }
}

impl From<rusqlite::Error> for EcsError {
    fn from(err: rusqlite::Error) -> Self {
        EcsError::Sql(err)
    }
}
