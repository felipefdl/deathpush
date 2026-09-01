use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use notify_debouncer_mini::{DebouncedEvent, new_debouncer};
use tauri::{Emitter, Manager, WebviewWindow};

pub struct WatcherHandle {
  stop_tx: mpsc::Sender<()>,
}

impl Drop for WatcherHandle {
  fn drop(&mut self) {
    let _ = self.stop_tx.send(());
  }
}

#[cfg(test)]
impl WatcherHandle {
  pub(crate) fn for_test() -> Self {
    let (stop_tx, _) = mpsc::channel();
    Self { stop_tx }
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

fn has_relevant_change(events: &[DebouncedEvent]) -> bool {
  events
    .iter()
    .any(|event| is_relevant_change(&event.path.to_string_lossy()))
}

pub fn start_watcher(window: &WebviewWindow, repo_root: &Path) -> notify::Result<WatcherHandle> {
  let (tx, rx) = mpsc::channel();
  let (stop_tx, stop_rx) = mpsc::channel();

  let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
  debouncer.watcher().watch(repo_root, notify::RecursiveMode::Recursive)?;

  let app_handle = window.app_handle().clone();
  std::thread::spawn(move || {
    let _debouncer = debouncer; // keep alive
    loop {
      match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(events) => {
          if let Ok(events) = events {
            let has_relevant = has_relevant_change(&events);
            if has_relevant {
              let _ = app_handle.emit("repository-changed", ());
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

  Ok(WatcherHandle { stop_tx })
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;

  use notify_debouncer_mini::{DebouncedEvent, DebouncedEventKind};

  use super::has_relevant_change;

  #[test]
  fn continuous_write_is_relevant() {
    let events = [DebouncedEvent::new(
      PathBuf::from("/repo/src/file.ts"),
      DebouncedEventKind::AnyContinuous,
    )];

    assert!(has_relevant_change(&events));
  }

  #[test]
  fn git_object_write_is_ignored() {
    let events = [DebouncedEvent::new(
      PathBuf::from("/repo/.git/objects/ab/cdef"),
      DebouncedEventKind::AnyContinuous,
    )];

    assert!(!has_relevant_change(&events));
  }
}
