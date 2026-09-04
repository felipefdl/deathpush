use deathpush_core::config::settings::DiffLayout;
use deathpush_core::session::types::FileSelection;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::ResourceGroupKind;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;

use super::panel::DiffPanel;
use crate::actions::ToggleDiffLayout;
use crate::theme::hsla;

pub fn header_suffix(selection: &FileSelection) -> &'static str {
  if selection.group_kind == ResourceGroupKind::Merge {
    "(Merge)"
  } else if selection.staged {
    "(Staged)"
  } else {
    "(Working Tree)"
  }
}

pub fn file_name(path: &str) -> &str {
  path.rsplit(['/', '\\']).next().unwrap_or(path)
}

pub fn render_header(
  selection: &FileSelection,
  layout: DiffLayout,
  view: WeakEntity<DiffPanel>,
  palette: UiPalette,
  _cx: &App,
) -> impl IntoElement {
  let path = selection.path.clone();
  let (layout_icon, layout_tooltip) = match layout {
    DiffLayout::Inline => ("icons/split-horizontal.svg", "Switch to side by side"),
    DiffLayout::SideBySide => ("icons/list-flat.svg", "Switch to inline"),
  };
  div()
    .h(px(28.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .justify_between()
    .px_3()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .child(
          div()
            .min_w_0()
            .truncate()
            .text_size(px(13.0))
            .text_color(hsla(palette.foreground))
            .child(file_name(&selection.path).to_string()),
        )
        .child(
          div()
            .text_size(px(13.0))
            .text_color(hsla(palette.muted_foreground))
            .child(header_suffix(selection).to_string()),
        ),
    )
    .child(
      div()
        .flex()
        .items_center()
        .gap_1()
        .child(
          Button::new("diff-history")
            .ghost()
            .xsmall()
            .icon(Icon::empty().path("icons/history.svg"))
            .tooltip("Show File History")
            .on_click({
              let view = view.clone();
              let path = path.clone();
              move |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                  this.open_file_history(path.clone(), window, cx);
                });
              }
            }),
        )
        .child(
          Button::new("diff-layout")
            .ghost()
            .xsmall()
            .icon(Icon::empty().path(layout_icon))
            .tooltip(layout_tooltip)
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleDiffLayout), cx)),
        ),
    )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn suffix_and_file_name() {
    let sel = |staged, kind| FileSelection {
      path: "a/b/c.rs".into(),
      staged,
      group_kind: kind,
    };
    assert_eq!(header_suffix(&sel(false, ResourceGroupKind::Merge)), "(Merge)");
    assert_eq!(header_suffix(&sel(true, ResourceGroupKind::Index)), "(Staged)");
    assert_eq!(
      header_suffix(&sel(false, ResourceGroupKind::WorkingTree)),
      "(Working Tree)"
    );
    assert_eq!(file_name("a/b/c.rs"), "c.rs");
    assert_eq!(file_name("a\\b\\c.rs"), "c.rs");
    assert_eq!(file_name("c.rs"), "c.rs");
  }
}
