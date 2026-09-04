use deathpush_core::session::types::{Intent, SyncKind};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::*;

use super::view::{ChangesChrome, ChangesView};
use crate::repo::state::NetworkOp;

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
      .xsmall()
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
    .justify_end()
    .gap_1()
    .px_2()
    .child(
      tool("stage-all", "icons/add.svg", "Stage All Changes")
        .disabled(!can_stage_all)
        .on_click(cx.listener(|this, _, window, cx| this.send(Intent::StageAll, window, cx))),
    )
    .child(
      tool("refresh", "icons/refresh.svg", "Refresh").on_click(cx.listener(|this, _, window, cx| {
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
        "icons/sync.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Pull,
        Intent::Pull { rebase: false },
      ),
      SyncKind::Push => (
        "icons/sync.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Push,
        Intent::Push {
          force: false,
          confirmed: false,
        },
      ),
      SyncKind::PullThenPush => (
        "icons/sync.svg",
        format!("Sync: {behind}↓ {ahead}↑"),
        NetworkOp::Sync,
        Intent::Sync,
      ),
    };
    row = row.child(
      Button::new("sync")
        .ghost()
        .xsmall()
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

  row.child(
    tool("more", "icons/ellipsis.svg", "More Actions...")
      .dropdown_menu(|menu, _, _| menu.item(PopupMenuItem::new("More Actions...").disabled(true))),
  )
}
