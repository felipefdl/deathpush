use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{Emitter, Manager, WebviewWindow};

use crate::error::{Error, Result};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub type TerminalState = Mutex<HashMap<u64, PtySession>>;

#[derive(serde::Serialize, Clone)]
struct TerminalDataEvent {
  id: u64,
  data: String,
}

pub struct PtySession {
  pub id: u64,
  pub shell_name: String,
  pub child_pid: u32,
  pub window_label: String,
  writer: Arc<Mutex<Box<dyn Write + Send>>>,
  master: Box<dyn MasterPty + Send>,
}

impl PtySession {
  pub fn spawn(window: WebviewWindow, cwd: &str, cols: u16, rows: u16, window_label: String) -> Result<Self> {
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let pty_system = native_pty_system();
    let pair = pty_system
      .openpty(PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
      })
      .map_err(|e| Error::Other(e.to_string()))?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let shell_name = std::path::Path::new(&shell)
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| shell.clone());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(cwd);

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| Error::Other(e.to_string()))?;
    let child_pid = child.process_id().unwrap_or(0);
    drop(pair.slave);

    let reader = pair
      .master
      .try_clone_reader()
      .map_err(|e| Error::Other(e.to_string()))?;
    let writer = pair
      .master
      .take_writer()
      .map_err(|e| Error::Other(e.to_string()))?;
    let writer = Arc::new(Mutex::new(writer));

    let session_id = id;
    let app_handle = window.app_handle().clone();
    let label_for_thread = window_label.clone();
    thread::spawn(move || {
      let mut reader = reader;
      let mut buf = [0u8; 8192];
      loop {
        match reader.read(&mut buf) {
          Ok(0) => break,
          Ok(n) => {
            let data = String::from_utf8_lossy(&buf[..n]).to_string();
            if let Some(w) = app_handle.get_webview_window(&label_for_thread) {
              let _ = w.emit("terminal:data", TerminalDataEvent { id: session_id, data });
            }
          }
          Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
          Err(_) => break,
        }
      }
      let exit_msg = "\r\n\x1b[90m[Process exited. Press any key to restart.]\x1b[0m".to_string();
      if let Some(w) = app_handle.get_webview_window(&label_for_thread) {
        let _ = w.emit(
          "terminal:data",
          TerminalDataEvent { id: session_id, data: exit_msg },
        );
        let _ = w.emit("terminal:exit", session_id);
      }
      let _ = child.wait();
    });

    Ok(Self {
      id,
      shell_name,
      child_pid,
      window_label,
      writer,
      master: pair.master,
    })
  }

  pub fn write_data(&self, data: &str) -> Result<()> {
    let mut writer = self.writer.lock().map_err(|e| Error::Other(e.to_string()))?;
    writer.write_all(data.as_bytes())?;
    writer.flush()?;
    Ok(())
  }

  pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
    self.master
      .resize(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
      })
      .map_err(|e| Error::Other(e.to_string()))
  }
}
