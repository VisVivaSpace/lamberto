use std::fmt;

/// Errors that can occur during configuration, ephemeris loading, solving, or output.
#[derive(Debug)]
pub enum LambertoError {
    /// Invalid or missing configuration.
    Config(String),
    /// Ephemeris query or body resolution failure.
    Ephemeris(String),
    /// Lambert solver failure.
    Solver(String),
    /// File or I/O error during output writing.
    Output(String),
}

impl fmt::Display for LambertoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LambertoError::Config(msg) => write!(f, "config error: {msg}"),
            LambertoError::Ephemeris(msg) => write!(f, "ephemeris error: {msg}"),
            LambertoError::Solver(msg) => write!(f, "solver error: {msg}"),
            LambertoError::Output(msg) => write!(f, "output error: {msg}"),
        }
    }
}

impl std::error::Error for LambertoError {}
