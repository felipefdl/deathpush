use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::Result;
use crate::git::status::{
  StatusScan, StatusScope, path_in_scopes, repository_status_from_entries, scan_baseline, scan_scopes,
};
use crate::git::watcher::{ClassifiedPath, WatcherMessage, should_watch_path};
use crate::types::{
  PathChangeKind, PathChangeScope, PathsChanged, RepoOperationState, RepositoryMetadata, RepositoryStatus, StatusEntry,
  StatusKey, StatusPatch, StatusPhase,
};

pub const COALESCE_MS: u64 = 75;
pub const STORM_EVENT_RATE: usize = 256;
pub const STORM_UNIQUE_SCOPES: usize = 128;
pub const STORM_BUSY_SCANS: u32 = 2;
pub const STORM_EXIT_QUIET_MS: u64 = 750;
pub const PATCH_CHUNK: usize = 256;
pub const STORM_SCAN_CAP: usize = 512;
pub const DIRTY_CAP: usize = 2048;
const STORM_COALESCE_MS: u64 = 500;
const SCAN_RETRY_MS: u64 = 75;
const WATCHER_CHANNEL_BOUND: usize = 512;

type PatchSink = Arc<dyn Fn(StatusPatch) + Send + Sync>;
type PathsSink = Arc<dyn Fn(PathsChanged) + Send + Sync>;

pub fn diff_status_maps(
  previous: &BTreeMap<StatusKey, StatusEntry>,
  next: &BTreeMap<StatusKey, StatusEntry>,
) -> (Vec<StatusEntry>, Vec<StatusKey>) {
  let mut upserts = Vec::new();
  let mut removals = Vec::new();
  for (key, entry) in next {
    match previous.get(key) {
      Some(old) if old == entry => {}
      _ => upserts.push(entry.clone()),
    }
  }
  for key in previous.keys() {
    if !next.contains_key(key) {
      removals.push(key.clone());
    }
  }
  (upserts, removals)
}

#[derive(Debug, Default)]
pub struct StormMachine {
  in_storm: bool,
  events: VecDeque<(Instant, String)>,
  busy_scans: u32,
  last_event: Option<Instant>,
}

impl StormMachine {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn in_storm(&self) -> bool {
    self.in_storm
  }

  pub fn enter(&mut self) {
    self.in_storm = true;
  }

  pub fn note_event(&mut self, now: Instant, scope_id: String) {
    self.last_event = Some(now);
    self.events.push_back((now, scope_id));
    let window = Duration::from_secs(1);
    while self
      .events
      .front()
      .is_some_and(|(time, _)| now.saturating_duration_since(*time) > window)
    {
      self.events.pop_front();
    }
    let unique = self
      .events
      .iter()
      .map(|(_, scope)| scope.as_str())
      .collect::<HashSet<_>>()
      .len();
    if self.events.len() >= STORM_EVENT_RATE || unique >= STORM_UNIQUE_SCOPES {
      self.in_storm = true;
    }
  }

  pub fn note_scan_finished(&mut self, pending: bool) {
    if pending {
      self.busy_scans += 1;
      if self.busy_scans >= STORM_BUSY_SCANS {
        self.in_storm = true;
      }
    } else {
      self.busy_scans = 0;
    }
  }

  pub fn should_exit(&self, now: Instant) -> bool {
    if !self.in_storm {
      return false;
    }
    match self.last_event {
      Some(last) => now.saturating_duration_since(last) >= Duration::from_millis(STORM_EXIT_QUIET_MS),
      None => true,
    }
  }

  pub fn exit(&mut self) {
    self.in_storm = false;
    self.events.clear();
    self.busy_scans = 0;
  }
}

struct CoordinatorState {
  generation: u64,
  revision: u64,
  entries: BTreeMap<StatusKey, StatusEntry>,
  metadata: Option<RepositoryMetadata>,
  dirty: HashSet<StatusScope>,
  overflow: bool,
  storm: StormMachine,
  scan_in_flight: bool,
  fail_next_scan: bool,
  fail_all_scans: bool,
  scan_failed: bool,
}

impl Default for CoordinatorState {
  fn default() -> Self {
    Self {
      generation: 0,
      revision: 0,
      entries: BTreeMap::new(),
      metadata: None,
      dirty: HashSet::new(),
      overflow: false,
      storm: StormMachine::new(),
      scan_in_flight: false,
      fail_next_scan: false,
      fail_all_scans: false,
      scan_failed: false,
    }
  }
}

pub struct StatusCoordinator {
  root: PathBuf,
  state: Mutex<CoordinatorState>,
  on_patch: Mutex<Option<PatchSink>>,
  on_paths: Mutex<Option<PathsSink>>,
  overflow_flag: Arc<AtomicBool>,
  scan_mutex: Mutex<()>,
  #[cfg(test)]
  during_scan: Mutex<Option<Box<dyn FnOnce() + Send>>>,
  #[cfg(test)]
  scan_attempts: std::sync::atomic::AtomicUsize,
}

impl StatusCoordinator {
  pub fn new(root: PathBuf) -> Self {
    Self {
      root,
      state: Mutex::new(CoordinatorState::default()),
      on_patch: Mutex::new(None),
      on_paths: Mutex::new(None),
      overflow_flag: Arc::new(AtomicBool::new(false)),
      scan_mutex: Mutex::new(()),
      #[cfg(test)]
      during_scan: Mutex::new(None),
      #[cfg(test)]
      scan_attempts: std::sync::atomic::AtomicUsize::new(0),
    }
  }

  #[cfg(test)]
  pub fn with_emitter(emit: impl Fn(StatusPatch) + Send + Sync + 'static) -> Self {
    Self {
      root: PathBuf::new(),
      state: Mutex::new(CoordinatorState::default()),
      on_patch: Mutex::new(Some(Arc::new(emit))),
      on_paths: Mutex::new(None),
      overflow_flag: Arc::new(AtomicBool::new(false)),
      scan_mutex: Mutex::new(()),
      #[cfg(test)]
      during_scan: Mutex::new(None),
      #[cfg(test)]
      scan_attempts: std::sync::atomic::AtomicUsize::new(0),
    }
  }

  pub fn bind_emitters(&self, on_patch: PatchSink, on_paths: PathsSink) {
    *self.on_patch.lock().unwrap_or_else(|err| err.into_inner()) = Some(on_patch);
    *self.on_paths.lock().unwrap_or_else(|err| err.into_inner()) = Some(on_paths);
  }

  pub fn snapshot(&self) -> RepositoryStatus {
    let state = self.lock();
    let metadata = state.metadata.clone().unwrap_or_else(|| RepositoryMetadata {
      root: self.root.to_string_lossy().to_string(),
      head_branch: None,
      head_commit: None,
      ahead: 0,
      behind: 0,
      operation_state: RepoOperationState::None,
    });
    let entries: Vec<StatusEntry> = state.entries.values().cloned().collect();
    repository_status_from_entries(metadata, &entries)
  }

  pub fn ensure_baseline(&self) -> Result<()> {
    let _scan_guard = self.scan_mutex.lock().unwrap_or_else(|err| err.into_inner());
    if self.lock().metadata.is_some() {
      return Ok(());
    }
    let Some(generation) = self.begin_scan() else {
      return Ok(());
    };
    let scan = match scan_baseline(&self.root) {
      Ok(scan) => scan,
      Err(err) => {
        self.end_scan();
        return Err(err);
      }
    };
    self.apply_scan_with_generation(generation, scan, None, StatusPhase::Settled);
    Ok(())
  }

  pub fn invalidate(&self, scope: StatusScope) {
    let mut state = self.lock();
    self.queue_scope(&mut state, scope);
  }

  pub fn invalidate_paths<I, S>(&self, paths: I)
  where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
  {
    let mut state = self.lock();
    for path in paths {
      self.queue_scope(&mut state, StatusScope::Exact(path.as_ref().to_string()));
    }
  }

  pub fn invalidate_paths_and_snapshot(&self, paths: &[String]) -> Result<RepositoryStatus> {
    self.invalidate_paths(paths.iter().map(String::as_str));
    self.scan_dirty_uncapped()?;
    Ok(self.snapshot())
  }

  pub fn invalidate_and_snapshot(&self, scope: StatusScope) -> Result<RepositoryStatus> {
    self.invalidate(scope);
    self.scan_dirty()?;
    Ok(self.snapshot())
  }

  pub fn overflow_flag(&self) -> Arc<AtomicBool> {
    Arc::clone(&self.overflow_flag)
  }

  pub fn in_storm(&self) -> bool {
    self.lock().storm.in_storm()
  }

  #[cfg(test)]
  pub fn dirty_promoted_to_repository(&self) -> bool {
    let state = self.lock();
    state.overflow && state.dirty.is_empty()
  }

  #[cfg(test)]
  pub fn apply_baseline_for_test(&self, entries: Vec<StatusEntry>) {
    let mut state = self.lock();
    state.generation += 1;
    let generation = state.generation;
    drop(state);
    self.apply_scan_with_generation(
      generation,
      StatusScan {
        entries,
        metadata: RepositoryMetadata {
          root: String::new(),
          head_branch: None,
          head_commit: None,
          ahead: 0,
          behind: 0,
          operation_state: RepoOperationState::None,
        },
      },
      None,
      StatusPhase::Settled,
    );
  }

  #[cfg(test)]
  pub fn force_storm_for_test(&self) {
    self.lock().storm.enter();
  }

  #[cfg(test)]
  pub fn take_scan_scopes_for_test(&self) -> Vec<StatusScope> {
    let mut state = self.lock();
    take_scan_scopes(&mut state, true)
      .map(|(scopes, _)| scopes)
      .unwrap_or_default()
  }

  #[cfg(test)]
  pub fn dirty_scopes_for_test(&self) -> HashSet<StatusScope> {
    self.lock().dirty.clone()
  }

  #[cfg(test)]
  pub fn fail_next_scan_for_test(&self) {
    self.lock().fail_next_scan = true;
  }

  #[cfg(test)]
  pub fn fail_all_scans_for_test(&self) {
    self.lock().fail_all_scans = true;
  }

  #[cfg(test)]
  pub fn scan_attempts_for_test(&self) -> usize {
    self.scan_attempts.load(Ordering::SeqCst)
  }

  #[cfg(test)]
  pub fn scan_dirty_with_extra_pending_for_test(&self, extra_pending: bool) -> Result<()> {
    self.scan_dirty_inner(|| extra_pending, true)
  }

  #[cfg(test)]
  pub fn scan_dirty_for_test(&self) -> Result<()> {
    self.scan_dirty()
  }

  #[cfg(test)]
  pub fn begin_scan_for_test(&self) -> bool {
    self.begin_scan().is_some()
  }

  #[cfg(test)]
  pub fn end_scan_for_test(&self) {
    self.end_scan();
  }

  #[cfg(test)]
  pub fn set_during_scan_hook_for_test(&self, hook: impl FnOnce() + Send + 'static) {
    *self.during_scan.lock().unwrap_or_else(|err| err.into_inner()) = Some(Box::new(hook));
  }

  #[cfg(test)]
  pub fn scan_from_channel_for_test(&self, rx: &mpsc::Receiver<WatcherMessage>) -> Result<()> {
    self.scan_from_channel(rx)
  }

  #[cfg(test)]
  fn run_during_scan_hook(&self) {
    if let Some(hook) = self.during_scan.lock().unwrap_or_else(|err| err.into_inner()).take() {
      hook();
    }
  }

  pub fn ingest(&self, message: WatcherMessage) {
    match message {
      WatcherMessage::Overflow => {
        let mut state = self.lock();
        state.overflow = true;
        state.dirty.clear();
        state.storm.enter();
        state.storm.note_event(Instant::now(), "*".into());
        let generation = state.generation;
        let storm = true;
        drop(state);
        self.emit_paths(PathsChanged {
          paths: vec![],
          kind: PathChangeKind::Structural,
          scope: PathChangeScope::Repository,
          generation,
          storm,
        });
      }
      WatcherMessage::Path(classified) => self.ingest_path(classified),
    }
  }

  pub fn ingest_path(&self, classified: ClassifiedPath) {
    if classified.kind == PathChangeKind::Content
      && let Ok(repo) = git2::Repository::open(&self.root)
      && !should_watch_path(&repo, &classified.relative)
    {
      return;
    }

    let scope = match classified.scope {
      PathChangeScope::Exact => StatusScope::Exact(classified.relative.clone()),
      PathChangeScope::Subtree => StatusScope::Subtree(classified.relative.clone()),
      PathChangeScope::Repository => StatusScope::Repository,
    };

    let mut state = self.lock();
    let now = Instant::now();
    state.storm.note_event(now, classified.relative.clone());
    self.queue_scope(&mut state, scope);
    let generation = state.generation;
    let storm = state.storm.in_storm();
    drop(state);

    self.emit_paths(PathsChanged {
      paths: vec![classified.relative],
      kind: classified.kind,
      scope: classified.scope,
      generation,
      storm,
    });
  }

  pub fn run_loop(&self, rx: mpsc::Receiver<WatcherMessage>) {
    loop {
      if self.overflow_flag.swap(false, Ordering::SeqCst) {
        self.ingest(WatcherMessage::Overflow);
      }

      let now = Instant::now();
      if self.lock().storm.should_exit(now) {
        let _ = self.exit_storm_with_baseline();
        continue;
      }

      let (has_dirty, scan_failed) = {
        let state = self.lock();
        (!state.dirty.is_empty() || state.overflow, state.scan_failed)
      };
      if !has_dirty || scan_failed {
        if has_dirty {
          let deadline = Instant::now() + Duration::from_millis(SCAN_RETRY_MS);
          loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
              break;
            }
            match rx.recv_timeout(remaining) {
              Ok(message) => self.ingest(message),
              Err(RecvTimeoutError::Timeout) => break,
              Err(RecvTimeoutError::Disconnected) => return,
            }
          }

          if self.overflow_flag.swap(false, Ordering::SeqCst) {
            self.ingest(WatcherMessage::Overflow);
          }

          if self.lock().storm.should_exit(Instant::now()) {
            let _ = self.exit_storm_with_baseline();
            continue;
          }
        } else {
          let wait = storm_recv_timeout(&self.lock().storm, now);
          let first = match wait {
            None => match rx.recv() {
              Ok(message) => message,
              Err(_) => return,
            },
            Some(timeout) => match rx.recv_timeout(timeout) {
              Ok(message) => message,
              Err(RecvTimeoutError::Timeout) => continue,
              Err(RecvTimeoutError::Disconnected) => return,
            },
          };
          self.ingest(first);

          let coalesce = if self.in_storm() {
            Duration::from_millis(STORM_COALESCE_MS)
          } else {
            Duration::from_millis(COALESCE_MS)
          };
          let mut deadline = Instant::now() + coalesce;
          if let Some(quiet) = storm_recv_timeout(&self.lock().storm, Instant::now()) {
            let quiet_deadline = Instant::now() + quiet;
            if quiet_deadline < deadline {
              deadline = quiet_deadline;
            }
          }
          loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
              break;
            }
            match rx.recv_timeout(remaining) {
              Ok(message) => self.ingest(message),
              Err(RecvTimeoutError::Timeout) => break,
              Err(RecvTimeoutError::Disconnected) => return,
            }
          }

          if self.overflow_flag.swap(false, Ordering::SeqCst) {
            self.ingest(WatcherMessage::Overflow);
          }

          if self.lock().storm.should_exit(Instant::now()) {
            let _ = self.exit_storm_with_baseline();
            continue;
          }
        }
      }
      let _ = self.scan_from_channel(&rx);
    }
  }

  fn scan_from_channel(&self, rx: &mpsc::Receiver<WatcherMessage>) -> Result<()> {
    while let Ok(message) = rx.try_recv() {
      self.ingest(message);
    }
    self.scan_dirty_inner(
      || {
        let mut extra_pending = self.overflow_flag.load(Ordering::SeqCst);
        while let Ok(message) = rx.try_recv() {
          extra_pending = true;
          self.ingest(message);
        }
        extra_pending
      },
      true,
    )
  }

  pub fn spawn_worker(self: &Arc<Self>) -> mpsc::SyncSender<WatcherMessage> {
    let (tx, rx) = mpsc::sync_channel(WATCHER_CHANNEL_BOUND);
    let coordinator = Arc::clone(self);
    std::thread::spawn(move || coordinator.run_loop(rx));
    tx
  }

  fn queue_scope(&self, state: &mut CoordinatorState, scope: StatusScope) {
    if matches!(scope, StatusScope::Repository) {
      state.dirty.clear();
      state.overflow = true;
      return;
    }
    if state.overflow {
      return;
    }
    state.dirty.insert(scope);
    if state.dirty.len() >= STORM_UNIQUE_SCOPES {
      state.storm.enter();
    }
    if state.dirty.len() > DIRTY_CAP {
      state.dirty.clear();
      state.overflow = true;
      state.storm.enter();
    }
  }

  fn begin_scan(&self) -> Option<u64> {
    let mut state = self.lock();
    if state.scan_in_flight {
      return None;
    }
    state.scan_in_flight = true;
    state.generation += 1;
    Some(state.generation)
  }

  fn end_scan(&self) {
    self.lock().scan_in_flight = false;
  }

  fn scan_dirty(&self) -> Result<()> {
    self.scan_dirty_inner(|| false, true)
  }

  fn scan_dirty_uncapped(&self) -> Result<()> {
    self.scan_dirty_inner(|| false, false)
  }

  fn scan_dirty_inner(&self, drain: impl FnOnce() -> bool, apply_storm_cap: bool) -> Result<()> {
    let _scan_guard = self.scan_mutex.lock().unwrap_or_else(|err| err.into_inner());
    let prepared = {
      let mut state = self.lock();
      match take_scan_scopes(&mut state, apply_storm_cap) {
        None => None,
        Some((scopes, storm)) => {
          if state.scan_in_flight {
            for scope in scopes {
              self.queue_scope(&mut state, scope);
            }
            return Ok(());
          }
          state.scan_in_flight = true;
          state.generation += 1;
          Some((scopes, storm, state.generation))
        }
      }
    };

    let Some((scopes, storm, generation)) = prepared else {
      let extra_pending = drain();
      let mut state = self.lock();
      let pending = extra_pending || state.overflow || self.overflow_flag.load(Ordering::SeqCst);
      if pending {
        state.storm.note_scan_finished(true);
      }
      return Ok(());
    };

    #[cfg(test)]
    self.scan_attempts.fetch_add(1, Ordering::SeqCst);
    let fail_next = {
      let mut state = self.lock();
      if state.fail_all_scans || state.fail_next_scan {
        state.fail_next_scan = false;
        true
      } else {
        false
      }
    };
    if fail_next {
      self.fail_scan(scopes);
      return Err(crate::error::Error::Other("scan failed".into()));
    }

    #[cfg(test)]
    self.run_during_scan_hook();

    let scan = match scan_result(&self.root, &scopes) {
      Ok(scan) => scan,
      Err(err) => {
        self.fail_scan(scopes);
        if is_index_locked(&err) {
          return Ok(());
        }
        return Err(err);
      }
    };

    let scoped = if scopes.iter().any(|scope| matches!(scope, StatusScope::Repository)) {
      None
    } else {
      Some(scopes)
    };
    let phase = if storm {
      StatusPhase::Storm
    } else {
      StatusPhase::Settled
    };
    self.apply_scan_with_generation(generation, scan, scoped.as_deref(), phase);

    let extra_pending = drain();
    let mut state = self.lock();
    let pending =
      extra_pending || !state.dirty.is_empty() || state.overflow || self.overflow_flag.load(Ordering::SeqCst);
    state.storm.note_scan_finished(pending);
    state.scan_failed = false;
    Ok(())
  }

  fn requeue_scopes(&self, scopes: Vec<StatusScope>) {
    let mut state = self.lock();
    for scope in scopes {
      self.queue_scope(&mut state, scope);
    }
  }

  fn fail_scan(&self, scopes: Vec<StatusScope>) {
    self.requeue_scopes(scopes);
    self.end_scan();
    self.lock().scan_failed = true;
  }

  fn exit_storm_with_baseline(&self) -> Result<()> {
    {
      let mut state = self.lock();
      state.storm.exit();
      state.dirty.clear();
      state.overflow = false;
    }
    let _scan_guard = self.scan_mutex.lock().unwrap_or_else(|err| err.into_inner());
    let Some(generation) = self.begin_scan() else {
      return Ok(());
    };
    let scan = match scan_baseline(&self.root) {
      Ok(scan) => scan,
      Err(err) => {
        self.end_scan();
        return Err(err);
      }
    };
    self.apply_scan_with_generation(generation, scan, None, StatusPhase::Settled);
    Ok(())
  }

  fn apply_scan_with_generation(
    &self,
    generation: u64,
    scan: StatusScan,
    scopes: Option<&[StatusScope]>,
    end_phase: StatusPhase,
  ) {
    let mut state = self.lock();
    if generation != state.generation {
      state.scan_in_flight = false;
      return;
    }
    let metadata = scan.metadata;

    let next_entries: BTreeMap<StatusKey, StatusEntry> = scan
      .entries
      .into_iter()
      .map(|entry| {
        (
          StatusKey {
            group: entry.group.clone(),
            path: entry.path.clone(),
          },
          entry,
        )
      })
      .collect();

    let (upserts, removals) = if let Some(scopes) = scopes {
      let previous: BTreeMap<StatusKey, StatusEntry> = state
        .entries
        .iter()
        .filter(|(key, _)| path_in_scopes(&key.path, scopes))
        .map(|(key, entry)| (key.clone(), entry.clone()))
        .collect();
      let (upserts, removals) = diff_status_maps(&previous, &next_entries);
      for key in &removals {
        state.entries.remove(key);
      }
      for (key, entry) in next_entries {
        state.entries.insert(key, entry);
      }
      (upserts, removals)
    } else {
      let previous = std::mem::take(&mut state.entries);
      let (upserts, removals) = diff_status_maps(&previous, &next_entries);
      state.entries = next_entries;
      (upserts, removals)
    };

    state.metadata = Some(metadata.clone());
    let mut remaining_upserts = upserts;
    let mut remaining_removals = removals;
    let mut emitted = false;

    while !emitted || !remaining_upserts.is_empty() || !remaining_removals.is_empty() {
      let mut budget = PATCH_CHUNK;
      let take_up = remaining_upserts.len().min(budget);
      let chunk_upserts: Vec<StatusEntry> = remaining_upserts.drain(..take_up).collect();
      budget -= take_up;
      let take_rm = remaining_removals.len().min(budget);
      let chunk_removals: Vec<StatusKey> = remaining_removals.drain(..take_rm).collect();
      let more = !remaining_upserts.is_empty() || !remaining_removals.is_empty();
      let base_revision = state.revision;
      state.revision += 1;
      let patch = StatusPatch {
        generation,
        base_revision,
        revision: state.revision,
        upserts: chunk_upserts,
        removals: chunk_removals,
        metadata: if more { None } else { Some(metadata.clone()) },
        phase: if more { StatusPhase::Scanning } else { end_phase },
      };
      emitted = true;
      drop(state);
      self.emit_patch(patch);
      state = self.lock();
    }
    state.scan_in_flight = false;
  }

  fn emit_patch(&self, patch: StatusPatch) {
    if let Some(emit) = self.on_patch.lock().unwrap_or_else(|err| err.into_inner()).clone() {
      emit(patch);
    }
  }

  fn emit_paths(&self, event: PathsChanged) {
    if let Some(emit) = self.on_paths.lock().unwrap_or_else(|err| err.into_inner()).clone() {
      emit(event);
    }
  }

  fn lock(&self) -> std::sync::MutexGuard<'_, CoordinatorState> {
    self.state.lock().unwrap_or_else(|err| err.into_inner())
  }
}

fn scan_result(root: &std::path::Path, scopes: &[StatusScope]) -> Result<StatusScan> {
  if scopes.iter().any(|scope| matches!(scope, StatusScope::Repository)) {
    scan_baseline(root)
  } else {
    scan_scopes(root, scopes)
  }
}

fn is_index_locked(err: &crate::error::Error) -> bool {
  match err {
    crate::error::Error::Git(git_err) => git_err.code() == git2::ErrorCode::Locked,
    _ => false,
  }
}

fn take_scan_scopes(state: &mut CoordinatorState, apply_storm_cap: bool) -> Option<(Vec<StatusScope>, bool)> {
  if state.overflow {
    state.dirty.clear();
    state.overflow = false;
    return Some((vec![StatusScope::Repository], state.storm.in_storm()));
  }
  if state.dirty.is_empty() {
    return None;
  }
  let mut scopes: Vec<StatusScope> = state.dirty.drain().collect();
  if apply_storm_cap && state.storm.in_storm() {
    let mut exact = Vec::new();
    let mut rest = Vec::new();
    for scope in scopes {
      if matches!(scope, StatusScope::Exact(_)) {
        exact.push(scope);
      } else {
        rest.push(scope);
      }
    }
    let mut chosen = Vec::new();
    for scope in exact.into_iter().chain(rest) {
      if chosen.len() >= STORM_SCAN_CAP {
        state.dirty.insert(scope);
      } else {
        chosen.push(scope);
      }
    }
    scopes = chosen;
  }
  Some((scopes, state.storm.in_storm()))
}

fn storm_recv_timeout(storm: &StormMachine, now: Instant) -> Option<Duration> {
  if !storm.in_storm() {
    return None;
  }
  let last = storm.last_event.unwrap_or(now);
  let quiet = Duration::from_millis(STORM_EXIT_QUIET_MS);
  let elapsed = now.saturating_duration_since(last);
  Some(quiet.saturating_sub(elapsed))
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;
  use std::time::{Duration, Instant};

  use super::{
    COALESCE_MS, DIRTY_CAP, PATCH_CHUNK, SCAN_RETRY_MS, STORM_BUSY_SCANS, STORM_EVENT_RATE, STORM_EXIT_QUIET_MS,
    STORM_SCAN_CAP, STORM_UNIQUE_SCOPES, StatusCoordinator, StormMachine, diff_status_maps,
  };
  use crate::git::status::StatusScope;
  use crate::git::watcher::{ClassifiedPath, WatcherMessage};
  use crate::types::{
    FileStatus, PathChangeKind, PathChangeScope, ResourceGroupKind, StatusEntry, StatusKey, StatusPhase,
  };

  fn entry(group: ResourceGroupKind, path: &str, status: FileStatus) -> StatusEntry {
    StatusEntry {
      group,
      path: path.to_string(),
      status,
      rename_path: None,
    }
  }

  fn key(group: ResourceGroupKind, path: &str) -> StatusKey {
    StatusKey {
      group,
      path: path.to_string(),
    }
  }

  #[test]
  fn storm_constants_match_spec() {
    assert_eq!(COALESCE_MS, 75);
    assert_eq!(STORM_EVENT_RATE, 256);
    assert_eq!(STORM_UNIQUE_SCOPES, 128);
    assert_eq!(STORM_BUSY_SCANS, 2);
    assert_eq!(STORM_EXIT_QUIET_MS, 750);
    assert_eq!(PATCH_CHUNK, 256);
    assert_eq!(STORM_SCAN_CAP, 512);
    assert_eq!(DIRTY_CAP, 2048);
  }

  #[test]
  fn patch_diff_upserts_changed_and_new_keys_and_removes_missing() {
    let mut previous = BTreeMap::new();
    previous.insert(
      key(ResourceGroupKind::WorkingTree, "a.rs"),
      entry(ResourceGroupKind::WorkingTree, "a.rs", FileStatus::Modified),
    );
    previous.insert(
      key(ResourceGroupKind::WorkingTree, "b.rs"),
      entry(ResourceGroupKind::WorkingTree, "b.rs", FileStatus::Untracked),
    );

    let mut next = BTreeMap::new();
    next.insert(
      key(ResourceGroupKind::WorkingTree, "b.rs"),
      entry(ResourceGroupKind::WorkingTree, "b.rs", FileStatus::Modified),
    );
    next.insert(
      key(ResourceGroupKind::Index, "c.rs"),
      entry(ResourceGroupKind::Index, "c.rs", FileStatus::IndexAdded),
    );

    let (upserts, removals) = diff_status_maps(&previous, &next);
    assert_eq!(upserts.len(), 2);
    assert!(
      upserts
        .iter()
        .any(|item| item.path == "b.rs" && item.status == FileStatus::Modified)
    );
    assert!(
      upserts
        .iter()
        .any(|item| item.path == "c.rs" && item.group == ResourceGroupKind::Index)
    );
    assert_eq!(removals, vec![key(ResourceGroupKind::WorkingTree, "a.rs")]);
  }

  #[test]
  fn storm_enters_at_256_events_per_second_and_exits_after_750ms_quiet() {
    let t0 = Instant::now();
    let mut storm = StormMachine::new();
    for index in 0..STORM_EVENT_RATE {
      storm.note_event(t0 + Duration::from_millis(index as u64), format!("p{index}"));
    }
    assert!(storm.in_storm());
    assert!(!storm.should_exit(t0 + Duration::from_millis(STORM_EXIT_QUIET_MS - 1)));
    assert!(storm.should_exit(t0 + Duration::from_millis((STORM_EVENT_RATE as u64 - 1) + STORM_EXIT_QUIET_MS)));
  }

  #[test]
  fn storm_enters_at_128_unique_scopes() {
    let t0 = Instant::now();
    let mut storm = StormMachine::new();
    for index in 0..STORM_UNIQUE_SCOPES {
      storm.note_event(t0, format!("scope-{index}"));
    }
    assert!(storm.in_storm());
  }

  #[test]
  fn storm_enters_after_two_busy_scans() {
    let mut storm = StormMachine::new();
    storm.note_scan_finished(true);
    assert!(!storm.in_storm());
    storm.note_scan_finished(true);
    assert!(storm.in_storm());
  }

  #[test]
  fn baseline_emits_chunks_of_256_then_settled() {
    let patches = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected = patches.clone();
    let coordinator = StatusCoordinator::with_emitter(move |patch| {
      collected.lock().unwrap_or_else(|err| err.into_inner()).push(patch);
    });
    let entries: Vec<StatusEntry> = (0..300)
      .map(|index| {
        entry(
          ResourceGroupKind::WorkingTree,
          &format!("f{index}.rs"),
          FileStatus::Untracked,
        )
      })
      .collect();
    coordinator.apply_baseline_for_test(entries);

    let patches = patches.lock().unwrap_or_else(|err| err.into_inner());
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].upserts.len(), 256);
    assert_eq!(patches[0].phase, StatusPhase::Scanning);
    assert_eq!(patches[1].upserts.len(), 44);
    assert_eq!(patches[1].phase, StatusPhase::Settled);
    assert_eq!(patches[1].base_revision, patches[0].revision);
    assert_eq!(patches[1].revision, patches[0].revision + 1);
    assert_eq!(patches[0].generation, patches[1].generation);
  }

  #[test]
  fn dirty_cap_discards_path_set_and_enters_storm() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    for index in 0..=DIRTY_CAP {
      coordinator.invalidate(StatusScope::Exact(format!("p{index}.rs")));
    }
    assert!(coordinator.in_storm());
    assert!(coordinator.dirty_promoted_to_repository());
  }

  #[test]
  fn apply_scan_emits_removals_in_first_chunk() {
    let patches = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected = patches.clone();
    let coordinator = StatusCoordinator::with_emitter(move |patch| {
      collected.lock().unwrap_or_else(|err| err.into_inner()).push(patch);
    });
    coordinator.apply_baseline_for_test(vec![
      entry(ResourceGroupKind::WorkingTree, "gone.rs", FileStatus::Untracked),
      entry(ResourceGroupKind::WorkingTree, "kept.rs", FileStatus::Untracked),
    ]);
    patches.lock().unwrap_or_else(|err| err.into_inner()).clear();

    coordinator.apply_baseline_for_test(vec![entry(
      ResourceGroupKind::WorkingTree,
      "kept.rs",
      FileStatus::Untracked,
    )]);

    let patches = patches.lock().unwrap_or_else(|err| err.into_inner());
    assert_eq!(patches.len(), 1);
    assert_eq!(
      patches[0].removals,
      vec![key(ResourceGroupKind::WorkingTree, "gone.rs")]
    );
    assert!(patches[0].upserts.is_empty());
  }

  #[test]
  fn apply_scan_chunks_removals_without_stalling() {
    let patches = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected = patches.clone();
    let coordinator = StatusCoordinator::with_emitter(move |patch| {
      collected.lock().unwrap_or_else(|err| err.into_inner()).push(patch);
    });
    let entries: Vec<StatusEntry> = (0..300)
      .map(|index| {
        entry(
          ResourceGroupKind::WorkingTree,
          &format!("gone{index}.rs"),
          FileStatus::Untracked,
        )
      })
      .collect();
    coordinator.apply_baseline_for_test(entries);
    patches.lock().unwrap_or_else(|err| err.into_inner()).clear();

    coordinator.apply_baseline_for_test(Vec::new());

    let patches = patches.lock().unwrap_or_else(|err| err.into_inner());
    assert_eq!(patches.len(), 2);
    assert_eq!(patches[0].removals.len(), 256);
    assert_eq!(patches[0].phase, StatusPhase::Scanning);
    assert_eq!(patches[1].removals.len(), 44);
    assert_eq!(patches[1].phase, StatusPhase::Settled);
  }

  #[test]
  fn storm_scan_includes_subtree_scopes() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    coordinator.force_storm_for_test();
    coordinator.invalidate(StatusScope::Subtree("src".into()));
    coordinator.invalidate(StatusScope::Exact("a.rs".into()));
    let taken = coordinator.take_scan_scopes_for_test();
    assert!(
      taken
        .iter()
        .any(|scope| matches!(scope, StatusScope::Exact(path) if path == "a.rs"))
    );
    assert!(
      taken
        .iter()
        .any(|scope| matches!(scope, StatusScope::Subtree(path) if path == "src"))
    );
    assert!(coordinator.dirty_scopes_for_test().is_empty());
  }

  #[test]
  fn storm_scan_does_not_fall_back_to_baseline_when_only_subtrees_are_dirty() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    coordinator.force_storm_for_test();
    coordinator.invalidate(StatusScope::Subtree("src[1]".into()));
    let taken = coordinator.take_scan_scopes_for_test();
    assert_eq!(taken, vec![StatusScope::Subtree("src[1]".into())]);
    assert!(coordinator.dirty_scopes_for_test().is_empty());
  }

  #[test]
  fn channel_len_counts_as_pending_for_busy_scans() {
    let mut storm = StormMachine::new();
    storm.note_scan_finished(false);
    assert!(!storm.in_storm());
    storm.note_scan_finished(true);
    storm.note_scan_finished(true);
    assert!(storm.in_storm());
  }

  #[test]
  fn recoverable_scan_error_requeues_scopes() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    coordinator.invalidate(StatusScope::Exact("a.rs".into()));
    coordinator.fail_next_scan_for_test();
    let err = coordinator.scan_dirty_for_test();
    assert!(err.is_err());
    assert!(
      coordinator
        .dirty_scopes_for_test()
        .iter()
        .any(|scope| matches!(scope, StatusScope::Exact(path) if path == "a.rs"))
    );
  }

  #[test]
  fn one_scan_in_flight_rejects_overlapping_start() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    assert!(coordinator.begin_scan_for_test());
    assert!(!coordinator.begin_scan_for_test());
    coordinator.end_scan_for_test();
    assert!(coordinator.begin_scan_for_test());
  }

  #[test]
  fn extra_pending_from_channel_counts_as_busy_scan() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    coordinator.scan_dirty_with_extra_pending_for_test(true).unwrap();
    assert!(!coordinator.in_storm());
    coordinator.scan_dirty_with_extra_pending_for_test(true).unwrap();
    assert!(coordinator.in_storm());
  }

  #[test]
  fn invalidate_paths_queues_exact_scopes() {
    let coordinator = StatusCoordinator::with_emitter(|_| {});
    coordinator.invalidate_paths(["src/a.rs", "src/b.rs"]);
    let dirty = coordinator.dirty_scopes_for_test();
    assert!(dirty.contains(&StatusScope::Exact("src/a.rs".into())));
    assert!(dirty.contains(&StatusScope::Exact("src/b.rs".into())));
    assert!(!dirty.iter().any(|scope| matches!(scope, StatusScope::Repository)));
  }

  fn watcher_exact(relative: &str) -> WatcherMessage {
    WatcherMessage::Path(ClassifiedPath {
      relative: relative.to_string(),
      kind: PathChangeKind::Content,
      scope: PathChangeScope::Exact,
    })
  }

  fn init_coordinator_repo() -> (tempfile::TempDir, StatusCoordinator) {
    let directory = tempfile::TempDir::new().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    let coordinator = StatusCoordinator::new(directory.path().to_path_buf());
    (directory, coordinator)
  }

  #[test]
  fn events_arriving_during_scan_count_as_pending() {
    let (dir, coordinator) = init_coordinator_repo();
    std::fs::write(dir.path().join("a.rs"), "a").unwrap();
    coordinator.invalidate(StatusScope::Exact("a.rs".into()));

    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    coordinator.set_during_scan_hook_for_test(move || {
      tx.send(watcher_exact("during.rs")).unwrap();
    });
    coordinator.scan_from_channel_for_test(&rx).unwrap();
    assert!(!coordinator.in_storm());
    assert!(
      coordinator
        .dirty_scopes_for_test()
        .contains(&StatusScope::Exact("during.rs".into()))
    );

    coordinator.invalidate(StatusScope::Exact("a.rs".into()));
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    coordinator.set_during_scan_hook_for_test(move || {
      tx.send(watcher_exact("during2.rs")).unwrap();
    });
    coordinator.scan_from_channel_for_test(&rx).unwrap();
    assert!(coordinator.in_storm());
  }

  #[test]
  fn already_drained_current_scan_events_are_not_pending() {
    let (dir, coordinator) = init_coordinator_repo();
    std::fs::write(dir.path().join("a.rs"), "a").unwrap();
    std::fs::write(dir.path().join("b.rs"), "b").unwrap();

    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    tx.send(watcher_exact("a.rs")).unwrap();
    coordinator.scan_from_channel_for_test(&rx).unwrap();
    assert!(!coordinator.in_storm());

    tx.send(watcher_exact("b.rs")).unwrap();
    coordinator.scan_from_channel_for_test(&rx).unwrap();
    assert!(!coordinator.in_storm());
  }

  #[test]
  fn invalidate_paths_and_snapshot_drains_all_exact_paths_past_storm_scan_cap() {
    let (dir, coordinator) = init_coordinator_repo();
    let count = STORM_SCAN_CAP + 1;
    let paths: Vec<String> = (0..count).map(|index| format!("p{index}.rs")).collect();
    for path in &paths {
      std::fs::write(dir.path().join(path), "x").unwrap();
    }
    let status = coordinator.invalidate_paths_and_snapshot(&paths).unwrap();
    assert!(
      coordinator.dirty_scopes_for_test().is_empty(),
      "sync invalidate must drain leftover storm-capped dirty scopes"
    );
    let files: Vec<_> = status
      .groups
      .iter()
      .flat_map(|group| group.files.iter())
      .map(|file| file.path.as_str())
      .collect();
    for path in &paths {
      assert!(files.contains(&path.as_str()), "missing {path} in snapshot");
    }
  }

  #[test]
  fn run_loop_scans_leftover_dirty_without_waiting_for_recv() {
    let (dir, coordinator) = init_coordinator_repo();
    std::fs::write(dir.path().join("first.rs"), "first").unwrap();
    std::fs::write(dir.path().join("leftover.rs"), "leftover").unwrap();

    let coordinator = std::sync::Arc::new(coordinator);
    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let hook_tx = tx.clone();
    coordinator.set_during_scan_hook_for_test(move || {
      hook_tx.send(watcher_exact("leftover.rs")).unwrap();
    });

    let worker = {
      let coordinator = std::sync::Arc::clone(&coordinator);
      std::thread::spawn(move || coordinator.run_loop(rx))
    };

    tx.send(watcher_exact("first.rs")).unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
      let status = coordinator.snapshot();
      let files: Vec<_> = status
        .groups
        .iter()
        .flat_map(|group| group.files.iter())
        .map(|file| file.path.as_str())
        .collect();
      if files.contains(&"leftover.rs") {
        break;
      }
      assert!(
        Instant::now() < deadline,
        "run_loop left leftover.rs dirty until another watcher message"
      );
      std::thread::sleep(Duration::from_millis(10));
    }

    assert!(
      coordinator.dirty_scopes_for_test().is_empty(),
      "leftover dirty must be scanned, not left queued"
    );

    drop(tx);
    worker.join().unwrap();
  }

  #[test]
  fn run_loop_does_not_rescan_immediately_after_failed_scan() {
    let (dir, coordinator) = init_coordinator_repo();
    std::fs::write(dir.path().join("fail.rs"), "fail").unwrap();

    let coordinator = std::sync::Arc::new(coordinator);
    coordinator.fail_all_scans_for_test();

    let (tx, rx) = std::sync::mpsc::sync_channel(8);
    let worker = {
      let coordinator = std::sync::Arc::clone(&coordinator);
      std::thread::spawn(move || coordinator.run_loop(rx))
    };

    tx.send(watcher_exact("fail.rs")).unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    while coordinator.scan_attempts_for_test() == 0 {
      assert!(Instant::now() < deadline, "run_loop never attempted a scan");
      std::thread::sleep(Duration::from_millis(5));
    }

    std::thread::sleep(Duration::from_millis(SCAN_RETRY_MS / 2));
    let attempts = coordinator.scan_attempts_for_test();
    assert_eq!(
      attempts, 1,
      "failed scan must not re-enter immediately without delay, attempts={attempts}"
    );
    assert!(
      coordinator
        .dirty_scopes_for_test()
        .iter()
        .any(|scope| matches!(scope, StatusScope::Exact(path) if path == "fail.rs")),
      "failed scan must retain dirty scopes"
    );

    drop(tx);
    worker.join().unwrap();
  }
}
