#![allow(dead_code)]

use std::sync::Arc;

use deathpush_core::session::types::{Intent, IntentOutcome, SessionSnapshot, SessionStatusEvent};
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

use super::state::{PayloadVerdict, RepoState};

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

  pub fn state_mut(&mut self) -> &mut RepoState {
    &mut self.state
  }

  /// Send an intent to core; the outcome applies on the foreground executor.
  pub fn dispatch(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
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

  fn fail(&mut self, message: String, cx: &mut Context<Self>) {
    self.state.pending_clear_file = false;
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
          }
        })
        .detach();
      }
    }
  }

  /// Cmd/Ctrl+Shift+G: reload the session state from scratch.
  pub fn reload(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
