//! Wheel routing for the diff body. The rows scroll sideways through one shared handle and the
//! list scrolls down through another. Left to their own listeners, each ran its own gesture lock
//! over the events it happened to receive, and the row listener applied any sideways drift the
//! moment its lock opened, so a vertical swipe could start a horizontal scroll. Every wheel event
//! over the diff now goes through one gesture lock, then keeps only its larger axis, the way VS
//! Code's `scrollPredominantAxis` does.

use gpui_kit::*;

/// The scroll to apply for one wheel event, with at most one axis non-zero.
/// `room` says which axes have anywhere to go; an axis without room does not compete.
pub fn route(
  lock: &mut OngoingScroll,
  event: &ScrollWheelEvent,
  line_height: Pixels,
  room: Point<bool>,
) -> Point<Pixels> {
  let mut delta = event.delta.pixel_delta(line_height);
  if event.delta.precise() {
    lock.filter(&mut delta, event.touch_phase);
  }
  if !room.x {
    delta.x = Pixels::ZERO;
  }
  if !room.y {
    delta.y = Pixels::ZERO;
  }
  if delta.x.abs() > delta.y.abs() {
    delta.y = Pixels::ZERO;
  } else {
    delta.x = Pixels::ZERO;
  }
  delta
}

/// Move `handle` by `delta`, kept inside its scrollable range. True when the offset changed.
pub fn scroll_by(handle: &ScrollHandle, delta: Point<Pixels>) -> bool {
  let max = handle.max_offset();
  let old = handle.offset();
  let new = point(
    (old.x + delta.x).clamp(-max.x, Pixels::ZERO),
    (old.y + delta.y).clamp(-max.y, Pixels::ZERO),
  );
  if new == old {
    return false;
  }
  handle.set_offset(new);
  true
}

#[cfg(test)]
mod tests {
  use gpui_kit::{OngoingScroll, Pixels, Point, ScrollDelta, ScrollWheelEvent, TouchPhase, point, px};

  use super::route;

  const LINE: Pixels = px(20.0);
  const BOTH: Point<bool> = Point { x: true, y: true };

  fn trackpad(x: f32, y: f32, touch_phase: TouchPhase) -> ScrollWheelEvent {
    ScrollWheelEvent {
      delta: ScrollDelta::Pixels(point(px(x), px(y))),
      touch_phase,
      ..Default::default()
    }
  }

  fn wheel(x: f32, y: f32) -> ScrollWheelEvent {
    ScrollWheelEvent {
      delta: ScrollDelta::Lines(point(x, y)),
      ..Default::default()
    }
  }

  #[test]
  fn a_vertical_swipe_that_drifts_sideways_never_scrolls_horizontally() {
    let mut lock = OngoingScroll::default();
    let swipe = [
      trackpad(1.0, -4.0, TouchPhase::Started),
      trackpad(3.0, -12.0, TouchPhase::Moved),
      trackpad(4.0, -3.0, TouchPhase::Moved),
      trackpad(6.0, -18.0, TouchPhase::Moved),
      trackpad(5.0, -2.0, TouchPhase::Moved),
    ];
    let mut moved = Point::<Pixels>::default();
    for event in &swipe {
      moved += route(&mut lock, event, LINE, BOTH);
    }
    assert_eq!(moved, point(Pixels::ZERO, px(-39.0)));
  }

  #[test]
  fn after_a_sideways_jerk_unlocks_the_gesture_one_event_never_moves_both_axes() {
    let mut lock = OngoingScroll::default();
    route(&mut lock, &trackpad(1.0, -4.0, TouchPhase::Started), LINE, BOTH);
    let jerk = route(&mut lock, &trackpad(9.0, -4.0, TouchPhase::Moved), LINE, BOTH);
    assert_eq!(jerk, point(px(9.0), Pixels::ZERO));
    for (x, y) in [(3.0, -12.0), (4.0, -15.0), (2.0, -10.0)] {
      let delta = route(&mut lock, &trackpad(x, y, TouchPhase::Moved), LINE, BOTH);
      assert_eq!(delta, point(Pixels::ZERO, px(y)));
    }
  }

  #[test]
  fn a_horizontal_swipe_still_scrolls_horizontally() {
    let mut lock = OngoingScroll::default();
    let first = route(&mut lock, &trackpad(-10.0, 1.0, TouchPhase::Started), LINE, BOTH);
    let second = route(&mut lock, &trackpad(-14.0, 3.0, TouchPhase::Moved), LINE, BOTH);
    assert_eq!(first, point(px(-10.0), Pixels::ZERO));
    assert_eq!(second, point(px(-14.0), Pixels::ZERO));
  }

  #[test]
  fn a_diagonal_wheel_click_keeps_its_larger_axis() {
    let mut lock = OngoingScroll::default();
    assert_eq!(
      route(&mut lock, &wheel(1.0, -3.0), LINE, BOTH),
      point(Pixels::ZERO, px(-60.0))
    );
    assert_eq!(
      route(&mut lock, &wheel(2.0, 1.0), LINE, BOTH),
      point(px(40.0), Pixels::ZERO)
    );
  }

  #[test]
  fn an_axis_without_room_does_not_swallow_the_event() {
    let mut lock = OngoingScroll::default();
    let no_sideways = point(false, true);
    assert_eq!(
      route(&mut lock, &wheel(2.0, -1.0), LINE, no_sideways),
      point(Pixels::ZERO, px(-20.0))
    );
  }
}
