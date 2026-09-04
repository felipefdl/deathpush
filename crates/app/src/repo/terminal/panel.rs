use std::collections::{HashMap, HashSet};

use deathpush_core::config::layout::PanelTab;
use gpui_kit::base::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::button::*;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::model::{SplitTree, TerminalModel};
use crate::actions::*;
use crate::repo::layout_model::LayoutModel;
use crate::repo::output_log::{OutputLog, format_line};
use crate::theme::{ActivePalette, hsla};

pub fn tab_label(tab: PanelTab) -> &'static str {
  match tab {
    PanelTab::GitOutput => "Output",
    PanelTab::Terminal => "Terminal",
  }
}

pub struct TerminalPanel {
  model: Entity<TerminalModel>,
  layout: Entity<LayoutModel>,
  output: Entity<OutputLog>,
  sidebar_state: Entity<ResizableState>,
  split_states: HashMap<u64, Entity<ResizableState>>,
}

impl TerminalPanel {
  pub fn new(
    model: Entity<TerminalModel>,
    layout: Entity<LayoutModel>,
    output: Entity<OutputLog>,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&model, |_, _, cx| cx.notify()).detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    cx.observe(&output, |_, _, cx| cx.notify()).detach();
    Self {
      model,
      layout,
      output,
      sidebar_state: cx.new(|_| ResizableState::default()),
      split_states: HashMap::new(),
    }
  }

  #[cfg(test)]
  pub(crate) fn model(&self) -> &Entity<TerminalModel> {
    &self.model
  }

  fn split_state(&mut self, id: u64, cx: &mut Context<Self>) -> Entity<ResizableState> {
    self
      .split_states
      .entry(id)
      .or_insert_with(|| cx.new(|_| ResizableState::default()))
      .clone()
  }

  fn prune_split_states(&mut self, cx: &App) {
    let live: HashSet<u64> = self
      .model
      .read(cx)
      .groups
      .iter()
      .flat_map(|group| group.tree.split_ids())
      .collect();
    self.split_states.retain(|id, _| live.contains(id));
  }

  #[cfg(test)]
  pub(crate) fn split_state_ids(&self) -> Vec<u64> {
    let mut ids: Vec<u64> = self.split_states.keys().copied().collect();
    ids.sort_unstable();
    ids
  }

  fn render_tree(&mut self, tree: &SplitTree, cx: &mut Context<Self>) -> AnyElement {
    match tree {
      SplitTree::Leaf(id) => self
        .model
        .read(cx)
        .panes
        .get(id)
        .map(|pane| pane.view.clone().into_any_element())
        .unwrap_or_else(|| div().size_full().into_any_element()),
      SplitTree::Split {
        id,
        axis,
        first,
        second,
      } => {
        let state = self.split_state(*id, cx);
        let first = self.render_tree(first, cx);
        let second = self.render_tree(second, cx);
        let key = SharedString::from(format!("term-split-{id}"));
        let group = match axis {
          Axis::Horizontal => h_resizable(key),
          Axis::Vertical => v_resizable(key),
        };
        group
          .with_state(&state)
          .child(resizable_panel().child(first))
          .child(resizable_panel().child(second))
          .into_any_element()
      }
    }
  }

  fn render_sidebar(&self, pane_ids: &[u64], cx: &App) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let model = self.model.clone();
    let layout = self.layout.clone();
    let rows: Vec<AnyElement> = pane_ids
      .iter()
      .copied()
      .map(|pane_id| {
        let name = model
          .read(cx)
          .panes
          .get(&pane_id)
          .map(|pane| pane.name())
          .unwrap_or_else(|| format!("Terminal {pane_id}"));
        let group = SharedString::from(format!("term-pane-{pane_id}"));
        let hover = group.clone();
        let activate = model.clone();
        let split_h = model.clone();
        let split_v = model.clone();
        let kill = model.clone();
        div()
          .id(SharedString::from(format!("term-row-{pane_id}")))
          .group(group)
          .h(px(22.0))
          .flex_shrink_0()
          .flex()
          .items_center()
          .gap_1()
          .px_1()
          .cursor_pointer()
          .hover(|el| el.bg(hsla(palette.list_hover)))
          .on_click(move |_, window, cx| {
            activate.update(cx, |model, cx| model.activate_pane(pane_id, window, cx));
          })
          .child(
            div()
              .flex_1()
              .min_w_0()
              .overflow_hidden()
              .text_ellipsis()
              .text_size(px(12.0))
              .child(name),
          )
          .child(sidebar_icon(
            format!("term-split-h-{pane_id}"),
            "icons/split-horizontal.svg",
            "Split Horizontally",
            hover.clone(),
            {
              let layout = layout.clone();
              move |window, cx| {
                layout.update(cx, |layout, cx| {
                  layout.set_terminal_visible(true, cx);
                  layout.set_panel_tab(PanelTab::Terminal, cx);
                });
                split_h.update(cx, |model, cx| {
                  model.set_panes_visible(true, cx);
                  model.split(pane_id, Axis::Vertical, window, cx);
                });
              }
            },
          ))
          .child(sidebar_icon(
            format!("term-split-v-{pane_id}"),
            "icons/split-vertical.svg",
            "Split Vertically",
            hover.clone(),
            {
              let layout = layout.clone();
              move |window, cx| {
                layout.update(cx, |layout, cx| {
                  layout.set_terminal_visible(true, cx);
                  layout.set_panel_tab(PanelTab::Terminal, cx);
                });
                split_v.update(cx, |model, cx| {
                  model.set_panes_visible(true, cx);
                  model.split(pane_id, Axis::Horizontal, window, cx);
                });
              }
            },
          ))
          .child(sidebar_icon(
            format!("term-kill-{pane_id}"),
            "icons/close.svg",
            "Kill Terminal",
            hover,
            move |window, cx| {
              kill.update(cx, |model, cx| model.kill_pane(pane_id, Some(window), cx));
            },
          ))
          .into_any_element()
      })
      .collect();
    div().size_full().flex().flex_col().children(rows)
  }

  fn render_output(&self, cx: &App) -> AnyElement {
    let palette = cx.global::<ActivePalette>().0;
    let lines: Vec<String> = self.output.read(cx).lines().iter().map(format_line).collect();
    if lines.is_empty() {
      return div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(hsla(palette.muted_foreground))
        .child("No git commands recorded yet.")
        .into_any_element();
    }
    let count = lines.len();
    let lines = std::sync::Arc::new(lines);
    uniform_list("git-output", count, move |range, _, _| {
      range
        .map(|i| {
          div()
            .px_2()
            .h(px(20.0))
            .text_size(px(12.0))
            .font_family("MesloLGS Nerd Font Mono")
            .child(lines[i].clone())
        })
        .collect()
    })
    .size_full()
    .into_any_element()
  }
}

fn sidebar_icon(
  id: String,
  icon: &'static str,
  tooltip: &'static str,
  hover: SharedString,
  on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
  Button::new(SharedString::from(id))
    .ghost()
    .xsmall()
    .icon(Icon::empty().path(icon))
    .tooltip(tooltip)
    .invisible()
    .group_hover(hover, |style| style.visible())
    .on_click(move |_, window, cx| {
      cx.stop_propagation();
      on_click(window, cx);
    })
}

fn header_icon(
  id: &'static str,
  icon: &'static str,
  tooltip: &'static str,
  action: Box<dyn Action>,
) -> impl IntoElement {
  Button::new(id)
    .ghost()
    .xsmall()
    .icon(Icon::empty().path(icon))
    .tooltip(tooltip)
    .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
}

impl Render for TerminalPanel {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.prune_split_states(cx);
    let palette = cx.global::<ActivePalette>().0;
    let layout = self.layout.read(cx).layout().clone();
    let active = layout.panel_tab;
    let maximized = layout.terminal_maximized;
    let show_actions = active == PanelTab::Terminal || maximized;
    let tree = self.model.read(cx).active_group().map(|group| group.tree.clone());
    let pane_ids = tree.as_ref().map(SplitTree::panes).unwrap_or_default();
    let show_sidebar = active == PanelTab::Terminal && pane_ids.len() > 1;
    let tab = |id: &'static str, tab: PanelTab, action: Box<dyn Action>| {
      let is_active = active == tab;
      div()
        .id(id)
        .px_3()
        .h_full()
        .flex()
        .items_center()
        .text_size(px(12.0))
        .cursor_pointer()
        .opacity(if is_active { 1.0 } else { 0.6 })
        .border_b_2()
        .border_color(if is_active {
          hsla(palette.ring)
        } else {
          hsla(palette.border.with_alpha(0))
        })
        .child(tab_label(tab))
        .on_click(move |_, window, cx| window.dispatch_action(action.boxed_clone(), cx))
    };
    let body: AnyElement = match active {
      PanelTab::GitOutput => self.render_output(cx),
      PanelTab::Terminal => {
        let panes = tree
          .as_ref()
          .map(|tree| self.render_tree(tree, cx))
          .unwrap_or_else(|| div().size_full().into_any_element());
        if show_sidebar {
          let layout_entity = self.layout.clone();
          let sidebar_index = 0;
          h_resizable("terminal-panes")
            .with_state(&self.sidebar_state)
            .on_resize(move |state, _, cx| {
              if let Some(width) = state.read(cx).sizes().get(sidebar_index).copied() {
                layout_entity.update(cx, |layout, cx| layout.set_terminal_sidebar_width(f32::from(width), cx));
              }
            })
            .child(
              resizable_panel()
                .size(px(layout.terminal_sidebar_width))
                .size_range(px(80.0)..px(400.0))
                .flex_none()
                .child(self.render_sidebar(&pane_ids, cx)),
            )
            .child(resizable_panel().child(panes))
            .into_any_element()
        } else {
          panes
        }
      }
    };
    div()
      .size_full()
      .flex()
      .flex_col()
      .bg(hsla(palette.sidebar))
      .text_color(hsla(palette.foreground))
      .child(
        div()
          .h(px(35.0))
          .flex_shrink_0()
          .flex()
          .items_center()
          .border_b_1()
          .border_color(hsla(palette.border))
          .child(tab("panel-tab-output", PanelTab::GitOutput, Box::new(ShowOutputTab)))
          .child(tab("panel-tab-terminal", PanelTab::Terminal, Box::new(ShowTerminalTab)))
          .child(div().flex_1())
          .when(show_actions, |el| {
            el.child(
              div()
                .flex()
                .items_center()
                .gap_1()
                .px_1()
                .child(header_icon(
                  "panel-new",
                  "icons/add.svg",
                  "New Terminal",
                  Box::new(NewTerminal),
                ))
                .child(div().w(px(1.0)).h(px(14.0)).bg(hsla(palette.border)))
                .child(header_icon(
                  "panel-split-h",
                  "icons/split-horizontal.svg",
                  "Split Terminal Horizontally",
                  Box::new(SplitTerminalHorizontal),
                ))
                .child(header_icon(
                  "panel-split-v",
                  "icons/split-vertical.svg",
                  "Split Terminal Vertically",
                  Box::new(SplitTerminalVertical),
                ))
                .child(header_icon(
                  "panel-maximize",
                  if maximized {
                    "icons/screen-normal.svg"
                  } else {
                    "icons/screen-full.svg"
                  },
                  if maximized {
                    "Restore Panel Size"
                  } else {
                    "Maximize Panel Size"
                  },
                  Box::new(ToggleTerminalMaximize),
                ))
                .child(header_icon(
                  "panel-close",
                  "icons/close.svg",
                  "Close Panel",
                  Box::new(ClosePanel),
                )),
            )
          }),
      )
      .child(div().flex_1().min_h_0().child(body))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use std::sync::Arc;

  use deathpush_core::Core;
  use deathpush_core::terminal::snapshot::{PaneSnapshot, Rgb, SnapshotCell};
  use gpui_kit::TestAppContext;

  use crate::config::AppConfig;
  use crate::repo::terminal::pane_view::PaneView;

  fn text_cell(ch: char) -> SnapshotCell {
    SnapshotCell {
      text: ch.to_string(),
      ..SnapshotCell::default()
    }
  }

  fn injected_snapshot(text: &str) -> Arc<PaneSnapshot> {
    let cols = text.chars().count() as u16;
    Arc::new(PaneSnapshot {
      seq: 1,
      cols,
      rows: 1,
      cells: text.chars().map(text_cell).collect(),
      cursor: None,
      background: Rgb(0, 0, 0),
      foreground: Rgb(255, 255, 255),
      cursor_color: None,
      viewport_offset: 0,
      scrollback_rows: 0,
      bell: false,
    })
  }

  #[gpui_kit::test]
  fn panel_renders_an_injected_pane(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let snapshot = injected_snapshot("hi");
    let layout_dir = config_dir.path().to_path_buf();
    let window = cx.add_window({
      let core = core.clone();
      let snapshot = snapshot.clone();
      let layout_dir = layout_dir.clone();
      move |_, cx| {
        let model = cx.new(|cx| {
          let mut model = TerminalModel::new(core, session, cx);
          let pane = cx.new(|cx| PaneView::new_unthreaded(1, cx));
          pane.update(cx, |view, _| view.set_snapshot(snapshot.clone()));
          model.insert_test_pane(pane, cx);
          model
        });
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, "/tmp", true));
        let output = cx.new(|_| OutputLog::default());
        TerminalPanel::new(model, layout, output, cx)
      }
    });
    window
      .update(cx, |_, window, _cx| {
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |panel, _, cx| {
        assert_eq!(panel.model().read(cx).groups.len(), 1);
        assert_eq!(panel.model().read(cx).panes.len(), 1);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn prune_split_states_drops_ids_not_in_any_tree(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let layout_dir = config_dir.path().to_path_buf();
    let window = cx.add_window({
      let core = core.clone();
      let layout_dir = layout_dir.clone();
      move |_, cx| {
        let model = cx.new(|cx| {
          let mut model = TerminalModel::new(core, session, cx);
          let a = cx.new(|cx| PaneView::new_unthreaded(1, cx));
          let a2 = cx.new(|cx| PaneView::new_unthreaded(3, cx));
          let b = cx.new(|cx| PaneView::new_unthreaded(2, cx));
          let b2 = cx.new(|cx| PaneView::new_unthreaded(4, cx));
          model.insert_test_pane(a, cx);
          model.insert_test_pane(b, cx);
          model.attach_test_pane(1, Axis::Horizontal, a2, cx);
          model.attach_test_pane(2, Axis::Vertical, b2, cx);
          model.activate_group(1, cx);
          model
        });
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, "/tmp", true));
        let output = cx.new(|_| OutputLog::default());
        TerminalPanel::new(model, layout, output, cx)
      }
    });
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |panel, _, cx| {
        assert_eq!(panel.split_state_ids().len(), 1);
        panel.model().update(cx, |model, cx| {
          assert!(model.activate_group(2, cx));
        });
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |panel, _, cx| {
        assert_eq!(panel.split_state_ids().len(), 2);
        let second = panel.model().read(cx).groups[1].id;
        panel.model().update(cx, |model, cx| model.kill_group(second, None, cx));
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |panel, _, _| {
        assert_eq!(panel.split_state_ids().len(), 1);
      })
      .unwrap();
  }
}
