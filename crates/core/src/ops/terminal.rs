use serde::Serialize;

use crate::core::Core;
use crate::error::{Error, Result};
use crate::pty::PtySession;
use crate::session::SessionId;
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
    let mut sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
    sessions.remove(&terminal);
    Ok(())
  }

  pub fn terminal_foreground_process(&self, terminal: u64) -> Result<String> {
    let (child_pid, shell_name) = {
      let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
      let session = sessions
        .get(&terminal)
        .ok_or(Error::Other("No terminal session".into()))?;
      (session.child_pid, session.shell_name.clone())
    };

    Ok(get_foreground_process_name(child_pid, &shell_name))
  }

  pub fn terminals_have_active_process(&self) -> Result<bool> {
    let snapshots: Vec<(u32, String)> = {
      let sessions = self.terminals.lock().map_err(|e| Error::Other(e.to_string()))?;
      sessions
        .values()
        .map(|session| (session.child_pid, session.shell_name.clone()))
        .collect()
    };
    Ok(
      snapshots
        .iter()
        .any(|(pid, shell)| get_foreground_process_name(*pid, shell) != *shell),
    )
  }
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
