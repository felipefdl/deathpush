use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceOs {
  Macos,
  Windows,
  Linux,
}

pub(crate) fn assets_dir() -> PathBuf {
  let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets");
  match std::env::current_exe() {
    Ok(exe) => resolve_resource_dir(&exe, host_os(), &fallback),
    Err(_) => fallback,
  }
}

pub(crate) fn host_os() -> ResourceOs {
  if cfg!(target_os = "macos") {
    ResourceOs::Macos
  } else if cfg!(target_os = "windows") {
    ResourceOs::Windows
  } else {
    ResourceOs::Linux
  }
}

/// Packaged macOS: `Contents/Resources` next to `Contents/MacOS/<exe>`.
/// Packaged Windows: the install directory (parent of the exe).
/// Linux and development builds: `fallback` (source-tree `assets`).
pub(crate) fn resolve_resource_dir(exe: &Path, os: ResourceOs, fallback: &Path) -> PathBuf {
  match os {
    ResourceOs::Macos => macos_bundle_resources(exe).unwrap_or_else(|| fallback.to_path_buf()),
    ResourceOs::Windows => windows_install_dir(exe).unwrap_or_else(|| fallback.to_path_buf()),
    ResourceOs::Linux => fallback.to_path_buf(),
  }
}

fn macos_bundle_resources(exe: &Path) -> Option<PathBuf> {
  let macos = exe.parent()?;
  if macos.file_name() != Some(OsStr::new("MacOS")) {
    return None;
  }
  let contents = macos.parent()?;
  if contents.file_name() != Some(OsStr::new("Contents")) {
    return None;
  }
  Some(contents.join("Resources"))
}

fn windows_install_dir(exe: &Path) -> Option<PathBuf> {
  let parent = exe.parent()?;
  if is_cargo_target_dir(parent) {
    return None;
  }
  Some(parent.to_path_buf())
}

fn is_cargo_target_dir(dir: &Path) -> bool {
  let Some(profile) = dir.file_name() else {
    return false;
  };
  if profile != "debug" && profile != "release" {
    return false;
  }
  let Some(parent) = dir.parent() else {
    return false;
  };
  parent.file_name() == Some(OsStr::new("target"))
    || parent.parent().and_then(|p| p.file_name()) == Some(OsStr::new("target"))
}

#[cfg(test)]
mod tests {
  use super::{ResourceOs, resolve_resource_dir};
  use std::path::Path;

  #[test]
  fn macos_bundle_points_at_contents_resources() {
    let exe = Path::new("/Applications/DeathPush.app/Contents/MacOS/deathpush");
    let fallback = Path::new("/src/assets");
    assert_eq!(
      resolve_resource_dir(exe, ResourceOs::Macos, fallback),
      Path::new("/Applications/DeathPush.app/Contents/Resources")
    );
  }

  #[test]
  fn macos_dev_uses_fallback() {
    let exe = Path::new("/src/target/debug/deathpush");
    let fallback = Path::new("/src/assets");
    assert_eq!(resolve_resource_dir(exe, ResourceOs::Macos, fallback), fallback);
  }

  #[test]
  fn windows_install_dir_is_next_to_the_exe() {
    let exe = Path::new("C:/Users/x/AppData/Local/DeathPush/deathpush.exe");
    let fallback = Path::new("C:/src/assets");
    assert_eq!(
      resolve_resource_dir(exe, ResourceOs::Windows, fallback),
      Path::new("C:/Users/x/AppData/Local/DeathPush")
    );
  }

  #[test]
  fn windows_dev_uses_fallback() {
    let exe = Path::new("C:/src/target/debug/deathpush.exe");
    let fallback = Path::new("C:/src/assets");
    assert_eq!(resolve_resource_dir(exe, ResourceOs::Windows, fallback), fallback);
  }

  #[test]
  fn windows_cross_target_uses_fallback() {
    let exe = Path::new("C:/src/target/x86_64-pc-windows-msvc/release/deathpush.exe");
    let fallback = Path::new("C:/src/assets");
    assert_eq!(resolve_resource_dir(exe, ResourceOs::Windows, fallback), fallback);
  }

  #[test]
  fn linux_uses_fallback() {
    let exe = Path::new("/usr/bin/deathpush");
    let fallback = Path::new("/src/assets");
    assert_eq!(resolve_resource_dir(exe, ResourceOs::Linux, fallback), fallback);
  }
}
