use std::collections::HashSet;

use deathpush_core::ops::repository::NestedRepository;
use deathpush_core::session::types::{
  DiffPayload, FileSelection, Intent, SessionActions, SessionPatch, SessionSnapshot, SessionStatusEvent,
  SessionStatusExtras,
};
use deathpush_core::types::{
  BranchEntry, CommitDetail, CommitEntry, FileBlame, FileContent, LastCommitInfo, RepositoryStatus, ResourceGroupKind,
  StashEntry, TagEntry,
};

/// The repository window's view of core, ported from the deleted repository store and session client.
/// Generation and revision guards decide what applies; nothing here talks to core.
#[derive(Debug, Default, Clone)]
pub struct RepoState {
  pub status: Option<RepositoryStatus>,
  pub selected_file: Option<FileSelection>,
  pub selected_load_id: u64,
  pub diff: Option<DiffPayload>,
  pub diff_load_id: Option<u64>,
  pub branches: Vec<BranchEntry>,
  pub stashes: Vec<StashEntry>,
  pub tags: Vec<TagEntry>,
  pub commit_log: Vec<CommitEntry>,
  pub selected_commit: Option<String>,
  pub commit_detail: Option<CommitDetail>,
  pub file_history_path: Option<String>,
  pub last_commit: Option<LastCommitInfo>,
  pub actions: Option<SessionActions>,
  pub amend_mode: bool,
  pub commit_message: String,
  pub file_filter: String,
  pub error: Option<String>,
  pub session_generation: u64,
  pub session_revision: u64,
  pub status_generation: u64,
  pub status_revision: u64,
  pub blame: Option<FileBlame>,
  pub open_file: Option<OpenFile>,
  pub cursor_line: Option<usize>,
  pub pending_clear_file: bool,
  pub running: HashSet<NetworkOp>,
  pub nested_repositories: Vec<NestedRepository>,
  pub committing: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpenFile {
  pub path: String,
  pub content: Option<FileContent>,
  pub pending_line: Option<usize>,
  pub load_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkOp {
  Pull,
  Push,
  Fetch,
  Sync,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadVerdict {
  Accept,
  Reject,
}

fn older_generation(generation: u64, current: u64) -> bool {
  generation < current
}

fn same_generation_older_revision(
  generation: u64,
  revision: u64,
  current_generation: u64,
  current_revision: u64,
) -> bool {
  generation == current_generation && revision < current_revision
}

fn newer_cursor(generation: u64, revision: u64, current_generation: u64, current_revision: u64) -> bool {
  generation > current_generation || (generation == current_generation && revision > current_revision)
}

fn older_cursor(generation: u64, revision: u64, current_generation: u64, current_revision: u64) -> bool {
  generation < current_generation || (generation == current_generation && revision < current_revision)
}

fn same_file(a: Option<&FileSelection>, b: Option<&FileSelection>) -> bool {
  match (a, b) {
    (None, None) => true,
    (Some(a), Some(b)) => a.path == b.path && a.staged == b.staged && a.group_kind == b.group_kind,
    _ => false,
  }
}

impl RepoState {
  pub fn root(&self) -> Option<&str> {
    self.status.as_ref().map(|status| status.root.as_str())
  }

  pub fn mark_commit_intent(&mut self, intent: &Intent) {
    if matches!(
      intent,
      Intent::Commit { .. } | Intent::CommitAndPush { .. } | Intent::CommitAndSync { .. }
    ) {
      self.committing = true;
    }
  }

  pub fn resolve_commit_outcome(&mut self, confirming: bool) {
    if !confirming {
      self.committing = false;
    }
  }

  pub fn network_busy(&self) -> bool {
    !self.running.is_empty()
  }

  pub fn has_changes(&self) -> bool {
    self.group_file_count(|_| true) > 0
  }

  pub fn staged_count(&self) -> usize {
    self.group_file_count(|kind| kind == ResourceGroupKind::Index)
  }

  #[allow(dead_code)]
  pub fn unstaged_count(&self) -> usize {
    self.group_file_count(|kind| matches!(kind, ResourceGroupKind::WorkingTree | ResourceGroupKind::Untracked))
  }

  #[allow(dead_code)]
  pub fn merge_count(&self) -> usize {
    self.group_file_count(|kind| kind == ResourceGroupKind::Merge)
  }

  fn group_file_count(&self, matches: impl Fn(ResourceGroupKind) -> bool) -> usize {
    self
      .status
      .as_ref()
      .map(|status| {
        status
          .groups
          .iter()
          .filter(|group| matches(group.kind))
          .map(|group| group.files.len())
          .sum()
      })
      .unwrap_or(0)
  }

  pub fn head_branch(&self) -> Option<&str> {
    self.status.as_ref().and_then(|status| status.head_branch.as_deref())
  }

  #[allow(dead_code)]
  pub fn selected_file(&self) -> Option<&FileSelection> {
    self.selected_file.as_ref()
  }

  fn same_root(&self, root: &str) -> bool {
    self.status.as_ref().is_none_or(|status| status.root == root)
  }

  fn set_groups(
    &mut self,
    snapshot_repo: &deathpush_core::session::types::SessionRepo,
    groups: &[deathpush_core::types::ResourceGroup],
    generation: u64,
    revision: u64,
  ) {
    self.status_generation = generation;
    self.status_revision = revision;
    self.status = Some(RepositoryStatus {
      root: snapshot_repo.root.clone(),
      head_branch: snapshot_repo.head_branch.clone(),
      head_commit: snapshot_repo.head_commit.clone(),
      ahead: snapshot_repo.ahead,
      behind: snapshot_repo.behind,
      groups: groups.to_vec(),
      operation_state: snapshot_repo.operation_state,
    });
  }

  fn set_file(&mut self, next: Option<FileSelection>) {
    let changed = !same_file(self.selected_file.as_ref(), next.as_ref());
    let cleared = next.is_none() && self.selected_file.is_some();
    if cleared || (next.is_some() && changed) {
      self.selected_load_id += 1;
    }
    if next.is_none() {
      self.diff = None;
      self.diff_load_id = None;
      self.blame = None;
      self.cursor_line = None;
    }
    self.selected_file = next;
  }

  fn apply_extras(&mut self, extras: Option<&SessionStatusExtras>) {
    let Some(extras) = extras else { return };
    if let Some(last_commit) = &extras.last_commit {
      self.last_commit = Some(last_commit.clone());
    }
    if let Some(branches) = &extras.branches {
      self.branches = branches.clone();
    }
    if let Some(tags) = &extras.tags {
      self.tags = tags.clone();
    }
    if let Some(commit_log) = &extras.commit_log {
      self.commit_log = commit_log.clone();
    }
    if let Some(stashes) = &extras.stashes {
      self.stashes = stashes.clone();
    }
  }

  /// A full snapshot from open, clone, refresh, or a HEAD-moving write.
  pub fn apply_snapshot(&mut self, snapshot: SessionSnapshot) {
    let same_root = self.same_root(&snapshot.repo.root);
    if older_generation(snapshot.session_generation, self.session_generation) {
      if same_root
        && newer_cursor(
          snapshot.status_generation,
          snapshot.status_revision,
          self.status_generation,
          self.status_revision,
        )
      {
        self.set_groups(
          &snapshot.repo,
          &snapshot.groups,
          snapshot.status_generation,
          snapshot.status_revision,
        );
      }
      return;
    }
    if same_generation_older_revision(
      snapshot.session_generation,
      snapshot.session_revision,
      self.session_generation,
      self.session_revision,
    ) {
      if same_root
        && !older_cursor(
          snapshot.status_generation,
          snapshot.status_revision,
          self.status_generation,
          self.status_revision,
        )
      {
        self.set_groups(
          &snapshot.repo,
          &snapshot.groups,
          snapshot.status_generation,
          snapshot.status_revision,
        );
      }
      return;
    }
    let apply_groups = !same_root
      || !older_cursor(
        snapshot.status_generation,
        snapshot.status_revision,
        self.status_generation,
        self.status_revision,
      );
    self.session_generation = snapshot.session_generation;
    self.session_revision = snapshot.session_revision;
    if apply_groups {
      self.set_groups(
        &snapshot.repo,
        &snapshot.groups,
        snapshot.status_generation,
        snapshot.status_revision,
      );
    }
    self.set_file(snapshot.selection.file.clone());
    self.amend_mode = snapshot.scm.amend_mode;
    self.commit_message = snapshot.scm.commit_message.clone();
    self.file_filter = snapshot.scm.file_filter.clone();
    self.commit_log = snapshot.commit_log;
    self.branches = snapshot.branches;
    self.stashes = snapshot.stashes;
    self.tags = snapshot.tags;
    self.selected_commit = snapshot.selection.commit;
    self.commit_detail = snapshot.commit_detail;
    self.file_history_path = snapshot.file_history_path;
    self.last_commit = snapshot.last_commit;
    self.actions = Some(snapshot.actions);
    self.error = snapshot.error;
  }

  /// A stamped patch from an intent.
  pub fn apply_patch(&mut self, patch: SessionPatch, generation: u64, revision: u64) {
    if older_generation(generation, self.session_generation)
      || same_generation_older_revision(generation, revision, self.session_generation, self.session_revision)
    {
      return;
    }
    self.session_generation = generation;
    self.session_revision = revision;
    match patch {
      SessionPatch::Actions { actions } => self.actions = Some(actions),
      SessionPatch::Scm { scm, actions } => {
        self.amend_mode = scm.amend_mode;
        self.commit_message = scm.commit_message;
        self.file_filter = scm.file_filter;
        self.actions = Some(actions);
      }
      SessionPatch::FileHistory { path, commit_log } => {
        self.file_history_path = path;
        self.commit_log = commit_log;
      }
      SessionPatch::CommitLog { commit_log } => self.commit_log = commit_log,
      SessionPatch::Commit { id, detail } => {
        self.selected_commit = id;
        self.commit_detail = detail;
      }
    }
  }

  /// A status event from the runtime (watcher, invalidation, refs or stash refresh).
  pub fn apply_status_event(&mut self, event: SessionStatusEvent) {
    let same_root = self.same_root(&event.repo.root);
    let apply_session = !older_generation(event.session_generation, self.session_generation)
      && !same_generation_older_revision(
        event.session_generation,
        event.session_revision,
        self.session_generation,
        self.session_revision,
      );
    let apply_groups = same_root
      && newer_cursor(
        event.status_generation,
        event.status_revision,
        self.status_generation,
        self.status_revision,
      );
    if !apply_session && !apply_groups {
      return;
    }
    if apply_session {
      self.session_generation = event.session_generation;
      self.session_revision = event.session_revision;
      let next = if self.pending_clear_file {
        self.selected_file.clone()
      } else {
        event.selection.file.clone()
      };
      self.set_file(next);
      self.actions = Some(event.actions.clone());
      self.selected_commit = event.selection.commit.clone();
      self.apply_extras(event.extras.as_ref());
    }
    if apply_groups {
      self.set_groups(
        &event.repo,
        &event.groups,
        event.status_generation,
        event.status_revision,
      );
    }
  }

  /// A stamped Ack. `clear_file` is true when the intent was `ClearFile`.
  pub fn apply_ack(&mut self, generation: Option<u64>, revision: Option<u64>, clear_file: bool) {
    if clear_file {
      self.pending_clear_file = false;
    }
    let (Some(generation), Some(revision)) = (generation, revision) else {
      return;
    };
    if older_generation(generation, self.session_generation)
      || same_generation_older_revision(generation, revision, self.session_generation, self.session_revision)
    {
      return;
    }
    self.session_generation = generation;
    self.session_revision = revision;
    if clear_file {
      self.selected_file = None;
      self.diff = None;
      self.blame = None;
      self.selected_load_id += 1;
    }
  }

  /// Decide whether a stamped Diff or Blame belongs to the current session, and advance the watermark when it is newer.
  pub fn accept_payload(&mut self, generation: u64, revision: u64, root_at_send: Option<&str>) -> PayloadVerdict {
    if generation != self.session_generation {
      return PayloadVerdict::Reject;
    }
    if let Some(root) = root_at_send
      && self.root() != Some(root)
    {
      return PayloadVerdict::Reject;
    }
    if same_generation_older_revision(generation, revision, self.session_generation, self.session_revision) {
      return PayloadVerdict::Reject;
    }
    if newer_cursor(generation, revision, self.session_generation, self.session_revision) {
      self.session_generation = generation;
      self.session_revision = revision;
    }
    PayloadVerdict::Accept
  }

  #[allow(dead_code)]
  pub fn reset(&mut self) {
    *self = RepoState::default();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use deathpush_core::session::types::{
    OperationActions, SessionRepo, SessionScm, SessionSelection, SyncAction, SyncKind,
  };
  use deathpush_core::types::{RepoOperationState, ResourceGroup, ResourceGroupKind, StatusPhase};

  fn repo(root: &str, branch: &str) -> SessionRepo {
    SessionRepo {
      root: root.into(),
      head_branch: Some(branch.into()),
      head_commit: Some("abc".into()),
      ahead: 0,
      behind: 0,
      operation_state: RepoOperationState::None,
      phase: StatusPhase::Settled,
    }
  }

  fn actions(label: &str) -> SessionActions {
    SessionActions {
      can_commit: false,
      commit_label: label.into(),
      commit_destructive: false,
      can_stage_all: false,
      can_unstage_all: false,
      can_discard_all: false,
      discard_all_destructive: false,
      sync: SyncAction {
        enabled: false,
        kind: SyncKind::Fetch,
        destructive: false,
      },
      operation: OperationActions {
        continue_op: false,
        abort: false,
        skip: false,
        abort_destructive: false,
      },
    }
  }

  fn group(label: &str) -> ResourceGroup {
    ResourceGroup {
      kind: ResourceGroupKind::WorkingTree,
      label: label.into(),
      files: vec![],
    }
  }

  fn snapshot(root: &str, sg: u64, sr: u64, stg: u64, str_: u64, file: Option<FileSelection>) -> SessionSnapshot {
    SessionSnapshot {
      session_generation: sg,
      session_revision: sr,
      status_generation: stg,
      status_revision: str_,
      repo: repo(root, "main"),
      groups: vec![group("snapshot")],
      selection: SessionSelection { file, commit: None },
      scm: SessionScm {
        amend_mode: false,
        commit_message: "msg".into(),
        file_filter: String::new(),
      },
      actions: actions("Commit"),
      last_commit: None,
      branches: vec![],
      stashes: vec![],
      tags: vec![],
      commit_log: vec![],
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  fn status_event(root: &str, sg: u64, sr: u64, stg: u64, str_: u64, label: &str) -> SessionStatusEvent {
    SessionStatusEvent {
      session_generation: sg,
      session_revision: sr,
      status_generation: stg,
      status_revision: str_,
      repo: repo(root, "main"),
      groups: vec![group(label)],
      actions: actions("Event"),
      selection: SessionSelection {
        file: None,
        commit: None,
      },
      extras: None,
    }
  }

  fn file(path: &str) -> FileSelection {
    FileSelection {
      path: path.into(),
      staged: false,
      group_kind: ResourceGroupKind::WorkingTree,
    }
  }

  #[test]
  fn snapshot_applies_everything_and_bumps_load_id_on_file_change() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 1, 1, 1, Some(file("a"))));
    assert_eq!(state.root(), Some("/r"));
    assert_eq!(state.selected_load_id, 1);
    assert_eq!(state.commit_message, "msg");
    state.apply_snapshot(snapshot("/r", 1, 2, 1, 2, Some(file("a"))));
    assert_eq!(state.selected_load_id, 1);
    state.apply_snapshot(snapshot("/r", 1, 3, 1, 3, None));
    assert_eq!(state.selected_load_id, 2);
    assert!(state.diff.is_none());
  }

  #[test]
  fn older_generation_snapshot_only_refreshes_newer_groups() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 2, 1, 1, 1, None));
    state.commit_message = "kept".into();
    let mut old = snapshot("/r", 1, 9, 1, 5, None);
    old.groups = vec![group("newer-status")];
    state.apply_snapshot(old);
    assert_eq!(state.commit_message, "kept");
    assert_eq!(state.session_generation, 2);
    assert_eq!(state.status_revision, 5);
    assert_eq!(state.status.as_ref().unwrap().groups[0].label, "newer-status");
  }

  #[test]
  fn new_generation_replaces_a_high_revision_session_of_another_repo() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/a", 1, 40, 1, 40, Some(file("x"))));
    state.apply_snapshot(snapshot("/b", 2, 0, 1, 0, None));
    assert_eq!(state.root(), Some("/b"));
    assert_eq!(state.session_generation, 2);
    assert_eq!(state.status_revision, 0);
  }

  #[test]
  fn patch_applies_only_when_not_older() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 5, 1, 1, None));
    state.apply_patch(
      SessionPatch::Actions {
        actions: actions("Old"),
      },
      1,
      4,
    );
    assert_eq!(state.actions.as_ref().unwrap().commit_label, "Commit");
    state.apply_patch(
      SessionPatch::Scm {
        scm: SessionScm {
          amend_mode: true,
          commit_message: "new".into(),
          file_filter: "f".into(),
        },
        actions: actions("New"),
      },
      1,
      6,
    );
    assert!(state.amend_mode);
    assert_eq!(state.commit_message, "new");
    assert_eq!(state.session_revision, 6);
    assert_eq!(state.selected_load_id, 0);
  }

  #[test]
  fn status_event_updates_groups_without_replacing_scm_text() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 1, 1, 1, None));
    state.commit_message = "typed".into();
    state.apply_status_event(status_event("/r", 1, 1, 1, 2, "changed"));
    assert_eq!(state.status.as_ref().unwrap().groups[0].label, "changed");
    assert_eq!(state.commit_message, "typed");
    assert_eq!(state.actions.as_ref().unwrap().commit_label, "Event");
  }

  #[test]
  fn older_status_event_keeps_newer_patch_actions_but_applies_newer_groups() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 1, 1, 1, None));
    state.apply_patch(
      SessionPatch::Actions {
        actions: actions("Patched"),
      },
      1,
      3,
    );
    state.apply_status_event(status_event("/r", 1, 2, 1, 2, "late"));
    assert_eq!(state.actions.as_ref().unwrap().commit_label, "Patched");
    assert_eq!(state.status.as_ref().unwrap().groups[0].label, "late");
    assert_eq!(state.session_revision, 3);
  }

  #[test]
  fn status_event_from_another_root_is_ignored_for_groups() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 1, 1, 1, None));
    state.apply_status_event(status_event("/other", 1, 2, 5, 5, "foreign"));
    assert_eq!(state.status.as_ref().unwrap().groups[0].label, "snapshot");
    assert_eq!(state.status_revision, 1);
  }

  #[test]
  fn pending_clear_file_keeps_the_previous_selection_until_the_ack() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 1, 1, 1, Some(file("a"))));
    state.pending_clear_file = true;
    let mut event = status_event("/r", 1, 2, 1, 2, "x");
    event.selection.file = Some(file("b"));
    state.apply_status_event(event);
    assert_eq!(state.selected_file.as_ref().unwrap().path, "a");
    state.apply_ack(Some(1), Some(3), true);
    assert!(state.selected_file.is_none());
    assert_eq!(state.selected_load_id, 2);
    assert!(!state.pending_clear_file);
  }

  #[test]
  fn ack_does_not_rewind_the_watermark() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 1, 5, 1, 1, None));
    state.apply_ack(Some(1), Some(4), false);
    assert_eq!(state.session_revision, 5);
    state.apply_ack(Some(1), Some(7), false);
    assert_eq!(state.session_revision, 7);
  }

  #[test]
  fn payload_verdicts_follow_generation_root_and_revision() {
    let mut state = RepoState::default();
    state.apply_snapshot(snapshot("/r", 2, 3, 1, 1, None));
    assert_eq!(state.accept_payload(1, 9, Some("/r")), PayloadVerdict::Reject);
    assert_eq!(state.accept_payload(2, 2, Some("/r")), PayloadVerdict::Reject);
    assert_eq!(state.accept_payload(2, 3, Some("/other")), PayloadVerdict::Reject);
    assert_eq!(state.accept_payload(2, 3, Some("/r")), PayloadVerdict::Accept);
    assert_eq!(state.session_revision, 3);
    assert_eq!(state.accept_payload(2, 5, None), PayloadVerdict::Accept);
    assert_eq!(state.session_revision, 5);
  }

  #[test]
  fn counts_and_busy_flags_read_the_groups() {
    use deathpush_core::types::{FileEntry, FileStatus};

    fn file(path: &str, status: FileStatus) -> FileEntry {
      FileEntry {
        path: path.into(),
        status,
        rename_path: None,
      }
    }

    let mut state = RepoState::default();
    assert!(!state.has_changes());
    state.status = Some(RepositoryStatus {
      root: "/r".into(),
      head_branch: Some("main".into()),
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups: vec![
        ResourceGroup {
          kind: ResourceGroupKind::Index,
          label: "Staged Changes".into(),
          files: vec![file("a.rs", FileStatus::IndexModified)],
        },
        ResourceGroup {
          kind: ResourceGroupKind::WorkingTree,
          label: "Changes".into(),
          files: vec![file("b.rs", FileStatus::Modified), file("c.rs", FileStatus::Untracked)],
        },
        ResourceGroup {
          kind: ResourceGroupKind::Merge,
          label: "Merge Changes".into(),
          files: vec![],
        },
      ],
      operation_state: RepoOperationState::None,
    });
    assert!(state.has_changes());
    assert_eq!(
      (state.staged_count(), state.unstaged_count(), state.merge_count()),
      (1, 2, 0)
    );
    assert!(!state.network_busy());
    state.running.insert(NetworkOp::Fetch);
    assert!(state.network_busy());
  }

  #[test]
  fn extras_apply_only_the_present_lists() {
    let mut state = RepoState::default();
    let mut snap = snapshot("/r", 1, 1, 1, 1, None);
    snap.branches = vec![BranchEntry {
      name: "main".into(),
      is_head: true,
      is_remote: false,
      upstream: None,
      ahead: 0,
      behind: 0,
    }];
    state.apply_snapshot(snap);
    let mut event = status_event("/r", 1, 2, 1, 2, "x");
    event.extras = Some(SessionStatusExtras {
      last_commit: None,
      branches: None,
      tags: Some(vec![]),
      commit_log: Some(vec![]),
      stashes: Some(vec![StashEntry {
        index: 0,
        message: "wip".into(),
      }]),
    });
    state.apply_status_event(event);
    assert_eq!(state.branches.len(), 1);
    assert_eq!(state.stashes.len(), 1);
  }

  #[test]
  fn committing_survives_confirmation_and_clears_otherwise() {
    let mut state = RepoState::default();
    assert!(!state.committing);
    state.mark_commit_intent(&Intent::Commit { confirmed: false });
    assert!(state.committing);
    state.resolve_commit_outcome(true);
    assert!(state.committing);
    state.mark_commit_intent(&Intent::Commit { confirmed: true });
    assert!(state.committing);
    state.resolve_commit_outcome(false);
    assert!(!state.committing);
    state.mark_commit_intent(&Intent::StageAll);
    assert!(!state.committing);
    state.mark_commit_intent(&Intent::CommitAndPush { confirmed: false });
    assert!(state.committing);
    state.resolve_commit_outcome(false);
    state.mark_commit_intent(&Intent::CommitAndSync { confirmed: false });
    assert!(state.committing);
  }
}
