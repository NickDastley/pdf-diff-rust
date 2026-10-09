//! Core types, configuration, and shared utilities for `pdf-diff`.
//!
//! This crate is the foundation of the workspace: the in-memory document
//! representation ("IR"), configuration handling, stable identifiers, and text
//! normalisation. It depends on almost nothing else so that the extraction and
//! diffing crates can both build on it without pulling in a PDF backend.

pub mod config;
pub mod error;
pub mod ids;
pub mod model;
pub mod text;

pub use config::{DiffConfig, load_config};
pub use error::ConfigError;
pub use ids::{build_element_id, round_half_to_even, sha1_hex};
pub use model::{
    DocumentIr, Element, ElementContent, ElementLayout, ElementType, ImageContent, PageMeta,
    TableCell, TableContent, TextAlignment, TextBlockContent, sorted_elements,
};
pub use text::normalize_text;
