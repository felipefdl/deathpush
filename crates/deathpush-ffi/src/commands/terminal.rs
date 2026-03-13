use deathpush_core::error::Error;
use deathpush_core::terminal::PtySession;
use deathpush_core::util::sync_command;

use crate::bridge::FfiTerminalSink;
use crate::session::{get_event_listener, manager};

#[derive(Debug, Clone, uniffi::Record)]
pub struct SpawnResult {
  pub id: u64,
  pub shell: String,
}

#[uniffi::export]
pub fn terminal_spawn(
  session_id: String,
  cols: u16,
  rows: u16,
  shell_path: Option<String>,
  shell_args: Option<String>,
) -> Result<SpawnResult, Error> {
  let mgr = manager();

  let cwd = {
    let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
    sessions
      .get(&session_id)
      .and_then(|s| s.cli_root.clone())
      .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()).into())
  };

  let cwd_str = cwd.to_string_lossy().to_string();

  let listener = get_event_listener().ok_or(Error::other("No event listener registered"))?;
  let terminal_sink = FfiTerminalSink::new(listener);

  let new_session = PtySession::spawn(terminal_sink, &cwd_str, cols, rows, session_id, shell_path, shell_args)?;
  let id = new_session.id;
  let shell = new_session.shell_name.clone();

  let mut sessions = mgr.terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
  sessions.insert(id, new_session);
  Ok(SpawnResult { id, shell })
}

#[uniffi::export]
pub fn terminal_write(terminal_id: u64, data: String) -> Result<(), Error> {
  let mgr = manager();
  let sessions = mgr.terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
  let session = sessions.get(&terminal_id).ok_or(Error::other("No terminal session"))?;
  session.write_data(&data)
}

#[uniffi::export]
pub fn terminal_resize(terminal_id: u64, cols: u16, rows: u16) -> Result<(), Error> {
  let mgr = manager();
  let sessions = mgr.terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(session) = sessions.get(&terminal_id) {
    session.resize(cols, rows)?;
  }
  Ok(())
}

#[uniffi::export]
pub fn terminal_kill(terminal_id: u64) -> Result<(), Error> {
  let mgr = manager();
  let mut sessions = mgr.terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
  sessions.remove(&terminal_id);
  Ok(())
}

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

#[uniffi::export]
pub fn terminal_foreground_process(terminal_id: u64) -> Result<String, Error> {
  let mgr = manager();
  let (child_pid, shell_name) = {
    let sessions = mgr.terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
    let session = sessions.get(&terminal_id).ok_or(Error::other("No terminal session"))?;
    (session.child_pid, session.shell_name.clone())
  };
  Ok(get_foreground_process_name(child_pid, &shell_name))
}
