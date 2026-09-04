use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use deathpush_core::ops::explorer::next_entry_name;
use deathpush_core::session::types::Intent;
use deathpush_core::types::{
  ExplorerEntry, FileStatus, PathChangeKind, PathsChanged, RepositoryStatus, ResourceGroupKind,
};
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

use super::super::model::RepoModel;

const REFRESH_COALESCE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
  pub path: String,
  pub name: String,
  pub is_directory: bool,
  pub is_symlink: bool,
  pub ignored: bool,
  pub children: Option<Vec<Node>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ClipboardOp {
  Cut,
  Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardMark {
  pub op: ClipboardOp,
  pub path: String,
  pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditState {
  Creating {
    parent: String,
    is_directory: bool,
    name: String,
  },
  Renaming {
    path: String,
    name: String,
  },
}

#[allow(dead_code)]
pub enum ExplorerEvent {
  Changed,
  Error(String),
  OpenFile { path: String, line: Option<usize> },
  Toast(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
  pub path: String,
  pub name: String,
  pub depth: usize,
  pub is_directory: bool,
  pub expanded: bool,
  pub ignored: bool,
  pub status: Option<FileStatus>,
  pub selected: bool,
}

pub struct ExplorerModel {
  core: Arc<Core>,
  session: SessionId,
  #[allow(dead_code)]
  root: String,
  pub roots: Vec<Node>,
  pub expanded: HashSet<String>,
  pub filter: String,
  pub selected: Vec<String>,
  pub anchor: Option<String>,
  pub clipboard: Option<ClipboardMark>,
  pub edit: Option<EditState>,
  refresh_pending: bool,
  last_refresh: Option<Instant>,
  refresh_generation: u64,
  select_after_load: Option<(String, bool)>,
}

impl EventEmitter<ExplorerEvent> for ExplorerModel {}

#[allow(dead_code)]
impl ExplorerModel {
  pub fn new(core: Arc<Core>, session: SessionId, root: String) -> Self {
    Self {
      core,
      session,
      root,
      roots: Vec::new(),
      expanded: HashSet::new(),
      filter: String::new(),
      selected: Vec::new(),
      anchor: None,
      clipboard: None,
      edit: None,
      refresh_pending: false,
      last_refresh: None,
      refresh_generation: 0,
      select_after_load: None,
    }
  }

  pub fn load(&mut self, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let session = self.session;
    let task = core
      .clone()
      .spawn(async move { core.list_repository_tree(session).await });
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let _ = this.update(cx, |this, cx| match result {
        Ok(Ok(entries)) => this.apply_tree(entries, cx),
        Ok(Err(err)) => this.fail(err.to_string(), cx),
        Err(err) => this.fail(err.to_string(), cx),
      });
    })
    .detach();
  }

  pub fn expand(&mut self, path: &str, cx: &mut Context<Self>) {
    self.expanded.insert(path.to_string());
    let needs_load = find_node(&self.roots, path).is_some_and(|node| node.is_directory && node.children.is_none());
    if needs_load {
      self.load_children(path, cx);
    }
    self.emit_changed(cx);
  }

  pub fn collapse(&mut self, path: &str, cx: &mut Context<Self>) {
    self.expanded.remove(path);
    self.emit_changed(cx);
  }

  pub fn set_filter(&mut self, filter: String, cx: &mut Context<Self>) {
    self.filter = filter;
    self.emit_changed(cx);
  }

  pub fn select(&mut self, path: &str, extend: bool, range: bool, cx: &mut Context<Self>) {
    if range {
      let rows = flatten(&self.roots, &self.expanded, &self.filter, None, &[]);
      let anchor = self.anchor.clone().unwrap_or_else(|| path.to_string());
      let start = rows.iter().position(|row| row.path == anchor);
      let end = rows.iter().position(|row| row.path == path);
      if let (Some(start), Some(end)) = (start, end) {
        let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
        self.selected = rows[lo..=hi].iter().map(|row| row.path.clone()).collect();
      } else {
        self.selected = vec![path.to_string()];
      }
    } else if extend {
      if let Some(index) = self.selected.iter().position(|item| item == path) {
        self.selected.remove(index);
      } else {
        self.selected.push(path.to_string());
      }
      self.anchor = Some(path.to_string());
    } else {
      self.selected = vec![path.to_string()];
      self.anchor = Some(path.to_string());
    }
    self.emit_changed(cx);
  }

  pub fn visible_rows(&self, status: Option<&RepositoryStatus>) -> Vec<Row> {
    flatten(&self.roots, &self.expanded, &self.filter, status, &self.selected)
  }

  pub fn mark(&mut self, op: ClipboardOp, cx: &mut Context<Self>) {
    let Some(path) = self.anchor.clone() else {
      return;
    };
    let Some(node) = find_node(&self.roots, &path) else {
      return;
    };
    self.clipboard = Some(ClipboardMark {
      op,
      path: node.path.clone(),
      is_directory: node.is_directory,
    });
    self.emit_changed(cx);
  }

  pub fn paste_target(&self) -> Option<String> {
    let Some(path) = self.selected.last() else {
      return Some(String::new());
    };
    if find_node(&self.roots, path).is_some_and(|node| node.is_directory) {
      Some(path.clone())
    } else {
      Some(parent_path(path))
    }
  }

  pub fn begin_create(&mut self, parent: &str, is_directory: bool, cx: &mut Context<Self>) {
    let existing = child_names(&self.roots, parent);
    let base = if is_directory { "New Folder" } else { "New File" };
    let name = next_entry_name(&existing, base);
    self.edit = Some(EditState::Creating {
      parent: parent.to_string(),
      is_directory,
      name,
    });
    self.emit_changed(cx);
  }

  pub fn begin_rename(&mut self, path: &str, cx: &mut Context<Self>) {
    let Some(node) = find_node(&self.roots, path) else {
      return;
    };
    self.edit = Some(EditState::Renaming {
      path: node.path.clone(),
      name: node.name.clone(),
    });
    self.emit_changed(cx);
  }

  pub fn commit_edit(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    let _ = window;
    let Some(edit) = self.edit.clone() else {
      return;
    };
    if name.is_empty() {
      self.cancel_edit(cx);
      return;
    }
    match edit {
      EditState::Creating {
        parent, is_directory, ..
      } => {
        let path = join_repo_path(&parent, &name);
        self.select_after_load = Some((path.clone(), is_directory));
        let core = self.core.clone();
        let session = self.session;
        let handle = core.runtime_handle().clone();
        let task = handle.spawn_blocking(move || {
          if is_directory {
            core.create_directory(session, &path)
          } else {
            core.write_file(session, &path, "").map(|_| ())
          }
        });
        self.await_unit(task, Some(name), cx);
      }
      EditState::Renaming { path, .. } => {
        let is_file = find_node(&self.roots, &path).is_some_and(|node| !node.is_directory);
        let new_path = join_repo_path(&parent_path(&path), &name);
        let core = self.core.clone();
        let session = self.session;
        let handle = core.runtime_handle().clone();
        let old_path = path.clone();
        let new_name = name.clone();
        let task = handle.spawn_blocking(move || core.rename_entry(session, &old_path, &new_name));
        cx.spawn(async move |this, cx| {
          let result = join_unit(task.await);
          let _ = this.update(cx, |this, cx| match result {
            Ok(()) => {
              this.edit = None;
              if is_file {
                cx.emit(ExplorerEvent::OpenFile {
                  path: new_path,
                  line: None,
                });
              }
              this.reload_tree(cx);
            }
            Err(message) => this.emit_core_error(message, &name, cx),
          });
        })
        .detach();
      }
    }
  }

  pub fn cancel_edit(&mut self, cx: &mut Context<Self>) {
    self.edit = None;
    self.emit_changed(cx);
  }

  pub fn on_paths_changed(&mut self, event: &PathsChanged, cx: &mut Context<Self>) {
    if !should_reload(event.kind.clone()) {
      return;
    }
    let already_pending = self.refresh_pending;
    self.refresh_pending = true;
    if self.last_refresh.is_none_or(|at| at.elapsed() >= REFRESH_COALESCE) {
      self.reload_tree(cx);
      return;
    }
    if already_pending {
      return;
    }
    let wait = REFRESH_COALESCE.saturating_sub(self.last_refresh.map(|at| at.elapsed()).unwrap_or(Duration::ZERO));
    self.schedule_reload(wait.as_millis().max(1) as u64, cx);
  }

  pub fn duplicate(&mut self, path: &str, cx: &mut Context<Self>) {
    let path = path.to_string();
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let task = handle.spawn_blocking(move || core.duplicate_entry(session, &path).map(|_| ()));
    self.await_unit(task, None, cx);
  }

  pub fn paste(
    &mut self,
    into: &str,
    on_conflict: Option<&'static str>,
    window: &mut Window,
    cx: &mut Context<Self>,
    done: impl FnOnce(Result<(), String>) + 'static,
  ) {
    let _ = window;
    let Some(mark) = self.clipboard.clone() else {
      done(Ok(()));
      return;
    };
    let core = self.core.clone();
    let session = self.session;
    let dest = into.to_string();
    let sources = vec![mark.path.clone()];
    let cut = mark.op == ClipboardOp::Cut;
    let handle = core.runtime_handle().clone();
    let task = handle.spawn_blocking(move || {
      if cut {
        core.move_entries(session, &sources, &dest, on_conflict)
      } else {
        core.copy_entries(session, &sources, &dest, on_conflict)
      }
    });
    cx.spawn(async move |this, cx| {
      let result = join_unit(task.await);
      let success = result.is_ok();
      let _ = this.update(cx, |this, cx| {
        if success && cut {
          this.clipboard = None;
        }
        if success {
          this.reload_tree(cx);
        }
      });
      done(result);
    })
    .detach();
  }

  pub fn import(
    &mut self,
    sources: Vec<String>,
    on_conflict: Option<&'static str>,
    window: &mut Window,
    cx: &mut Context<Self>,
    done: impl FnOnce(Result<(), String>) + 'static,
  ) {
    let _ = window;
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let task = handle.spawn_blocking(move || core.import_files(session, &sources, "", on_conflict));
    cx.spawn(async move |this, cx| {
      let result = join_unit(task.await);
      let success = result.is_ok();
      let _ = this.update(cx, |this, cx| {
        if success {
          this.reload_tree(cx);
        }
      });
      done(result);
    })
    .detach();
  }

  pub fn add_to_gitignore(
    &mut self,
    path: &str,
    repo: &Entity<RepoModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let path = path.to_string();
    repo.update(cx, |model, cx| {
      model.dispatch(Intent::AddToGitignore { path }, window, cx)
    });
  }

  pub fn delete(&mut self, path: &str, repo: &Entity<RepoModel>, window: &mut Window, cx: &mut Context<Self>) {
    let path = path.to_string();
    repo.update(cx, |model, cx| {
      model.dispatch(Intent::DeleteFile { path, confirmed: false }, window, cx);
    });
  }

  pub fn open_in_editor(&self, path: &str, cx: &mut Context<Self>) {
    let path = path.to_string();
    let core = self.core.clone();
    let session = self.session;
    let task = core
      .clone()
      .spawn(async move { core.open_in_editor(session, &path).await });
    self.await_async_unit(task, cx);
  }

  pub fn reveal(&self, path: &str, cx: &mut Context<Self>) {
    let path = path.to_string();
    let core = self.core.clone();
    let session = self.session;
    let task = core
      .clone()
      .spawn(async move { core.reveal_in_file_manager(session, &path).await });
    self.await_async_unit(task, cx);
  }

  fn reload_tree(&mut self, cx: &mut Context<Self>) {
    self.refresh_generation += 1;
    self.refresh_pending = false;
    self.last_refresh = Some(Instant::now());
    self.load(cx);
  }

  fn schedule_reload(&mut self, ms: u64, cx: &mut Context<Self>) {
    self.refresh_generation += 1;
    let token = self.refresh_generation;
    cx.spawn(async move |this, cx| {
      cx.background_executor().timer(Duration::from_millis(ms)).await;
      let _ = this.update(cx, |this, cx| {
        if this.refresh_generation != token {
          return;
        }
        if this.refresh_pending {
          this.reload_tree(cx);
        }
      });
    })
    .detach();
  }

  fn apply_tree(&mut self, entries: Vec<ExplorerEntry>, cx: &mut Context<Self>) {
    self.roots = build_tree(&entries);
    self.expanded.retain(|path| find_node(&self.roots, path).is_some());
    if let Some((path, is_directory)) = self.select_after_load.take() {
      if find_node(&self.roots, &path).is_none() {
        ensure_entry(&mut self.roots, &path, is_directory);
      }
      if find_node(&self.roots, &path).is_some() {
        self.selected = vec![path.clone()];
        self.anchor = Some(path);
      }
    }
    let to_load: Vec<String> = self
      .expanded
      .iter()
      .filter(|path| find_node(&self.roots, path).is_some_and(|node| node.is_directory && node.children.is_none()))
      .cloned()
      .collect();
    for path in to_load {
      self.load_children(&path, cx);
    }
    self.emit_changed(cx);
  }

  fn load_children(&mut self, path: &str, cx: &mut Context<Self>) {
    let path = path.to_string();
    let core = self.core.clone();
    let session = self.session;
    let handle = core.runtime_handle().clone();
    let listed = path.clone();
    let task = handle.spawn_blocking(move || core.list_repository_children(session, &listed));
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let _ = this.update(cx, |this, cx| match result {
        Ok(Ok(entries)) => {
          let children = children_from_listing(&path, &entries);
          if let Some(node) = find_node_mut(&mut this.roots, &path) {
            node.children = Some(children);
          }
          this.emit_changed(cx);
        }
        Ok(Err(err)) => this.fail(err.to_string(), cx),
        Err(err) => this.fail(err.to_string(), cx),
      });
    })
    .detach();
  }

  fn await_unit(
    &mut self,
    task: tokio::task::JoinHandle<deathpush_core::Result<()>>,
    exists_name: Option<String>,
    cx: &mut Context<Self>,
  ) {
    cx.spawn(async move |this, cx| {
      let result = join_unit(task.await);
      let _ = this.update(cx, |this, cx| match result {
        Ok(()) => {
          this.edit = None;
          this.reload_tree(cx);
        }
        Err(message) => {
          if let Some(name) = exists_name {
            this.emit_core_error(message, &name, cx);
          } else {
            this.fail(message, cx);
          }
        }
      });
    })
    .detach();
  }

  fn await_async_unit(&self, task: tokio::task::JoinHandle<deathpush_core::Result<()>>, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
      let result = join_unit(task.await);
      if let Err(message) = result {
        let _ = this.update(cx, |_, cx| {
          cx.emit(ExplorerEvent::Error(message));
        });
      }
    })
    .detach();
  }

  fn emit_core_error(&mut self, message: String, name: &str, cx: &mut Context<Self>) {
    if message.contains("already exists") {
      cx.emit(ExplorerEvent::Toast(format!("\"{name}\" already exists")));
    } else {
      cx.emit(ExplorerEvent::Error(message));
    }
    cx.notify();
  }

  fn fail(&mut self, message: String, cx: &mut Context<Self>) {
    cx.emit(ExplorerEvent::Error(message));
    cx.notify();
  }

  fn emit_changed(&mut self, cx: &mut Context<Self>) {
    cx.emit(ExplorerEvent::Changed);
    cx.notify();
  }
}

/// Pure: flat entries into a nested forest, folders first, case-insensitive order.
pub fn build_tree(entries: &[ExplorerEntry]) -> Vec<Node> {
  let mut roots = Vec::new();
  for entry in entries {
    if is_hidden(&entry.path) {
      continue;
    }
    insert_entry(&mut roots, entry);
  }
  sort_nodes(&mut roots);
  roots
}

/// Pure: rows for the current expansion and filter.
pub fn flatten(
  roots: &[Node],
  expanded: &HashSet<String>,
  filter: &str,
  status: Option<&RepositoryStatus>,
  selected: &[String],
) -> Vec<Row> {
  let mut rows = Vec::new();
  flatten_into(
    roots,
    0,
    &FlattenCtx {
      expanded,
      filter,
      filtering: !filter.is_empty(),
      status,
      selected,
    },
    &mut rows,
  );
  rows
}

/// Pure: whether a PathsChanged event should reload the tree.
pub fn should_reload(kind: PathChangeKind) -> bool {
  matches!(kind, PathChangeKind::Structural | PathChangeKind::Git)
}

struct FlattenCtx<'a> {
  expanded: &'a HashSet<String>,
  filter: &'a str,
  filtering: bool,
  status: Option<&'a RepositoryStatus>,
  selected: &'a [String],
}

fn flatten_into(nodes: &[Node], depth: usize, ctx: &FlattenCtx<'_>, rows: &mut Vec<Row>) {
  for node in nodes {
    if ctx.filtering && !visible_in_filter(node, ctx.filter) {
      continue;
    }
    let show_children =
      node.is_directory && node.children.is_some() && (ctx.filtering || ctx.expanded.contains(&node.path));
    rows.push(Row {
      path: node.path.clone(),
      name: node.name.clone(),
      depth,
      is_directory: node.is_directory,
      expanded: show_children,
      ignored: node.ignored,
      status: status_for(&node.path, node.is_directory, ctx.status),
      selected: ctx.selected.iter().any(|path| path == &node.path),
    });
    if show_children && let Some(children) = &node.children {
      flatten_into(children, depth + 1, ctx, rows);
    }
  }
}

fn visible_in_filter(node: &Node, filter: &str) -> bool {
  let needle = filter.to_lowercase();
  path_matches(&node.path, &node.name, &needle) || descendant_matches(node, &needle)
}

fn path_matches(path: &str, name: &str, needle: &str) -> bool {
  path.to_lowercase().contains(needle) || name.to_lowercase().contains(needle)
}

fn descendant_matches(node: &Node, needle: &str) -> bool {
  node.children.as_ref().is_some_and(|children| {
    children
      .iter()
      .any(|child| path_matches(&child.path, &child.name, needle) || descendant_matches(child, needle))
  })
}

fn status_for(path: &str, is_directory: bool, status: Option<&RepositoryStatus>) -> Option<FileStatus> {
  if is_directory {
    return None;
  }
  let status = status?;
  const ORDER: [ResourceGroupKind; 4] = [
    ResourceGroupKind::Index,
    ResourceGroupKind::WorkingTree,
    ResourceGroupKind::Untracked,
    ResourceGroupKind::Merge,
  ];
  for kind in ORDER {
    for group in &status.groups {
      if group.kind == kind
        && let Some(file) = group.files.iter().find(|file| file.path == path)
      {
        return Some(file.status.clone());
      }
    }
  }
  None
}

fn is_hidden(path: &str) -> bool {
  path
    .split(['/', '\\'])
    .any(|part| matches!(part, ".git" | ".svn" | ".hg" | ".DS_Store" | "Thumbs.db"))
}

fn insert_entry(nodes: &mut Vec<Node>, entry: &ExplorerEntry) {
  let path = entry.path.replace('\\', "/");
  let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
  if parts.is_empty() {
    return;
  }
  insert_parts(nodes, &parts, 0, entry);
}

fn insert_parts(nodes: &mut Vec<Node>, parts: &[&str], index: usize, entry: &ExplorerEntry) {
  let path = parts[..=index].join("/");
  let is_leaf = index + 1 == parts.len();
  let pos = match nodes.iter().position(|node| node.path == path) {
    Some(pos) => pos,
    None => {
      nodes.push(if is_leaf {
        leaf_node(entry)
      } else {
        intermediate_dir(&path, parts[index])
      });
      nodes.len() - 1
    }
  };
  if is_leaf {
    apply_leaf(&mut nodes[pos], entry);
    return;
  }
  if nodes[pos].children.is_none() {
    nodes[pos].children = Some(Vec::new());
  }
  nodes[pos].is_directory = true;
  insert_parts(nodes[pos].children.as_mut().unwrap(), parts, index + 1, entry);
}

fn apply_leaf(node: &mut Node, entry: &ExplorerEntry) {
  node.name = entry.name.clone();
  node.is_directory = entry.is_directory;
  node.is_symlink = entry.is_symlink;
  node.ignored = entry.ignored;
  if entry.is_directory {
    let empty = node.children.as_ref().is_none_or(|children| children.is_empty());
    if entry.ignored && empty {
      node.children = None;
    } else if node.children.is_none() {
      node.children = Some(Vec::new());
    }
  }
}

fn leaf_node(entry: &ExplorerEntry) -> Node {
  Node {
    path: entry.path.replace('\\', "/"),
    name: entry.name.clone(),
    is_directory: entry.is_directory,
    is_symlink: entry.is_symlink,
    ignored: entry.ignored,
    children: if entry.is_directory {
      if entry.ignored { None } else { Some(Vec::new()) }
    } else {
      None
    },
  }
}

fn intermediate_dir(path: &str, name: &str) -> Node {
  Node {
    path: path.to_string(),
    name: name.to_string(),
    is_directory: true,
    is_symlink: false,
    ignored: false,
    children: Some(Vec::new()),
  }
}

fn sort_nodes(nodes: &mut [Node]) {
  nodes.sort_by(|a, b| match (a.is_directory, b.is_directory) {
    (true, false) => std::cmp::Ordering::Less,
    (false, true) => std::cmp::Ordering::Greater,
    _ => a
      .name
      .to_lowercase()
      .cmp(&b.name.to_lowercase())
      .then_with(|| a.name.cmp(&b.name)),
  });
  for node in nodes.iter_mut() {
    if let Some(children) = &mut node.children {
      sort_nodes(children);
    }
  }
}

fn find_node<'a>(nodes: &'a [Node], path: &str) -> Option<&'a Node> {
  for node in nodes {
    if node.path == path {
      return Some(node);
    }
    if let Some(children) = &node.children
      && let Some(found) = find_node(children, path)
    {
      return Some(found);
    }
  }
  None
}

fn find_node_mut<'a>(nodes: &'a mut [Node], path: &str) -> Option<&'a mut Node> {
  for node in nodes {
    if node.path == path {
      return Some(node);
    }
    if let Some(children) = &mut node.children
      && let Some(found) = find_node_mut(children, path)
    {
      return Some(found);
    }
  }
  None
}

fn children_from_listing(parent: &str, entries: &[ExplorerEntry]) -> Vec<Node> {
  let forest = build_tree(entries);
  if parent.is_empty() {
    return forest;
  }
  forest
    .into_iter()
    .find(|node| node.path == parent)
    .and_then(|node| node.children)
    .unwrap_or_default()
}

fn ensure_entry(roots: &mut Vec<Node>, path: &str, is_directory: bool) {
  if find_node(roots, path).is_some() {
    return;
  }
  let name = path.rsplit('/').next().unwrap_or(path).to_string();
  insert_entry(
    roots,
    &ExplorerEntry {
      name,
      path: path.to_string(),
      is_directory,
      is_symlink: false,
      ignored: false,
    },
  );
  sort_nodes(roots);
}

fn child_names(nodes: &[Node], parent: &str) -> Vec<String> {
  if parent.is_empty() {
    return nodes.iter().map(|node| node.name.clone()).collect();
  }
  find_node(nodes, parent)
    .and_then(|node| node.children.as_ref())
    .map(|children| children.iter().map(|node| node.name.clone()).collect())
    .unwrap_or_default()
}

fn parent_path(path: &str) -> String {
  match path.rsplit_once('/') {
    Some((parent, _)) => parent.to_string(),
    None => String::new(),
  }
}

fn join_repo_path(parent: &str, name: &str) -> String {
  if parent.is_empty() {
    name.to_string()
  } else {
    format!("{parent}/{name}")
  }
}

fn join_unit(result: Result<deathpush_core::Result<()>, tokio::task::JoinError>) -> Result<(), String> {
  match result {
    Ok(Ok(())) => Ok(()),
    Ok(Err(err)) => Err(err.to_string()),
    Err(err) => Err(err.to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use std::collections::HashSet;

  use deathpush_core::types::{
    ExplorerEntry, FileEntry, FileStatus, PathChangeKind, RepoOperationState, RepositoryStatus, ResourceGroup,
    ResourceGroupKind,
  };

  fn entry(path: &str, dir: bool, ignored: bool) -> ExplorerEntry {
    ExplorerEntry {
      name: path.rsplit('/').next().unwrap().into(),
      path: path.into(),
      is_directory: dir,
      is_symlink: false,
      ignored,
    }
  }

  #[test]
  fn build_tree_nests_and_sorts_folders_first() {
    let roots = build_tree(&[
      entry("src/main.rs", false, false),
      entry("README.md", false, false),
      entry("src/lib.rs", false, false),
      entry("node_modules", true, true),
      entry("b.txt", false, false),
    ]);
    let names: Vec<&str> = roots.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, vec!["node_modules", "src", "b.txt", "README.md"]);
    let src = &roots[1];
    assert!(src.is_directory && src.children.as_ref().unwrap().len() == 2);
    assert!(
      roots[0].ignored && roots[0].children.is_none(),
      "ignored directory is a stub"
    );
  }

  #[test]
  fn flatten_follows_expansion_and_filter() {
    let roots = build_tree(&[
      entry("src/main.rs", false, false),
      entry("src/util/a.rs", false, false),
      entry("b.txt", false, false),
    ]);
    let mut expanded = HashSet::new();
    let rows = flatten(&roots, &expanded, "", None, &[]);
    assert_eq!(
      rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
      vec!["src", "b.txt"]
    );
    expanded.insert("src".into());
    let rows = flatten(&roots, &expanded, "", None, &[]);
    assert_eq!(
      rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
      vec!["src", "src/util", "src/main.rs", "b.txt"]
    );
    let rows = flatten(&roots, &HashSet::new(), "a.rs", None, &[]);
    assert_eq!(
      rows.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
      vec!["src", "src/util", "src/util/a.rs"]
    );
    assert_eq!(rows[1].depth, 1);
  }

  #[test]
  fn flatten_carries_status_and_selection() {
    let roots = build_tree(&[entry("a.rs", false, false)]);
    let status = RepositoryStatus {
      root: "/r".into(),
      head_branch: None,
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups: vec![ResourceGroup {
        kind: ResourceGroupKind::WorkingTree,
        label: "Changes".into(),
        files: vec![FileEntry {
          path: "a.rs".into(),
          status: FileStatus::Modified,
          rename_path: None,
        }],
      }],
      operation_state: RepoOperationState::None,
    };
    let rows = flatten(&roots, &HashSet::new(), "", Some(&status), &["a.rs".to_string()]);
    assert_eq!(rows[0].status, Some(FileStatus::Modified));
    assert!(rows[0].selected);
  }

  #[test]
  fn reload_only_for_structural_and_git_changes() {
    assert!(should_reload(PathChangeKind::Structural));
    assert!(should_reload(PathChangeKind::Git));
    assert!(!should_reload(PathChangeKind::Content));
  }
}
