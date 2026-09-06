use deathpush_core::session::types::{Intent, SyncKind};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::{Disableable, Icon, Sizable, Size};
use gpui_kit::*;

use super::overflow::{OverflowState, build_menu};
use super::view::{ChangesChrome, ChangesView};
use crate::repo::state::NetworkOp;
use crate::theme::ActivePalette;

pub fn render_toolbar(chrome: &ChangesChrome, cx: &mut Context<ChangesView>) -> impl IntoElement {
  let actions = chrome.actions.as_ref();
  let can_stage_all = actions.is_some_and(|actions| actions.can_stage_all);
  let busy = chrome.network_busy;
  let (ahead, behind) = (chrome.ahead, chrome.behind);
  let sync = actions.map(|actions| actions.sync.clone());
  let show_sync = sync.as_ref().is_some_and(|sync| sync.enabled);

  let tool = |id: &'static str, path: &'static str, tooltip: &'static str| {
    Button::new(id)
      .ghost()
      .with_size(Size::Medium)
      .w(px(22.0))
      .h(px(22.0))
      .icon(Icon::empty().path(path))
      .tooltip(tooltip)
  };

  let mut row = div()
    .h(px(35.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .justify_center()
    .gap_1()
    .px_2()
    .child(
      tool("stage-all", "icons/plus.svg", "Stage All Changes")
        .disabled(!can_stage_all)
        .on_click(cx.listener(|this, _, window, cx| this.send(Intent::StageAll, window, cx))),
    )
    .child(
      tool("refresh", "icons/refresh-cw.svg", "Refresh").on_click(cx.listener(|this, _, window, cx| {
        this.model.update(cx, |model, cx| {
          model.dispatch(Intent::RefreshStatus, window, cx);
          model.refresh_nested_repositories(cx);
        });
      })),
    );

  if show_sync && let Some(sync) = sync {
    let (path, tooltip, op, intent) = match sync.kind {
      SyncKind::Fetch => (
        "icons/cloud-download.svg",
        "Fetch".to_string(),
        NetworkOp::Fetch,
        Intent::Fetch { prune: true },
      ),
      SyncKind::Pull => (
        "icons/arrow-down-up.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Pull,
        Intent::Pull { rebase: false },
      ),
      SyncKind::Push => (
        "icons/arrow-down-up.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Push,
        Intent::Push {
          force: false,
          confirmed: false,
        },
      ),
      SyncKind::PullThenPush => (
        "icons/arrow-down-up.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Sync,
        Intent::Sync,
      ),
    };
    row = row.child(
      Button::new("sync")
        .ghost()
        .with_size(Size::Medium)
        .w(px(22.0))
        .h(px(22.0))
        .icon(Icon::empty().path(path))
        .tooltip(tooltip)
        .loading(busy)
        .disabled(busy)
        .on_click(cx.listener(move |this, _, window, cx| {
          this
            .model
            .update(cx, |model, cx| model.dispatch_network(op, intent.clone(), window, cx));
        })),
    );
  }

  let view = cx.weak_entity();
  row.child(
    tool("more", "icons/ellipsis.svg", "More Actions...").dropdown_menu(move |menu, _, cx| {
      let Some(entity) = view.upgrade() else {
        return menu;
      };
      let palette = cx.global::<ActivePalette>().0;
      let overflow = OverflowState::from_state(entity.read(cx).model.read(cx).state(), &palette);
      build_menu(menu, view.clone(), &overflow, cx)
    }),
  )
}
