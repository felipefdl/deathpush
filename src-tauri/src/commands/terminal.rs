use std::sync::Mutex;

use serde::Serialize;
use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::pty::{PtySession, TauriTerminalSink, TerminalState};
use crate::util::sync_command;

#[derive(Serialize)]
pub struct SpawnResult {
  pub id: u64,
  pub shell: String,
}

#[tauri::command]
pub async fn terminal_spawn(
  cols: u16,
  rows: u16,
  shell_path: Option<String>,
  shell_args: Option<String>,
  window: WebviewWindow,
  repo_state: State<'_, Mutex<AppRepoState>>,
  terminal_state: State<'_, TerminalState>,
) -> Result<SpawnResult> {
  let cwd = {
    let guard = repo_state.lock().map_err(|e| Error::other(e.to_string()))?;
    guard
      .get(window.label())
      .and_then(|s| s.cli_root.clone())
      .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()).into())
  };

  let label = window.label().to_string();
  let cwd_str = cwd.to_string_lossy();
  let terminal_sink = TauriTerminalSink::new(&window);
  let new_session = PtySession::spawn(terminal_sink, &cwd_str, cols, rows, label, shell_path, shell_args)?;
  let id = new_session.id;
  let shell = new_session.shell_name.clone();

  let mut sessions = terminal_state.lock().map_err(|e| Error::other(e.to_string()))?;
  sessions.insert(id, new_session);
  Ok(SpawnResult { id, shell })
}

#[tauri::command]
pub async fn terminal_write(id: u64, data: String, state: State<'_, TerminalState>) -> Result<()> {
  let sessions = state.lock().map_err(|e| Error::other(e.to_string()))?;
  let session = sessions.get(&id).ok_or(Error::other("No terminal session"))?;
  session.write_data(&data)
}

#[tauri::command]
pub async fn terminal_resize(id: u64, cols: u16, rows: u16, state: State<'_, TerminalState>) -> Result<()> {
  let sessions = state.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(session) = sessions.get(&id) {
    session.resize(cols, rows)?;
  }
  Ok(())
}

#[tauri::command]
pub async fn terminal_kill(id: u64, state: State<'_, TerminalState>) -> Result<()> {
  let mut sessions = state.lock().map_err(|e| Error::other(e.to_string()))?;
  sessions.remove(&id);
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

#[tauri::command]
pub async fn terminal_foreground_process(id: u64, state: State<'_, TerminalState>) -> Result<String> {
  let (child_pid, shell_name) = {
    let sessions = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let session = sessions.get(&id).ok_or(Error::other("No terminal session"))?;
    (session.child_pid, session.shell_name.clone())
  };

  Ok(get_foreground_process_name(child_pid, &shell_name))
}
