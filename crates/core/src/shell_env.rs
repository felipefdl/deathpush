use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

static RESOLVED_ENV: OnceLock<HashMap<String, String>> = OnceLock::new();
static RESOLVER: OnceLock<ShellEnvResolver> = OnceLock::new();

#[derive(Clone, Copy)]
enum Resolution {
  #[cfg(not(windows))]
  Resolved,
  InheritedFallback,
}

struct ResolverState {
  worker: Mutex<Option<JoinHandle<()>>>,
  resolution: OnceLock<Resolution>,
}

#[derive(Clone)]
pub struct ShellEnvResolver {
  state: Arc<ResolverState>,
}

impl ShellEnvResolver {
  /// Start resolving the user's full shell environment on a background thread.
  pub fn start() -> Self {
    let state = Arc::new(ResolverState {
      worker: Mutex::new(None),
      resolution: OnceLock::new(),
    });
    let resolver = Self { state };

    #[cfg(not(windows))]
    {
      let worker_state = Arc::clone(&resolver.state);
      let worker = std::thread::spawn(move || {
        let resolution = match resolve_shell_env() {
          Ok(env) => {
            tracing::info!("resolved shell environment with {} variables", env.len());
            let _ = RESOLVED_ENV.set(env);
            Resolution::Resolved
          }
          Err(err) => {
            tracing::warn!("failed to resolve shell environment, using inherited env: {err}");
            Resolution::InheritedFallback
          }
        };
        let _ = worker_state.resolution.set(resolution);
      });
      let mut worker_slot = resolver
        .state
        .worker
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
      *worker_slot = Some(worker);
    }

    #[cfg(windows)]
    {
      let _ = resolver.state.resolution.set(Resolution::InheritedFallback);
    }

    let _ = RESOLVER.set(resolver.clone());
    resolver
  }

  pub async fn wait(&self) -> Option<&'static HashMap<String, String>> {
    if let Some(resolved) = self.completed_result() {
      return resolved;
    }

    let resolver = self.clone();
    match tokio::task::spawn_blocking(move || resolver.join_worker()).await {
      Ok(resolved) => resolved,
      Err(err) => {
        tracing::warn!("failed to join shell environment resolver: {err}");
        None
      }
    }
  }

  fn completed_result(&self) -> Option<Option<&'static HashMap<String, String>>> {
    self.state.resolution.get().map(|resolution| match resolution {
      #[cfg(not(windows))]
      Resolution::Resolved => get(),
      Resolution::InheritedFallback => None,
    })
  }

  fn join_worker(&self) -> Option<&'static HashMap<String, String>> {
    if let Some(resolved) = self.completed_result() {
      return resolved;
    }

    let mut worker_slot = self
      .state
      .worker
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(worker) = worker_slot.take()
      && worker.join().is_err()
    {
      tracing::warn!("shell environment resolver thread panicked, using inherited env");
      let _ = self.state.resolution.set(Resolution::InheritedFallback);
    }
    drop(worker_slot);

    self.completed_result().flatten()
  }
}

pub(crate) async fn wait_for_resolved_env() -> Option<&'static HashMap<String, String>> {
  match RESOLVER.get() {
    Some(resolver) => resolver.wait().await,
    None => get(),
  }
}

pub(crate) fn wait_for_resolved_env_blocking() -> Option<&'static HashMap<String, String>> {
  let Some(resolver) = RESOLVER.get() else {
    return get();
  };

  if let Some(resolved) = resolver.completed_result() {
    return resolved;
  }

  if let Ok(runtime) = tokio::runtime::Handle::try_current()
    && matches!(runtime.runtime_flavor(), tokio::runtime::RuntimeFlavor::MultiThread)
  {
    return tokio::task::block_in_place(|| resolver.join_worker());
  }

  resolver.join_worker()
}

/// Get the cached resolved environment, if available.
pub fn get() -> Option<&'static HashMap<String, String>> {
  RESOLVED_ENV.get()
}

#[cfg(not(windows))]
fn shell_env_timeout() -> std::time::Duration {
  #[cfg(test)]
  if let Ok(ms) = std::env::var("DEATHPUSH_SHELL_ENV_TIMEOUT_MS")
    && let Ok(ms) = ms.parse::<u64>()
  {
    return std::time::Duration::from_millis(ms);
  }
  std::time::Duration::from_secs(10)
}

#[cfg(not(windows))]
fn resolve_shell_env() -> std::result::Result<HashMap<String, String>, String> {
  use std::io::Read;
  use std::sync::mpsc;

  let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
  let timeout = shell_env_timeout();

  let mut command = shell_env_command(&shell);
  command
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null())
    .stdin(std::process::Stdio::null());

  let mut child = command
    .spawn()
    .map_err(|e| format!("failed to spawn shell '{shell}': {e}"))?;
  let mut stdout = child
    .stdout
    .take()
    .ok_or_else(|| format!("shell '{shell}' stdout was not piped"))?;

  let (tx, rx) = mpsc::channel();
  std::thread::spawn(move || {
    let mut buf = Vec::new();
    let _ = stdout.read_to_end(&mut buf);
    let _ = tx.send(buf);
  });

  let output = match rx.recv_timeout(timeout) {
    Ok(stdout) => {
      let status = child
        .wait()
        .map_err(|e| format!("failed to wait for shell '{shell}': {e}"))?;
      std::process::Output {
        status,
        stdout,
        stderr: Vec::new(),
      }
    }
    Err(_) => {
      let _ = child.kill();
      let _ = child.wait();
      return Err(format!(
        "shell env resolution timed out after {}s (shell: {shell})",
        timeout.as_secs().max(1)
      ));
    }
  };

  if !output.status.success() {
    return Err(format!("shell '{shell}' exited with status {}", output.status));
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
    return Err(format!("resolved only {} variables, expected at least 5", env.len()));
  }

  Ok(env)
}

#[cfg(not(windows))]
fn shell_env_command(shell: &str) -> std::process::Command {
  #[cfg(test)]
  if let Ok(command) = std::env::var("DEATHPUSH_SHELL_ENV_CMD") {
    let mut fake_shell = std::process::Command::new("/bin/sh");
    fake_shell.args(["-c", &command]);
    return fake_shell;
  }

  let mut login_shell = std::process::Command::new(shell);
  login_shell.args(["-i", "-l", "-c", "/usr/bin/env -0"]);
  login_shell
}

#[cfg(not(windows))]
fn is_sanitized_prefix(key: &str) -> bool {
  const PREFIXES: &[&str] = &["TAURI_", "__TAURI_", "WEBKIT_", "GDK_", "ELECTRON_", "VSCODE_"];
  PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

#[cfg(all(test, not(windows)))]
mod tests {
  use std::ffi::OsString;
  use std::sync::Mutex;
  use std::time::{Duration, Instant};

  use super::*;

  static SHELL_ENV_TEST: Mutex<()> = Mutex::new(());
  struct FakeCommandGuard {
    original: Option<OsString>,
  }

  impl FakeCommandGuard {
    fn set(command: &str) -> Self {
      let original = std::env::var_os("DEATHPUSH_SHELL_ENV_CMD");
      // SAFETY: this is the only test that reads or writes this test-only variable.
      unsafe {
        std::env::set_var("DEATHPUSH_SHELL_ENV_CMD", command);
      }
      Self { original }
    }

    fn replace(&self, command: &str) {
      // SAFETY: this is the only test that reads or writes this test-only variable.
      unsafe {
        std::env::set_var("DEATHPUSH_SHELL_ENV_CMD", command);
      }
    }
  }

  impl Drop for FakeCommandGuard {
    fn drop(&mut self) {
      // SAFETY: this is the only test that reads or writes this test-only variable.
      unsafe {
        match &self.original {
          Some(value) => std::env::set_var("DEATHPUSH_SHELL_ENV_CMD", value),
          None => std::env::remove_var("DEATHPUSH_SHELL_ENV_CMD"),
        }
      }
    }
  }

  struct TimeoutMsGuard {
    original: Option<OsString>,
  }

  impl TimeoutMsGuard {
    fn set(ms: &str) -> Self {
      let original = std::env::var_os("DEATHPUSH_SHELL_ENV_TIMEOUT_MS");
      // SAFETY: test-only timeout override, restored on drop.
      unsafe {
        std::env::set_var("DEATHPUSH_SHELL_ENV_TIMEOUT_MS", ms);
      }
      Self { original }
    }
  }

  impl Drop for TimeoutMsGuard {
    fn drop(&mut self) {
      // SAFETY: restores the test-only timeout override.
      unsafe {
        match &self.original {
          Some(value) => std::env::set_var("DEATHPUSH_SHELL_ENV_TIMEOUT_MS", value),
          None => std::env::remove_var("DEATHPUSH_SHELL_ENV_TIMEOUT_MS"),
        }
      }
    }
  }

  #[tokio::test]
  #[allow(clippy::await_holding_lock)]
  async fn shell_env_resolves_in_background_with_inherited_fallback() {
    let _lock = SHELL_ENV_TEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let fake_command = FakeCommandGuard::set("sleep 30");
    let started_at = Instant::now();
    let resolver = ShellEnvResolver::start();
    assert!(started_at.elapsed() < Duration::from_millis(200));

    let waited_at = Instant::now();
    let resolved = tokio::time::timeout(Duration::from_secs(11), resolver.wait())
      .await
      .expect("resolver should enforce its 10 second timeout");
    assert!(waited_at.elapsed() >= Duration::from_secs(10));
    assert!(resolved.is_none());
    assert!(get().is_none());

    fake_command.replace("printf 'FOO=bar\\0PATH=/usr/bin\\0HOME=/tmp\\0USER=t\\0SHELL=/bin/sh\\0'");
    let resolver = ShellEnvResolver::start();
    let resolved = tokio::time::timeout(Duration::from_secs(1), resolver.wait())
      .await
      .expect("fake shell command should resolve promptly")
      .expect("fake shell command should produce a resolved environment");

    assert_eq!(resolved.get("FOO").map(String::as_str), Some("bar"));
    assert_eq!(resolved.get("PATH").map(String::as_str), Some("/usr/bin"));
    assert_eq!(resolved.get("HOME").map(String::as_str), Some("/tmp"));
    assert_eq!(resolved.get("USER").map(String::as_str), Some("t"));
    assert_eq!(resolved.get("SHELL").map(String::as_str), Some("/bin/sh"));
  }

  #[test]
  fn shell_env_timeout_kills_hung_child() {
    let _lock = SHELL_ENV_TEST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _timeout = TimeoutMsGuard::set("200");
    let pid_file = tempfile::NamedTempFile::new().unwrap();
    let pid_path = pid_file.path().to_string_lossy().replace('\'', "");
    let _fake = FakeCommandGuard::set(&format!("echo $$ > '{pid_path}'; exec sleep 30"));

    let started_at = Instant::now();
    let result = resolve_shell_env();
    assert!(result.is_err(), "hung shell must time out, got {result:?}");
    assert!(
      started_at.elapsed() < Duration::from_secs(2),
      "timeout must kill instead of waiting on sleep, elapsed={:?}",
      started_at.elapsed()
    );

    std::thread::sleep(Duration::from_millis(50));
    let pid_text = std::fs::read_to_string(pid_file.path()).unwrap();
    let pid = pid_text.trim().parse::<i32>().expect("pid file");
    let alive = std::process::Command::new("kill")
      .args(["-0", &pid.to_string()])
      .status()
      .map(|status| status.success())
      .unwrap_or(true);
    assert!(!alive, "timed-out shell child {pid} must be reaped");
  }
}
