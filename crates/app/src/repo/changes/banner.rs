use deathpush_core::session::types::Intent;
use deathpush_core::types::RepoOperationState;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::view::{ChangesChrome, ChangesView};
use crate::theme::{ActivePalette, hsla};

pub fn render_banner(chrome: &ChangesChrome, cx: &mut Context<ChangesView>) -> Option<impl IntoElement> {
  if chrome.operation_state == RepoOperationState::None {
    return None;
  }
  let palette = cx.global::<ActivePalette>().0;
  let label = match chrome.operation_state {
    RepoOperationState::Merging => "Merge in progress",
    RepoOperationState::Rebasing => "Rebase in progress",
    RepoOperationState::CherryPicking => "Cherry-pick in progress",
    RepoOperationState::Reverting => "Revert in progress",
    RepoOperationState::None => "Operation in progress",
  };
  let actions = chrome.actions.as_ref();
  let show_continue = actions.is_some_and(|actions| actions.operation.continue_op);
  let show_skip = actions.is_some_and(|actions| actions.operation.skip);
  let show_abort = actions.is_some_and(|actions| actions.operation.abort);

  Some(
    div()
      .w_full()
      .flex()
      .items_center()
      .gap_2()
      .px_2()
      .py_1()
      .bg(hsla(palette.warning.with_alpha(40)))
      .child(
        svg()
          .path("icons/triangle-alert.svg")
          .size(px(14.0))
          .text_color(hsla(palette.warning)),
      )
      .child(div().flex_1().text_size(px(12.0)).child(label))
      .when(show_continue, |el| {
        el.child(
          Button::new("op-continue")
            .outline()
            .xsmall()
            .label("Continue")
            .on_click(cx.listener(|this, _, window, cx| this.send(Intent::OperationContinue, window, cx))),
        )
      })
      .when(show_skip, |el| {
        el.child(
          Button::new("op-skip")
            .outline()
            .xsmall()
            .label("Skip")
            .on_click(cx.listener(|this, _, window, cx| this.send(Intent::OperationSkip, window, cx))),
        )
      })
      .when(show_abort, |el| {
        el.child(
          Button::new("op-abort")
            .danger()
            .outline()
            .xsmall()
            .label("Abort")
            .on_click(cx.listener(|this, _, window, cx| this.send(Intent::OperationAbort, window, cx))),
        )
      }),
  )
}
