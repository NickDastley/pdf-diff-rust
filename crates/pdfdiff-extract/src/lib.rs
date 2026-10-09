//! PDF-to-IR extraction.
//!
//! Defines the [`Extractor`] trait: the seam between the PDF engine and the
//! diffing logic. Downstream code never sees a PDF, only the resulting
//! [`DocumentIr`], which keeps the backend swappable and the algorithms easy to
//! test.

use std::path::Path;

use pdfdiff_core::{DiffConfig, DocumentIr};

/// Anything that can turn a PDF file into a document IR.
///
/// Implementations are object-safe (`&self`, no generic methods), so callers can
/// hold a `Box<dyn Extractor>` and choose a backend at runtime.
pub trait Extractor {
    /// Extract the IR for the PDF at `path`, honouring extraction configuration.
    fn extract(&self, path: &Path, config: &DiffConfig) -> Result<DocumentIr, ExtractError>;
}

/// Errors produced while extracting a PDF.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("failed to open PDF {path}: {reason}")]
    Open { path: String, reason: String },

    #[error("PDF backend error: {0}")]
    Backend(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
