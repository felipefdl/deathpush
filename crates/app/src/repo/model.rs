use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::session::types::{Intent, IntentOutcome, SessionSnapshot, SessionStatusEvent};
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

use super::state::{NetworkOp, PayloadVerdict, RepoState};

pub enum RepoEvent {
  /// State changed; views re-read `state()`.
  Changed,
  /// A failed intent; the shell shows the toast.
  Error(String),
}

/// One window's repository session: applies core outcomes and events to `RepoState` and sends intents.
pub struct RepoModel {
  core: Arc<Core>,
  session: SessionId,
  state: RepoState,
}

impl EventEmitter<RepoEvent> for RepoModel {}

impl RepoModel {
  pub fn new(core: Arc<Core>, session: SessionId, snapshot: SessionSnapshot) -> Self {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot);
    Self { core, session, state }
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
        if self
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
