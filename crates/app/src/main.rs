mod actions;
mod assets;
mod cli_install;
mod config;
mod keymap;
mod menus;
mod open_requests;
mod overlays;
mod repo;
mod shell;
mod theme;
mod title_bar;
mod updater;
mod welcome;
mod window;
mod zoom;

use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::Core;
use gpui_kit::*;

use crate::open_requests::{OpenRequest, OpenRequests};
use crate::window::{WindowRegistry, on_window_closed, open_shell_window};

fn assets_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn main() {
  tracing_subscriber::fmt::init();
  let core = Core::new(assets_dir()).expect("core failed to start");
  let initial_path = std::env::args().nth(1).map(PathBuf::from).filter(|p| p.is_dir());
  let mut requests = OpenRequests::new();
  let rx = requests.rx.take().expect("receiver");
  let url_tx = requests.tx.clone();
  let reopen_tx = requests.tx.clone();

  let app = gpui_kit::application()
    .with_assets(assets::AppAssets)
    .with_quit_mode(if cfg!(target_os = "macos") {
      QuitMode::Explicit
    } else {
      QuitMode::LastWindowClosed
    });
  app.on_open_urls(move |urls| {
    for request in OpenRequests::from_urls(&urls) {
      let _ = url_tx.unbounded_send(request);
    }
  });
  app.on_reopen(move |_cx| {
    let _ = reopen_tx.unbounded_send(OpenRequest::NewWindow);
  });

  app.run(move |cx| {
    gpui_kit::init(cx);
    cx.set_http_client(Arc::new(
      reqwest_client::ReqwestClient::user_agent(&format!("deathpush/{}", env!("CARGO_PKG_VERSION")))
        .expect("http client"),
    ));
    cx.text_system()
      .add_fonts(assets::font_files())
      .expect("bundled fonts load");
    cx.set_app_identity("com.deathpush.app", "DeathPush");
    config::AppConfig::init(cx);
    theme::init(cx);
    cx.bind_keys(keymap::bindings());
    let mut registry = WindowRegistry::default();
    registry.core = Some(core.clone());
    cx.set_global(registry);
    menus::refresh_menus(cx);

    cx.on_action(|_: &actions::Quit, cx| {
      config::AppConfig::save_now(cx);
      cx.quit();
    });
    cx.on_action(|_: &actions::Hide, cx| cx.hide());
    cx.on_action(|_: &actions::HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &actions::ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &actions::NewWindow, cx| {
      open_shell_window(None, cx);
    });
    cx.on_window_closed(|cx, window_id| on_window_closed(window_id, cx))
      .detach();

    if !cfg!(target_os = "macos") {
      cx.register_url_scheme("deathpush").detach();
    }

    cx.activate(true);
    open_shell_window(initial_path.clone(), cx);

    let mut rx = rx;
    cx.spawn(async move |cx| {
      use futures::StreamExt;
      while let Some(request) = rx.next().await {
        cx.update(|cx| match request {
          OpenRequest::Repository(path) => {
            open_shell_window(Some(path), cx);
          }
          OpenRequest::NewWindow => {
            if cx.windows().is_empty() {
              open_shell_window(None, cx);
            }
          }
        });
      }
    })
    .detach();
  });
}
