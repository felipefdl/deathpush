use deathpush_core::session::types::Intent;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_kit::*;

use super::banner::render_banner;
use super::commit_box::render_commit_box;
use super::filter::{self, FILTER_DEBOUNCE_MS};
use super::toolbar::render_toolbar;
use crate::actions::*;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::{RepoEvent, RepoModel};
use crate::theme::{ActivePalette, hsla};

pub struct ChangesView {
  pub(crate) model: Entity<RepoModel>,
  #[allow(dead_code)]
  layout: Entity<LayoutModel>,
  pub(crate) commit: Entity<TextareaState>,
  filter: Entity<InputState>,
  filter_text: String,
  filter_generation: u64,
  commit_generation: u64,
  pub(crate) committing: bool,
  window_handle: AnyWindowHandle,
  focus_handle: FocusHandle,
}

impl ChangesView {
  pub fn new(
    model: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let state = model.read(cx).state();
    let commit_message = state.commit_message.clone();
    let file_filter = state.file_filter.clone();
    let commit = cx.new(|cx| {
      TextareaState::new(window, cx)
        .placeholder("commit message")
        .auto_grow(2, 9)
        .default_value(commit_message)
    });
    let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter files..."));
    if !file_filter.is_empty() {
      filter.update(cx, |state, cx| state.set_value(file_filter.clone(), window, cx));
    }

    cx.subscribe(&commit, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.commit_generation + 1;
        filter::debounce(cx, &mut this.commit_generation, FILTER_DEBOUNCE_MS, move |this, cx| {
          if this.commit_generation != token {
            return;
          }
          let message = this.commit.read(cx).value().to_string();
          this.dispatch_intent(Intent::SetCommitMessage { message }, cx);
        });
      }
    })
    .detach();
    cx.subscribe(&filter, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.filter_generation + 1;
        filter::debounce(cx, &mut this.filter_generation, FILTER_DEBOUNCE_MS, move |this, cx| {
          if this.filter_generation != token {
            return;
          }
          let filter = this.filter.read(cx).value().to_string();
          this.filter_text = filter.clone();
          this.dispatch_intent(Intent::SetFileFilter { filter }, cx);
        });
      }
    })
    .detach();
    cx.subscribe(&model, |this, model, event: &RepoEvent, cx| {
      this.committing = false;
      if matches!(event, RepoEvent::Changed) {
        let message = model.read(cx).state().commit_message.clone();
        let current = this.commit.read(cx).value().to_string();
        if message != current {
          let handle = this.window_handle;
          let commit = this.commit.clone();
          let _ = handle.update(cx, |_, window, cx| {
            commit.update(cx, |state, cx| state.set_value(message, window, cx));
          });
        }
      }
      cx.notify();
    })
    .detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();

    Self {
      model,
      layout,
      commit,
      filter,
      filter_text: String::new(),
      filter_generation: 0,
      commit_generation: 0,
      committing: false,
      window_handle: window.window_handle(),
      focus_handle: cx.focus_handle(),
    }
  }

  pub fn focus_commit(&self, window: &mut Window, cx: &mut App) {
    self.commit.update(cx, |state, cx| state.focus(window, cx));
  }

  pub fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.committing = true;
    cx.notify();
    self.send(Intent::Commit { confirmed: false }, window, cx);
  }

  #[allow(dead_code)]
  pub fn filter_text(&self) -> &str {
    &self.filter_text
  }

  pub(crate) fn send(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }

  fn dispatch_intent(&self, intent: Intent, cx: &mut Context<Self>) {
    let model = self.model.clone();
    let _ = self.window_handle.update(cx, |_, window, cx| {
      model.update(cx, |model, cx| model.dispatch(intent, window, cx));
    });
  }

  fn render_empty_repo(cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    div()
      .size_full()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_3()
      .child(
        div()
          .text_size(px(13.0))
          .text_color(hsla(palette.muted_foreground))
          .child("No repository open"),
      )
      .child(
        Button::new("open-repo")
          .outline()
          .label("Open Repository")
          .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenRepository), cx)),
      )
  }

  fn render_watermark(cx: &App) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    div()
      .flex_1()
      .min_h_0()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_2()
      .child(
        svg()
          .path("brand/deathpush.svg")
          .size(px(48.0))
          .text_color(hsla(palette.mark))
          .opacity(0.12),
      )
      .child(
        div()
          .text_size(px(13.0))
          .text_color(hsla(palette.foreground))
          .opacity(0.18)
          .child("No changes"),
      )
  }
}

impl Render for ChangesView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let state = self.model.read(cx).state().clone();
    let mut root = div()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus_handle)
      .key_context("Changes")
      .on_action(cx.listener(|this, _: &CommitFromBox, window, cx| this.commit(window, cx)))
      .on_action(cx.listener(|this, _: &CommitAmendMode, window, cx| {
        this.send(Intent::SetAmend { enabled: true }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndPush, window, cx| {
        this.send(Intent::CommitAndPush { confirmed: false }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndSync, window, cx| {
        this.send(Intent::CommitAndSync { confirmed: false }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &RefreshStatus, window, cx| {
        this.send(Intent::RefreshStatus, window, cx);
        this.model.update(cx, |model, cx| model.refresh_nested_repositories(cx));
      }))
      .on_action(cx.listener(|this, _: &OperationContinue, window, cx| {
        this.send(Intent::OperationContinue, window, cx);
      }))
      .on_action(cx.listener(|this, _: &OperationSkip, window, cx| {
        this.send(Intent::OperationSkip, window, cx);
      }))
      .on_action(cx.listener(|this, _: &OperationAbort, window, cx| {
        this.send(Intent::OperationAbort, window, cx);
      }))
      .on_action(cx.listener(|this, _: &FocusCommitMessage, window, cx| {
        this.focus_commit(window, cx);
      }));

    if state.status.is_none() {
      return root.child(Self::render_empty_repo(cx));
    }

    root = root.child(render_toolbar(&state, cx));
    if let Some(banner) = render_banner(&state, cx) {
      root = root.child(banner);
    }
    root = root.child(render_commit_box(self, &state, window, cx));

    let empty_groups = !state.has_changes() && state.stashes.is_empty() && state.nested_repositories.is_empty();
    if state.has_changes() {
      let palette = cx.global::<ActivePalette>().0;
      root = root.child(
        div().px_2().pb_2().child(
          Input::new(&self.filter)
            .small()
            .h(px(26.0))
            .w_full()
            .rounded_md()
            .bg(hsla(palette.input))
            .cleanable(true)
            .prefix(
              svg()
                .path("icons/search.svg")
                .size(px(14.0))
                .text_color(hsla(palette.muted_foreground)),
            ),
        ),
      );
    }
    if empty_groups {
      root = root.child(Self::render_watermark(cx));
    } else {
      root = root.child(div().flex_1().min_h_0().child(div()));
    }
    root
  }
}
