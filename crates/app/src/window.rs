use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::Core;
use deathpush_core::config::windows::SavedWindow;
use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;

use crate::config::AppConfig;
use crate::shell::Shell;

/// Which core session each window owns, so closing a window closes its session.
#[derive(Default)]
pub struct WindowRegistry {
  pub core: Option<Arc<Core>>,
  pub sessions: Vec<(WindowId, deathpush_core::SessionId)>,
  next_index: usize,
}

impl Global for WindowRegistry {}

pub fn window_options(saved: SavedWindow) -> WindowOptions {
  let bounds = Bounds {
    origin: point(px(saved.x), px(saved.y)),
    size: size(px(saved.width.max(640.0)), px(saved.height.max(480.0))),
  };
  let mut options = if cfg!(target_os = "windows") {
    WindowOptions::default()
  } else {
    TitleBar::window_options()
  };
  options.window_bounds = Some(if saved.maximized {
    WindowBounds::Maximized(bounds)
  } else {
    WindowBounds::Windowed(bounds)
  });
  options.window_min_size = Some(size(px(640.0), px(480.0)));
  options.window_decorations = Some(if cfg!(target_os = "linux") {
    WindowDecorations::Client
  } else {
    WindowDecorations::Server
  });
  options.app_id = Some("com.deathpush.app".into());
  if cfg!(target_os = "windows") {
    options.titlebar = Some(TitlebarOptions {
      title: Some("DeathPush".into()),
      ..Default::default()
    });
  }
  options
}

/// Open a shell window; `initial` opens that repository at once.
pub fn open_shell_window(initial: Option<PathBuf>, cx: &mut App) -> Option<WindowHandle<Root>> {
  let core = cx.global::<WindowRegistry>().core.clone()?;
  let index = {
    let registry = cx.global_mut::<WindowRegistry>();
    let index = registry.next_index;
    registry.next_index += 1;
    index
  };
  let saved = AppConfig::get(cx).windows.bounds_for(index);
  let handle = cx
    .open_window(window_options(saved), |window, cx| {
      let shell = cx.new(|cx| Shell::new(core.clone(), index, initial.clone(), window, cx));
      let session = shell.read(cx).session;
      cx.global_mut::<WindowRegistry>()
        .sessions
        .push((window.window_handle().window_id(), session));
      cx.new(|cx| Root::new(shell, window, cx))
    })
    .ok()?;
  Some(handle)
}

/// Called from `App::on_window_closed`.
pub fn on_window_closed(window_id: WindowId, cx: &mut App) {
  let registry = cx.global_mut::<WindowRegistry>();
  let Some(position) = registry.sessions.iter().position(|(id, _)| *id == window_id) else {
    return;
  };
  let (_, session) = registry.sessions.remove(position);
  if let Some(core) = registry.core.clone() {
    let runtime = core.clone();
    drop(runtime.spawn(async move { core.close_session(session).await }));
  }
  AppConfig::save_now(cx);
}
