use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::config::recent_files::{load_recent_files, save_recent_files};
use deathpush_core::session::types::{Intent, IntentOutcome, SessionSnapshot, SessionStatusEvent};
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

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

impl RepoModel {
  pub fn new(core: Arc<Core>, session: SessionId, snapshot: SessionSnapshot) -> Self {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot);
    Self {
      core,
      session,
      state,
      blame_requested: None,
    }
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

  pub(crate) fn session(&self) -> SessionId {
    self.session
  }

  /// Send an intent to core; the outcome applies on the foreground executor.
  pub fn dispatch(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
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

  pub fn close_file(&mut self, cx: &mut Context<Self>) {
    if self.state.open_file.is_none() && self.state.cursor_line.is_none() {
      return;
    }
    self.state.open_file = None;
    self.state.cursor_line = None;
    self.state.blame = None;
    self.blame_requested = None;
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
    let write_path = path.clone();
    let task = handle.spawn_blocking(move || core.write_file(session, &write_path, &content));
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let _ = this.update(cx, |this, cx| match result {
        Ok(Ok(result)) => {
          let hash = result.content_hash;
          let (path_match, current_hash) = match this.state.open_file.as_ref() {
            Some(open) if open.path == path => (true, open.content.as_ref().map(|file| file.content_hash.clone())),
            _ => (false, None),
          };
          if path_match && current_hash.as_deref() == Some(expected_hash.as_str()) {
            if let Some(file) = this.state.open_file.as_mut().and_then(|open| open.content.as_mut()) {
              file.content_hash = hash.clone();
              file.content = written;
            }
            cx.emit(RepoEvent::Saved { path, hash, generation });
            cx.notify();
          } else if path_match
            && !retry
            && let Some(current) = current_hash
          {
            this.spawn_write(path, written, current, generation, true, cx);
          }
        }
        Ok(Err(err)) => this.fail(err.to_string(), cx),
        Err(err) => this.fail(err.to_string(), cx),
      });
    })
    .detach();
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
}
