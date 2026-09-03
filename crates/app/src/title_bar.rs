use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, TitleBar};
use gpui_kit::*;

use crate::menus::{MenuContext, linux_rows};

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
    let _ = window;
    return Some(
      TitleBar::new()
        .on_close_window(|_, window, _| window.remove_window())
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
