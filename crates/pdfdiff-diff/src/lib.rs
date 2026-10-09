//! Structural diffing of two document IRs.
//!
//! Depends only on `pdfdiff-core`; it never touches a PDF backend.

/// Errors produced while diffing two document IRs.
#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("document IR error: {0}")]
    Ir(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
