use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::error::{Error, Result};
use crate::events::TerminalSink;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub type TerminalState = Mutex<HashMap<u64, PtySession>>;

pub struct PtySession {
  pub id: u64,
  pub shell_name: String,
  pub child_pid: u32,
  pub session_label: String,
  writer: Arc<Mutex<Box<dyn Write + Send>>>,
  master: Box<dyn MasterPty + Send>,
}

impl PtySession {
  pub fn spawn(
    terminal_sink: Arc<dyn TerminalSink>,
    cwd: &str,
    cols: u16,
    rows: u16,
    session_label: String,
    shell_path: Option<String>,
    shell_args: Option<String>,
  ) -> Result<Self> {
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let pty_system = native_pty_system();
    let pair = pty_system
      .openpty(PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
      })
      .map_err(|e| Error::Other { message: e.to_string() })?;

    let default_shell = if cfg!(windows) {
      std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    } else {
      std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
    };
    let shell = shell_path.filter(|s| !s.is_empty()).unwrap_or(default_shell);
    let shell_name = std::path::Path::new(&shell)
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| shell.clone());
    let mut cmd = CommandBuilder::new(&shell);
    let default_args = default_shell_args(&shell_name);
    let args_str = shell_args.unwrap_or(default_args);
    for arg in args_str.split_whitespace() {
      cmd.arg(arg);
    }
    if let Some(resolved) = crate::shell_env::get() {
      cmd.env_clear();
      for (key, value) in resolved {
        cmd.env(key, value);
      }
    }
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(cwd);

    let mut child = pair
      .slave
      .spawn_command(cmd)
      .map_err(|e| Error::Other { message: e.to_string() })?;
    let child_pid = child.process_id().unwrap_or(0);
    drop(pair.slave);

    let reader = pair
      .master
      .try_clone_reader()
      .map_err(|e| Error::Other { message: e.to_string() })?;
    let writer = pair
      .master
      .take_writer()
      .map_err(|e| Error::Other { message: e.to_string() })?;
    let writer = Arc::new(Mutex::new(writer));

    let session_id = id;
    let sink = terminal_sink;
    #[cfg(windows)]
    let writer_for_reader = Arc::clone(&writer);
    thread::spawn(move || {
      let mut reader = reader;
      let mut buf = [0u8; 65536];
      // Leftover bytes from an incomplete UTF-8 sequence at the end of the
      // previous read. Max UTF-8 char is 4 bytes, so this is tiny.
      let mut leftover = [0u8; 4];
      let mut leftover_len: usize = 0;
      loop {
        // Place leftover bytes at the start of the buffer, then read after them.
        buf[..leftover_len].copy_from_slice(&leftover[..leftover_len]);
        match reader.read(&mut buf[leftover_len..]) {
          Ok(0) => break,
          Ok(n) => {
            let total = leftover_len + n;
            leftover_len = 0;

            // Find the longest valid UTF-8 prefix. If the tail has an
            // incomplete multi-byte sequence, hold it back for the next read
            // instead of replacing it with U+FFFD.
            let valid_up_to = match std::str::from_utf8(&buf[..total]) {
              Ok(_) => total,
              Err(e) => {
                let valid = e.valid_up_to();
                let remaining = total - valid;
                // An incomplete sequence at the end (1-3 trailing bytes) is
                // carried over. Anything else is a genuine decoding error --
                // skip the bad byte so we don't loop forever.
                if e.error_len().is_none() && remaining <= 3 {
                  leftover[..remaining].copy_from_slice(&buf[valid..total]);
                  leftover_len = remaining;
                  valid
                } else {
                  // Skip past the bad byte(s) to include them (lossy).
                  valid + e.error_len().unwrap_or(1)
                }
              }
            };

            if valid_up_to == 0 {
              continue;
            }

            // SAFETY: we verified buf[..valid_up_to] is valid UTF-8 above
            // (or up to the error boundary which is also valid).
            #[allow(unused_mut)]
            let mut data = String::from_utf8_lossy(&buf[..valid_up_to]).to_string();

            // Windows ConPTY fix: portable-pty 0.9.0 sets PSEUDOCONSOLE_INHERIT_CURSOR,
            // causing ConPTY to send a Device Status Report (\x1b[6n) at startup.
            // If we don't respond with a cursor position, ConPTY deadlocks all output.
            // Respond with position (1,1) and strip the sequence from forwarded data.
            #[cfg(windows)]
            if data.contains("\x1b[6n") {
              if let Ok(mut w) = writer_for_reader.lock() {
                let _ = w.write_all(b"\x1b[1;1R");
                let _ = w.flush();
              }
              data = data.replace("\x1b[6n", "");
              if data.is_empty() {
                continue;
              }
            }

            sink.on_data(session_id, data);
          }
          Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
          Err(_) => break,
        }
      }
      let exit_msg = "\r\n\x1b[90m[Process exited. Press any key to restart.]\x1b[0m".to_string();
      sink.on_data(session_id, exit_msg);
      sink.on_exit(session_id);
      let _ = child.wait();
    });

    Ok(Self {
      id,
      shell_name,
      child_pid,
      session_label,
      writer,
      master: pair.master,
    })
  }

  pub fn write_data(&self, data: &str) -> Result<()> {
    let mut writer = self
      .writer
      .lock()
      .map_err(|e| Error::Other { message: e.to_string() })?;
    writer.write_all(data.as_bytes())?;
    writer.flush()?;
    Ok(())
  }

  pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
    self
      .master
      .resize(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
      })
      .map_err(|e| Error::Other { message: e.to_string() })
  }
}

/// Determine default shell arguments per platform, matching VS Code behavior:
/// - macOS + zsh/bash: `--login` (sources profile files)
/// - Linux: no args (profile is already sourced via resolved env)
/// - Windows: no args
fn default_shell_args(shell_name: &str) -> String {
  #[cfg(target_os = "macos")]
  {
    if shell_name == "zsh" || shell_name == "bash" {
      return "--login".to_string();
    }
  }

  #[cfg(not(target_os = "macos"))]
  let _ = shell_name;

  String::new()
}
