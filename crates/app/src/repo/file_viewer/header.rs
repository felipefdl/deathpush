use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::states::ViewerKind;
use super::view::FileViewer;
use crate::theme::hsla;

pub fn breadcrumbs(path: &str) -> Vec<String> {
  path.split(['/', '\\']).map(str::to_string).collect()
}

pub fn render_header(
  path: &str,
  dirty: bool,
  kind: ViewerKind,
  view: WeakEntity<FileViewer>,
  palette: UiPalette,
  _cx: &App,
) -> impl IntoElement {
  let full_path = path.to_string();
  let crumb = breadcrumbs(path).join(" / ");
  let show_reveal = matches!(kind, ViewerKind::Text | ViewerKind::Image);
  div()
    .h(px(35.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .justify_between()
    .px_3()
    .gap_2()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .flex_1()
        .child(
          div()
            .id("file-breadcrumbs")
            .min_w_0()
            .flex_1()
            .truncate()
            .text_size(px(12.0))
            .text_color(hsla(palette.foreground))
            .child(crumb)
            .tooltip(move |window, cx| Tooltip::new(full_path.clone()).build(window, cx)),
        )
        .when(dirty, |el| {
          el.child(
            div()
              .text_size(px(12.0))
              .text_color(hsla(palette.muted_foreground))
              .child(" *"),
          )
        }),
    )
    .child(
      div()
        .flex()
        .items_center()
        .gap_1()
        .when(show_reveal, |el| {
          let view = view.clone();
          el.child(
            tool("file-reveal", "icons/folder-opened.svg", "Reveal in Finder").on_click(move |_, _, cx| {
              let _ = view.update(cx, |this, cx| this.reveal(cx));
            }),
          )
        })
        .child({
          let view = view.clone();
          tool("file-open-editor", "icons/link-external.svg", "Open in Editor").on_click(move |_, _, cx| {
            let _ = view.update(cx, |this, cx| this.open_external(cx));
          })
        }),
    )
}

fn tool(id: &'static str, path: &'static str, tooltip: &'static str) -> Button {
  Button::new(id)
    .ghost()
    .xsmall()
    .w(px(22.0))
    .h(px(22.0))
    .icon(Icon::empty().path(path))
    .tooltip(tooltip)
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn breadcrumbs_split_both_separators() {
    assert_eq!(breadcrumbs("src/a/b.rs"), vec!["src", "a", "b.rs"]);
    assert_eq!(breadcrumbs("src\\a\\b.rs"), vec!["src", "a", "b.rs"]);
  }
}
