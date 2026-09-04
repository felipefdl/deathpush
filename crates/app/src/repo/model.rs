use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use deathpush_core::config::recent_files::{load_recent_files, save_recent_files};
use deathpush_core::session::types::{Intent, IntentOutcome, SessionSnapshot, SessionStatusEvent};
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

use super::file_viewer::autosave::should_retry_skipped_write;
use super::state::{NetworkOp, OpenFile, PayloadVerdict, RepoState};
use crate::config::AppConfig;

pub enum RepoEvent {
  /// State changed; views re-read `state()`.
  Changed,
  /// A failed intent; the shell shows the toast.
  Error(String),
  /// A file write finished; the viewer completes `SaveState` from this, not from sync.
  Saved {
    path: String,
    hash: String,
    generation: u64,
  },
}

/// One window's repository session: applies core outcomes and events to `RepoState` and sends intents.
pub struct RepoModel {
  core: Arc<Core>,
  session: SessionId,
  state: RepoState,
  blame_requested: Option<String>,
  latest_write: Option<(String, u64)>,
  pending_write: Option<PendingWrite>,
  write_gen: u64,
  reservations: Arc<Mutex<ReservationTable>>,
  parked_writes: HashMap<String, ParkedWrite>,
}

impl EventEmitter<RepoEvent> for RepoModel {}

/// Whether to dispatch `OpenBlame` for the open path.
pub fn should_request_blame(
  blame_enabled: bool,
  dirty: bool,
  requested_path: Option<&str>,
  path: &str,
  has_content: bool,
) -> bool {
  blame_enabled && !dirty && has_content && requested_path != Some(path)
}

/// New path for the open file after a rename or move of that path or an ancestor.
pub fn retarget_open_path(current: Option<&str>, old_path: &str, new_path: &str) -> Option<String> {
  let current = current?;
  if current == old_path {
    return Some(new_path.to_string());
  }
  if old_path.is_empty() {
    return None;
  }
  current
    .strip_prefix(old_path)
    .and_then(|rest| rest.strip_prefix('/'))
    .map(|rest| {
      if new_path.is_empty() {
        rest.to_string()
      } else {
        format!("{new_path}/{rest}")
      }
    })
}

/// Where an in-flight write should land, if it should still hit disk.
pub fn write_path_still_current<'a>(
  open_path: Option<&'a str>,
  requested_path: &'a str,
  latest_write: Option<(&'a str, u64)>,
  generation: u64,
) -> Option<&'a str> {
  let open = open_path?;
  if open == requested_path {
    return Some(requested_path);
  }
  match latest_write {
    Some((tracked, latest_gen)) if tracked == open && latest_gen == generation => Some(open),
    _ => None,
  }
}

/// Same-path open keeps the buffer and only updates `pending_line`.
pub fn open_file_reuses_buffer(current: Option<&str>, path: &str) -> bool {
  current == Some(path)
}

/// A path mutation must wait for an in-flight write when it is the open file or an ancestor of it.
pub fn mutation_awaits_pending_write(open_path: Option<&str>, mutated_path: &str) -> bool {
  let Some(open) = open_path else {
    return false;
  };
  open == mutated_path
    || (!mutated_path.is_empty()
      && open
        .strip_prefix(mutated_path)
        .is_some_and(|rest| rest.starts_with('/')))
}

pub async fn await_pending_write(waiter: Option<tokio::sync::watch::Receiver<bool>>) {
  let Some(mut rx) = waiter else {
    return;
  };
  if *rx.borrow() {
    return;
  }
  let _ = rx.changed().await;
}

/// A write to `write_path` must not start while `reserved_path` (or an ancestor) is mutating.
pub fn write_blocked_by_reservation(reserved_path: &str, write_path: &str) -> bool {
  write_path == reserved_path
    || (!reserved_path.is_empty()
      && write_path
        .strip_prefix(reserved_path)
        .is_some_and(|rest| rest.starts_with('/')))
}

/// Where a parked write should land after the reservation is released. `None` drops it (delete).
pub fn parked_write_after_release(parked_path: &str, reserved_path: &str, new_path: Option<&str>) -> Option<String> {
  if !write_blocked_by_reservation(reserved_path, parked_path) {
    return Some(parked_path.to_string());
  }
  match new_path {
    None => None,
    Some(new_path) => retarget_open_path(Some(parked_path), reserved_path, new_path),
  }
}

pub fn should_replace_parked(existing: Option<u64>, incoming: u64) -> bool {
  existing.is_none_or(|generation| incoming > generation)
}

#[derive(Default)]
struct ReservationTable {
  next_id: u64,
  reserved: Vec<(u64, String)>,
  pending_flush: Vec<String>,
}

impl ReservationTable {
  fn insert(&mut self, path: String) -> u64 {
    let id = self.next_id;
    self.next_id = self.next_id.wrapping_add(1);
    self.reserved.push((id, path));
    id
  }

  fn remove(&mut self, id: u64) {
    self.reserved.retain(|(reserved_id, _)| *reserved_id != id);
  }

  fn is_blocked(&self, write_path: &str) -> bool {
    self
      .reserved
      .iter()
      .any(|(_, path)| write_blocked_by_reservation(path, write_path))
  }
}

fn lock_reservations(table: &Mutex<ReservationTable>) -> MutexGuard<'_, ReservationTable> {
  table.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub struct Reservation {
  id: u64,
  path: String,
  table: Arc<Mutex<ReservationTable>>,
  released: bool,
}

impl Reservation {
  fn acquire(table: Arc<Mutex<ReservationTable>>, path: &str) -> Self {
    let id = lock_reservations(&table).insert(path.to_string());
    Self {
      id,
      path: path.to_string(),
      table,
      released: false,
    }
  }

  fn release(&mut self) {
    if self.released {
      return;
    }
    lock_reservations(&self.table).remove(self.id);
    self.released = true;
  }
}

impl Drop for Reservation {
  fn drop(&mut self) {
    if self.released {
      return;
    }
    let mut table = lock_reservations(&self.table);
    table.remove(self.id);
    table.pending_flush.push(self.path.clone());
    self.released = true;
  }
}

struct PendingWrite {
  id: u64,
  path: String,
  done: tokio::sync::watch::Receiver<bool>,
}

struct ParkedWrite {
  content: String,
  expected_hash: String,
  generation: u64,
}

fn join_core_unit(result: Result<deathpush_core::Result<()>, tokio::task::JoinError>) -> Result<(), String> {
  match result {
    Ok(Ok(())) => Ok(()),
    Ok(Err(err)) => Err(err.to_string()),
    Err(err) => Err(err.to_string()),
  }
}

impl RepoModel {
  pub fn new(core: Arc<Core>, session: SessionId, snapshot: SessionSnapshot) -> Self {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot);
    Self {
      core,
      session,
      state,
      blame_requested: None,
      latest_write: None,
      pending_write: None,
      write_gen: 0,
      reservations: Arc::new(Mutex::new(ReservationTable::default())),
      parked_writes: HashMap::new(),
    }
  }

  pub fn pending_write_waiter(&self, mutated_path: &str) -> Option<tokio::sync::watch::Receiver<bool>> {
    let Some(pending) = &self.pending_write else {
      return None;
    };
    let open = self.state.open_file.as_ref().map(|open| open.path.as_str());
    if mutation_awaits_pending_write(open, mutated_path)
      || mutation_awaits_pending_write(Some(pending.path.as_str()), mutated_path)
    {
      Some(pending.done.clone())
    } else {
      None
    }
  }

  pub fn reserve_path(&self, path: &str) -> Reservation {
    Reservation::acquire(self.reservations.clone(), path)
  }

  fn write_is_reserved(&self, write_path: &str) -> bool {
    lock_reservations(&self.reservations).is_blocked(write_path)
  }

  fn flush_dropped_reservations(&mut self, cx: &mut Context<Self>) {
    let paths = {
      let mut table = lock_reservations(&self.reservations);
      std::mem::take(&mut table.pending_flush)
    };
    for path in paths {
      self.flush_parked_writes(&path, Some(&path), cx);
    }
  }

  fn park_write(&mut self, path: String, content: String, expected_hash: String, generation: u64) {
    let existing = self.parked_writes.get(&path).map(|parked| parked.generation);
    if !should_replace_parked(existing, generation) {
      return;
    }
    self.parked_writes.insert(
      path,
      ParkedWrite {
        content,
        expected_hash,
        generation,
      },
    );
  }

  fn finish_path_mutation(
    &mut self,
    mut reservation: Reservation,
    new_path: Option<&str>,
    succeeded: bool,
    cx: &mut Context<Self>,
  ) {
    let old = reservation.path.clone();
    if succeeded && let Some(new_path) = new_path {
      self.retarget_open_file(&old, new_path, cx);
    }
    let flush_to = if succeeded {
      new_path.map(str::to_string)
    } else {
      Some(old.clone())
    };
    reservation.release();
    self.flush_parked_writes(&old, flush_to.as_deref(), cx);
  }

  fn flush_parked_writes(&mut self, reserved_path: &str, new_path: Option<&str>, cx: &mut Context<Self>) {
    let keys: Vec<String> = self.parked_writes.keys().cloned().collect();
    for key in keys {
      if !write_blocked_by_reservation(reserved_path, &key) {
        continue;
      }
      let Some(parked) = self.parked_writes.remove(&key) else {
        continue;
      };
      let Some(path) = parked_write_after_release(&key, reserved_path, new_path) else {
        continue;
      };
      self.spawn_write(path, parked.content, parked.expected_hash, parked.generation, false, cx);
    }
  }

  pub fn with_path_mutation(
    &mut self,
    path: String,
    new_path: Option<String>,
    op: impl FnOnce() -> tokio::task::JoinHandle<deathpush_core::Result<()>> + 'static,
    cx: &mut Context<Self>,
    done: impl FnOnce(Result<(), String>) + 'static,
  ) {
    let reservation = self.reserve_path(&path);
    let waiter = self.pending_write_waiter(&path);
    cx.spawn(async move |this, cx| {
      await_pending_write(waiter).await;
      let result = join_core_unit(op().await);
      let _ = this.update(cx, |this, cx| {
        this.finish_path_mutation(reservation, new_path.as_deref(), result.is_ok(), cx);
      });
      done(result);
    })
    .detach();
  }

  pub fn state(&self) -> &RepoState {
    &self.state
  }

  #[allow(dead_code)]
  pub fn state_mut(&mut self) -> &mut RepoState {
    &mut self.state
  }

  pub(crate) fn core(&self) -> Arc<Core> {
    self.core.clone()
  }

  pub fn session(&self) -> SessionId {
    self.session
  }

  pub fn fuzzy_find_files(
    &self,
    query: String,
    max_results: usize,
  ) -> tokio::task::JoinHandle<deathpush_core::Result<Vec<deathpush_core::types::FuzzyFileResult>>> {
    let core = self.core.clone();
    let session = self.session;
    self
      .core
      .runtime_handle()
      .spawn_blocking(move || core.fuzzy_find_files(session, &query, max_results))
  }

  pub fn search_file_contents(
    &self,
    query: String,
    max_results: usize,
  ) -> tokio::task::JoinHandle<deathpush_core::Result<Vec<deathpush_core::types::ContentSearchResult>>> {
    let core = self.core.clone();
    let session = self.session;
    self
      .core
      .spawn(async move { core.search_file_contents(session, &query, max_results).await })
  }

  /// Send an intent to core; the outcome applies on the foreground executor.
  pub fn dispatch(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    if matches!(&intent, Intent::DeleteFile { confirmed: true, .. }) {
      self.dispatch_confirmed_delete(intent, window, cx);
      return;
    }
    self.state.mark_commit_intent(&intent);
    let clear_file = matches!(intent, Intent::ClearFile);
    if clear_file {
      self.state.pending_clear_file = true;
    }
    let root_at_send = self.state.root().map(str::to_string);
    let sent = intent.clone();
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let task = runtime.spawn(async move { core.session_intent(session, intent).await });
    cx.spawn_in(window, async move |this, cx| {
      let result = task.await;
      let _ = this.update_in(cx, |this, window, cx| {
        match result {
          Ok(Ok(outcome)) => this.apply_outcome(sent, outcome, root_at_send, clear_file, window, cx),
          Ok(Err(err)) => this.fail(err.to_string(), cx),
          Err(err) => this.fail(err.to_string(), cx),
        }
        cx.emit(RepoEvent::Changed);
        cx.notify();
      });
    })
    .detach();
  }

  fn dispatch_confirmed_delete(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    let path = match &intent {
      Intent::DeleteFile { path, confirmed: true } => path.clone(),
      _ => return,
    };
    let reservation = self.reserve_path(&path);
    let waiter = self.pending_write_waiter(&path);
    self.state.mark_commit_intent(&intent);
    let root_at_send = self.state.root().map(str::to_string);
    let sent = intent.clone();
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    cx.spawn_in(window, async move |this, cx| {
      await_pending_write(waiter).await;
      let task = runtime.spawn(async move { core.session_intent(session, intent).await });
      let result = task.await;
      let _ = this.update_in(cx, |this, window, cx| {
        let ok = matches!(&result, Ok(Ok(_)));
        this.finish_path_mutation(reservation, None, ok, cx);
        match result {
          Ok(Ok(outcome)) => this.apply_outcome(sent, outcome, root_at_send, false, window, cx),
          Ok(Err(err)) => this.fail(err.to_string(), cx),
          Err(err) => this.fail(err.to_string(), cx),
        }
        cx.emit(RepoEvent::Changed);
        cx.notify();
      });
    })
    .detach();
  }

  /// Dispatches a network intent and tracks it in `state.running` until the outcome arrives.
  pub fn dispatch_network(&mut self, op: NetworkOp, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    if self.state.network_busy() {
      return;
    }
    self.state.running.insert(op);
    cx.notify();
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let sent = intent.clone();
    let root_at_send = self.state.root().map(str::to_string);
    let task = runtime.spawn(async move { core.session_intent(session, intent).await });
    cx.spawn_in(window, async move |this, cx| {
      let result = task.await;
      let _ = this.update_in(cx, |this, window, cx| {
        this.state.running.remove(&op);
        match result {
          Ok(Ok(outcome)) => this.apply_outcome(sent, outcome, root_at_send, false, window, cx),
          Ok(Err(err)) => this.fail(err.to_string(), cx),
          Err(err) => this.fail(err.to_string(), cx),
        }
        cx.emit(RepoEvent::Changed);
        cx.notify();
      });
    })
    .detach();
  }

  pub fn refresh_nested_repositories(&mut self, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let task = handle.spawn_blocking(move || core.discover_nested_repositories(session));
    cx.spawn(async move |this, cx| {
      if let Ok(Ok(repos)) = task.await {
        let _ = this.update(cx, |this, cx| {
          this.state.nested_repositories = repos;
          cx.emit(RepoEvent::Changed);
          cx.notify();
        });
      }
    })
    .detach();
  }

  pub fn open_in_editor(&self, path: String, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let task = runtime.spawn(async move { core.open_in_editor(session, &path).await });
    cx.spawn(async move |this, cx| {
      let message = match task.await {
        Ok(Ok(())) => None,
        Ok(Err(err)) => Some(err.to_string()),
        Err(err) => Some(err.to_string()),
      };
      if let Some(message) = message {
        let _ = this.update(cx, |_, cx| {
          cx.emit(RepoEvent::Error(message));
        });
      }
    })
    .detach();
  }

  pub fn reveal_in_file_manager(&self, path: String, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let task = runtime.spawn(async move { core.reveal_in_file_manager(session, &path).await });
    cx.spawn(async move |this, cx| {
      let message = match task.await {
        Ok(Ok(())) => None,
        Ok(Err(err)) => Some(err.to_string()),
        Err(err) => Some(err.to_string()),
      };
      if let Some(message) = message {
        let _ = this.update(cx, |_, cx| {
          cx.emit(RepoEvent::Error(message));
        });
      }
    })
    .detach();
  }

  pub fn root_path(&self) -> Option<PathBuf> {
    self.state.root().map(PathBuf::from)
  }

  pub fn open_file(&mut self, path: &str, line: Option<usize>, cx: &mut Context<Self>) {
    if open_file_reuses_buffer(self.state.open_file.as_ref().map(|open| open.path.as_str()), path) {
      if let Some(open) = self.state.open_file.as_mut() {
        open.pending_line = line;
      }
      cx.emit(RepoEvent::Changed);
      cx.notify();
      return;
    }
    let load_id = self.state.open_file.as_ref().map(|open| open.load_id + 1).unwrap_or(1);
    self.state.open_file = Some(OpenFile {
      path: path.to_string(),
      content: None,
      pending_line: line,
      load_id,
      dirty: false,
    });
    self.state.cursor_line = line;
    self.state.blame = None;
    self.blame_requested = None;
    self.latest_write = None;
    cx.emit(RepoEvent::Changed);
    cx.notify();
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let path = path.to_string();
    let task = handle.spawn_blocking(move || core.read_file_content(session, &path));
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let _ = this.update(cx, |this, cx| this.apply_loaded_content(load_id, result, cx));
    })
    .detach();
  }

  pub fn retarget_open_file(&mut self, old_path: &str, new_path: &str, cx: &mut Context<Self>) {
    let Some(next) = retarget_open_path(
      self.state.open_file.as_ref().map(|open| open.path.as_str()),
      old_path,
      new_path,
    ) else {
      return;
    };
    if let Some(open) = self.state.open_file.as_mut() {
      open.path = next;
    }
    if let Some((tracked, _)) = &mut self.latest_write
      && let Some(tracked_next) = retarget_open_path(Some(tracked.as_str()), old_path, new_path)
    {
      *tracked = tracked_next;
    }
    cx.emit(RepoEvent::Changed);
    cx.notify();
  }

  pub fn close_file(&mut self, cx: &mut Context<Self>) {
    if self.state.open_file.is_none() && self.state.cursor_line.is_none() {
      return;
    }
    self.state.open_file = None;
    self.state.cursor_line = None;
    self.state.blame = None;
    self.blame_requested = None;
    self.latest_write = None;
    cx.emit(RepoEvent::Changed);
    cx.notify();
  }

  pub fn reload_open_file(&mut self, cx: &mut Context<Self>) {
    let Some(open) = self.state.open_file.as_ref() else {
      return;
    };
    let load_id = open.load_id;
    let path = open.path.clone();
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let task = handle.spawn_blocking(move || core.read_file_content(session, &path));
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let _ = this.update(cx, |this, cx| this.apply_loaded_content(load_id, result, cx));
    })
    .detach();
  }

  pub fn set_cursor_line(&mut self, line: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
    self.state.cursor_line = line;
    cx.notify();
    if line.is_none() {
      return;
    }
    self.maybe_request_blame(window, cx);
  }

  pub fn mark_open_file_dirty(&mut self, cx: &mut Context<Self>) {
    if let Some(open) = self.state.open_file.as_mut() {
      open.dirty = true;
    }
    self.state.blame = None;
    self.blame_requested = None;
    cx.emit(RepoEvent::Changed);
    cx.notify();
  }

  pub fn mark_open_file_saved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(open) = self.state.open_file.as_mut() {
      open.dirty = false;
    }
    self.blame_requested = None;
    self.maybe_request_blame(window, cx);
    cx.emit(RepoEvent::Changed);
    cx.notify();
  }

  fn maybe_request_blame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(open) = self.state.open_file.as_ref() else {
      return;
    };
    if !should_request_blame(
      AppConfig::get(cx).settings.git.blame,
      open.dirty,
      self.blame_requested.as_deref(),
      &open.path,
      open.content.is_some(),
    ) {
      return;
    }
    let path = open.path.clone();
    self.blame_requested = Some(path.clone());
    self.dispatch(Intent::OpenBlame { path }, window, cx);
  }

  pub fn write_open_file(
    &mut self,
    path: String,
    content: String,
    expected_hash: String,
    generation: u64,
    cx: &mut Context<Self>,
  ) {
    match &mut self.latest_write {
      Some((tracked, latest)) if tracked == &path => {
        if generation > *latest {
          *latest = generation;
        }
      }
      _ => self.latest_write = Some((path.clone(), generation)),
    }
    self.flush_dropped_reservations(cx);
    if self.write_is_reserved(&path) {
      self.park_write(path, content, expected_hash, generation);
      return;
    }
    self.spawn_write(path, content, expected_hash, generation, false, cx);
  }

  fn spawn_write(
    &mut self,
    path: String,
    content: String,
    expected_hash: String,
    generation: u64,
    retry: bool,
    cx: &mut Context<Self>,
  ) {
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let written = content.clone();
    let prev = self.pending_write.as_ref().map(|pending| pending.done.clone());
    self.write_gen = self.write_gen.wrapping_add(1);
    let id = self.write_gen;
    let (tx, rx) = tokio::sync::watch::channel(false);
    self.pending_write = Some(PendingWrite {
      id,
      path: path.clone(),
      done: rx,
    });
    cx.spawn(async move |this, cx| {
      await_pending_write(prev).await;
      let dest = match this.update(cx, |this, _cx| {
        let dest = write_path_still_current(
          this.state.open_file.as_ref().map(|open| open.path.as_str()),
          &path,
          this
            .latest_write
            .as_ref()
            .map(|(tracked, latest)| (tracked.as_str(), *latest)),
          generation,
        )
        .map(str::to_string)?;
        if this.write_is_reserved(&dest) {
          this.park_write(dest, written.clone(), expected_hash.clone(), generation);
          this.clear_pending_write(id);
          return None;
        }
        Some(dest)
      }) {
        Ok(Some(dest)) => dest,
        _ => {
          let _ = this.update(cx, |this, _cx| this.clear_pending_write(id));
          let _ = tx.send(true);
          return;
        }
      };
      let write_dest = dest.clone();
      let task = handle.spawn_blocking(move || core.write_file(session, &write_dest, &content));
      let result = task.await;
      let _ = this.update(cx, |this, cx| {
        this.clear_pending_write(id);
        match result {
          Ok(Ok(result)) => {
            let hash = result.content_hash;
            let (path_match, current_hash) = match this.state.open_file.as_ref() {
              Some(open) if open.path == dest => (true, open.content.as_ref().map(|file| file.content_hash.clone())),
              _ => (false, None),
            };
            if path_match && current_hash.as_deref() == Some(expected_hash.as_str()) {
              if let Some(file) = this.state.open_file.as_mut().and_then(|open| open.content.as_mut()) {
                file.content_hash = hash.clone();
                file.content = written;
              }
              cx.emit(RepoEvent::Saved {
                path: dest,
                hash,
                generation,
              });
              cx.notify();
            } else if let Some(current) = current_hash {
              let latest = this
                .latest_write
                .as_ref()
                .filter(|(tracked, _)| tracked == &dest)
                .map(|(_, latest)| *latest)
                .unwrap_or(0);
              if should_retry_skipped_write(path_match, retry, generation, latest) {
                this.spawn_write(dest, written, current, generation, true, cx);
              }
            }
          }
          Ok(Err(err)) => this.fail(err.to_string(), cx),
          Err(err) => this.fail(err.to_string(), cx),
        }
      });
      let _ = tx.send(true);
    })
    .detach();
  }

  fn clear_pending_write(&mut self, id: u64) {
    if self.pending_write.as_ref().is_some_and(|pending| pending.id == id) {
      self.pending_write = None;
    }
  }

  fn apply_loaded_content(
    &mut self,
    load_id: u64,
    result: Result<deathpush_core::Result<deathpush_core::types::FileContent>, tokio::task::JoinError>,
    cx: &mut Context<Self>,
  ) {
    if self.state.open_file.as_ref().is_none_or(|open| open.load_id != load_id) {
      return;
    }
    match result {
      Ok(Ok(content)) => {
        if self.state.open_file.as_ref().is_some_and(|open| open.dirty) {
          return;
        }
        if let Some(open) = self.state.open_file.as_mut() {
          open.content = Some(content);
        }
        cx.emit(RepoEvent::Changed);
        cx.notify();
      }
      Ok(Err(_)) | Err(_) => self.close_file(cx),
    }
  }

  pub fn record_recent_file(&self, path: &str, cx: &App) {
    let Some(root) = self.state.root().map(str::to_string) else {
      return;
    };
    let dir = AppConfig::get(cx).dir().to_path_buf();
    let path = path.to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let handle = self.core.runtime_handle().clone();
    drop(handle.spawn_blocking(move || {
      let mut files = load_recent_files(&dir, &root);
      files.add(&path, &now);
      save_recent_files(&dir, &root, &files)
    }));
  }

  fn fail(&mut self, message: String, cx: &mut Context<Self>) {
    self.state.pending_clear_file = false;
    self.state.resolve_commit_outcome(false);
    self.state.error = Some(message.clone());
    cx.emit(RepoEvent::Error(message));
  }

  fn apply_outcome(
    &mut self,
    sent: Intent,
    outcome: IntentOutcome,
    root_at_send: Option<String>,
    clear_file: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let confirming = matches!(outcome, IntentOutcome::NeedsConfirmation { .. });
    self.state.resolve_commit_outcome(confirming);
    match outcome {
      IntentOutcome::Snapshot { snapshot } => self.state.apply_snapshot(*snapshot),
      IntentOutcome::Patch {
        patch,
        session_generation,
        session_revision,
      } => self.state.apply_patch(patch, session_generation, session_revision),
      IntentOutcome::Ack {
        session_generation,
        session_revision,
      } => self.state.apply_ack(session_generation, session_revision, clear_file),
      IntentOutcome::Diff {
        payload,
        session_generation,
        session_revision,
      } => {
        if self
          .state
          .accept_payload(session_generation, session_revision, root_at_send.as_deref())
          == PayloadVerdict::Accept
        {
          self.state.diff = Some(payload);
          self.state.diff_load_id = Some(self.state.selected_load_id);
        }
      }
      IntentOutcome::Blame {
        payload,
        session_generation,
        session_revision,
      } => {
        let dirty = self.state.open_file.as_ref().is_some_and(|open| open.dirty);
        if !dirty
          && self
            .state
            .accept_payload(session_generation, session_revision, root_at_send.as_deref())
            == PayloadVerdict::Accept
        {
          self.state.blame = Some(payload);
        }
      }
      IntentOutcome::NeedsConfirmation { message, .. } => {
        let answer = window.prompt(
          PromptLevel::Warning,
          "Confirm",
          Some(&message),
          &["Continue", "Cancel"],
          cx,
        );
        cx.spawn_in(window, async move |this, cx| {
          if let Ok(0) = answer.await {
            let _ = this.update_in(cx, |this, window, cx| this.dispatch(sent.confirmed(), window, cx));
          } else {
            let _ = this.update_in(cx, |this, _, cx| {
              this.state.resolve_commit_outcome(false);
              cx.emit(RepoEvent::Changed);
              cx.notify();
            });
          }
        })
        .detach();
      }
    }
  }

  /// Cmd/Ctrl+Shift+G: reload the session state from scratch.
  pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.refresh_nested_repositories(cx);
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let task = runtime.spawn(async move { core.session_snapshot(session).await });
    cx.spawn_in(window, async move |this, cx| {
      let result = task.await;
      let _ = this.update_in(cx, |this, _, cx| {
        match result {
          Ok(Ok(snapshot)) => this.state.apply_snapshot(snapshot),
          Ok(Err(err)) => this.fail(err.to_string(), cx),
          Err(err) => this.fail(err.to_string(), cx),
        }
        cx.emit(RepoEvent::Changed);
        cx.notify();
      });
    })
    .detach();
  }

  pub fn apply_status_event(&mut self, event: SessionStatusEvent, cx: &mut Context<Self>) {
    self.state.apply_status_event(event);
    self.flush_dropped_reservations(cx);
    cx.emit(RepoEvent::Changed);
    cx.notify();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn requests_blame_again_after_save() {
    assert!(
      !should_request_blame(true, true, None, "a.rs", true),
      "dirty does not request"
    );
    assert!(
      !should_request_blame(true, false, Some("a.rs"), "a.rs", true),
      "already requested for this path"
    );
    assert!(
      should_request_blame(true, false, None, "a.rs", true),
      "after save, requested is cleared"
    );
    assert!(!should_request_blame(false, false, None, "a.rs", true));
    assert!(!should_request_blame(true, false, None, "a.rs", false));
  }

  #[test]
  fn retarget_keeps_the_open_buffer_on_the_new_path() {
    assert_eq!(
      retarget_open_path(Some("src/a.rs"), "src/a.rs", "src/b.rs").as_deref(),
      Some("src/b.rs")
    );
    assert_eq!(retarget_open_path(Some("src/x.rs"), "src/a.rs", "src/b.rs"), None);
    assert_eq!(retarget_open_path(None, "src/a.rs", "src/b.rs"), None);
    assert_eq!(
      retarget_open_path(Some("src/a.rs"), "src", "lib").as_deref(),
      Some("lib/a.rs")
    );
    assert_eq!(
      retarget_open_path(Some("src2/a.rs"), "src", "lib"),
      None,
      "a sibling prefix does not retarget"
    );
  }

  #[test]
  fn in_flight_write_follows_the_current_path_or_drops() {
    assert_eq!(
      write_path_still_current(Some("src/a.rs"), "src/a.rs", Some(("src/a.rs", 1)), 1),
      Some("src/a.rs")
    );
    assert_eq!(
      write_path_still_current(Some("lib/a.rs"), "src/a.rs", Some(("lib/a.rs", 1)), 1),
      Some("lib/a.rs"),
      "retarget routes the write to the new path"
    );
    assert_eq!(
      write_path_still_current(Some("b.rs"), "a.rs", None, 1),
      None,
      "a path switch drops the write"
    );
    assert_eq!(
      write_path_still_current(Some("b.rs"), "a.rs", Some(("b.rs", 2)), 1),
      None,
      "a later write on another file does not take the old bytes"
    );
    assert_eq!(write_path_still_current(None, "a.rs", Some(("a.rs", 1)), 1), None);
  }

  #[test]
  fn opening_the_same_path_only_sets_pending_line() {
    assert!(open_file_reuses_buffer(Some("a.rs"), "a.rs"));
    assert!(!open_file_reuses_buffer(Some("a.rs"), "b.rs"));
    assert!(!open_file_reuses_buffer(None, "a.rs"));
  }

  #[test]
  fn path_mutation_awaits_a_write_to_the_open_file_or_its_ancestor() {
    assert!(mutation_awaits_pending_write(Some("src/a.rs"), "src/a.rs"));
    assert!(mutation_awaits_pending_write(Some("src/a.rs"), "src"));
    assert!(!mutation_awaits_pending_write(Some("src/a.rs"), "lib"));
    assert!(!mutation_awaits_pending_write(Some("src/a.rs"), "src2"));
    assert!(!mutation_awaits_pending_write(Some("src/a.rs"), "src/a"));
    assert!(!mutation_awaits_pending_write(None, "src/a.rs"));
  }

  #[test]
  fn reservation_blocks_the_path_and_descendants_not_siblings() {
    assert!(write_blocked_by_reservation("src", "src"));
    assert!(write_blocked_by_reservation("src", "src/a.rs"));
    assert!(write_blocked_by_reservation("src/a.rs", "src/a.rs"));
    assert!(!write_blocked_by_reservation("src", "src2/a.rs"));
    assert!(!write_blocked_by_reservation("src/a.rs", "src/b.rs"));
    assert!(!write_blocked_by_reservation("src", "lib/a.rs"));
  }

  #[test]
  fn parked_write_re_flushes_to_the_retargeted_path() {
    assert_eq!(
      parked_write_after_release("src/a.rs", "src", Some("lib")).as_deref(),
      Some("lib/a.rs")
    );
    assert_eq!(
      parked_write_after_release("src/a.rs", "src/a.rs", Some("src/b.rs")).as_deref(),
      Some("src/b.rs")
    );
    assert!(should_replace_parked(None, 1));
    assert!(should_replace_parked(Some(1), 2));
    assert!(!should_replace_parked(Some(2), 2));
    assert!(!should_replace_parked(Some(3), 2));
  }

  #[test]
  fn delete_drops_parked_writes_for_the_deleted_path() {
    assert_eq!(parked_write_after_release("src/a.rs", "src/a.rs", None), None);
    assert_eq!(parked_write_after_release("src/a.rs", "src", None), None);
    assert_eq!(
      parked_write_after_release("lib/a.rs", "src", None).as_deref(),
      Some("lib/a.rs")
    );
  }

  #[test]
  fn dropping_the_guard_clears_the_reservation() {
    let table = Arc::new(Mutex::new(ReservationTable::default()));
    let guard = Reservation::acquire(table.clone(), "src/a.rs");
    assert!(lock_reservations(&table).is_blocked("src/a.rs"));
    drop(guard);
    assert!(!lock_reservations(&table).is_blocked("src/a.rs"));
  }
}
