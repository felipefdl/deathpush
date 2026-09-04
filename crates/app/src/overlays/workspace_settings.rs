use deathpush_core::config::settings::WorkspaceEntry;
use gpui_kit::component::button::*;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::frame::{backdrop, dialog_frame};
use crate::actions::{Cancel, Confirm};
use crate::theme::{ActivePalette, hsla};

pub enum WorkspaceEvent {
  Close,
  Save(Vec<WorkspaceEntry>),
}

struct Row {
  directory: Entity<InputState>,
  depth: u32,
}

pub struct WorkspaceSettingsDialog {
  rows: Vec<Row>,
}

impl EventEmitter<WorkspaceEvent> for WorkspaceSettingsDialog {}

impl WorkspaceSettingsDialog {
  pub fn new(entries: &[WorkspaceEntry], window: &mut Window, cx: &mut Context<Self>) -> Self {
    let mut dialog = Self { rows: Vec::new() };
    if entries.is_empty() {
      dialog.push_row(String::new(), 1, window, cx);
    } else {
      for entry in entries {
        dialog.push_row(entry.directory.clone(), entry.scan_depth.clamp(1, 5), window, cx);
      }
    }
    if let Some(first) = dialog.rows.first() {
      first.directory.update(cx, |state, cx| state.focus(window, cx));
    }
    dialog
  }

  fn push_row(&mut self, directory: String, depth: u32, window: &mut Window, cx: &mut Context<Self>) {
    let state = cx.new(|cx| {
      let mut state = InputState::new(window, cx).placeholder("Select a directory...");
      state.set_value(directory, window, cx);
      state
    });
    self.rows.push(Row {
      directory: state,
      depth,
    });
  }

  fn add_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.push_row(String::new(), 1, window, cx);
    if let Some(last) = self.rows.last() {
      last.directory.update(cx, |state, cx| state.focus(window, cx));
    }
    cx.notify();
  }

  /// Rows with a non-blank directory, as the spec's OK saves them.
  pub fn entries(&self, cx: &App) -> Vec<WorkspaceEntry> {
    self
      .rows
      .iter()
      .filter_map(|row| {
        let directory = row.directory.read(cx).value().trim().to_string();
        (!directory.is_empty()).then_some(WorkspaceEntry {
          directory,
          scan_depth: row.depth,
        })
      })
      .collect()
  }

  fn save(&mut self, cx: &mut Context<Self>) {
    let entries = self.entries(cx);
    cx.emit(WorkspaceEvent::Save(entries));
  }

  fn browse(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
      files: false,
      directories: true,
      multiple: false,
      prompt: Some("Select Git Projects Directory".into()),
    });
    let Some(state) = self.rows.get(index).map(|row| row.directory.clone()) else {
      return;
    };
    cx.spawn_in(window, async move |_, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await
        && let Some(path) = paths.first()
      {
        let text = path.to_string_lossy().into_owned();
        let _ = cx.update(|window, cx| state.update(cx, |state, cx| state.set_value(text, window, cx)));
      }
    })
    .detach();
  }
}

impl Render for WorkspaceSettingsDialog {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let only_one = self.rows.len() == 1;
    let rows: Vec<AnyElement> = self
      .rows
      .iter()
      .enumerate()
      .map(|(index, row)| {
        let depth = row.depth;
        div()
          .flex()
          .items_center()
          .gap_2()
          .mb_2()
          .child(div().flex_1().child(Input::new(&row.directory).h(px(28.0))))
          .child(
            Button::new(SharedString::from(format!("browse-{index}")))
              .outline()
              .w(px(28.0))
              .h(px(28.0))
              .icon(Icon::empty().path("icons/folder.svg"))
              .tooltip("Browse...")
              .on_click(cx.listener(move |this, _, window, cx| this.browse(index, window, cx))),
          )
          .child(
            div()
              .flex()
              .items_center()
              .gap_1()
              .child(
                Button::new(SharedString::from(format!("depth-down-{index}")))
                  .ghost()
                  .xsmall()
                  .icon(Icon::empty().path("icons/chevron-left.svg"))
                  .disabled(depth <= 1)
                  .on_click(cx.listener(move |this, _, _, cx| {
                    this.rows[index].depth = (this.rows[index].depth - 1).max(1);
                    cx.notify();
                  })),
              )
              .child(
                div()
                  .w(px(16.0))
                  .text_size(px(11.0))
                  .text_center()
                  .child(depth.to_string()),
              )
              .child(
                Button::new(SharedString::from(format!("depth-up-{index}")))
                  .ghost()
                  .xsmall()
                  .icon(Icon::empty().path("icons/chevron-right.svg"))
                  .disabled(depth >= 5)
                  .on_click(cx.listener(move |this, _, _, cx| {
                    this.rows[index].depth = (this.rows[index].depth + 1).min(5);
                    cx.notify();
                  })),
              ),
          )
          .when(!only_one, |el| {
            el.child(
              Button::new(SharedString::from(format!("remove-{index}")))
                .ghost()
                .xsmall()
                .icon(Icon::empty().path("icons/close.svg"))
                .tooltip("Remove")
                .on_click(cx.listener(move |this, _, _, cx| {
                  this.rows.remove(index);
                  cx.notify();
                })),
            )
          })
          .into_any_element()
      })
      .collect();
    backdrop("workspace-backdrop", |_, _| {}, cx)
      .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.emit(WorkspaceEvent::Close)))
      .child(
        dialog_frame(440.0, "Workspace Settings", cx)
          .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(WorkspaceEvent::Close)))
          .on_action(cx.listener(|this, _: &Confirm, _, cx| this.save(cx)))
          .child(div().text_size(px(12.0)).text_color(hsla(palette.muted_foreground)).mb(px(12.0)).child(
            "Add directories containing your Git repositories. The scan depth controls how many levels deep to search for projects within each directory.",
          ))
          .child(div().id("workspace-settings-rows").max_h(px(200.0)).overflow_y_scroll().children(rows))
          .child(
            Button::new("add-directory")
              .link()
              .small()
              .icon(Icon::empty().path("icons/add.svg"))
              .label("Add Directory")
              .on_click(cx.listener(|this, _, window, cx| this.add_row(window, cx))),
          )
          .child(
            div()
              .flex()
              .justify_end()
              .gap_2()
              .mt(px(12.0))
              .child(
                Button::new("cancel")
                  .outline()
                  .small()
                  .label("Cancel")
                  .on_click(cx.listener(|_, _, _, cx| cx.emit(WorkspaceEvent::Close))),
              )
              .child(Button::new("ok").primary().small().label("OK").on_click(cx.listener(|this, _, _, cx| this.save(cx)))),
          ),
      )
  }
}
