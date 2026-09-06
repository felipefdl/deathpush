use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::store::{read_json, write_json_atomic};
use crate::content_hash::sha256_utf8;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecentFile {
  pub path: String,
  pub last_opened: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecentFiles {
  pub files: Vec<RecentFile>,
}

pub const MAX_RECENT_FILES: usize = 20;

impl RecentFiles {
  /// Moves or inserts `path` at the front with `now`; truncates to MAX_RECENT_FILES.
  pub fn add(&mut self, path: &str, now: &str) {
    self.files.retain(|file| file.path != path);
    self.files.insert(
      0,
      RecentFile {
        path: path.to_string(),
        last_opened: now.to_string(),
      },
    );
    self.files.truncate(MAX_RECENT_FILES);
  }

  pub fn remove(&mut self, path: &str) {
    self.files.retain(|file| file.path != path);
  }

  /// Newest first.
  pub fn paths(&self) -> Vec<&str> {
    self.files.iter().map(|file| file.path.as_str()).collect()
  }
}

/// `<config dir>/projects/<first 16 hex of sha256(root)>-recent-files.json`.
pub fn recent_files_path(config_dir: &Path, root: &str) -> PathBuf {
  let hash = sha256_utf8(root);
  config_dir
    .join("projects")
    .join(format!("{}-recent-files.json", &hash[..16]))
}

pub fn load_recent_files(config_dir: &Path, root: &str) -> RecentFiles {
  read_json::<RecentFiles>(&recent_files_path(config_dir, root))
}

pub fn save_recent_files(config_dir: &Path, root: &str, files: &RecentFiles) -> Result<()> {
  write_json_atomic(&recent_files_path(config_dir, root), files)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn add_moves_to_front_and_caps() {
    let mut files = RecentFiles::default();
    for i in 0..25 {
      files.add(&format!("f{i}"), "2026-09-04T00:00:00Z");
    }
    assert_eq!(files.files.len(), MAX_RECENT_FILES);
    assert_eq!(files.paths()[0], "f24");
    files.add("f10", "2026-09-05T00:00:00Z");
    assert_eq!(files.paths()[0], "f10");
    assert_eq!(files.files.iter().filter(|f| f.path == "f10").count(), 1);
    files.remove("f10");
    assert!(!files.paths().contains(&"f10"));
  }

  #[test]
  fn round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let mut files = RecentFiles::default();
    files.add("src/main.rs", "2026-09-04T00:00:00Z");
    save_recent_files(dir.path(), "/repo", &files).unwrap();
    assert_eq!(load_recent_files(dir.path(), "/repo"), files);
    assert!(
      recent_files_path(dir.path(), "/repo")
        .to_string_lossy()
        .ends_with("-recent-files.json")
    );
  }
}
