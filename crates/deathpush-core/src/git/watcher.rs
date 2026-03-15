use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};

use crate::events::EventSink;

pub type WatcherState = Mutex<HashMap<String, WatcherHandle>>;

pub struct WatcherHandle {
  stop_tx: mpsc::Sender<()>,
}

impl Drop for WatcherHandle {
  fn drop(&mut self) {
    let _ = self.stop_tx.send(());
  }
}

fn is_relevant_change(path: &str) -> bool {
  // Allow all working tree changes
  if !path.contains(".git/") && !path.contains(".git\\") {
    return true;
  }
  // Inside .git/: allow status-relevant files (HEAD, index, refs, config, etc.)
  // but exclude transient and bulk files that cause rapid-fire events.
  if path.contains("index.lock")
    || path.contains(".git/objects/")
    || path.contains(".git\\objects\\")
    || path.contains(".git/logs/")
    || path.contains(".git\\logs\\")
    || path.contains(".watchman-cookie-")
  {
    return false;
  }
  true
}

pub fn start_watcher(
  session_id: &str,
  event_sink: Arc<dyn EventSink>,
  repo_root: &Path,
  watcher_state: &WatcherState,
) -> notify::Result<()> {
  let (tx, rx) = mpsc::channel();
  let (stop_tx, stop_rx) = mpsc::channel();

  let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
  debouncer.watcher().watch(repo_root, notify::RecursiveMode::Recursive)?;

  let session_id = session_id.to_string();
  let label = session_id.clone();
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
              is_relevant_change(&path)
            });
            if has_relevant {
              event_sink.emit_repository_changed(&session_id);
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
