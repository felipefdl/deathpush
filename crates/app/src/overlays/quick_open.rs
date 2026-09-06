use std::collections::HashSet;
use std::time::Duration;

use deathpush_core::config::layout::{MainView, SidebarView};
use deathpush_core::config::recent_files::load_recent_files;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::{ContentSearchResult, FuzzyFileResult};
use gpui_kit::component::IndexPath;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::frame::backdrop;
use crate::actions::Cancel;
use crate::config::AppConfig;
use crate::keymap::CONTEXT_DIALOG;
use crate::repo::RepoView;
use crate::theme::{ActivePalette, hsla};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
  Files { text: String, line: Option<usize> },
  GoToLine(usize),
  Content(String),
  ContentEmpty,
}

pub fn parse_query(input: &str) -> Query {
  if let Some(rest) = input.strip_prefix('#') {
    if rest.is_empty() {
      Query::ContentEmpty
    } else {
      Query::Content(rest.to_string())
    }
  } else if let Some(rest) = input.strip_prefix(':') {
    Query::GoToLine(rest.parse().unwrap_or(0))
  } else if let Some((text, suffix)) = input.rsplit_once(':')
    && !suffix.is_empty()
    && suffix.chars().all(|ch| ch.is_ascii_digit())
  {
    Query::Files {
      text: text.to_string(),
      line: suffix.parse().ok(),
    }
  } else {
    Query::Files {
      text: input.to_string(),
      line: None,
    }
  }
}

pub const FILE_DEBOUNCE_MS: u64 = 100;
pub const CONTENT_DEBOUNCE_MS: u64 = 300;
pub const MAX_RESULTS: usize = 100;
pub const PLACEHOLDER: &str = "Search files by name (append : to go to line, # to search content)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
  File {
    path: String,
    matches: Vec<usize>,
    line: Option<usize>,
    recent: bool,
  },
  Content {
    path: String,
    line: usize,
    snippet: String,
  },
}

pub fn file_rows(
  results: &[FuzzyFileResult],
  recents: &[&str],
  line: Option<usize>,
  empty_query: bool,
) -> (Vec<RowKind>, Vec<RowKind>) {
  let to_file = |result: &FuzzyFileResult, recent: bool| RowKind::File {
    path: result.path.clone(),
    matches: result.match_positions.clone(),
    line,
    recent,
  };
  if empty_query {
    let recent_rows: Vec<RowKind> = recents
      .iter()
      .filter_map(|path| {
        results
          .iter()
          .find(|result| result.path == *path)
          .map(|result| to_file(result, true))
      })
      .collect();
    if !recent_rows.is_empty() {
      let recent_set: HashSet<&str> = recents.iter().copied().collect();
      let files = results
        .iter()
        .filter(|result| !recent_set.contains(result.path.as_str()))
        .map(|result| to_file(result, false))
        .collect();
      return (recent_rows, files);
    }
  }
  (
    Vec::new(),
    results.iter().map(|result| to_file(result, false)).collect(),
  )
}

pub fn empty_message(query: &Query, loading: bool) -> Option<String> {
  match query {
    Query::Files { .. } => Some("No matching files".into()),
    Query::ContentEmpty => Some("Type to search file contents".into()),
    Query::Content(_) if loading => Some("Searching...".into()),
    Query::Content(_) => Some("No results".into()),
    Query::GoToLine(0) => Some("Type a line number to go to.".into()),
    Query::GoToLine(n) => Some(format!("Go to line {n} in current file. Press Enter to confirm.")),
  }
}

pub enum QuickOpenEvent {
  Close,
}

pub struct QuickOpen {
  repo: Entity<RepoView>,
  state: Entity<CommandState>,
  query: Query,
  recent: Vec<RowKind>,
  items: Vec<RowKind>,
  recents: Vec<String>,
  loading: bool,
  debounce_generation: u64,
}

impl EventEmitter<QuickOpenEvent> for QuickOpen {}

impl QuickOpen {
  pub fn new(repo: Entity<RepoView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let state = cx.new(|cx| CommandState::new(window, cx));
    state.update(cx, |state, cx| state.focus(window, cx));
    let (recents, handle) = {
      let model = repo.read(cx).model().clone();
      let recents = model
        .read(cx)
        .state()
        .root()
        .map(|root| {
          load_recent_files(AppConfig::get(cx).dir(), root)
            .paths()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();
      let handle = model.read(cx).fuzzy_find_files(String::new(), MAX_RESULTS);
      (recents, handle)
    };
    cx.spawn(async move |this, cx| {
      let result = handle.await;
      let _ = this.update(cx, |this, cx| this.apply_files(1, result, cx));
    })
    .detach();
    Self {
      repo,
      state,
      query: Query::Files {
        text: String::new(),
        line: None,
      },
      recent: Vec::new(),
      items: Vec::new(),
      recents,
      loading: true,
      debounce_generation: 1,
    }
  }

  pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
    self.state.update(cx, |state, cx| state.focus(window, cx));
  }

  fn on_query_change(&mut self, input: &str, cx: &mut Context<Self>) {
    let query = parse_query(input);
    let same_kind = std::mem::discriminant(&self.query) == std::mem::discriminant(&query);
    self.query = query.clone();
    self.debounce_generation += 1;
    let generation = self.debounce_generation;
    if !same_kind {
      self.recent.clear();
      self.items.clear();
    }
    match query {
      Query::ContentEmpty | Query::GoToLine(_) => {
        self.recent.clear();
        self.items.clear();
        self.loading = false;
        cx.notify();
      }
      Query::Files { text, .. } => self.schedule_files(generation, text, cx),
      Query::Content(text) => self.schedule_content(generation, text, cx),
    }
  }

  fn schedule_files(&mut self, generation: u64, text: String, cx: &mut Context<Self>) {
    self.loading = true;
    cx.notify();
    cx.spawn(async move |this, cx| {
      cx.background_executor()
        .timer(Duration::from_millis(FILE_DEBOUNCE_MS))
        .await;
      let handle = this
        .update(cx, |this, cx| {
          (this.debounce_generation == generation)
            .then(|| this.repo.read(cx).model().read(cx).fuzzy_find_files(text, MAX_RESULTS))
        })
        .ok()
        .flatten();
      let Some(handle) = handle else {
        return;
      };
      let result = handle.await;
      let _ = this.update(cx, |this, cx| this.apply_files(generation, result, cx));
    })
    .detach();
  }

  fn schedule_content(&mut self, generation: u64, text: String, cx: &mut Context<Self>) {
    self.loading = true;
    cx.notify();
    cx.spawn(async move |this, cx| {
      cx.background_executor()
        .timer(Duration::from_millis(CONTENT_DEBOUNCE_MS))
        .await;
      let handle = this
        .update(cx, |this, cx| {
          (this.debounce_generation == generation).then(|| {
            this
              .repo
              .read(cx)
              .model()
              .read(cx)
              .search_file_contents(text, MAX_RESULTS)
          })
        })
        .ok()
        .flatten();
      let Some(handle) = handle else {
        return;
      };
      let result = handle.await;
      let _ = this.update(cx, |this, cx| this.apply_content(generation, result, cx));
    })
    .detach();
  }

  fn apply_files(
    &mut self,
    generation: u64,
    result: Result<deathpush_core::Result<Vec<FuzzyFileResult>>, tokio::task::JoinError>,
    cx: &mut Context<Self>,
  ) {
    if self.debounce_generation != generation || !matches!(self.query, Query::Files { .. }) {
      return;
    }
    self.loading = false;
    match result {
      Ok(Ok(results)) => {
        let recents: Vec<&str> = self.recents.iter().map(String::as_str).collect();
        let (empty_query, line) = match &self.query {
          Query::Files { text, line } => (text.is_empty(), *line),
          _ => (false, None),
        };
        let (recent, items) = file_rows(&results, &recents, line, empty_query);
        self.recent = recent;
        self.items = items;
      }
      Ok(Err(_)) | Err(_) => {
        self.recent.clear();
        self.items.clear();
      }
    }
    cx.notify();
  }

  fn apply_content(
    &mut self,
    generation: u64,
    result: Result<deathpush_core::Result<Vec<ContentSearchResult>>, tokio::task::JoinError>,
    cx: &mut Context<Self>,
  ) {
    if self.debounce_generation != generation || !matches!(self.query, Query::Content(_)) {
      return;
    }
    self.loading = false;
    match result {
      Ok(Ok(results)) => {
        self.recent.clear();
        self.items = results
          .into_iter()
          .map(|hit| RowKind::Content {
            path: hit.path,
            line: hit.line_number,
            snippet: hit.line_content.trim().to_string(),
          })
          .collect();
      }
      Ok(Err(_)) | Err(_) => {
        self.recent.clear();
        self.items.clear();
      }
    }
    cx.notify();
  }

  fn confirm(&mut self, index: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
    if let Query::GoToLine(n) = self.query {
      self.confirm_go_to_line(n, window, cx);
      return;
    }
    let row = self.row_at(index).cloned();
    if let Some(row) = row {
      self.open_row(&row, window, cx);
    }
  }

  fn row_at(&self, index: IndexPath) -> Option<&RowKind> {
    if self.recent.is_empty() {
      self.items.get(index.row)
    } else {
      match index.section {
        0 => self.recent.get(index.row),
        1 => self.items.get(index.row),
        _ => None,
      }
    }
  }

  fn open_row(&mut self, row: &RowKind, window: &mut Window, cx: &mut Context<Self>) {
    match row {
      RowKind::File { path, line, .. } => self.open_path(path.clone(), *line, window, cx),
      RowKind::Content { path, line, .. } => self.open_path(path.clone(), Some(*line), window, cx),
    }
  }

  fn open_path(&mut self, path: String, line: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
    let explorer = self.repo.read(cx).explorer().clone();
    let layout = self.repo.read(cx).layout().clone();
    explorer.update(cx, |explorer, cx| explorer.open_file(&path, line, window, cx));
    layout.update(cx, |layout, cx| layout.select_sidebar_view(SidebarView::Explorer, cx));
    cx.emit(QuickOpenEvent::Close);
  }

  fn confirm_go_to_line(&mut self, n: usize, window: &mut Window, cx: &mut Context<Self>) {
    let _ = window;
    if n == 0 {
      return;
    }
    let model = self.repo.read(cx).model().clone();
    let layout = self.repo.read(cx).layout().clone();
    let path = model.read(cx).state().open_file.as_ref().map(|open| open.path.clone());
    if let Some(path) = path {
      model.update(cx, |model, cx| model.open_file(&path, Some(n), cx));
      layout.update(cx, |layout, cx| {
        layout.dock_terminal(cx);
        layout.select_main_view(MainView::File, cx);
      });
    }
    cx.emit(QuickOpenEvent::Close);
  }

  fn close(&mut self, cx: &mut Context<Self>) {
    cx.emit(QuickOpenEvent::Close);
  }
}

impl Render for QuickOpen {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let this = cx.entity().downgrade();
    let query = self.query.clone();
    let loading = self.loading;
    let grouped = !self.recent.is_empty();
    let recent_items: Vec<CommandItem> = self.recent.iter().map(|row| command_item(row, palette)).collect();
    let file_items: Vec<CommandItem> = self.items.iter().map(|row| command_item(row, palette)).collect();
    let on_query = this.clone();
    let on_confirm = this.clone();
    let on_cancel = this;
    let empty_query = query.clone();
    let mut command = Command::new(&self.state)
      .filterable(false)
      .placeholder(PLACEHOLDER)
      .max_h(px(440.))
      .bordered(false)
      .w_full()
      .bg(hsla(palette.sidebar))
      .text_size(px(13.))
      .on_query(move |query, _, cx| {
        let query = query.to_string();
        let _ = on_query.update(cx, |this, cx| this.on_query_change(&query, cx));
      })
      .on_confirm(move |index, window, cx| {
        let _ = on_confirm.update(cx, |this, cx| this.confirm(index, window, cx));
      })
      .on_cancel(move |_, cx| {
        let _ = on_cancel.update(cx, |this, cx| this.close(cx));
      })
      .empty(move |_, _, _| empty_element(&empty_query, loading, palette))
      .header(move |_, _, _| loading_bar(loading, palette));
    command = if grouped {
      command
        .group(CommandGroup::new().label("recently opened").items(recent_items))
        .group(CommandGroup::new().label("files").items(file_items))
    } else if let Query::GoToLine(n) = self.query
      && n > 0
    {
      command.item(CommandItem::new().child(move |_, _| go_to_line_element(n, palette)))
    } else {
      command.items(file_items)
    };
    backdrop("quick-open-backdrop", |_, _| {}, cx)
      .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
      .child(
        div()
          .key_context(CONTEXT_DIALOG)
          .occlude()
          .mt(px(60.))
          .w(px(600.))
          .overflow_hidden()
          .bg(hsla(palette.sidebar))
          .border_1()
          .border_color(hsla(palette.border))
          .rounded_lg()
          .shadow_lg()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_action(cx.listener(|this, _: &Cancel, _, cx| this.close(cx)))
          .child(command),
      )
  }
}

fn command_item(row: &RowKind, palette: UiPalette) -> CommandItem {
  let row = row.clone();
  CommandItem::new().child(move |_, _| render_row(&row, palette))
}

fn render_row(row: &RowKind, palette: UiPalette) -> AnyElement {
  match row {
    RowKind::File {
      path, matches, line, ..
    } => file_row(path, matches, *line, palette),
    RowKind::Content { path, line, snippet } => content_row(path, *line, snippet, palette),
  }
}

fn file_row(path: &str, matches: &[usize], line: Option<usize>, palette: UiPalette) -> AnyElement {
  let (name, directory) = split_path(path);
  let highlight = hsla(palette.list_active_foreground);
  let muted = hsla(palette.muted_foreground);
  div()
    .h(px(26.))
    .w_full()
    .flex()
    .items_center()
    .gap_1()
    .text_size(px(13.))
    .child(file_icon(muted))
    .child(highlighted_filename(path, name, matches, highlight))
    .when_some(line, |el, line| el.child(format!(":{line}")))
    .when_some(directory.map(str::to_string), |el, directory| {
      el.child(
        div()
          .min_w_0()
          .flex_1()
          .overflow_hidden()
          .text_ellipsis()
          .text_size(px(11.))
          .text_color(muted)
          .child(directory),
      )
    })
    .into_any_element()
}

fn content_row(path: &str, line: usize, snippet: &str, palette: UiPalette) -> AnyElement {
  let (name, directory) = split_path(path);
  let muted = hsla(palette.muted_foreground);
  div()
    .h(px(26.))
    .w_full()
    .flex()
    .items_center()
    .gap_1()
    .text_size(px(13.))
    .child(file_icon(muted))
    .child(format!("{name}:{line}"))
    .when_some(directory.map(str::to_string), |el, directory| {
      el.child(
        div()
          .min_w_0()
          .overflow_hidden()
          .text_ellipsis()
          .text_size(px(11.))
          .text_color(muted)
          .child(directory),
      )
    })
    .child(
      div()
        .min_w_0()
        .flex_1()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(12.))
        .text_color(muted)
        .child(snippet.to_string()),
    )
    .into_any_element()
}

fn file_icon(color: Hsla) -> impl IntoElement {
  svg()
    .path("icons/file.svg")
    .size(px(16.))
    .flex_shrink_0()
    .text_color(color)
}

fn split_path(path: &str) -> (&str, Option<&str>) {
  match path.rsplit_once('/') {
    Some((directory, name)) => (name, Some(directory)),
    None => (path, None),
  }
}

fn highlighted_filename(path: &str, name: &str, matches: &[usize], highlight: Hsla) -> AnyElement {
  let path_chars: Vec<char> = path.chars().collect();
  let name_chars: Vec<char> = name.chars().collect();
  let name_start = path_chars.len().saturating_sub(name_chars.len());
  let matched: HashSet<usize> = matches.iter().copied().collect();
  let mut runs: Vec<(String, bool)> = Vec::new();
  for (index, ch) in name_chars.into_iter().enumerate() {
    let hit = matched.contains(&(name_start + index));
    match runs.last_mut() {
      Some((text, flag)) if *flag == hit => text.push(ch),
      _ => runs.push((ch.to_string(), hit)),
    }
  }
  div()
    .flex()
    .flex_row()
    .flex_shrink_0()
    .children(
      runs
        .into_iter()
        .map(|(text, hit)| div().when(hit, |el| el.text_color(highlight)).child(text)),
    )
    .into_any_element()
}

fn empty_element(query: &Query, loading: bool, palette: UiPalette) -> AnyElement {
  let muted = hsla(palette.muted_foreground);
  match query {
    Query::GoToLine(n) if *n > 0 => go_to_line_element(*n, palette),
    _ => div()
      .w_full()
      .py_6()
      .text_center()
      .text_sm()
      .text_color(muted)
      .child(empty_message(query, loading).unwrap_or_default())
      .into_any_element(),
  }
}

fn go_to_line_element(n: usize, palette: UiPalette) -> AnyElement {
  div()
    .w_full()
    .py_6()
    .flex()
    .justify_center()
    .text_sm()
    .text_color(hsla(palette.muted_foreground))
    .child(
      div()
        .flex()
        .flex_row()
        .child("Go to line ")
        .child(div().font_weight(FontWeight::BOLD).child(n.to_string()))
        .child(" in current file. Press Enter to confirm."),
    )
    .into_any_element()
}

fn loading_bar(loading: bool, palette: UiPalette) -> impl IntoElement {
  div()
    .h(px(2.))
    .w_full()
    .relative()
    .overflow_hidden()
    .when(loading, |el| {
      el.child(
        div()
          .absolute()
          .h_full()
          .w(px(120.))
          .bg(hsla(palette.primary))
          .with_animation(
            "quick-open-loading",
            Animation::new(Duration::from_millis(1500)).repeat(),
            |this, delta| this.left(px(-120.0 + delta * 720.0)),
          ),
      )
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn parse_query_modes() {
    assert_eq!(
      parse_query(""),
      Query::Files {
        text: "".into(),
        line: None
      }
    );
    assert_eq!(
      parse_query("main:12"),
      Query::Files {
        text: "main".into(),
        line: Some(12)
      }
    );
    assert_eq!(parse_query(":12"), Query::GoToLine(12));
    assert_eq!(parse_query(":"), Query::GoToLine(0));
    assert_eq!(parse_query("#"), Query::ContentEmpty);
    assert_eq!(parse_query("#foo bar"), Query::Content("foo bar".into()));
  }

  #[test]
  fn empty_messages_follow_the_spec() {
    assert_eq!(
      empty_message(
        &Query::Files {
          text: "x".into(),
          line: None
        },
        false
      )
      .as_deref(),
      Some("No matching files")
    );
    assert_eq!(
      empty_message(
        &Query::Files {
          text: "x".into(),
          line: None
        },
        true
      )
      .as_deref(),
      Some("No matching files")
    );
    assert_eq!(
      empty_message(&Query::ContentEmpty, false).as_deref(),
      Some("Type to search file contents")
    );
    assert_eq!(
      empty_message(&Query::Content("a".into()), true).as_deref(),
      Some("Searching...")
    );
    assert_eq!(
      empty_message(&Query::Content("a".into()), false).as_deref(),
      Some("No results")
    );
    assert_eq!(
      empty_message(&Query::GoToLine(12), false).as_deref(),
      Some("Go to line 12 in current file. Press Enter to confirm.")
    );
    assert_eq!(
      empty_message(&Query::GoToLine(0), false).as_deref(),
      Some("Type a line number to go to.")
    );
  }

  #[test]
  fn file_rows_group_recents_only_for_the_empty_query() {
    let results = vec![
      FuzzyFileResult {
        path: "a.rs".into(),
        score: 1,
        match_positions: vec![],
      },
      FuzzyFileResult {
        path: "b.rs".into(),
        score: 1,
        match_positions: vec![],
      },
    ];
    let (recent, files) = file_rows(&results, &["b.rs"], None, true);
    assert_eq!(recent.len(), 1);
    assert_eq!(files.len(), 1);
    let (recent, files) = file_rows(&results, &["b.rs"], Some(3), false);
    assert!(recent.is_empty() && files.len() == 2);
    assert!(matches!(&files[0], RowKind::File { line: Some(3), .. }));
  }
}
