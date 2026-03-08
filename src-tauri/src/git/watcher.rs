use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
use tauri::{Emitter, WebviewWindow};

pub type WatcherState = Mutex<HashMap<String, WatcherHandle>>;

pub struct WatcherHandle {
  stop_tx: mpsc::Sender<()>,
}

impl Drop for WatcherHandle {
  fn drop(&mut self) {
    let _ = self.stop_tx.send(());
  }
}

pub fn start_watcher(window: &WebviewWindow, repo_root: &Path, watcher_state: &WatcherState) -> notify::Result<()> {
  let (tx, rx) = mpsc::channel();
  let (stop_tx, stop_rx) = mpsc::channel();

  let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
  debouncer.watcher().watch(repo_root, notify::RecursiveMode::Recursive)?;

  let window_clone = window.clone();
  let label = window.label().to_string();
  std::thread::spawn(move || {
    let _debouncer = debouncer; // keep alive
    loop {
      match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(events) => {
          if let Ok(events) = events {
            let has_relevant = events.iter().any(|e| {
              if e.kind != DebouncedEventKind::Any {
                return false;
              }
              let path = e.path.to_string_lossy();
              // Exclude all .git/ internal changes (cross-platform separators).
              // Working tree changes from branch switches, staging, etc. are
              // handled by Tauri commands calling setStatus directly.
              !path.contains(".git/") && !path.contains(".git\\")
            });
            if has_relevant {
              let _ = window_clone.emit("repository-changed", ());
            }
          }
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
      }
      if stop_rx.try_recv().is_ok() {
        break;
      }
    }
  });

  let mut watchers = watcher_state
    .lock()
    .map_err(|_| notify::Error::generic("lock poisoned"))?;
  watchers.insert(label, WatcherHandle { stop_tx });

  Ok(())
}
