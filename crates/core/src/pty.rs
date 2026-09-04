#[cfg(not(unix))]
use portable_pty::ChildKiller;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::events::{CoreEvent, EventHub};
use crate::session::SessionId;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

pub type TerminalState = Mutex<HashMap<u64, PtySession>>;

pub struct PtySession {
  pub id: u64,
  pub shell_name: String,
  pub child_pid: u32,
  pub session: SessionId,
  writer: Arc<Mutex<Box<dyn Write + Send>>>,
  master: Option<Box<dyn MasterPty + Send>>,
  child: Mutex<Box<dyn Child + Send + Sync>>,
  reader: Option<JoinHandle<()>>,
}

impl PtySession {
  pub fn spawn(
    cwd: &str,
    cols: u16,
    rows: u16,
    session: SessionId,
    shell_path: Option<String>,
    shell_args: Option<String>,
    hub: Arc<EventHub>,
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
      .map_err(|e| Error::Other(e.to_string()))?;

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
    if let Some(resolved) = crate::shell_env::wait_for_resolved_env_blocking() {
      cmd.env_clear();
      for (key, value) in resolved {
        cmd.env(key, value);
      }
    }
    cmd.env("TERM", "xterm-256color");
    cmd.cwd(cwd);

    let child = pair.slave.spawn_command(cmd).map_err(|e| Error::Other(e.to_string()))?;
    let child_pid = child.process_id().unwrap_or(0);
    drop(pair.slave);

    let reader = pair
      .master
      .try_clone_reader()
      .map_err(|e| Error::Other(e.to_string()))?;
    let writer = pair.master.take_writer().map_err(|e| Error::Other(e.to_string()))?;
    let writer = Arc::new(Mutex::new(writer));

    let session_id = id;
    let thread_hub = hub.clone();
    #[cfg(windows)]
    let writer_for_reader = Arc::clone(&writer);
    let reader = thread::spawn(move || {
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

            thread_hub.send(session, CoreEvent::TerminalData { id: session_id, data });
          }
          Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
          Err(_) => break,
        }
      }
      let exit_msg = "\r\n\x1b[90m[Process exited. Press any key to restart.]\x1b[0m".to_string();
      thread_hub.send(
        session,
        CoreEvent::TerminalData {
          id: session_id,
          data: exit_msg,
        },
      );
      thread_hub.send(session, CoreEvent::TerminalExited { id: session_id });
    });

    Ok(Self {
      id,
      shell_name,
      child_pid,
      session,
      writer,
      master: Some(pair.master),
      child: Mutex::new(child),
      reader: Some(reader),
    })
  }

  /// Kill the child, close the PTY master so the reader exits, and join the reader.
  ///
  /// Unix: SIGTERM, then SIGKILL on the process group if the child ignores TERM.
  /// The reader join waits at most 2s, then detaches.
  pub fn shutdown(&mut self) {
    self.signal_exit();
    self.master.take();
    if let Some(reader) = self.reader.take() {
      let (done, rx) = std::sync::mpsc::channel();
      thread::spawn(move || {
        let _ = reader.join();
        let _ = done.send(());
      });
      if rx.recv_timeout(Duration::from_secs(2)).is_err() {
        tracing::warn!(id = self.id, "terminal reader did not exit within 2s; detaching");
      }
    }
    if let Ok(mut child) = self.child.lock() {
      let _ = child.wait();
    }
  }

  fn signal_exit(&self) {
    #[cfg(unix)]
    {
      if self.child_pid == 0 {
        return;
      }
      let pid = self.child_pid as i32;
      // SAFETY: pid is the PTY session leader from spawn (`setsid`).
      unsafe {
        libc::kill(pid, libc::SIGTERM);
      }
      let start = Instant::now();
      while start.elapsed() < Duration::from_millis(100) {
        if self.child_has_exited() {
          return;
        }
        thread::sleep(Duration::from_millis(10));
      }
      // SAFETY: negative pid signals the process group so trapped TERM descendants die.
      unsafe {
        libc::kill(-pid, libc::SIGKILL);
        libc::kill(pid, libc::SIGKILL);
      }
    }
    #[cfg(not(unix))]
    {
      if let Ok(mut child) = self.child.lock() {
        let _ = child.kill();
      }
    }
  }

  fn child_has_exited(&self) -> bool {
    match self.child.lock() {
      Ok(mut child) => child.try_wait().ok().flatten().is_some(),
      Err(_) => true,
    }
  }

  pub fn write_data(&self, data: &str) -> Result<()> {
    let mut writer = self.writer.lock().map_err(|e| Error::Other(e.to_string()))?;
    writer.write_all(data.as_bytes())?;
    writer.flush()?;
    Ok(())
  }

  pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
    let Some(master) = self.master.as_ref() else {
      return Ok(());
    };
    master
      .resize(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
      })
      .map_err(|e| Error::Other(e.to_string()))
  }
}

impl Drop for PtySession {
  fn drop(&mut self) {
    self.shutdown();
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
