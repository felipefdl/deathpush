use serde::Serialize;

use crate::core::Core;
use crate::error::{Error, Result};
use crate::pty::PtySession;
use crate::session::SessionId;
#[cfg(not(windows))]
use crate::util::sync_command;

#[derive(Serialize)]
pub struct SpawnResult {
  pub id: u64,
  pub shell: String,
}

impl Core {
  pub fn terminal_spawn(
    &self,
    id: SessionId,
    cols: u16,
    rows: u16,
    shell_path: Option<String>,
    shell_args: Option<String>,
  ) -> Result<SpawnResult> {
    let cwd = self
      .repo_root(id)
      .unwrap_or_else(|_| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()).into());
    let cwd_str = cwd.to_string_lossy();
    let session = PtySession::spawn(&cwd_str, cols, rows, id, shell_path, shell_args, self.hub.clone())?;
    let terminal = session.id;
    let shell = session.shell_name.clone();
    let mut sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
    sessions.insert(terminal, session);
    Ok(SpawnResult { id: terminal, shell })
  }

  pub fn terminal_write(&self, terminal: u64, data: &str) -> Result<()> {
    let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
    let session = sessions
      .get(&terminal)
      .ok_or(Error::Other("No terminal session".into()))?;
    session.write_data(data)
  }

  pub fn terminal_resize(&self, terminal: u64, cols: u16, rows: u16) -> Result<()> {
    let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
    if let Some(session) = sessions.get(&terminal) {
      session.resize(cols, rows)?;
    }
    Ok(())
  }

  pub fn terminal_kill(&self, terminal: u64) -> Result<()> {
    let session = {
      let mut sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
      sessions.remove(&terminal)
    };
    if let Some(session) = session {
      self.runtime_handle().spawn_blocking(move || {
        let mut session = session;
        session.shutdown();
      });
    }
    Ok(())
  }

  #[cfg(test)]
  pub fn terminal_kill_blocking(&self, terminal: u64) -> Result<()> {
    let session = {
      let mut sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
      sessions.remove(&terminal)
    };
    let Some(session) = session else {
      return Ok(());
    };
    let (tx, rx) = std::sync::mpsc::channel();
    self.runtime_handle().spawn_blocking(move || {
      let mut session = session;
      session.shutdown();
      let _ = tx.send(());
    });
    rx.recv_timeout(std::time::Duration::from_secs(3))
      .map_err(|err| Error::Other(err.to_string()))?;
    Ok(())
  }

  /// Foreground process name for `terminal` in `session`.
  ///
  /// Unix discovery uses `pgrep` and `ps`. On Windows the name stays the shell name;
  /// no process is queried.
  pub fn terminal_foreground_process(&self, session: SessionId, terminal: u64) -> Result<String> {
    let (child_pid, shell_name) = {
      let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
      let pty = sessions
        .get(&terminal)
        .ok_or(Error::Other("No terminal session".into()))?;
      if pty.session != session {
        return Err(Error::Other("No terminal session".into()));
      }
      (pty.child_pid, pty.shell_name.clone())
    };

    Ok(get_foreground_process_name(child_pid, &shell_name))
  }

  /// Whether any terminal in `session` has a child process other than the shell.
  ///
  /// Unix only. On Windows this is always false; process discovery does not run.
  pub fn terminals_have_active_process(&self, session: SessionId) -> Result<bool> {
    #[cfg(windows)]
    {
      let _ = session;
      Ok(false)
    }
    #[cfg(not(windows))]
    {
      let snapshots: Vec<(u32, String)> = {
        let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
        sessions
          .values()
          .filter(|pty| pty.session == session)
          .map(|pty| (pty.child_pid, pty.shell_name.clone()))
          .collect()
      };
      Ok(
        snapshots
          .iter()
          .any(|(pid, shell)| get_foreground_process_name(*pid, shell) != *shell),
      )
    }
  }

  #[cfg(test)]
  pub fn terminal_pid(&self, terminal: u64) -> Option<u32> {
    self
      .terminals
      .lock()
      .ok()?
      .get(&terminal)
      .map(|session| session.child_pid)
  }
}

#[cfg(windows)]
fn get_foreground_process_name(_shell_pid: u32, shell_name: &str) -> String {
  shell_name.to_string()
}

#[cfg(not(windows))]
fn get_foreground_process_name(shell_pid: u32, shell_name: &str) -> String {
  let Ok(output) = sync_command("pgrep").args(["-P", &shell_pid.to_string()]).output() else {
    return shell_name.to_string();
  };

  if !output.status.success() {
    return shell_name.to_string();
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let Some(last_pid) = stdout.trim().lines().last() else {
    return shell_name.to_string();
  };

  let Ok(name_output) = sync_command("ps").args(["-o", "comm=", "-p", last_pid.trim()]).output() else {
    return shell_name.to_string();
  };

  let name = String::from_utf8_lossy(&name_output.stdout).trim().to_string();
  if name.is_empty() {
    return shell_name.to_string();
  }

  std::path::Path::new(&name)
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or(name)
}

#[cfg(test)]
mod tests {
  use crate::Core;
  #[cfg(unix)]
  use std::time::Duration;

  fn core() -> std::sync::Arc<Core> {
    let dir = tempfile::TempDir::new().unwrap();
    Core::new(dir.path().to_path_buf()).unwrap()
  }

  #[cfg(unix)]
  fn pid_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
      .args(["-0", &pid.to_string()])
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .status()
      .map(|status| status.success())
      .unwrap_or(false)
  }

  #[cfg(unix)]
  fn wait_until(timeout: Duration, mut pred: impl FnMut() -> bool) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
      if pred() {
        return true;
      }
      std::thread::sleep(Duration::from_millis(50));
    }
    false
  }

  #[cfg(unix)]
  fn hold_script() -> (tempfile::TempDir, String) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("hold.sh");
    std::fs::write(&path, "#!/bin/sh\nsleep 30\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = path.to_string_lossy().into_owned();
    (dir, path)
  }

  #[cfg(unix)]
  #[test]
  fn terminal_kill_terminates_the_child() {
    let core = core();
    let (session, _events) = core.open_session();
    let spawned = core
      .terminal_spawn(session, 80, 24, Some("/bin/sleep".into()), Some("30".into()))
      .unwrap();
    let pid = core.terminal_pid(spawned.id).expect("spawned pid");
    assert!(pid_alive(pid), "sleep should be running before kill");
    core.terminal_kill(spawned.id).unwrap();
    assert!(core.terminal_pid(spawned.id).is_none());
    assert!(
      wait_until(Duration::from_secs(2), || !pid_alive(pid)),
      "sleep pid {pid} should be gone after terminal_kill"
    );
  }

  #[cfg(unix)]
  #[test]
  fn terminals_have_active_process_is_session_scoped() {
    let core = core();
    let (session_a, _a) = core.open_session();
    let (session_b, _b) = core.open_session();
    let (_dir, script) = hold_script();
    let spawned = core
      .terminal_spawn(session_a, 80, 24, Some(script), Some(String::new()))
      .unwrap();
    assert_ne!(spawned.shell, "sleep");
    assert!(
      wait_until(Duration::from_secs(3), || {
        core.terminals_have_active_process(session_a).unwrap_or(false)
      }),
      "session A should see the sleep child"
    );
    assert!(
      !core.terminals_have_active_process(session_b).unwrap(),
      "session B must ignore A's child"
    );
    let name = core.terminal_foreground_process(session_a, spawned.id).unwrap();
    assert_eq!(name, "sleep");
    assert!(core.terminal_foreground_process(session_b, spawned.id).is_err());
    core.terminal_kill(spawned.id).unwrap();
  }

  #[cfg(unix)]
  fn trap_term_tree() -> (tempfile::TempDir, String, String, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::TempDir::new().unwrap();
    let child = dir.path().join("trap-child.sh");
    std::fs::write(&child, "#!/bin/sh\ntrap \"\" TERM\nsleep 30\n").unwrap();
    std::fs::set_permissions(&child, std::fs::Permissions::from_mode(0o755)).unwrap();
    let parent = dir.path().join("trap-parent.sh");
    std::fs::write(&parent, "#!/bin/sh\n\"$1\" &\necho $! > \"$2\"\nwait\n").unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    let pidfile = dir.path().join("child.pid");
    (
      dir,
      parent.to_string_lossy().into_owned(),
      child.to_string_lossy().into_owned(),
      pidfile,
    )
  }

  #[cfg(unix)]
  fn read_pidfile(path: &std::path::Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
  }

  #[cfg(unix)]
  #[test]
  fn terminal_kill_tears_down_a_term_resistant_child() {
    let core = core();
    let (session, _events) = core.open_session();
    let (_dir, parent, child, pidfile) = trap_term_tree();
    let args = format!("{} {}", child, pidfile.display());
    let spawned = core.terminal_spawn(session, 80, 24, Some(parent), Some(args)).unwrap();
    let leader = core.terminal_pid(spawned.id).expect("spawned pid");
    assert!(pid_alive(leader), "parent shell should be running before kill");
    assert!(
      wait_until(Duration::from_secs(2), || read_pidfile(&pidfile).is_some_and(pid_alive)),
      "descendant pidfile should appear"
    );
    let descendant = read_pidfile(&pidfile).expect("descendant pid");
    assert_ne!(descendant, leader);
    let started = std::time::Instant::now();
    core.terminal_kill_blocking(spawned.id).unwrap();
    assert!(core.terminal_pid(spawned.id).is_none());
    assert!(!pid_alive(leader), "leader pid {leader} should be gone after teardown");
    assert!(
      !pid_alive(descendant),
      "descendant pid {descendant} should be gone after group SIGKILL"
    );
    assert!(
      started.elapsed() < Duration::from_secs(2),
      "teardown must finish within the join deadline"
    );
  }

  #[cfg(windows)]
  #[test]
  fn terminals_have_active_process_is_false_on_windows() {
    let core = core();
    let (session, _events) = core.open_session();
    let spawned = core.terminal_spawn(session, 80, 24, None, None).unwrap();
    assert!(!core.terminals_have_active_process(session).unwrap());
    let name = core.terminal_foreground_process(session, spawned.id).unwrap();
    assert_eq!(name, spawned.shell);
    core.terminal_kill(spawned.id).unwrap();
  }
}
