//! Configuration loading.
//!
//! Precedence, highest wins per field: environment > TOML file > defaults.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

/// Effective diff configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    // Similarity thresholds
    pub hard_rewrite_min: f64,
    pub short_text_rewrite_min: f64,
    pub short_text_token_limit: i32,

    // Matching behaviour
    pub page_proximity_weight: f64,
    pub max_token_delta: i32,

    // First-character filter (performance optimisation)
    pub first_char_filter_min_tokens: i32,

    // Recurring element detection (headers/footers)
    pub recurring_frequency_threshold: f64,
    pub recurring_vertical_margin: f64,

    // Move detection
    pub detect_moved_blocks: bool,

    // Image matching parameters
    pub image_similarity_threshold: f64,
    pub image_max_size_delta: f64,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            hard_rewrite_min: 0.60,
            short_text_rewrite_min: 0.50,
            short_text_token_limit: 10,
            page_proximity_weight: 0.12,
            max_token_delta: 20,
            first_char_filter_min_tokens: 3,
            recurring_frequency_threshold: 0.75,
            recurring_vertical_margin: 0.20,
            detect_moved_blocks: true,
            image_similarity_threshold: 0.70,
            image_max_size_delta: 0.35,
        }
    }
}

impl DiffConfig {
    /// Validate every field, returning the first problem found.
    pub fn validate(&self) -> Result<(), ConfigError> {
        check_unit("hard_rewrite_min", self.hard_rewrite_min)?;
        check_unit("short_text_rewrite_min", self.short_text_rewrite_min)?;
        check_non_negative("short_text_token_limit", self.short_text_token_limit)?;
        check_unit("page_proximity_weight", self.page_proximity_weight)?;
        check_non_negative("max_token_delta", self.max_token_delta)?;
        check_non_negative(
            "first_char_filter_min_tokens",
            self.first_char_filter_min_tokens,
        )?;
        check_unit(
            "recurring_frequency_threshold",
            self.recurring_frequency_threshold,
        )?;
        check_range(
            "recurring_vertical_margin",
            self.recurring_vertical_margin,
            0.0,
            0.5,
        )?;
        check_unit(
            "image_similarity_threshold",
            self.image_similarity_threshold,
        )?;
        check_unit("image_max_size_delta", self.image_max_size_delta)?;
        Ok(())
    }

    /// Parse a TOML document, applying defaults for missing fields.
    ///
    /// Expected shape:
    ///
    /// ```toml
    /// [diff]
    /// max_token_delta = 5
    /// ```
    pub fn from_toml_str(contents: &str) -> Result<Self, ConfigError> {
        // A small wrapper so only the `[diff]` table is read; unknown top-level
        // keys are ignored.
        #[derive(Deserialize)]
        struct ConfigFile {
            #[serde(default)]
            diff: DiffConfig,
        }

        let file: ConfigFile = toml::from_str(contents)?;
        Ok(file.diff)
    }

    /// Overwrite this config from a TOML string.
    pub fn apply_toml_str(&mut self, contents: &str) -> Result<(), ConfigError> {
        *self = Self::from_toml_str(contents)?;
        Ok(())
    }

    /// Apply environment overrides, using `get` as the environment lookup.
    ///
    /// `get` is injected rather than reading the process environment directly so
    /// that callers can supply an environment of their choosing. Empty values
    /// are ignored; values that fail to parse produce [`ConfigError::InvalidEnv`].
    /// `PDF_DIFF_DETECT_MOVED_BLOCKS` is true when its lowercased value is
    /// `"true"`, `"1"`, or `"yes"`, and false otherwise.
    pub fn apply_env(&mut self, get: impl Fn(&str) -> Option<String>) -> Result<(), ConfigError> {
        if let Some(v) = env_f64(&get, "PDF_DIFF_SIMILARITY_REWRITE_MIN")? {
            self.hard_rewrite_min = v;
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_SHORT_TEXT_REWRITE_MIN")? {
            self.short_text_rewrite_min = v;
        }
        if let Some(v) = env_i32(&get, "PDF_DIFF_SHORT_TEXT_TOKEN_LIMIT")? {
            self.short_text_token_limit = v;
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_PAGE_PROXIMITY_WEIGHT")? {
            self.page_proximity_weight = v;
        }
        if let Some(v) = env_i32(&get, "PDF_DIFF_MAX_TOKEN_DELTA")? {
            self.max_token_delta = v;
        }
        if let Some(v) = env_i32(&get, "PDF_DIFF_FIRST_CHAR_FILTER_MIN_TOKENS")? {
            self.first_char_filter_min_tokens = v;
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_RECURRING_FREQUENCY_THRESHOLD")? {
            self.recurring_frequency_threshold = v;
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_RECURRING_VERTICAL_MARGIN")? {
            self.recurring_vertical_margin = v;
        }
        if let Some(v) = env_value(&get, "PDF_DIFF_DETECT_MOVED_BLOCKS") {
            self.detect_moved_blocks =
                matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_IMAGE_SIMILARITY_THRESHOLD")? {
            self.image_similarity_threshold = v;
        }
        if let Some(v) = env_f64(&get, "PDF_DIFF_IMAGE_MAX_SIZE_DELTA")? {
            self.image_max_size_delta = v;
        }
        Ok(())
    }

    /// Serialize into a JSON value for embedding in a diff report.
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("DiffConfig is always serializable")
    }
}

/// Load configuration from an optional TOML path plus the process environment,
/// then validate it. A `None` or non-existent path is skipped.
pub fn load_config(path: Option<&Path>) -> Result<DiffConfig, ConfigError> {
    let mut cfg = DiffConfig::default();

    if let Some(path) = path {
        if path.exists() {
            let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            cfg.apply_toml_str(&contents)?;
        }
    }

    cfg.apply_env(|key| std::env::var(key).ok())?;
    cfg.validate()?;
    Ok(cfg)
}

fn env_value(get: &impl Fn(&str) -> Option<String>, key: &str) -> Option<String> {
    match get(key) {
        Some(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn env_f64(
    get: &impl Fn(&str) -> Option<String>,
    env: &'static str,
) -> Result<Option<f64>, ConfigError> {
    match env_value(get, env) {
        Some(value) => value
            .parse::<f64>()
            .map(Some)
            .map_err(|err| ConfigError::InvalidEnv {
                env,
                value,
                reason: err.to_string(),
            }),
        None => Ok(None),
    }
}

fn env_i32(
    get: &impl Fn(&str) -> Option<String>,
    env: &'static str,
) -> Result<Option<i32>, ConfigError> {
    match env_value(get, env) {
        Some(value) => value
            .parse::<i32>()
            .map(Some)
            .map_err(|err| ConfigError::InvalidEnv {
                env,
                value,
                reason: err.to_string(),
            }),
        None => Ok(None),
    }
}

fn check_unit(field: &'static str, value: f64) -> Result<(), ConfigError> {
    check_range(field, value, 0.0, 1.0)
}

fn check_range(field: &'static str, value: f64, min: f64, max: f64) -> Result<(), ConfigError> {
    if !(min..=max).contains(&value) {
        return Err(ConfigError::OutOfRange {
            field,
            min,
            max,
            value,
        });
    }
    Ok(())
}

fn check_non_negative(field: &'static str, value: i32) -> Result<(), ConfigError> {
    if value < 0 {
        return Err(ConfigError::Negative {
            field,
            value: i64::from(value),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn defaults_are_expected() {
        let cfg = DiffConfig::default();
        assert_eq!(cfg.hard_rewrite_min, 0.60);
        assert_eq!(cfg.short_text_rewrite_min, 0.50);
        assert_eq!(cfg.short_text_token_limit, 10);
        assert_eq!(cfg.page_proximity_weight, 0.12);
        assert_eq!(cfg.max_token_delta, 20);
        assert_eq!(cfg.first_char_filter_min_tokens, 3);
        assert_eq!(cfg.recurring_frequency_threshold, 0.75);
        assert_eq!(cfg.recurring_vertical_margin, 0.20);
        assert!(cfg.detect_moved_blocks);
        assert_eq!(cfg.image_similarity_threshold, 0.70);
        assert_eq!(cfg.image_max_size_delta, 0.35);
        cfg.validate().expect("defaults are valid");
    }

    #[test]
    fn toml_then_env_precedence() {
        let toml = "[diff]\nhard_rewrite_min = 0.45\npage_proximity_weight = 0.15\n";
        let mut cfg = DiffConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.hard_rewrite_min, 0.45);
        assert_eq!(cfg.page_proximity_weight, 0.15);

        let e = env(&[("PDF_DIFF_SIMILARITY_REWRITE_MIN", "0.50")]);
        cfg.apply_env(|k| e.get(k).cloned()).unwrap();
        assert_eq!(cfg.hard_rewrite_min, 0.50);
        assert_eq!(cfg.page_proximity_weight, 0.15);
    }

    #[test]
    fn toml_missing_fields_fall_back_to_defaults() {
        let cfg = DiffConfig::from_toml_str("[diff]\nmax_token_delta = 5").unwrap();
        assert_eq!(cfg.max_token_delta, 5);
        assert_eq!(cfg.hard_rewrite_min, 0.60);
    }

    #[test]
    fn validation_rejects_out_of_range() {
        let err = DiffConfig {
            hard_rewrite_min: 1.5,
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::OutOfRange {
                field: "hard_rewrite_min",
                ..
            }
        ));
    }

    #[test]
    fn validation_rejects_negative_counts() {
        let err = DiffConfig {
            max_token_delta: -1,
            ..Default::default()
        }
        .validate()
        .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Negative {
                field: "max_token_delta",
                value: -1
            }
        ));
    }

    #[test]
    fn env_bool_accepts_true_variants() {
        for raw in ["true", "1", "yes", "TRUE", "Yes"] {
            let e = env(&[("PDF_DIFF_DETECT_MOVED_BLOCKS", raw)]);
            let mut cfg = DiffConfig::default();
            cfg.apply_env(|k| e.get(k).cloned()).unwrap();
            assert!(cfg.detect_moved_blocks, "expected true for {raw:?}");
        }
    }

    #[test]
    fn env_bool_rejects_other_variants_as_false() {
        for raw in ["false", "0", "no", "anything-else"] {
            let e = env(&[("PDF_DIFF_DETECT_MOVED_BLOCKS", raw)]);
            let mut cfg = DiffConfig::default();
            cfg.apply_env(|k| e.get(k).cloned()).unwrap();
            assert!(!cfg.detect_moved_blocks, "expected false for {raw:?}");
        }
    }

    #[test]
    fn empty_env_values_are_ignored() {
        let e = env(&[("PDF_DIFF_MAX_TOKEN_DELTA", "")]);
        let mut cfg = DiffConfig::default();
        cfg.apply_env(|k| e.get(k).cloned()).unwrap();
        assert_eq!(cfg.max_token_delta, 20);
    }

    #[test]
    fn invalid_env_value_is_an_error() {
        let e = env(&[("PDF_DIFF_MAX_TOKEN_DELTA", "not-a-number")]);
        let mut cfg = DiffConfig::default();
        let err = cfg.apply_env(|k| e.get(k).cloned()).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::InvalidEnv {
                env: "PDF_DIFF_MAX_TOKEN_DELTA",
                ..
            }
        ));
    }

    #[test]
    fn to_dict_contains_all_fields() {
        let value = DiffConfig::default().to_dict();
        for field in [
            "hard_rewrite_min",
            "short_text_rewrite_min",
            "short_text_token_limit",
            "page_proximity_weight",
            "max_token_delta",
            "first_char_filter_min_tokens",
            "recurring_frequency_threshold",
            "recurring_vertical_margin",
            "detect_moved_blocks",
            "image_similarity_threshold",
            "image_max_size_delta",
        ] {
            assert!(value.get(field).is_some(), "missing field {field}");
        }
    }
}
