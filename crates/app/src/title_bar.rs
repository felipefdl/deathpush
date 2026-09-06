use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, TitleBar};
use gpui_kit::*;

use crate::actions::CloseWindow;
use crate::menus::{MenuContext, linux_rows};

fn dispatch_close_window(_: &ClickEvent, window: &mut Window, cx: &mut App) {
  window.dispatch_action(Box::new(CloseWindow), cx);
}

/// macOS: a draggable strip beside the traffic lights with the centered title.
/// Linux: the client-side bar with a menu button, the title, and the window controls.
/// Windows: nothing; the OS title bar is used.
pub fn render_title_bar(
  title: SharedString,
  menu_ctx: MenuContext,
  window: &mut Window,
  cx: &mut App,
) -> Option<AnyElement> {
  if cfg!(target_os = "windows") {
    return None;
  }
  let title_el = div()
    .flex_1()
    .flex()
    .items_center()
    .justify_center()
    .text_size(px(12.0))
    .text_color(cx.theme().muted_foreground)
    .child(title);
  if cfg!(target_os = "linux") {
    let menu_button = Button::new("app-menu")
      .ghost()
      .small()
      .icon(Icon::new(IconName::Menu))
      .dropdown_menu(move |mut menu, _, _| {
        let rows = linux_rows(&menu_ctx);
        for row in rows {
          if row.separator_before {
            menu = menu.separator();
          }
          menu = menu.menu_with_disabled(row.label, row.action, row.disabled);
        }
        menu.min_w(px(260.0))
      });
    return Some(
      TitleBar::new()
        .on_close_window(dispatch_close_window)
        .child(
          div()
            .flex()
            .items_center()
            .w_full()
            .h_full()
            .child(menu_button)
            .child(title_el),
        )
        .into_any_element(),
    );
  }
  let _ = (menu_ctx, window);
  Some(
    TitleBar::new()
      .child(div().flex().items_center().w_full().h_full().child(title_el))
      .into_any_element(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  struct CloseProbe {
    focus: FocusHandle,
    closed: bool,
  }

  impl CloseProbe {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
      let focus = cx.focus_handle();
      focus.focus(window, cx);
      Self { focus, closed: false }
    }
  }

  impl Render for CloseProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
      div()
        .id("close-probe")
        .track_focus(&self.focus)
        .on_action(cx.listener(|this, _: &CloseWindow, _, _| this.closed = true))
    }
  }

  #[gpui_kit::test]
  fn linux_close_dispatches_close_window(cx: &mut TestAppContext) {
    let (probe, cx) = cx.add_window_view(CloseProbe::new);
    cx.update(|window, cx| {
      let _ = window.draw(cx);
      dispatch_close_window(&ClickEvent::default(), window, cx);
    });
    assert!(probe.read_with(cx, |probe, _| probe.closed));
  }
}
