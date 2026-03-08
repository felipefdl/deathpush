use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::error::Error;

const SYMLINK_NAMES: &[&str] = &["dp", "deathpush"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliInstallStatus {
  pub installed: bool,
  pub dp_path: Option<String>,
  pub deathpush_path: Option<String>,
}

fn install_dir() -> PathBuf {
  if cfg!(target_os = "windows") {
    // %LOCALAPPDATA%\DeathPush\bin -- added to PATH by the installer
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| r"C:\Users\Public\AppData\Local".into());
    PathBuf::from(local_app_data).join("DeathPush").join("bin")
  } else {
    PathBuf::from("/usr/local/bin")
  }
}

fn find_cli_path(name: &str) -> PathBuf {
  let dir = install_dir();
  if cfg!(target_os = "windows") {
    dir.join(format!("{name}.cmd"))
  } else {
    dir.join(name)
  }
}

#[tauri::command]
pub async fn check_cli_installed() -> Result<CliInstallStatus, Error> {
  let dp = find_cli_path("dp");
  let deathpush = find_cli_path("deathpush");

  let dp_exists = dp.exists();
  let deathpush_exists = deathpush.exists();

  Ok(CliInstallStatus {
    installed: dp_exists && deathpush_exists,
    dp_path: if dp_exists {
      Some(dp.to_string_lossy().into())
    } else {
      None
    },
    deathpush_path: if deathpush_exists {
      Some(deathpush.to_string_lossy().into())
    } else {
      None
    },
  })
}

#[tauri::command]
pub async fn install_cli(app: AppHandle) -> Result<(), Error> {
  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| Error::Other(format!("Failed to resolve resource dir: {e}")))?;

  if cfg!(target_os = "windows") {
    install_windows(&resource_dir)
  } else {
    install_unix(&resource_dir)
  }
}

#[tauri::command]
pub async fn uninstall_cli() -> Result<(), Error> {
  if cfg!(target_os = "windows") {
    uninstall_windows()
  } else {
    uninstall_unix()
  }
}

// ---------------------------------------------------------------------------
// Unix (macOS + Linux)
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn install_unix(resource_dir: &Path) -> Result<(), Error> {
  let script_path = resource_dir.join("resources/bin/dp");

  if !script_path.exists() {
    return Err(Error::Other(format!(
      "CLI script not found at {}",
      script_path.display()
    )));
  }

  // Ensure script is executable
  {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&script_path)
      .map_err(|e| Error::Other(format!("Failed to read script metadata: {e}")))?
      .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms)
      .map_err(|e| Error::Other(format!("Failed to set script permissions: {e}")))?;
  }

  let dir = install_dir();
  if dir_is_writable(&dir) {
    create_symlinks(&script_path, &dir)
  } else {
    install_with_elevated(&script_path, &dir)
  }
}

#[cfg(not(unix))]
fn install_unix(_resource_dir: &Path) -> Result<(), Error> {
  Err(Error::Other("Unix install not supported on this platform".into()))
}

#[cfg(unix)]
fn uninstall_unix() -> Result<(), Error> {
  let dir = install_dir();
  if dir_is_writable(&dir) {
    remove_symlinks(&dir)
  } else {
    uninstall_with_elevated(&dir)
  }
}

#[cfg(not(unix))]
fn uninstall_unix() -> Result<(), Error> {
  Err(Error::Other("Unix uninstall not supported on this platform".into()))
}

#[cfg(unix)]
fn create_symlinks(script_path: &Path, dir: &Path) -> Result<(), Error> {
  let script_str = script_path.to_string_lossy();
  for name in SYMLINK_NAMES {
    let link = dir.join(name);
    if link.exists() || link.symlink_metadata().is_ok() {
      std::fs::remove_file(&link)
        .map_err(|e| Error::Other(format!("Failed to remove existing {}: {e}", link.display())))?;
    }
    std::os::unix::fs::symlink(&*script_str, &link)
      .map_err(|e| Error::Other(format!("Failed to create symlink {}: {e}", link.display())))?;
  }
  Ok(())
}

#[cfg(unix)]
fn remove_symlinks(dir: &Path) -> Result<(), Error> {
  for name in SYMLINK_NAMES {
    let link = dir.join(name);
    if link.exists() || link.symlink_metadata().is_ok() {
      std::fs::remove_file(&link).map_err(|e| Error::Other(format!("Failed to remove {}: {e}", link.display())))?;
    }
  }
  Ok(())
}

#[cfg(unix)]
fn install_with_elevated(script_path: &Path, dir: &Path) -> Result<(), Error> {
  let script_str = script_path.to_string_lossy();
  let dir_str = dir.to_string_lossy();
  let mut cmds = Vec::new();
  cmds.push(format!("mkdir -p '{dir_str}'"));
  for name in SYMLINK_NAMES {
    let link = format!("{dir_str}/{name}");
    cmds.push(format!("rm -f '{link}'"));
    cmds.push(format!("ln -s '{script_str}' '{link}'"));
  }
  let shell_cmd = cmds.join(" && ");

  if cfg!(target_os = "macos") {
    run_osascript_sudo(&shell_cmd, "install the command line tool")
  } else {
    run_pkexec_sudo(&shell_cmd, "install the command line tool")
  }
}

#[cfg(unix)]
fn uninstall_with_elevated(dir: &Path) -> Result<(), Error> {
  let dir_str = dir.to_string_lossy();
  let mut cmds = Vec::new();
  for name in SYMLINK_NAMES {
    cmds.push(format!("rm -f '{dir_str}/{name}'"));
  }
  let shell_cmd = cmds.join(" && ");

  if cfg!(target_os = "macos") {
    run_osascript_sudo(&shell_cmd, "uninstall the command line tool")
  } else {
    run_pkexec_sudo(&shell_cmd, "uninstall the command line tool")
  }
}

#[cfg(unix)]
fn run_osascript_sudo(shell_cmd: &str, description: &str) -> Result<(), Error> {
  let escaped = shell_cmd.replace('\\', "\\\\").replace('"', "\\\"");
  let script = format!("do shell script \"{}\" with administrator privileges", escaped);

  let output = std::process::Command::new("osascript")
    .args(["-e", &script])
    .output()
    .map_err(|e| Error::Other(format!("Failed to {description}: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("User canceled") || stderr.contains("-128") {
      return Err(Error::Other("Authorization cancelled".into()));
    }
    return Err(Error::Other(format!("Failed to {description}: {stderr}")));
  }

  Ok(())
}

#[cfg(unix)]
fn run_pkexec_sudo(shell_cmd: &str, description: &str) -> Result<(), Error> {
  let output = std::process::Command::new("pkexec")
    .args(["sh", "-c", shell_cmd])
    .output()
    .map_err(|e| Error::Other(format!("Failed to {description}: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("dismissed") || stderr.contains("Not authorized") {
      return Err(Error::Other("Authorization cancelled".into()));
    }
    return Err(Error::Other(format!("Failed to {description}: {stderr}")));
  }

  Ok(())
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn install_windows(resource_dir: &Path) -> Result<(), Error> {
  let script_path = resource_dir.join("resources/bin/dp.cmd");

  if !script_path.exists() {
    return Err(Error::Other(format!(
      "CLI script not found at {}",
      script_path.display()
    )));
  }

  let dir = install_dir();
  std::fs::create_dir_all(&dir).map_err(|e| Error::Other(format!("Failed to create {}: {e}", dir.display())))?;

  // Copy the .cmd script as dp.cmd and deathpush.cmd
  for name in SYMLINK_NAMES {
    let dest = dir.join(format!("{name}.cmd"));
    std::fs::copy(&script_path, &dest)
      .map_err(|e| Error::Other(format!("Failed to copy to {}: {e}", dest.display())))?;
  }

  // Add to user PATH if not already present
  add_to_user_path(&dir)?;

  Ok(())
}

#[cfg(not(target_os = "windows"))]
fn install_windows(_resource_dir: &Path) -> Result<(), Error> {
  Err(Error::Other("Windows install not supported on this platform".into()))
}

#[cfg(target_os = "windows")]
fn uninstall_windows() -> Result<(), Error> {
  let dir = install_dir();

  for name in SYMLINK_NAMES {
    let path = dir.join(format!("{name}.cmd"));
    if path.exists() {
      std::fs::remove_file(&path).map_err(|e| Error::Other(format!("Failed to remove {}: {e}", path.display())))?;
    }
  }

  // Remove from PATH if the directory is now empty
  let is_empty = std::fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true);
  if is_empty {
    let _ = std::fs::remove_dir(&dir);
    remove_from_user_path(&dir)?;
  }

  Ok(())
}

#[cfg(not(target_os = "windows"))]
fn uninstall_windows() -> Result<(), Error> {
  Err(Error::Other("Windows uninstall not supported on this platform".into()))
}

#[cfg(target_os = "windows")]
fn add_to_user_path(dir: &Path) -> Result<(), Error> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let env = hkcu
    .open_subkey_with_flags(r"Environment", KEY_READ | KEY_WRITE)
    .map_err(|e| Error::Other(format!("Failed to open registry: {e}")))?;

  let current_path: String = env.get_value("Path").unwrap_or_default();
  let dir_str = dir.to_string_lossy();

  if current_path.split(';').any(|p| p.eq_ignore_ascii_case(&dir_str)) {
    return Ok(());
  }

  let new_path = if current_path.is_empty() {
    dir_str.to_string()
  } else {
    format!("{current_path};{dir_str}")
  };

  env
    .set_value("Path", &new_path)
    .map_err(|e| Error::Other(format!("Failed to update PATH: {e}")))?;

  // Broadcast WM_SETTINGCHANGE so running shells pick up the change
  broadcast_environment_change();

  Ok(())
}

#[cfg(target_os = "windows")]
fn remove_from_user_path(dir: &Path) -> Result<(), Error> {
  use winreg::RegKey;
  use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};

  let hkcu = RegKey::predef(HKEY_CURRENT_USER);
  let env = hkcu
    .open_subkey_with_flags(r"Environment", KEY_READ | KEY_WRITE)
    .map_err(|e| Error::Other(format!("Failed to open registry: {e}")))?;

  let current_path: String = env.get_value("Path").unwrap_or_default();
  let dir_str = dir.to_string_lossy();

  let new_path: String = current_path
    .split(';')
    .filter(|p| !p.eq_ignore_ascii_case(&dir_str))
    .collect::<Vec<_>>()
    .join(";");

  env
    .set_value("Path", &new_path)
    .map_err(|e| Error::Other(format!("Failed to update PATH: {e}")))?;

  broadcast_environment_change();

  Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_environment_change() {
  use std::ffi::OsStr;
  use std::os::windows::ffi::OsStrExt;
  use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
  };

  let env: Vec<u16> = OsStr::new("Environment")
    .encode_wide()
    .chain(std::iter::once(0))
    .collect();

  unsafe {
    let mut _result = 0usize;
    SendMessageTimeoutW(
      HWND_BROADCAST,
      WM_SETTINGCHANGE,
      0,
      env.as_ptr() as isize,
      SMTO_ABORTIFHUNG,
      5000,
      &mut _result as *mut usize,
    );
  }
}

#[cfg(unix)]
fn dir_is_writable(dir: &Path) -> bool {
  if !dir.exists() {
    return false;
  }
  let test_path = dir.join(".deathpush-write-test");
  if std::fs::write(&test_path, "").is_ok() {
    let _ = std::fs::remove_file(&test_path);
    true
  } else {
    false
  }
}
