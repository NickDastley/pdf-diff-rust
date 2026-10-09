//! Error types for `pdfdiff-core`.

use std::path::PathBuf;

/// Everything that can go wrong while loading or validating configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// A numeric field fell outside its allowed range, e.g. a probability > 1.
    #[error("{field} must be within [{min}, {max}], got {value}")]
    OutOfRange {
        field: &'static str,
        min: f64,
        max: f64,
        value: f64,
    },

    /// A count-like field was negative.
    #[error("{field} must be non-negative, got {value}")]
    Negative { field: &'static str, value: i64 },

    /// An environment variable was set but could not be parsed.
    #[error("invalid value for {env}: {value} ({reason})")]
    InvalidEnv {
        env: &'static str,
        value: String,
        reason: String,
    },

    /// The configuration file could not be read from disk.
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The configuration file was not valid TOML or did not match the schema.
    #[error("failed to parse config TOML: {0}")]
    Toml(#[from] toml::de::Error),
}
