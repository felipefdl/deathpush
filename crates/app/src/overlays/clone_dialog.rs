use gpui_kit::component::button::*;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::frame::{backdrop, dialog_frame};
use crate::actions::{Cancel, Confirm};
use crate::theme::{ActivePalette, hsla};

pub enum CloneEvent {
  Close,
  Clone { url: String, directory: String },
}

pub struct CloneDialog {
  url: Entity<InputState>,
  directory: Entity<InputState>,
  cloning: bool,
}

impl EventEmitter<CloneEvent> for CloneDialog {}

impl CloneDialog {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let url = cx.new(|cx| InputState::new(window, cx).placeholder("https://github.com/user/repo.git"));
    let directory = cx.new(|cx| InputState::new(window, cx).placeholder("Select a directory..."));
    for state in [&url, &directory] {
      cx.subscribe(state, |_, _, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
          cx.notify();
        }
      })
      .detach();
    }
    url.update(cx, |state, cx| state.focus(window, cx));
    Self {
      url,
      directory,
      cloning: false,
    }
  }

  pub fn set_cloning(&mut self, cloning: bool, cx: &mut Context<Self>) {
    self.cloning = cloning;
    cx.notify();
  }

  fn values(&self, cx: &App) -> (String, String) {
    (
      self.url.read(cx).value().trim().to_string(),
      self.directory.read(cx).value().trim().to_string(),
    )
  }

  fn can_clone(&self, cx: &App) -> bool {
    let (url, directory) = self.values(cx);
    !self.cloning && !url.is_empty() && !directory.is_empty()
  }

  fn submit(&mut self, cx: &mut Context<Self>) {
    if self.can_clone(cx) {
      let (url, directory) = self.values(cx);
      cx.emit(CloneEvent::Clone { url, directory });
    }
  }

  fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
      files: false,
      directories: true,
      multiple: false,
      prompt: Some("Choose directory to clone into".into()),
    });
    let directory = self.directory.clone();
    cx.spawn_in(window, async move |_, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await
        && let Some(path) = paths.first()
      {
        let text = path.to_string_lossy().into_owned();
        let _ = cx.update(|window, cx| directory.update(cx, |state, cx| state.set_value(text, window, cx)));
      }
    })
    .detach();
  }
}

impl Render for CloneDialog {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let can_clone = self.can_clone(cx);
    let cloning = self.cloning;
    let label = |text: &'static str| div().text_size(px(12.0)).mb(px(4.0)).child(text);
    backdrop(
      "clone-backdrop",
      |_, cx| {
        let _ = cx;
      },
      cx,
    )
    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(CloneEvent::Close)))
    .child(
      dialog_frame(440.0, "Clone Repository", cx)
        .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(CloneEvent::Close)))
        .on_action(cx.listener(|this, _: &Confirm, _, cx| this.submit(cx)))
        .child(label("Repository URL"))
        .child(
          div()
            .mb(px(12.0))
            .child(Input::new(&self.url).h(px(28.0)).disabled(cloning)),
        )
        .child(label("Directory"))
        .child(
          div()
            .flex()
            .gap_2()
            .mb(px(16.0))
            .child(
              div()
                .flex_1()
                .child(Input::new(&self.directory).h(px(28.0)).disabled(cloning)),
            )
            .child(
              Button::new("browse")
                .outline()
                .w(px(28.0))
                .h(px(28.0))
                .icon(Icon::empty().path("icons/folder.svg"))
                .disabled(cloning)
                .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            ),
        )
        .child(
          div()
            .flex()
            .justify_end()
            .gap_2()
            .child(
              Button::new("cancel")
                .outline()
                .small()
                .label("Cancel")
                .disabled(cloning)
                .on_click(cx.listener(|_, _, _, cx| cx.emit(CloneEvent::Close))),
            )
            .child(
              Button::new("clone")
                .primary()
                .small()
                .min_w(px(88.0))
                .label(if cloning { "Cloning..." } else { "Clone" })
                .disabled(!can_clone)
                .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
            ),
        )
        .text_color(hsla(palette.foreground)),
    )
  }
}
