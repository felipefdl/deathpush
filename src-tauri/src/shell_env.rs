use std::collections::HashMap;
use std::sync::OnceLock;

static RESOLVED_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Resolve and cache the user's full shell environment at startup.
///
/// On Unix, GUI apps launched from Dock/Finder/Spotlight inherit a minimal
/// environment that lacks PATH entries, credentials, and tool configs from
/// the user's shell profile. This spawns a hidden login+interactive shell
/// to capture the real environment.
pub fn init() {
  #[cfg(not(windows))]
  {
    match resolve_shell_env() {
      Ok(env) => {
        tracing::info!("resolved shell environment with {} variables", env.len());
        let _ = RESOLVED_ENV.set(env);
      }
      Err(err) => {
        tracing::warn!("failed to resolve shell environment, using inherited env: {err}");
      }
    }
  }
}

/// Get the cached resolved environment, if available.
pub fn get() -> Option<&'static HashMap<String, String>> {
  RESOLVED_ENV.get()
}

#[cfg(not(windows))]
fn resolve_shell_env() -> std::result::Result<HashMap<String, String>, String> {
  use std::process::Command;
  use std::sync::mpsc;
  use std::time::Duration;

  let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

  let (tx, rx) = mpsc::channel();

  let shell_clone = shell.clone();
  std::thread::spawn(move || {
    let result = Command::new(&shell_clone)
      .args(["-i", "-l", "-c", "/usr/bin/env -0"])
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::null())
      .stdin(std::process::Stdio::null())
      .output();
    let _ = tx.send(result);
  });

  let output = rx
    .recv_timeout(Duration::from_secs(10))
    .map_err(|_| format!("shell env resolution timed out after 10s (shell: {shell})"))?
    .map_err(|e| format!("failed to spawn shell '{shell}': {e}"))?;

  if !output.status.success() {
    return Err(format!(
      "shell '{shell}' exited with status {}",
      output.status
    ));
  }

  let env: HashMap<String, String> = output
    .stdout
    .split(|&b| b == 0)
    .filter_map(|entry| {
      let s = std::str::from_utf8(entry).ok()?;
      let (key, value) = s.split_once('=')?;
      if key.is_empty() {
        return None;
      }
      Some((key.to_string(), value.to_string()))
    })
    .filter(|(key, _)| !is_sanitized_prefix(key))
    .collect();

  if env.len() < 5 {
    return Err(format!(
      "resolved only {} variables, expected at least 5",
      env.len()
    ));
  }

  Ok(env)
}

#[cfg(not(windows))]
fn is_sanitized_prefix(key: &str) -> bool {
  const PREFIXES: &[&str] = &[
    "TAURI_",
    "__TAURI_",
    "WEBKIT_",
    "GDK_",
    "ELECTRON_",
    "VSCODE_",
  ];
  PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}
