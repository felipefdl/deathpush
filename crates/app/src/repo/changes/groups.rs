use std::sync::Arc;

use deathpush_core::ops::repository::NestedRepository;
use deathpush_core::session::types::Intent;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::{FileStatus, ResourceGroupKind, StashEntry};
use gpui_kit::base::{resizable_panel, v_resizable};
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;

use super::filter::matches_filter;
use super::rows::{render_file_row, render_nested_row, render_stash_row};
use super::view::ChangesView;
use crate::config::AppConfig;
use crate::repo::state::RepoState;
use crate::theme::{ActivePalette, hsla};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GroupId {
  Merge,
  Staged,
  Changes,
  Stashes,
  Nested,
}

impl GroupId {
  pub fn pane_id(self) -> &'static str {
    match self {
      Self::Merge => "scm.merge",
      Self::Staged => "scm.staged",
      Self::Changes => "scm.changes",
      Self::Stashes => "scm.stashes",
      Self::Nested => "scm.nested",
    }
  }

  pub fn label(self) -> &'static str {
    match self {
      Self::Merge => "Merge Changes",
      Self::Staged => "Staged Changes",
      Self::Changes => "Changes",
      Self::Stashes => "Stashes",
      Self::Nested => "Nested Repositories",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRow {
  pub path: String,
  pub status: FileStatus,
  pub staged: bool,
  pub group_kind: ResourceGroupKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupBody {
  Files(Vec<FileRow>),
  Stashes(Vec<StashEntry>),
  Nested(Vec<NestedRepository>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
  pub id: GroupId,
  pub count: usize,
  pub body: GroupBody,
}

/// Non-empty groups after filtering, in spec order: Merge, Staged, Changes, Stashes, Nested.
pub fn assemble_groups(state: &RepoState, filter: &str) -> Vec<Group> {
  let mut groups = Vec::new();
  let files = |kind: ResourceGroupKind, staged: bool| -> Vec<FileRow> {
    state
      .status
      .as_ref()
      .map(|status| {
        status
          .groups
          .iter()
          .filter(|group| group.kind == kind)
          .flat_map(|group| group.files.iter())
          .filter(|file| matches_filter(&file.path, filter))
          .map(|file| FileRow {
            path: file.path.clone(),
            status: file.status.clone(),
            staged,
            group_kind: kind,
          })
          .collect()
      })
      .unwrap_or_default()
  };

  let merge = files(ResourceGroupKind::Merge, false);
  if !merge.is_empty() {
    groups.push(Group {
      id: GroupId::Merge,
      count: merge.len(),
      body: GroupBody::Files(merge),
    });
  }
  let staged = files(ResourceGroupKind::Index, true);
  if !staged.is_empty() {
    groups.push(Group {
      id: GroupId::Staged,
      count: staged.len(),
      body: GroupBody::Files(staged),
    });
  }
  let mut changes = files(ResourceGroupKind::WorkingTree, false);
  changes.extend(files(ResourceGroupKind::Untracked, false));
  if !changes.is_empty() {
    groups.push(Group {
      id: GroupId::Changes,
      count: changes.len(),
      body: GroupBody::Files(changes),
    });
  }
  if !state.stashes.is_empty() {
    groups.push(Group {
      id: GroupId::Stashes,
      count: state.stashes.len(),
      body: GroupBody::Stashes(state.stashes.clone()),
    });
  }
  if !state.nested_repositories.is_empty() {
    groups.push(Group {
      id: GroupId::Nested,
      count: state.nested_repositories.len(),
      body: GroupBody::Nested(state.nested_repositories.clone()),
    });
  }
  groups
}

pub fn render_groups(
  view: &ChangesView,
  groups: &[Group],
  window: &mut Window,
  cx: &mut Context<ChangesView>,
) -> impl IntoElement {
  let _ = window;
  let palette = cx.global::<ActivePalette>().0;
  let density = AppConfig::get(cx).settings.ui.tree_density;
  let (expanded, collapsed_groups) = {
    let layout = view.layout.read(cx);
    let expanded: Vec<Group> = groups
      .iter()
      .filter(|group| !layout.is_collapsed(group.id.pane_id()))
      .cloned()
      .collect();
    let collapsed_groups: Vec<Group> = groups
      .iter()
      .filter(|group| layout.is_collapsed(group.id.pane_id()))
      .cloned()
      .collect();
    (expanded, collapsed_groups)
  };
  let weak = cx.weak_entity();

  let mut root = div().flex_1().min_h_0().flex().flex_col();
  if !expanded.is_empty() {
    let mut panels = v_resizable("scm-groups").with_state(&view.groups_state);
    for group in &expanded {
      panels = panels.child(
        resizable_panel()
          .size_range(px(44.0)..Pixels::MAX)
          .child(render_expanded_group(group, view, &weak, density, &palette, cx)),
      );
    }
    root = root.child(panels);
  }
  for group in &collapsed_groups {
    root = root.child(render_header(group, true, &palette, cx));
  }
  root
}

fn render_expanded_group(
  group: &Group,
  view: &ChangesView,
  weak: &WeakEntity<ChangesView>,
  density: deathpush_core::config::settings::TreeDensity,
  palette: &UiPalette,
  cx: &mut Context<ChangesView>,
) -> impl IntoElement {
  div()
    .flex()
    .flex_col()
    .size_full()
    .min_h_0()
    .child(render_header(group, false, palette, cx))
    .child(
      div()
        .flex_1()
        .min_h_0()
        .child(render_group_body(group, view, weak, density, palette)),
    )
}

fn render_header(
  group: &Group,
  collapsed: bool,
  palette: &UiPalette,
  cx: &mut Context<ChangesView>,
) -> impl IntoElement {
  let hover_group = SharedString::from(format!("scm-header-{}", group.id.pane_id()));
  let pane_id = group.id.pane_id();
  let mut header = div()
    .id(SharedString::from(format!("scm-header-{}", pane_id)))
    .group(hover_group.clone())
    .h(px(22.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .on_click(cx.listener(move |this, _, _, cx| {
      this
        .layout
        .update(cx, |layout, cx| layout.toggle_pane_collapsed(pane_id, cx));
    }))
    .child(
      svg()
        .path(if collapsed {
          "icons/chevron-right.svg"
        } else {
          "icons/chevron-down.svg"
        })
        .size(px(12.0))
        .text_color(hsla(palette.muted_foreground)),
    )
    .child(
      div()
        .flex_1()
        .min_w_0()
        .text_size(px(11.0))
        .font_weight(FontWeight::BOLD)
        .text_color(hsla(palette.muted_foreground))
        .child(group.id.label().to_uppercase()),
    )
    .child(Badge::new().count(group.count).child(div().size(px(12.0))));

  match group.id {
    GroupId::Staged => {
      header = header.child(
        Button::new("scm-unstage-all")
          .ghost()
          .xsmall()
          .icon(Icon::empty().path("icons/remove.svg"))
          .tooltip("Unstage All")
          .invisible()
          .group_hover(hover_group, |style| style.visible())
          .on_click(cx.listener(|this, _, window, cx| {
            cx.stop_propagation();
            this.send(Intent::UnstageAll, window, cx);
          })),
      );
    }
    GroupId::Changes => {
      let paths: Vec<String> = match &group.body {
        GroupBody::Files(rows) => rows.iter().map(|row| row.path.clone()).collect(),
        _ => Vec::new(),
      };
      let discard_paths = paths.clone();
      header = header
        .child(
          Button::new("scm-discard-all")
            .ghost()
            .xsmall()
            .icon(Icon::empty().path("icons/clear-all.svg"))
            .tooltip("Discard All Changes")
            .invisible()
            .group_hover(hover_group.clone(), |style| style.visible())
            .on_click(cx.listener(move |this, _, window, cx| {
              cx.stop_propagation();
              this.send(
                Intent::Discard {
                  paths: discard_paths.clone(),
                  confirmed: false,
                },
                window,
                cx,
              );
            })),
        )
        .child(
          Button::new("scm-stage-all")
            .ghost()
            .xsmall()
            .icon(Icon::empty().path("icons/add.svg"))
            .tooltip("Stage All Changes")
            .invisible()
            .group_hover(hover_group, |style| style.visible())
            .on_click(cx.listener(move |this, _, window, cx| {
              cx.stop_propagation();
              this.send(Intent::Stage { paths: paths.clone() }, window, cx);
            })),
        );
    }
    _ => {}
  }
  header
}

fn render_group_body(
  group: &Group,
  view: &ChangesView,
  weak: &WeakEntity<ChangesView>,
  density: deathpush_core::config::settings::TreeDensity,
  palette: &UiPalette,
) -> AnyElement {
  match &group.body {
    GroupBody::Files(rows) if rows.len() > 200 => {
      let rows = Arc::new(rows.clone());
      let group_id = group.id;
      let weak = weak.clone();
      let selected = view.selected.clone();
      let palette = *palette;
      uniform_list(
        SharedString::from(format!("scm-files-{}", group.id.pane_id())),
        rows.len(),
        move |range, _, _cx| {
          range
            .filter_map(|index| {
              let row = rows.get(index)?;
              let is_selected = selected.contains(&(row.group_kind, row.path.clone()));
              Some(
                render_file_row(row, is_selected, density, group_id, index, weak.clone(), &palette).into_any_element(),
              )
            })
            .collect()
        },
      )
      .size_full()
      .into_any_element()
    }
    GroupBody::Files(rows) => div()
      .id(SharedString::from(format!("scm-files-{}", group.id.pane_id())))
      .size_full()
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .children(rows.iter().enumerate().map(|(index, row)| {
        let selected = view.selected.contains(&(row.group_kind, row.path.clone()));
        render_file_row(row, selected, density, group.id, index, weak.clone(), palette)
      }))
      .into_any_element(),
    GroupBody::Stashes(stashes) => div()
      .id("scm-stashes")
      .size_full()
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .children(
        stashes
          .iter()
          .map(|stash| render_stash_row(stash, weak.clone(), palette)),
      )
      .into_any_element(),
    GroupBody::Nested(repos) => div()
      .id("scm-nested")
      .size_full()
      .overflow_y_scroll()
      .flex()
      .flex_col()
      .children(repos.iter().map(|repo| render_nested_row(repo, weak.clone(), palette)))
      .into_any_element(),
  }
}

#[cfg(test)]
mod tests {
  #![allow(clippy::field_reassign_with_default)]

  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::types::{FileEntry, RepositoryStatus, ResourceGroup};

  fn file(path: &str, status: FileStatus) -> FileEntry {
    FileEntry {
      path: path.into(),
      status,
      rename_path: None,
    }
  }

  fn group(kind: ResourceGroupKind, files: Vec<FileEntry>) -> ResourceGroup {
    ResourceGroup {
      kind,
      label: String::new(),
      files,
    }
  }

  fn status(groups: Vec<ResourceGroup>) -> RepositoryStatus {
    RepositoryStatus {
      root: "/r".into(),
      head_branch: Some("main".into()),
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups,
      operation_state: deathpush_core::types::RepoOperationState::None,
    }
  }

  #[test]
  fn groups_follow_spec_order_and_drop_empty_ones() {
    let mut state = RepoState::default();
    state.status = Some(status(vec![
      group(ResourceGroupKind::Index, vec![file("a.rs", FileStatus::IndexModified)]),
      group(ResourceGroupKind::WorkingTree, vec![file("b.rs", FileStatus::Modified)]),
      group(ResourceGroupKind::Untracked, vec![file("c.txt", FileStatus::Untracked)]),
      group(ResourceGroupKind::Merge, vec![]),
    ]));
    state.stashes = vec![StashEntry {
      index: 0,
      message: "wip".into(),
    }];
    let groups = assemble_groups(&state, "");
    let ids: Vec<GroupId> = groups.iter().map(|g| g.id).collect();
    assert_eq!(ids, vec![GroupId::Staged, GroupId::Changes, GroupId::Stashes]);
    let GroupBody::Files(changes) = &groups[1].body else {
      panic!()
    };
    assert_eq!(changes.len(), 2, "unstaged and untracked share the Changes group");
    assert!(changes.iter().all(|row| !row.staged));
    let GroupBody::Files(staged) = &groups[0].body else {
      panic!()
    };
    assert!(staged[0].staged && staged[0].group_kind == ResourceGroupKind::Index);
  }

  #[test]
  fn filter_removes_groups_without_matches_but_keeps_stashes_and_nested() {
    let mut state = RepoState::default();
    state.status = Some(status(vec![
      group(ResourceGroupKind::Index, vec![file("a.rs", FileStatus::IndexModified)]),
      group(ResourceGroupKind::WorkingTree, vec![file("b.rs", FileStatus::Modified)]),
    ]));
    state.stashes = vec![StashEntry {
      index: 0,
      message: "wip".into(),
    }];
    state.nested_repositories = vec![NestedRepository {
      path: "vendor/x".into(),
      name: "x".into(),
      branch: Some("main".into()),
    }];
    let groups = assemble_groups(&state, "b.rs");
    let ids: Vec<GroupId> = groups.iter().map(|g| g.id).collect();
    assert_eq!(ids, vec![GroupId::Changes, GroupId::Stashes, GroupId::Nested]);
    assert_eq!(groups[0].count, 1);
  }

  #[test]
  fn merge_group_comes_first_and_counts() {
    let mut state = RepoState::default();
    state.status = Some(status(vec![
      group(ResourceGroupKind::WorkingTree, vec![file("b.rs", FileStatus::Modified)]),
      group(ResourceGroupKind::Merge, vec![file("m.rs", FileStatus::BothModified)]),
    ]));
    let groups = assemble_groups(&state, "");
    assert_eq!(groups[0].id, GroupId::Merge);
    assert_eq!(groups[0].count, 1);
    let GroupBody::Files(rows) = &groups[0].body else {
      panic!()
    };
    assert_eq!(rows[0].group_kind, ResourceGroupKind::Merge);
    assert!(!rows[0].staged);
  }
}
