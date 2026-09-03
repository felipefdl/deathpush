use gpui_kit::component::Root;
use gpui_kit::component::button::*;
use gpui_kit::*;

actions!(deathpush, [ZoomIn, ZoomOut, ZoomReset, Quit]);

const BASE_REM: f32 = 16.0;

struct Zoom {
  level: i32,
}

impl Global for Zoom {}

fn zoom_scale(level: i32) -> f32 {
  1.2f32.powi(level)
}

fn adjust_zoom(cx: &mut App, next: impl Fn(i32) -> i32) {
  let level = next(cx.global::<Zoom>().level).clamp(-5, 9);
  cx.global_mut::<Zoom>().level = level;
  let Some(window) = cx.active_window() else {
    return;
  };
  let _ = window.update(cx, |_, window, _| {
    window.set_rem_size(px(BASE_REM * zoom_scale(level)));
    window.refresh();
  });
}

struct Hello;

impl Render for Hello {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let percent = (zoom_scale(cx.global::<Zoom>().level) * 100.0).round() as i32;
    div()
      .flex()
      .flex_col()
      .gap_2()
      .p_4()
      .size_full()
      .child(format!(
        "DeathPush on GPUI. Zoom {percent}%. Cmd+= and Cmd+- change it."
      ))
      .child(Button::new("push").primary().label("Push"))
  }
}

fn main() {
  tracing_subscriber::fmt::init();
  gpui_kit::application().run(|cx| {
    gpui_kit::init(cx);
    cx.set_global(Zoom { level: 0 });
    cx.bind_keys([
      KeyBinding::new("cmd-=", ZoomIn, None),
      KeyBinding::new("cmd--", ZoomOut, None),
      KeyBinding::new("cmd-0", ZoomReset, None),
      KeyBinding::new("cmd-q", Quit, None),
    ]);
    cx.on_action(|_: &ZoomIn, cx| adjust_zoom(cx, |level| level + 1));
    cx.on_action(|_: &ZoomOut, cx| adjust_zoom(cx, |level| level - 1));
    cx.on_action(|_: &ZoomReset, cx| adjust_zoom(cx, |_| 0));
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.spawn(async move |cx| {
      cx.open_window(WindowOptions::default(), |window, cx| {
        let view = cx.new(|_| Hello);
        cx.new(|cx| Root::new(view, window, cx))
      })
      .expect("failed to open window");
    })
    .detach();
  });
}
