use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::git::cli::GitCommandEvent;
use crate::session::SessionId;
use crate::session::types::SessionStatusEvent;
use crate::types::PathsChanged;

/// Everything core tells a session about. Replaces the former Tauri emits.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum CoreEvent {
  SessionStatus(SessionStatusEvent),
  PathsChanged(PathsChanged),
  WatcherError(String),
  GitCommand(GitCommandEvent),
  TerminalData { id: u64, data: String },
  TerminalExited { id: u64 },
}

#[derive(Default)]
pub struct EventHub {
  senders: Mutex<HashMap<SessionId, UnboundedSender<CoreEvent>>>,
}

impl EventHub {
  pub fn subscribe(&self, id: SessionId) -> UnboundedReceiver<CoreEvent> {
    let (tx, rx) = unbounded_channel();
    self.lock().insert(id, tx);
    rx
  }

  pub fn unsubscribe(&self, id: SessionId) {
    self.lock().remove(&id);
  }

  pub fn send(&self, id: SessionId, event: CoreEvent) {
    if let Some(tx) = self.lock().get(&id) {
      let _ = tx.send(event);
    }
  }

  pub fn broadcast(&self, event: CoreEvent) {
    for tx in self.lock().values() {
      let _ = tx.send(event.clone());
    }
  }

  fn lock(&self) -> MutexGuard<'_, HashMap<SessionId, UnboundedSender<CoreEvent>>> {
    self.senders.lock().unwrap_or_else(|err| err.into_inner())
  }
}

#[cfg(test)]
mod tests {
  use super::{CoreEvent, EventHub};
  use crate::session::SessionId;

  #[test]
  fn send_reaches_only_the_subscribed_session() {
    let hub = EventHub::default();
    let mut a = hub.subscribe(SessionId(1));
    let mut b = hub.subscribe(SessionId(2));
    hub.send(SessionId(1), CoreEvent::WatcherError("x".into()));
    assert!(matches!(a.try_recv(), Ok(CoreEvent::WatcherError(msg)) if msg == "x"));
    assert!(b.try_recv().is_err());
  }

  #[test]
  fn broadcast_reaches_every_session() {
    let hub = EventHub::default();
    let mut a = hub.subscribe(SessionId(1));
    let mut b = hub.subscribe(SessionId(2));
    hub.broadcast(CoreEvent::TerminalExited { id: 7 });
    assert!(matches!(a.try_recv(), Ok(CoreEvent::TerminalExited { id: 7 })));
    assert!(matches!(b.try_recv(), Ok(CoreEvent::TerminalExited { id: 7 })));
  }

  #[test]
  fn unsubscribe_drops_the_sender() {
    let hub = EventHub::default();
    let mut a = hub.subscribe(SessionId(1));
    hub.unsubscribe(SessionId(1));
    hub.send(SessionId(1), CoreEvent::WatcherError("late".into()));
    assert!(a.try_recv().is_err());
  }
}
