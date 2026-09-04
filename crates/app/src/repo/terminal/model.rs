use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use deathpush_core::terminal::pane::PaneHandle;
use deathpush_core::{Core, SessionId};
use gpui_kit::*;

use super::names::default_name;
use super::pane_view::{self, PaneView};
use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitTree {
  Leaf(u64),
  Split {
    id: u64,
    axis: Axis,
    first: Box<SplitTree>,
    second: Box<SplitTree>,
  },
}

impl SplitTree {
  pub fn split(&mut self, pane: u64, axis: Axis, new_pane: u64, split_id: u64) -> bool {
    match self {
      SplitTree::Leaf(id) if *id == pane => {
        *self = SplitTree::Split {
          id: split_id,
          axis,
          first: Box::new(SplitTree::Leaf(pane)),
          second: Box::new(SplitTree::Leaf(new_pane)),
        };
        true
      }
      SplitTree::Leaf(_) => false,
      SplitTree::Split { first, second, .. } => {
        first.split(pane, axis, new_pane, split_id) || second.split(pane, axis, new_pane, split_id)
      }
    }
  }

  pub fn remove(&mut self, pane: u64) -> bool {
    match self {
      SplitTree::Leaf(id) => *id == pane,
      SplitTree::Split { .. } => {
        let taken = std::mem::replace(self, SplitTree::Leaf(0));
        let SplitTree::Split {
          id,
          axis,
          mut first,
          mut second,
        } = taken
        else {
          unreachable!()
        };
        if matches!(first.as_ref(), SplitTree::Leaf(id) if *id == pane) {
          *self = *second;
          return true;
        }
        if matches!(second.as_ref(), SplitTree::Leaf(id) if *id == pane) {
          *self = *first;
          return true;
        }
        let removed = first.remove(pane) || second.remove(pane);
        *self = SplitTree::Split {
          id,
          axis,
          first,
          second,
        };
        removed
      }
    }
  }

  pub fn panes(&self) -> Vec<u64> {
    match self {
      SplitTree::Leaf(id) => vec![*id],
      SplitTree::Split { first, second, .. } => {
        let mut panes = first.panes();
        panes.extend(second.panes());
        panes
      }
    }
  }
}

pub struct Group {
  pub id: u64,
  pub tree: SplitTree,
  pub active: u64,
  #[allow(dead_code)]
  pub next_index: usize,
}

pub struct PaneInfo {
  pub id: u64,
  pub default_name: String,
  pub shell: Option<String>,
  pub foreground: Option<String>,
  pub view: Entity<PaneView>,
  handle: Option<Arc<PaneHandle>>,
}

impl PaneInfo {
  pub fn name(&self) -> String {
    super::names::display_name(&self.default_name, self.shell.as_deref(), self.foreground.as_deref())
  }
}

pub struct TerminalModel {
  core: Arc<Core>,
  session: SessionId,
  #[allow(dead_code)]
  root: String,
  pub groups: Vec<Group>,
  pub active_group: Option<u64>,
  pub panes: HashMap<u64, PaneInfo>,
  next_group: u64,
  next_split: u64,
  counter: usize,
  polling: bool,
}

impl TerminalModel {
  pub fn new(core: Arc<Core>, session: SessionId, root: String, _cx: &mut Context<Self>) -> Self {
    Self {
      core,
      session,
      root,
      groups: Vec::new(),
      active_group: None,
      panes: HashMap::new(),
      next_group: 1,
      next_split: 1,
      counter: 0,
      polling: false,
    }
  }

  pub fn new_group(&mut self, window: &mut Window, cx: &mut Context<Self>) -> u64 {
    let Some(pane) = self.spawn_pane(cx) else {
      return 0;
    };
    let id = self.next_group;
    self.next_group += 1;
    self.groups.push(Group {
      id,
      tree: SplitTree::Leaf(pane),
      active: pane,
      next_index: 0,
    });
    self.active_group = Some(id);
    self.activate_pane(pane, window, cx);
    self.ensure_name_poll(cx);
    cx.notify();
    id
  }

  pub fn split(&mut self, pane: u64, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
    let Some(group_id) = self.group_id_for_pane(pane) else {
      return;
    };
    let Some(new_pane) = self.spawn_pane(cx) else {
      return;
    };
    let split_id = self.alloc_split_id();
    if let Some(group) = self.groups.iter_mut().find(|group| group.id == group_id)
      && group.tree.split(pane, axis, new_pane, split_id)
    {
      group.active = new_pane;
    }
    self.active_group = Some(group_id);
    self.activate_pane(new_pane, window, cx);
    self.ensure_name_poll(cx);
    cx.notify();
  }

  pub fn kill_pane(&mut self, pane: u64, cx: &mut Context<Self>) {
    let _ = self.core.terminal_kill(pane);
    self.remove_pane(pane, cx);
  }

  pub fn kill_group(&mut self, group: u64, cx: &mut Context<Self>) {
    let Some(index) = self.groups.iter().position(|item| item.id == group) else {
      return;
    };
    let removed = self.groups.remove(index);
    for pane in removed.tree.panes() {
      let _ = self.core.terminal_kill(pane);
      self.panes.remove(&pane);
    }
    if self.active_group == Some(removed.id) {
      self.active_group = self.groups.get(index).or(self.groups.last()).map(|item| item.id);
      if let Some(pane) = self.active_pane() {
        self.set_active_flags(pane, cx);
      }
    }
    cx.notify();
  }

  pub fn activate_group(&mut self, index: usize, cx: &mut Context<Self>) {
    if index == 0 || index > self.groups.len() {
      return;
    }
    let group = &self.groups[index - 1];
    self.active_group = Some(group.id);
    let pane = group.active;
    self.set_active_flags(pane, cx);
    cx.notify();
  }

  pub fn activate_pane(&mut self, pane: u64, window: &mut Window, cx: &mut Context<Self>) {
    self.mark_active(pane, cx);
    if let Some(info) = self.panes.get(&pane) {
      info.view.update(cx, |view, cx| view.focus(window, cx));
    }
  }

  pub fn on_data(&mut self, id: u64, data: &str) {
    if let Some(pane) = self.panes.get(&id)
      && let Some(handle) = &pane.handle
    {
      handle.push_bytes(data.as_bytes());
    }
  }

  pub fn on_exited(&mut self, id: u64, cx: &mut Context<Self>) {
    self.remove_pane(id, cx);
  }

  pub fn poll_names(&mut self, cx: &mut Context<Self>) {
    let ids: Vec<u64> = self.panes.keys().copied().collect();
    let mut changed = false;
    for id in ids {
      let Ok(name) = self.core.terminal_foreground_process(id) else {
        continue;
      };
      if let Some(pane) = self.panes.get_mut(&id) {
        let next = if name.is_empty() { None } else { Some(name) };
        if pane.foreground != next {
          pane.foreground = next;
          changed = true;
        }
      }
    }
    if changed {
      cx.notify();
    }
  }

  pub fn active_pane(&self) -> Option<u64> {
    let group_id = self.active_group?;
    self
      .groups
      .iter()
      .find(|group| group.id == group_id)
      .map(|group| group.active)
  }

  #[allow(dead_code)]
  pub fn has_active_process(&self) -> bool {
    self.core.terminals_have_active_process().unwrap_or(false)
  }

  pub fn active_group(&self) -> Option<&Group> {
    let id = self.active_group?;
    self.groups.iter().find(|group| group.id == id)
  }

  pub fn set_panes_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
    for pane in self.panes.values() {
      pane.view.update(cx, |view, cx| view.set_visible(visible, cx));
    }
  }

  pub fn ensure_group(&mut self, window: &mut Window, cx: &mut Context<Self>) -> u64 {
    if let Some(id) = self.active_group {
      return id;
    }
    self.new_group(window, cx)
  }

  fn spawn_pane(&mut self, cx: &mut Context<Self>) -> Option<u64> {
    if self.core.repo_root(self.session).is_err() {
      return None;
    }
    let settings = AppConfig::get(cx).settings.terminal.clone();
    let shell_path = if settings.shell_path.is_empty() {
      None
    } else {
      Some(settings.shell_path)
    };
    let spawned = match self.core.terminal_spawn(self.session, 80, 24, shell_path, None) {
      Ok(spawned) => spawned,
      Err(err) => {
        tracing::warn!(%err, "terminal spawn failed");
        return None;
      }
    };
    let id = spawned.id;
    let (wake, wake_rx) = pane_view::wake_pair();
    let core = self.core.clone();
    let handle = match PaneHandle::spawn(
      80,
      24,
      Some(settings.scrollback as usize * 1024),
      Box::new(move |bytes| {
        let _ = core.terminal_write(id, &String::from_utf8_lossy(&bytes));
      }),
      wake,
    ) {
      Ok(handle) => Arc::new(handle),
      Err(err) => {
        tracing::warn!(%err, "pane thread spawn failed");
        let _ = self.core.terminal_kill(id);
        return None;
      }
    };
    let view_handle = handle.clone();
    let view = cx.new(|cx| PaneView::new(id, view_handle, wake_rx, cx));
    self.subscribe_pane(&view, cx);
    self.counter += 1;
    let shell = if spawned.shell.is_empty() {
      None
    } else {
      Some(spawned.shell)
    };
    self.panes.insert(
      id,
      PaneInfo {
        id,
        default_name: default_name(self.counter),
        shell,
        foreground: None,
        view,
        handle: Some(handle),
      },
    );
    Some(id)
  }

  fn remove_pane(&mut self, pane: u64, cx: &mut Context<Self>) {
    let Some(group_id) = self.group_id_for_pane(pane) else {
      self.panes.remove(&pane);
      cx.notify();
      return;
    };
    let Some(index) = self.groups.iter().position(|group| group.id == group_id) else {
      return;
    };
    if self.groups[index].tree.panes().len() <= 1 {
      self.kill_group(group_id, cx);
      return;
    }
    let was_active_group = self.active_group == Some(group_id);
    let group = &mut self.groups[index];
    let ids = group.tree.panes();
    let slot = ids.iter().position(|&id| id == pane).unwrap_or(0);
    group.tree.remove(pane);
    self.panes.remove(&pane);
    let remaining = group.tree.panes();
    if let Some(&next) = remaining.get(slot.min(remaining.len().saturating_sub(1))) {
      group.active = next;
      if was_active_group {
        self.set_active_flags(next, cx);
      }
    }
    cx.notify();
  }

  fn alloc_split_id(&mut self) -> u64 {
    let id = self.next_split;
    self.next_split += 1;
    id
  }

  fn group_id_for_pane(&self, pane: u64) -> Option<u64> {
    self
      .groups
      .iter()
      .find(|group| group.tree.panes().contains(&pane))
      .map(|group| group.id)
  }

  fn mark_active(&mut self, pane: u64, cx: &mut Context<Self>) {
    if let Some(group_id) = self.group_id_for_pane(pane) {
      self.active_group = Some(group_id);
      if let Some(group) = self.groups.iter_mut().find(|group| group.id == group_id) {
        group.active = pane;
      }
    }
    self.set_active_flags(pane, cx);
    cx.notify();
  }

  fn subscribe_pane(&mut self, view: &Entity<PaneView>, cx: &mut Context<Self>) {
    cx.subscribe(view, |this, _, event: &pane_view::PaneEvent, cx| {
      let pane_view::PaneEvent::Focused(id) = *event;
      this.mark_active(id, cx);
    })
    .detach();
  }

  fn set_active_flags(&self, pane: u64, cx: &mut Context<Self>) {
    for info in self.panes.values() {
      let active = info.id == pane;
      info.view.update(cx, |view, cx| view.set_active(active, cx));
    }
  }

  fn ensure_name_poll(&mut self, cx: &mut Context<Self>) {
    if self.polling || self.panes.is_empty() {
      return;
    }
    self.polling = true;
    cx.spawn(async move |this, cx| {
      loop {
        cx.background_executor().timer(Duration::from_secs(1)).await;
        let keep = this
          .update(cx, |this, cx| {
            this.poll_names(cx);
            !this.panes.is_empty()
          })
          .unwrap_or(false);
        if !keep {
          break;
        }
      }
      let _ = this.update(cx, |this, _| this.polling = false);
    })
    .detach();
  }

  #[cfg(test)]
  pub(crate) fn insert_test_pane(&mut self, view: Entity<PaneView>, cx: &mut Context<Self>) -> u64 {
    let id = view.read(cx).id;
    self.subscribe_pane(&view, cx);
    self.counter += 1;
    self.panes.insert(
      id,
      PaneInfo {
        id,
        default_name: default_name(self.counter),
        shell: None,
        foreground: None,
        view,
        handle: None,
      },
    );
    let group_id = self.next_group;
    self.next_group += 1;
    self.groups.push(Group {
      id: group_id,
      tree: SplitTree::Leaf(id),
      active: id,
      next_index: 0,
    });
    self.active_group = Some(group_id);
    cx.notify();
    group_id
  }

  #[cfg(test)]
  pub(crate) fn attach_test_pane(
    &mut self,
    split_of: u64,
    axis: Axis,
    view: Entity<PaneView>,
    cx: &mut Context<Self>,
  ) -> u64 {
    let id = view.read(cx).id;
    self.subscribe_pane(&view, cx);
    self.counter += 1;
    self.panes.insert(
      id,
      PaneInfo {
        id,
        default_name: default_name(self.counter),
        shell: None,
        foreground: None,
        view,
        handle: None,
      },
    );
    let split_id = self.alloc_split_id();
    if let Some(group) = self
      .groups
      .iter_mut()
      .find(|group| group.tree.panes().contains(&split_of))
    {
      group.tree.split(split_of, axis, id, split_id);
      group.active = id;
    }
    cx.notify();
    id
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  use gpui_kit::TestAppContext;

  #[test]
  fn split_tree_split_remove_and_collapse() {
    let mut tree = SplitTree::Leaf(1);
    assert_eq!(tree.panes(), vec![1]);
    assert!(tree.split(1, Axis::Horizontal, 2, 10));
    assert_eq!(tree.panes(), vec![1, 2]);
    match &tree {
      SplitTree::Split {
        id,
        axis: Axis::Horizontal,
        ..
      } => assert_eq!(*id, 10),
      other => panic!("expected horizontal split, got {other:?}"),
    }
    assert!(tree.split(2, Axis::Vertical, 3, 11));
    assert_eq!(tree.panes(), vec![1, 2, 3]);
    match &tree {
      SplitTree::Split { id, second, .. } => {
        assert_eq!(*id, 10);
        match second.as_ref() {
          SplitTree::Split {
            id,
            axis: Axis::Vertical,
            ..
          } => assert_eq!(*id, 11),
          other => panic!("expected nested vertical split, got {other:?}"),
        }
      }
      other => panic!("expected parent split, got {other:?}"),
    }
    assert!(!tree.split(99, Axis::Horizontal, 4, 12));
    assert!(tree.remove(2));
    assert_eq!(tree.panes(), vec![1, 3]);
    match &tree {
      SplitTree::Split { id, .. } => assert_eq!(*id, 10),
      other => panic!("parent split id should survive child removal, got {other:?}"),
    }
    assert!(tree.remove(3));
    assert_eq!(tree, SplitTree::Leaf(1));
    assert!(tree.remove(1));
    assert_eq!(tree, SplitTree::Leaf(1));
  }

  #[gpui_kit::test]
  fn activate_group_by_index_ignores_out_of_range(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let resource_dir = tempfile::TempDir::new().unwrap();
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let model = cx.new(|cx| {
      let mut model = TerminalModel::new(core, session, "/tmp".into(), cx);
      let first = cx.new(|cx| PaneView::new_unthreaded(1, cx));
      let second = cx.new(|cx| PaneView::new_unthreaded(2, cx));
      model.insert_test_pane(first, cx);
      model.insert_test_pane(second, cx);
      model
    });
    model.update(cx, |model, cx| {
      assert_eq!(model.groups.len(), 2);
      let first = model.groups[0].id;
      let second = model.groups[1].id;
      assert_eq!(model.active_group, Some(second));
      model.activate_group(1, cx);
      assert_eq!(model.active_group, Some(first));
      model.activate_group(9, cx);
      assert_eq!(model.active_group, Some(first));
      model.activate_group(0, cx);
      assert_eq!(model.active_group, Some(first));
    });
  }

  #[gpui_kit::test]
  fn remove_pane_in_background_group_keeps_active_group(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let resource_dir = tempfile::TempDir::new().unwrap();
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let model = cx.new(|cx| {
      let mut model = TerminalModel::new(core, session, "/tmp".into(), cx);
      let first = cx.new(|cx| PaneView::new_unthreaded(1, cx));
      let second = cx.new(|cx| PaneView::new_unthreaded(2, cx));
      let third = cx.new(|cx| PaneView::new_unthreaded(3, cx));
      model.insert_test_pane(first, cx);
      model.insert_test_pane(second, cx);
      model.attach_test_pane(2, Axis::Horizontal, third, cx);
      model
    });
    model.update(cx, |model, cx| {
      let foreground = model.groups[0].id;
      let background = model.groups[1].id;
      model.activate_group(1, cx);
      assert_eq!(model.active_group, Some(foreground));
      assert_eq!(model.groups[1].tree.panes(), vec![2, 3]);
      model.on_exited(3, cx);
      assert_eq!(model.active_group, Some(foreground));
      assert_eq!(model.groups.len(), 2);
      assert_eq!(model.groups[1].id, background);
      assert_eq!(model.groups[1].tree, SplitTree::Leaf(2));
    });
  }

  #[gpui_kit::test]
  fn focused_pane_becomes_the_active_pane(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let resource_dir = tempfile::TempDir::new().unwrap();
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let model = cx.new(|cx| {
      let mut model = TerminalModel::new(core, session, "/tmp".into(), cx);
      let first = cx.new(|cx| PaneView::new_unthreaded(1, cx));
      let second = cx.new(|cx| PaneView::new_unthreaded(2, cx));
      model.insert_test_pane(first, cx);
      model.attach_test_pane(1, Axis::Vertical, second, cx);
      model
    });
    let first_view = model.read_with(cx, |model, _| model.panes[&1].view.clone());
    first_view.update(cx, |_, cx| cx.emit(super::super::pane_view::PaneEvent::Focused(1)));
    model.update(cx, |model, cx| {
      assert_eq!(model.groups[0].active, 1);
      assert!(model.panes[&1].view.read(cx).active());
      assert!(!model.panes[&2].view.read(cx).active());
    });
  }

  fn first_split_id(tree: &SplitTree) -> u64 {
    match tree {
      SplitTree::Split { id, .. } => *id,
      other => panic!("expected a split, got {other:?}"),
    }
  }

  #[gpui_kit::test]
  fn split_ids_are_unique_across_groups(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let resource_dir = tempfile::TempDir::new().unwrap();
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let model = cx.new(|cx| {
      let mut model = TerminalModel::new(core, session, "/tmp".into(), cx);
      let a = cx.new(|cx| PaneView::new_unthreaded(1, cx));
      let a2 = cx.new(|cx| PaneView::new_unthreaded(3, cx));
      let b = cx.new(|cx| PaneView::new_unthreaded(2, cx));
      let b2 = cx.new(|cx| PaneView::new_unthreaded(4, cx));
      model.insert_test_pane(a, cx);
      model.insert_test_pane(b, cx);
      model.attach_test_pane(1, Axis::Horizontal, a2, cx);
      model.attach_test_pane(2, Axis::Vertical, b2, cx);
      model
    });
    model.update(cx, |model, _| {
      let id_a = first_split_id(&model.groups[0].tree);
      let id_b = first_split_id(&model.groups[1].tree);
      assert_ne!(
        id_a, id_b,
        "groups must not share a split id (panel keys ResizableState by it)"
      );
    });
  }
}
