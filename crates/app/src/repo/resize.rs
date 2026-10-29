//! Regression for the stock gpui-kit resizable handle: a release ends the drag before the next
//! paint, and a nested divider's drag leaves its ancestor alone.

use std::rc::Rc;

use gpui_kit::base::ResizableState;
use gpui_kit::prelude::*;
use gpui_kit::*;

mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::base::{h_resizable, resizable_panel, v_resizable};
  use std::cell::Cell;

  struct Harness {
    outer: Entity<ResizableState>,
    inner: Entity<ResizableState>,
    releases: Rc<Cell<usize>>,
  }

  impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
      let outer_releases = self.releases.clone();
      let inner_releases = self.releases.clone();
      div().w(px(600.0)).h(px(300.0)).child(
        h_resizable("outer")
          .with_state(&self.outer)
          .on_resize(move |_, _, _| {
            outer_releases.set(outer_releases.get() + 1);
          })
          .child(resizable_panel().size(px(300.0)).child(div().size_full()))
          .child(
            resizable_panel().size(px(300.0)).child(
              v_resizable("inner")
                .with_state(&self.inner)
                .on_resize(move |_, _, _| {
                  inner_releases.set(inner_releases.get() + 1);
                })
                .child(resizable_panel().size(px(150.0)).child(div().size_full()))
                .child(
                  resizable_panel()
                    .size(px(150.0))
                    .child(div().size_full().debug_selector(|| "nested-second".into())),
                ),
            ),
          ),
      )
    }
  }

  fn pointer_drag(window: &mut Window, cx: &mut App, from: Point<Pixels>, to: Point<Pixels>) {
    window.dispatch_event(
      PlatformInput::MouseDown(MouseDownEvent {
        position: from,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
      }),
      cx,
    );
    // The first move past the press starts the gpui drag, the next ones resize.
    for position in [
      point(from.x + (to.x - from.x) / 2.0, from.y + (to.y - from.y) / 2.0),
      to,
    ] {
      window.dispatch_event(
        PlatformInput::MouseMove(MouseMoveEvent {
          position,
          pressed_button: Some(MouseButton::Left),
          modifiers: Modifiers::default(),
        }),
        cx,
      );
    }
    window.dispatch_event(
      PlatformInput::MouseUp(MouseUpEvent {
        position: to,
        button: MouseButton::Left,
        modifiers: Modifiers::default(),
        click_count: 1,
      }),
      cx,
    );
  }

  #[gpui_kit::test]
  fn release_stops_resizing_before_repaint_and_nested_drag_leaves_ancestor_alone(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      crate::config::AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let outer = cx.new(|_| ResizableState::default());
    let inner = cx.new(|_| ResizableState::default());
    let releases = Rc::new(Cell::new(0));
    let (_, cx) = cx.add_window_view({
      let outer = outer.clone();
      let inner = inner.clone();
      let releases = releases.clone();
      move |_, _| Harness { outer, inner, releases }
    });
    cx.update(|window, cx| {
      window.draw(cx).clear(cx);
    });
    cx.update(|window, cx| {
      window.draw(cx).clear(cx);
    });
    let nested = cx.debug_bounds("nested-second").unwrap();
    cx.update(|window, cx| {
      pointer_drag(
        window,
        cx,
        point(nested.left() - px(2.0), px(50.0)),
        point(nested.left() + px(40.0), px(50.0)),
      );
      let after_release = outer.read(cx).sizes().clone();
      assert!(
        after_release[0] > px(300.0),
        "pointer drag must resize the outer divider"
      );
      window.dispatch_event(
        PlatformInput::MouseMove(MouseMoveEvent {
          position: point(px(480.0), px(70.0)),
          pressed_button: None,
          modifiers: Modifiers::default(),
        }),
        cx,
      );
      assert_eq!(
        outer.read(cx).sizes(),
        &after_release,
        "release must stop the drag before another paint"
      );
      window.draw(cx).clear(cx);
    });
    let nested = cx.debug_bounds("nested-second").unwrap();
    cx.update(|window, cx| {
      let outer_sizes = outer.read(cx).sizes().clone();
      let inner_sizes = inner.read(cx).sizes().clone();
      pointer_drag(
        window,
        cx,
        point(nested.left() + px(60.0), nested.top() - px(2.0)),
        point(nested.left() + px(80.0), nested.top() + px(20.0)),
      );
      assert_ne!(
        inner.read(cx).sizes(),
        &inner_sizes,
        "nested divider must receive its own drag"
      );
      assert_eq!(
        outer.read(cx).sizes(),
        &outer_sizes,
        "nested drag must not resize its ancestor"
      );
      let after_release = inner.read(cx).sizes().clone();
      window.dispatch_event(
        PlatformInput::MouseMove(MouseMoveEvent {
          position: point(px(510.0), px(220.0)),
          pressed_button: None,
          modifiers: Modifiers::default(),
        }),
        cx,
      );
      assert_eq!(inner.read(cx).sizes(), &after_release);
      assert_eq!(outer.read(cx).sizes(), &outer_sizes);
    });
    assert_eq!(
      releases.get(),
      2,
      "each actual release must finish only its own divider"
    );
  }
}
