use deathpush_core::session::types::Intent;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::{BranchEntry, ResourceGroupKind};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::*;

use super::view::ChangesView;
use crate::repo::model::RepoModel;
use crate::repo::state::{NetworkOp, RepoState};

#[derive(Clone, Copy)]
pub struct OverflowState<'a> {
  pub has_branch: bool,
  pub network_busy: bool,
  pub can_stage_all: bool,
  pub can_unstage_all: bool,
  pub can_discard_all: bool,
  pub has_staged: bool,
  pub has_stashes: bool,
  pub has_commit: bool,
  pub palette: &'a UiPalette,
}

impl<'a> OverflowState<'a> {
  pub fn from_state(state: &RepoState, palette: &'a UiPalette) -> OverflowState<'a> {
    OverflowState {
      has_branch: state.head_branch().is_some(),
      network_busy: state.network_busy(),
      can_stage_all: state.actions.as_ref().is_some_and(|actions| actions.can_stage_all),
      can_unstage_all: state.actions.as_ref().is_some_and(|actions| actions.can_unstage_all),
      can_discard_all: state.actions.as_ref().is_some_and(|actions| actions.can_discard_all),
      has_staged: state.staged_count() > 0,
      has_stashes: !state.stashes.is_empty(),
      has_commit: state.last_commit.is_some(),
      palette,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowItem {
  Pull,
  PullRebase,
  Push,
  PushForce,
  Fetch,
  Sync,
  MergeBranch,
  RebaseBranch,
  StageAll,
  UnstageAll,
  DiscardAll,
  Stash,
  StashIncludeUntracked,
  StashStagedOnly,
  StashPop,
  UndoCommit,
  OpenRepository,
  CloneRepository,
}

impl OverflowItem {
  pub const ORDER: [OverflowItem; 18] = [
    Self::Pull,
    Self::PullRebase,
    Self::Push,
    Self::PushForce,
    Self::Fetch,
    Self::Sync,
    Self::MergeBranch,
    Self::RebaseBranch,
    Self::StageAll,
    Self::UnstageAll,
    Self::DiscardAll,
    Self::Stash,
    Self::StashIncludeUntracked,
    Self::StashStagedOnly,
    Self::StashPop,
    Self::UndoCommit,
    Self::OpenRepository,
    Self::CloneRepository,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Self::Pull => "Pull",
      Self::PullRebase => "Pull (Rebase)",
      Self::Push => "Push",
      Self::PushForce => "Push (Force)",
      Self::Fetch => "Fetch",
      Self::Sync => "Sync",
      Self::MergeBranch => "Merge Branch...",
      Self::RebaseBranch => "Rebase Branch...",
      Self::StageAll => "Stage All Changes",
      Self::UnstageAll => "Unstage All Changes",
      Self::DiscardAll => "Discard All Changes",
      Self::Stash => "Stash Changes",
      Self::StashIncludeUntracked => "Stash (Include Untracked)",
      Self::StashStagedOnly => "Stash Staged Only",
      Self::StashPop => "Stash Pop (Latest)",
      Self::UndoCommit => "Undo Last Commit",
      Self::OpenRepository => "Open Repository...",
      Self::CloneRepository => "Clone Repository...",
    }
  }

  pub fn enabled(self, s: &OverflowState) -> bool {
    match self {
      Self::Pull | Self::PullRebase | Self::Push | Self::PushForce | Self::Sync => s.has_branch && !s.network_busy,
      Self::Fetch => !s.network_busy,
      Self::MergeBranch | Self::RebaseBranch => s.has_branch,
      Self::StageAll => s.can_stage_all,
      Self::UnstageAll => s.can_unstage_all,
      Self::DiscardAll => s.can_discard_all,
      Self::StashStagedOnly => s.has_staged,
      Self::StashPop => s.has_stashes,
      Self::UndoCommit => s.has_commit,
      Self::Stash | Self::StashIncludeUntracked | Self::OpenRepository | Self::CloneRepository => true,
    }
  }
}

pub fn build_menu(menu: PopupMenu, view: WeakEntity<ChangesView>, state: &OverflowState, cx: &App) -> PopupMenu {
  let _ = (state.palette, cx);
  let mut menu = menu.min_w(px(200.));
  for item in OverflowItem::ORDER {
    let view = view.clone();
    menu = menu.item(
      PopupMenuItem::new(item.label())
        .disabled(!item.enabled(state))
        .on_click(move |_, window, cx| {
          let _ = view.update(cx, |this, cx| this.activate_overflow(item, window, cx));
        }),
    );
  }
  menu
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchListMode {
  Merge,
  Rebase,
}

impl BranchListMode {
  pub fn header(self) -> &'static str {
    match self {
      Self::Merge => "Merge",
      Self::Rebase => "Rebase onto",
    }
  }

  pub fn intent(self, name: String) -> Intent {
    match self {
      Self::Merge => Intent::MergeBranch { name },
      Self::Rebase => Intent::RebaseBranch { name },
    }
  }
}

pub fn filter_branches<'a>(branches: &'a [BranchEntry], current: Option<&str>, query: &str) -> Vec<&'a BranchEntry> {
  let query = query.to_lowercase();
  branches
    .iter()
    .filter(|branch| {
      if branch.is_head || current.is_some_and(|name| branch.name == name) {
        return false;
      }
      query.is_empty() || branch.name.to_lowercase().contains(&query)
    })
    .collect()
}

pub fn network_intent(item: OverflowItem) -> Option<(NetworkOp, Intent)> {
  match item {
    OverflowItem::Pull => Some((NetworkOp::Pull, Intent::Pull { rebase: false })),
    OverflowItem::PullRebase => Some((NetworkOp::Pull, Intent::Pull { rebase: true })),
    OverflowItem::Push => Some((
      NetworkOp::Push,
      Intent::Push {
        force: false,
        confirmed: false,
      },
    )),
    OverflowItem::PushForce => Some((
      NetworkOp::Push,
      Intent::Push {
        force: true,
        confirmed: false,
      },
    )),
    OverflowItem::Fetch => Some((NetworkOp::Fetch, Intent::Fetch { prune: true })),
    OverflowItem::Sync => Some((NetworkOp::Sync, Intent::Sync)),
    _ => None,
  }
}

pub fn discard_all_paths(state: &RepoState) -> Vec<String> {
  state
    .status
    .as_ref()
    .map(|status| {
      status
        .groups
        .iter()
        .filter(|group| group.kind != ResourceGroupKind::Index)
        .flat_map(|group| group.files.iter().map(|file| file.path.clone()))
        .collect()
    })
    .unwrap_or_default()
}

/// Model-only overflow actions shared by `ChangesView` and the `RepoView` Git menu handlers.
pub fn dispatch_item(model: &Entity<RepoModel>, item: OverflowItem, window: &mut Window, cx: &mut App) -> bool {
  if let Some((op, intent)) = network_intent(item) {
    model.update(cx, |model, cx| model.dispatch_network(op, intent, window, cx));
    return true;
  }
  match item {
    OverflowItem::DiscardAll => {
      let paths = discard_all_paths(model.read(cx).state());
      model.update(cx, |model, cx| {
        model.dispatch(
          Intent::Discard {
            paths,
            confirmed: false,
          },
          window,
          cx,
        );
      });
      true
    }
    OverflowItem::StashIncludeUntracked => {
      model.update(cx, |model, cx| {
        model.dispatch(
          Intent::StashSave {
            include_untracked: true,
            staged_only: false,
            message: None,
          },
          window,
          cx,
        );
      });
      true
    }
    OverflowItem::StashStagedOnly => {
      model.update(cx, |model, cx| {
        model.dispatch(
          Intent::StashSave {
            include_untracked: false,
            staged_only: true,
            message: None,
          },
          window,
          cx,
        );
      });
      true
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::theme::{ThemeKind, ThemeSpec, ThemeStyle, UiPalette};
  use deathpush_core::types::BranchEntry;

  fn palette() -> UiPalette {
    let spec = ThemeSpec {
      name: "test".into(),
      kind: ThemeKind::Dark,
      style: ThemeStyle::default(),
    };
    UiPalette::resolve(&spec, &ThemeStyle::default())
  }

  fn branch(name: &str, is_remote: bool) -> BranchEntry {
    BranchEntry {
      name: name.into(),
      is_head: false,
      is_remote,
      upstream: None,
      ahead: 0,
      behind: 0,
    }
  }

  #[test]
  fn overflow_order_and_labels_match_the_spec() {
    let labels: Vec<&str> = OverflowItem::ORDER.iter().map(|i| i.label()).collect();
    assert_eq!(
      labels,
      vec![
        "Pull",
        "Pull (Rebase)",
        "Push",
        "Push (Force)",
        "Fetch",
        "Sync",
        "Merge Branch...",
        "Rebase Branch...",
        "Stage All Changes",
        "Unstage All Changes",
        "Discard All Changes",
        "Stash Changes",
        "Stash (Include Untracked)",
        "Stash Staged Only",
        "Stash Pop (Latest)",
        "Undo Last Commit",
        "Open Repository...",
        "Clone Repository...",
      ]
    );
  }

  #[test]
  fn disabled_rules() {
    let p = palette();
    let base = OverflowState {
      has_branch: true,
      network_busy: false,
      can_stage_all: true,
      can_unstage_all: true,
      can_discard_all: true,
      has_staged: true,
      has_stashes: true,
      has_commit: true,
      palette: &p,
    };
    assert!(OverflowItem::ORDER.iter().all(|i| i.enabled(&base)));
    let detached = OverflowState {
      has_branch: false,
      ..base
    };
    for i in [
      OverflowItem::Pull,
      OverflowItem::PullRebase,
      OverflowItem::Push,
      OverflowItem::PushForce,
      OverflowItem::Sync,
      OverflowItem::MergeBranch,
      OverflowItem::RebaseBranch,
    ] {
      assert!(!i.enabled(&detached), "{i:?} needs a branch");
    }
    assert!(OverflowItem::Fetch.enabled(&detached));
    let busy = OverflowState {
      network_busy: true,
      ..base
    };
    for i in [
      OverflowItem::Pull,
      OverflowItem::PullRebase,
      OverflowItem::Push,
      OverflowItem::PushForce,
      OverflowItem::Fetch,
      OverflowItem::Sync,
    ] {
      assert!(!i.enabled(&busy));
    }
    assert!(OverflowItem::StageAll.enabled(&busy));
    let nothing = OverflowState {
      can_stage_all: false,
      can_unstage_all: false,
      can_discard_all: false,
      has_staged: false,
      has_stashes: false,
      has_commit: false,
      ..base
    };
    for i in [
      OverflowItem::StageAll,
      OverflowItem::UnstageAll,
      OverflowItem::DiscardAll,
      OverflowItem::StashStagedOnly,
      OverflowItem::StashPop,
      OverflowItem::UndoCommit,
    ] {
      assert!(!i.enabled(&nothing));
    }
    assert!(OverflowItem::Stash.enabled(&nothing) && OverflowItem::OpenRepository.enabled(&nothing));
  }

  #[test]
  fn branch_list_excludes_current_and_filters() {
    let branches = vec![
      branch("main", false),
      branch("feature/x", false),
      branch("origin/main", true),
    ];
    let all = filter_branches(&branches, Some("main"), "");
    assert_eq!(
      all.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
      vec!["feature/x", "origin/main"]
    );
    let some = filter_branches(&branches, Some("main"), "FEAT");
    assert_eq!(some.len(), 1);
    assert_eq!(BranchListMode::Merge.header(), "Merge");
    assert_eq!(BranchListMode::Rebase.header(), "Rebase onto");
  }
}
