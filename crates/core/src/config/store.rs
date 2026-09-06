use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Result;

/// `~/Library/Application Support/DeathPush`, `~/.config/deathpush`, or `%APPDATA%\DeathPush`.
pub fn config_dir() -> PathBuf {
  let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
  if cfg!(target_os = "linux") {
    base.join("deathpush")
  } else {
    base.join("DeathPush")
  }
}

/// A missing or unreadable file yields the default. A corrupt file is logged and yields the default.
pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
  match std::fs::read_to_string(path) {
    Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
      tracing::warn!("ignoring corrupt {}: {err}", path.display());
      T::default()
    }),
    Err(_) => T::default(),
  }
}

/// Write to a sibling temp file, then rename over the target.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  let temp = path.with_extension("json.tmp");
  let text = serde_json::to_string_pretty(value).map_err(|err| crate::error::Error::Other(err.to_string()))?;
  std::fs::write(&temp, text)?;
  std::fs::rename(&temp, path)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde::Deserialize;

  #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
  struct Sample {
    #[serde(default)]
    count: u32,
  }

  #[test]
  fn round_trips_and_leaves_no_temp_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("nested").join("sample.json");
    write_json_atomic(&path, &Sample { count: 7 }).unwrap();
    assert_eq!(read_json::<Sample>(&path), Sample { count: 7 });
    assert!(!path.with_extension("json.tmp").exists());
  }

  #[test]
  fn missing_and_corrupt_files_yield_default() {
    let dir = tempfile::TempDir::new().unwrap();
    let missing = dir.path().join("missing.json");
    assert_eq!(read_json::<Sample>(&missing), Sample::default());
    let corrupt = dir.path().join("corrupt.json");
    std::fs::write(&corrupt, "{ not json").unwrap();
    assert_eq!(read_json::<Sample>(&corrupt), Sample::default());
  }
}
