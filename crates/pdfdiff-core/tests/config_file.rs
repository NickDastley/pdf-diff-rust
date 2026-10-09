//! Integration test: `load_config` reads a real TOML file from disk.
//!
//! Unit tests use an injected env lookup; this test covers the file path and
//! I/O error mapping, which only exists at the integration boundary.

use std::fs;
use std::path::PathBuf;

use pdfdiff_core::{DiffConfig, load_config};

/// A unique temporary directory for one test, cleaned up on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let unique = format!(
            "pdfdiff-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn loads_toml_from_disk() {
    let dir = TempDir::new("config");
    let file = dir.path().join("config.toml");
    fs::write(&file, "[diff]\npage_proximity_weight = 0.2\n").unwrap();

    let cfg = load_config(Some(file.as_path())).unwrap();
    assert_eq!(cfg.page_proximity_weight, 0.2);
    // defaults for everything else
    assert_eq!(
        cfg,
        DiffConfig {
            page_proximity_weight: 0.2,
            ..Default::default()
        }
    );
}

#[test]
fn missing_path_is_skipped() {
    let cfg = load_config(Some(std::path::Path::new("/does/not/exist.toml"))).unwrap();
    assert_eq!(cfg, DiffConfig::default());
}
