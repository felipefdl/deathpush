use deathpush_core::config::layout::PanelTab;
use gpui_kit::component::button::*;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::actions::*;
use crate::repo::output_log::{OutputLog, format_line};
use crate::theme::{ActivePalette, hsla};

pub fn tab_label(tab: PanelTab) -> &'static str {
  match tab {
    PanelTab::GitOutput => "Output",
    PanelTab::Terminal => "Terminal",
  }
}

/// Header with the two tabs and the actions, then the Output log or the terminal slot.
pub fn render_terminal_panel(
  active: PanelTab,
  maximized: bool,
  output: &Entity<OutputLog>,
  cx: &App,
) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let tab = |id: &'static str, tab: PanelTab, action: Box<dyn Action>| {
    let is_active = active == tab;
    div()
      .id(id)
      .px_3()
      .h_full()
      .flex()
      .items_center()
      .text_size(px(12.0))
      .cursor_pointer()
      .opacity(if is_active { 1.0 } else { 0.6 })
      .border_b_2()
      .border_color(if is_active {
        hsla(palette.ring)
      } else {
        hsla(palette.border.with_alpha(0))
      })
      .child(tab_label(tab))
      .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
  };
  let icon_button = |id: &'static str, icon: &'static str, tooltip: &'static str, action: Box<dyn Action>| {
    Button::new(id)
      .ghost()
      .xsmall()
      .icon(Icon::empty().path(icon))
      .tooltip(tooltip)
      .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
  };
  let show_actions = active == PanelTab::Terminal || maximized;
  let lines: Vec<String> = output.read(cx).lines().iter().map(format_line).collect();
  let body: AnyElement = match active {
    PanelTab::GitOutput if lines.is_empty() => div()
      .size_full()
      .flex()
      .items_center()
      .justify_center()
      .text_size(px(12.0))
      .text_color(hsla(palette.muted_foreground))
      .child("No git commands recorded yet.")
      .into_any_element(),
    PanelTab::GitOutput => {
      let count = lines.len();
      let lines = std::sync::Arc::new(lines);
      uniform_list("git-output", count, move |range, _, _| {
        range
          .map(|i| {
            div()
              .px_2()
              .h(px(20.0))
              .text_size(px(12.0))
              .font_family("MesloLGS Nerd Font Mono")
              .child(lines[i].clone())
          })
          .collect()
      })
      .size_full()
      .into_any_element()
    }
    PanelTab::Terminal => div().size_full().into_any_element(),
  };
  div()
    .size_full()
    .flex()
    .flex_col()
    .bg(hsla(palette.sidebar))
    .text_color(hsla(palette.foreground))
    .child(
      div()
        .h(px(28.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(hsla(palette.border))
        .child(tab("panel-tab-output", PanelTab::GitOutput, Box::new(ShowOutputTab)))
        .child(tab("panel-tab-terminal", PanelTab::Terminal, Box::new(ShowTerminalTab)))
        .child(div().flex_1())
        .when(show_actions, |el| {
          el.child(
            div()
              .flex()
              .items_center()
              .gap_1()
              .px_1()
              .child(icon_button(
                "panel-new",
                "icons/add.svg",
                "New Terminal",
                Box::new(NewTerminal),
              ))
              .child(div().w(px(1.0)).h(px(14.0)).bg(hsla(palette.border)))
              .child(icon_button(
                "panel-split-h",
                "icons/split-horizontal.svg",
                "Split Terminal Horizontally",
                Box::new(SplitTerminalHorizontal),
              ))
              .child(icon_button(
                "panel-split-v",
                "icons/split-vertical.svg",
                "Split Terminal Vertically",
                Box::new(SplitTerminalVertical),
              ))
              .child(icon_button(
                "panel-maximize",
                if maximized {
                  "icons/screen-normal.svg"
                } else {
                  "icons/screen-full.svg"
                },
                if maximized {
                  "Restore Panel Size"
                } else {
                  "Maximize Panel Size"
                },
                Box::new(ToggleTerminalMaximize),
              ))
              .child(icon_button(
                "panel-close",
                "icons/close.svg",
                "Close Panel",
                Box::new(ClosePanel),
              )),
          )
        }),
    )
    .child(div().flex_1().min_h_0().child(body))
}
