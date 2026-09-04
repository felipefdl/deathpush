use std::collections::HashSet;
use std::path::PathBuf;

use deathpush_core::config::settings::WorkspaceEntry;
use deathpush_core::ops::repository::{WorkspaceScanEntry, scan_workspace_projects};
use deathpush_core::types::ProjectInfo;
use deathpush_core::workspace::WorkspaceRow;
use gpui_kit::component::button::*;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::rows::{Highlight, Pane, empty_recent_copy, empty_workspace_copy, recent_indices, step, workspace_rows};
use crate::actions::*;
use crate::config::AppConfig;
use crate::theme::{ActivePalette, hsla};

pub enum WelcomeEvent {
  Open(PathBuf),
  Clone,
  ConfigureWorkspace,
}

pub struct WelcomeView {
  recent_filter: Entity<InputState>,
  workspace_filter: Entity<InputState>,
  projects: Vec<ProjectInfo>,
  expanded: HashSet<String>,
  highlight: Highlight,
  scan_generation: u64,
}

impl EventEmitter<WelcomeEvent> for WelcomeView {}

const PRIMARY_LABEL: &str = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl+" };

impl WelcomeView {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let recent_filter =
      cx.new(|cx| InputState::new(window, cx).placeholder(format!("Filter recent ({PRIMARY_LABEL}1)")));
    let workspace_filter =
      cx.new(|cx| InputState::new(window, cx).placeholder(format!("Filter workspace ({PRIMARY_LABEL}2)")));
    for state in [&recent_filter, &workspace_filter] {
      cx.subscribe(state, |this, _, event: &InputEvent, cx| {
        if matches!(event, InputEvent::Change) {
          this.highlight = Highlight::default();
          cx.notify();
        }
      })
      .detach();
    }
    let mut view = Self {
      recent_filter,
      workspace_filter,
      projects: Vec::new(),
      expanded: HashSet::new(),
      highlight: Highlight::default(),
      scan_generation: 0,
    };
    view.rescan(cx);
    view
  }

  fn workspaces(cx: &App) -> Vec<WorkspaceEntry> {
    AppConfig::get(cx).settings.projects.workspaces.clone()
  }

  /// Scan every configured directory in the background; the list keeps its content until results land.
  pub fn rescan(&mut self, cx: &mut Context<Self>) {
    let entries: Vec<WorkspaceScanEntry> = Self::workspaces(cx)
      .into_iter()
      .filter(|ws| !ws.directory.trim().is_empty())
      .map(|ws| WorkspaceScanEntry {
        directory: ws.directory,
        depth: ws.scan_depth,
      })
      .collect();
    self.scan_generation += 1;
    let generation = self.scan_generation;
    if entries.is_empty() {
      self.projects.clear();
      cx.notify();
      return;
    }
    let task = cx.background_spawn(async move { scan_workspace_projects(&entries).unwrap_or_default() });
    cx.spawn(async move |this, cx| {
      let projects = task.await;
      let _ = this.update(cx, |this, cx| {
        if this.scan_generation == generation {
          this.projects = projects;
          cx.notify();
        }
      });
    })
    .detach();
  }

  pub fn focus_recent_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.recent_filter.update(cx, |state, cx| state.focus(window, cx));
  }

  pub fn focus_workspace_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.workspace_filter.update(cx, |state, cx| state.focus(window, cx));
  }

  fn active_pane(&self, window: &Window, cx: &App) -> Pane {
    if self.workspace_filter.focus_handle(cx).is_focused(window) {
      Pane::Workspace
    } else {
      Pane::Recent
    }
  }

  fn recent_query(&self, cx: &App) -> String {
    self.recent_filter.read(cx).value().to_string()
  }

  fn workspace_query(&self, cx: &App) -> String {
    self.workspace_filter.read(cx).value().to_string()
  }

  fn current_rows(&self, cx: &App) -> Vec<WorkspaceRow> {
    let keyboard = self.highlight.pane == Some(Pane::Workspace);
    workspace_rows(
      &self.projects,
      &Self::workspaces(cx),
      &self.workspace_query(cx),
      &self.expanded,
      keyboard,
    )
  }

  fn move_highlight(&mut self, delta: isize, window: &Window, cx: &mut Context<Self>) {
    let pane = self.active_pane(window, cx);
    let len = match pane {
      Pane::Recent => recent_indices(&AppConfig::get(cx).recents, &self.recent_query(cx)).len(),
      Pane::Workspace => self.current_rows(cx).len(),
    };
    self.highlight = step(&self.highlight, pane, len, delta);
    cx.notify();
  }

  fn confirm_highlight(&mut self, cx: &mut Context<Self>) {
    match self.highlight.pane {
      Some(Pane::Recent) => {
        let recents = AppConfig::get(cx).recents.sorted();
        let indices = recent_indices(&AppConfig::get(cx).recents, &self.recent_query(cx));
        if let Some(project) = indices.get(self.highlight.index).and_then(|i| recents.get(*i)) {
          cx.emit(WelcomeEvent::Open(PathBuf::from(&project.path)));
        }
      }
      Some(Pane::Workspace) => {
        if let Some(row) = self.current_rows(cx).get(self.highlight.index).cloned() {
          match row {
            WorkspaceRow::Project { path, .. } => cx.emit(WelcomeEvent::Open(PathBuf::from(path))),
            WorkspaceRow::Folder { key, .. } => self.toggle_folder(key, cx),
          }
        }
      }
      None => {}
    }
  }

  fn toggle_folder(&mut self, key: String, cx: &mut Context<Self>) {
    if !self.expanded.remove(&key) {
      self.expanded.insert(key);
    }
    cx.notify();
  }

  fn remove_recent(&mut self, path: String, cx: &mut Context<Self>) {
    AppConfig::update(cx, move |config| config.recents.remove(&path));
    cx.notify();
  }

  fn filter_focused(&self, window: &Window, cx: &App) -> bool {
    self.recent_filter.focus_handle(cx).is_focused(window) || self.workspace_filter.focus_handle(cx).is_focused(window)
  }

  fn handle_list_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    let modifiers = &event.keystroke.modifiers;
    if modifiers.control || modifiers.platform || modifiers.alt {
      return;
    }
    let filter_focused = self.filter_focused(window, cx);
    let handled = match event.keystroke.key.as_str() {
      "up" if filter_focused => {
        self.move_highlight(-1, window, cx);
        true
      }
      "down" if filter_focused => {
        self.move_highlight(1, window, cx);
        true
      }
      "enter" if filter_focused => {
        self.confirm_highlight(cx);
        true
      }
      "left" | "right" if filter_focused => {
        if self.highlight.pane == Some(Pane::Workspace) {
          self.confirm_highlight(cx);
        }
        true
      }
      "space" if filter_focused && self.highlight.pane == Some(Pane::Workspace) => {
        self.confirm_highlight(cx);
        true
      }
      "escape" => {
        self.highlight = Highlight::default();
        window.blur(cx);
        cx.notify();
        true
      }
      _ => false,
    };
    if handled {
      cx.stop_propagation();
    }
  }

  fn render_recent_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let config = AppConfig::get(cx);
    let recents = config.recents.sorted();
    let query = self.recent_query(cx);
    let indices = recent_indices(&config.recents, &query);
    let highlighted = (self.highlight.pane == Some(Pane::Recent)).then_some(self.highlight.index);
    let palette = cx.global::<ActivePalette>().0;
    if indices.is_empty() {
      return div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(cx.theme().muted_foreground)
        .child(empty_recent_copy(!recents.is_empty()))
        .into_any_element();
    }
    let rows: Vec<AnyElement> = indices
      .iter()
      .enumerate()
      .filter_map(|(position, index)| recents.get(*index).cloned().map(|project| (position, project)))
      .map(|(position, project)| {
        let path = project.path.clone();
        let remove_path = project.path.clone();
        let is_highlighted = highlighted == Some(position);
        let group = format!("recent-{position}");
        div()
          .id(SharedString::from(format!("recent-row-{position}")))
          .group(group.clone())
          .flex()
          .items_center()
          .gap_2()
          .h(px(44.0))
          .px_2()
          .py(px(8.0))
          .rounded_sm()
          .cursor_pointer()
          .when(is_highlighted, |el| el.bg(hsla(palette.list_active)))
          .when(!is_highlighted, |el| el.hover(|el| el.bg(hsla(palette.list_hover))))
          .on_click(cx.listener(move |_, _, _, cx| cx.emit(WelcomeEvent::Open(PathBuf::from(&path)))))
          .child(
            svg()
              .path("icons/repo.svg")
              .size(px(16.0))
              .text_color(hsla(palette.muted_foreground)),
          )
          .child(
            div()
              .flex_1()
              .min_w_0()
              .flex()
              .flex_col()
              .child(div().text_size(px(13.0)).child(project.name.clone()))
              .child(
                div()
                  .text_size(px(11.0))
                  .text_color(cx.theme().muted_foreground)
                  .truncate()
                  .child(project.path.clone()),
              ),
          )
          .child(
            Button::new(SharedString::from(format!("remove-{position}")))
              .ghost()
              .xsmall()
              .icon(Icon::empty().path("icons/close.svg"))
              .tooltip("Remove from recents")
              .invisible()
              .group_hover(group, |style| style.visible())
              .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.remove_recent(remove_path.clone(), cx);
              })),
          )
          .into_any_element()
      })
      .collect();
    div()
      .id("recent-list")
      .flex_1()
      .min_h_0()
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .px_1()
      .children(rows)
      .into_any_element()
  }

  fn render_workspace_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let workspaces = Self::workspaces(cx);
    let rows = self.current_rows(cx);
    let highlighted = (self.highlight.pane == Some(Pane::Workspace)).then_some(self.highlight.index);
    let palette = cx.global::<ActivePalette>().0;
    if rows.is_empty() {
      return div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(cx.theme().muted_foreground)
        .child(empty_workspace_copy(!workspaces.is_empty()))
        .into_any_element();
    }
    let elements: Vec<AnyElement> = rows
      .into_iter()
      .enumerate()
      .map(|(position, row)| {
        let is_highlighted = highlighted == Some(position);
        let base = div()
          .id(SharedString::from(format!("ws-row-{position}")))
          .flex()
          .items_center()
          .gap_2()
          .h(px(44.0))
          .px_2()
          .py(px(8.0))
          .rounded_sm()
          .cursor_pointer()
          .when(is_highlighted, |el| el.bg(hsla(palette.list_active)))
          .when(!is_highlighted, |el| el.hover(|el| el.bg(hsla(palette.list_hover))));
        match row {
          WorkspaceRow::Folder {
            key,
            name,
            depth,
            expanded,
          } => base
            .pl(px(12.0 + 16.0 * depth as f32))
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_folder(key.clone(), cx)))
            .child(
              svg()
                .path("icons/chevron-right.svg")
                .size(px(14.0))
                .text_color(hsla(palette.muted_foreground))
                .when(expanded, |el| {
                  el.with_transformation(Transformation::rotate(Radians(std::f32::consts::FRAC_PI_2)))
                }),
            )
            .child(
              svg()
                .path("icons/folder.svg")
                .size(px(16.0))
                .text_color(hsla(palette.muted_foreground)),
            )
            .child(div().text_size(px(13.0)).child(name))
            .into_any_element(),
          WorkspaceRow::Project { name, path, depth } => {
            let open_path = path.clone();
            base
              .pl(px(12.0 + 16.0 * depth as f32))
              .on_click(cx.listener(move |_, _, _, cx| cx.emit(WelcomeEvent::Open(PathBuf::from(&open_path)))))
              .child(
                svg()
                  .path("icons/repo.svg")
                  .size(px(16.0))
                  .text_color(hsla(palette.muted_foreground)),
              )
              .child(
                div()
                  .flex_1()
                  .min_w_0()
                  .flex()
                  .flex_col()
                  .child(div().text_size(px(13.0)).child(name))
                  .child(
                    div()
                      .text_size(px(11.0))
                      .text_color(cx.theme().muted_foreground)
                      .truncate()
                      .child(path),
                  ),
              )
              .into_any_element()
          }
        }
      })
      .collect();
    div()
      .id("workspace-list")
      .flex_1()
      .min_h_0()
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .px_1()
      .children(elements)
      .into_any_element()
  }

  fn render_header(&self, title: &'static str, cx: &App) -> impl IntoElement {
    div()
      .text_size(px(11.0))
      .font_weight(FontWeight::BOLD)
      .text_color(cx.theme().muted_foreground)
      .child(title.to_uppercase())
  }

  fn render_filter(&self, filter: &Entity<InputState>, cx: &App) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    Input::new(filter).small().h(px(26.0)).w_full().rounded_md().prefix(
      svg()
        .path("icons/search.svg")
        .size(px(14.0))
        .text_color(hsla(palette.muted_foreground)),
    )
  }

  fn render_list_box(&self, body: AnyElement, cx: &App) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    div()
      .w_full()
      .h(px(320.0))
      .flex_shrink_0()
      .flex()
      .flex_col()
      .bg(hsla(palette.sidebar))
      .border_1()
      .border_color(hsla(palette.border))
      .rounded_md()
      .child(body)
  }

  fn render_column(
    &self,
    title: &'static str,
    filter: &Entity<InputState>,
    body: AnyElement,
    footer: Option<AnyElement>,
    cx: &App,
  ) -> impl IntoElement {
    div()
      .flex_1()
      .min_w_0()
      .flex()
      .flex_col()
      .gap_2()
      .child(self.render_header(title, cx))
      .child(self.render_filter(filter, cx))
      .child(self.render_list_box(body, cx))
      .children(footer)
  }
}

impl Render for WelcomeView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let version = format!("Version {} ({})", env!("CARGO_PKG_VERSION"), env!("DEATHPUSH_GIT_HASH"));
    let recent_body = self.render_recent_list(cx).into_any_element();
    let workspace_body = self.render_workspace_list(cx).into_any_element();
    let configure = div()
      .flex()
      .child(
        Button::new("configure-workspace")
          .outline()
          .small()
          .label("Configure Workspace...")
          .on_click(cx.listener(|_, _, _, cx| cx.emit(WelcomeEvent::ConfigureWorkspace))),
      )
      .into_any_element();
    div()
      .size_full()
      .flex()
      .flex_col()
      .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.handle_list_key(event, window, cx)))
      .on_action(cx.listener(|this, _: &FocusRecentFilter, window, cx| this.focus_recent_filter(window, cx)))
      .on_action(cx.listener(|this, _: &FocusWorkspaceFilter, window, cx| this.focus_workspace_filter(window, cx)))
      .child(
        div()
          .flex_1()
          .flex()
          .flex_col()
          .items_center()
          .justify_center()
          .gap_0()
          .child(
            svg()
              .path("brand/deathpush.svg")
              .size(px(80.0))
              .text_color(cx.theme().foreground)
              .opacity(0.6)
              .mb(px(16.0)),
          )
          .child(
            div()
              .text_size(px(20.0))
              .font_weight(FontWeight::SEMIBOLD)
              .mb(px(20.0))
              .child("DeathPush"),
          )
          .child(
            div()
              .flex()
              .gap_2()
              .mb(px(24.0))
              .child(
                Button::new("open-repository")
                  .outline()
                  .icon(Icon::empty().path("icons/folder.svg"))
                  .label("Open Repository")
                  .on_click(cx.listener(|_, _, window, cx| window.dispatch_action(Box::new(OpenRepository), cx))),
              )
              .child(
                Button::new("clone-repository")
                  .outline()
                  .icon(Icon::empty().path("icons/cloud-download.svg"))
                  .label("Clone Repository")
                  .on_click(cx.listener(|_, _, _, cx| cx.emit(WelcomeEvent::Clone))),
              ),
          )
          .child(
            div()
              .flex()
              .gap(px(12.0))
              .w(px(760.0))
              .max_w_full()
              .px_4()
              .child(self.render_column("Recent", &self.recent_filter, recent_body, None, cx))
              .child(self.render_column("Workspace", &self.workspace_filter, workspace_body, Some(configure), cx)),
          ),
      )
      .child(
        div()
          .flex()
          .justify_center()
          .py_2()
          .text_size(px(11.0))
          .text_color(cx.theme().muted_foreground)
          .child(version),
      )
  }
}
