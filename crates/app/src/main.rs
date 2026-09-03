use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::session::types::{Intent, IntentOutcome};
use deathpush_core::{Core, CoreEvent, SessionId};
use gpui_kit::component::Root;
use gpui_kit::*;
use tokio::sync::mpsc::UnboundedReceiver;

actions!(deathpush, [Quit]);

fn assets_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

struct Shell {
  core: Arc<Core>,
  session: SessionId,
  title: SharedString,
  status_events: usize,
  error: Option<SharedString>,
}

impl Shell {
  fn new(core: Arc<Core>, session: SessionId, events: UnboundedReceiver<CoreEvent>, cx: &mut Context<Self>) -> Self {
    let shell = Self {
      core,
      session,
      title: "DeathPush".into(),
      status_events: 0,
      error: None,
    };
    shell.listen(events, cx);
    shell
  }

  fn listen(&self, mut events: UnboundedReceiver<CoreEvent>, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
      while let Some(event) = events.recv().await {
        let alive = this.update(cx, |this, cx| {
          if matches!(event, CoreEvent::SessionStatus(_)) {
            this.status_events += 1;
            cx.notify();
          }
        });
        if alive.is_err() {
          break;
        }
      }
    })
    .detach();
  }

  fn open(&mut self, path: String, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let session = self.session;
    let task = {
      let runtime = core.clone();
      runtime.spawn(async move { core.session_intent(session, Intent::OpenRepository { path }).await })
    };
    cx.spawn(async move |this, cx| {
      let result = task.await;
      this
        .update(cx, |this, cx| {
          match result {
            Ok(Ok(IntentOutcome::Snapshot { snapshot })) => {
              this.title =
                deathpush_core::ops::window_title(&snapshot.repo.root, snapshot.repo.head_branch.as_deref()).into();
            }
            Ok(Ok(other)) => this.error = Some(format!("unexpected outcome: {other:?}").into()),
            Ok(Err(err)) => this.error = Some(err.to_string().into()),
            Err(err) => this.error = Some(err.to_string().into()),
          }
          cx.notify();
        })
        .ok();
    })
    .detach();
  }
}

impl Render for Shell {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .gap_2()
      .p_4()
      .size_full()
      .child(self.title.clone())
      .child(format!("status events: {}", self.status_events))
      .children(
        self
          .error
          .clone()
          .map(|error| div().text_color(rgb(0xf85149)).child(error)),
      )
  }
}

fn main() {
  tracing_subscriber::fmt::init();
  let core = Core::new(assets_dir()).expect("core failed to start");
  let initial_path = std::env::args().nth(1).filter(|p| std::path::Path::new(p).is_dir());
  gpui_kit::application().run(move |cx| {
    gpui_kit::init(cx);
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.on_action(|_: &Quit, cx| cx.quit());
    let core = core.clone();
    cx.spawn(async move |cx| {
      cx.open_window(WindowOptions::default(), |window, cx| {
        let (session, events) = core.open_session();
        let view = cx.new(|cx| Shell::new(core.clone(), session, events, cx));
        if let Some(path) = initial_path.clone() {
          view.update(cx, |shell, cx| shell.open(path, cx));
        }
        cx.new(|cx| Root::new(view, window, cx))
      })
      .expect("failed to open window");
    })
    .detach();
  });
}
