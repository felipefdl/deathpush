use std::rc::Rc;

use gpui_kit::base::{ResizableState, ResizeHandleRenderer};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::theme::{ActivePalette, hsla};

#[derive(Clone, Copy)]
struct DragStart {
  position: Point<Pixels>,
  size: Pixels,
}

/// Keeps the panel group's sizing rules, but reads pointer state at event time.
/// The upstream handle captures drag state at paint time and can miss a quick release.
pub fn divider(
  state: Entity<ResizableState>,
  on_resize: impl Fn(&Entity<ResizableState>, &mut Window, &mut App) + 'static,
) -> ResizeHandleRenderer {
  let on_resize = Rc::new(on_resize);
  Rc::new(move |handle, window, cx| {
    let axis = handle.axis();
    let drag = window.use_keyed_state(
      SharedString::from(format!("resize-pointer-{}", state.entity_id())),
      cx,
      |_, _| None::<DragStart>,
    );
    let palette = cx.global::<ActivePalette>().0;
    let active = drag.read(cx).is_some();
    let pointer = canvas(|_, _, _| (), {
      let drag = drag.clone();
      let state = state.clone();
      let on_resize = on_resize.clone();
      move |_, _, window, _| {
        window.on_mouse_event({
          let drag = drag.clone();
          let state = state.clone();
          let on_resize = on_resize.clone();
          move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Capture {
              return;
            }
            let Some(start) = *drag.read(cx) else { return };
            if event.pressed_button != Some(MouseButton::Left) {
              drag.update(cx, |drag, _| *drag = None);
              on_resize(&state, window, cx);
              window.refresh();
              return;
            }
            let delta = match axis {
              Axis::Horizontal => event.position.x - start.position.x,
              Axis::Vertical => event.position.y - start.position.y,
            };
            state.update(cx, |state, cx| state.resize_panel(0, start.size + delta, window, cx));
          }
        });
        window.on_mouse_event({
          let drag = drag.clone();
          let state = state.clone();
          let on_resize = on_resize.clone();
          move |_: &MouseUpEvent, phase, window, cx| {
            if phase == DispatchPhase::Capture && drag.read(cx).is_some() {
              drag.update(cx, |drag, _| *drag = None);
              on_resize(&state, window, cx);
              window.refresh();
            }
          }
        });
      }
    });
    Some(
      div()
        .id("resize-pointer")
        .relative()
        .flex_none()
        .when(axis == Axis::Horizontal, |el| {
          el.w(px(9.0)).h_full().ml(px(-4.0)).px(px(4.0)).cursor_col_resize()
        })
        .when(axis == Axis::Vertical, |el| {
          el.h(px(9.0)).w_full().mt(px(-4.0)).py(px(4.0)).cursor_row_resize()
        })
        .on_mouse_down(MouseButton::Left, {
          let state = state.clone();
          move |event, window, cx| {
            if let Some(size) = state.read(cx).sizes().first().copied() {
              drag.update(cx, |drag, _| {
                *drag = Some(DragStart {
                  position: event.position,
                  size,
                })
              });
              window.refresh();
            }
            cx.stop_propagation();
          }
        })
        .child(
          div()
            .flex_none()
            .bg(hsla(if active { palette.ring } else { palette.border }))
            .group_hover("handle", |el| el.bg(hsla(palette.ring)))
            .when(axis == Axis::Horizontal, |el| el.w(px(1.0)).h_full())
            .when(axis == Axis::Vertical, |el| el.h(px(1.0)).w_full()),
        )
        .child(pointer.absolute().size_full())
        .into_any_element(),
    )
  })
}

#[cfg(test)]
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
          .with_handle_appearance(divider(self.outer.clone(), move |_, _, _| {
            outer_releases.set(outer_releases.get() + 1);
          }))
          .child(resizable_panel().size(px(300.0)).child(div().size_full()))
          .child(
            resizable_panel().size(px(300.0)).child(
              v_resizable("inner")
                .with_state(&self.inner)
                .with_handle_appearance(divider(self.inner.clone(), move |_, _, _| {
                  inner_releases.set(inner_releases.get() + 1);
                }))
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
    window.dispatch_event(
      PlatformInput::MouseMove(MouseMoveEvent {
        position: to,
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::default(),
      }),
      cx,
    );
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
        point(nested.left(), px(50.0)),
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
        point(nested.left() + px(60.0), nested.top()),
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
