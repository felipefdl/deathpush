use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::git::blame::{get_file_blame, get_file_log};

use crate::git::cli::GitCli;
use crate::git::diff::{ScmFileDiff, commit_file_diff, scm_file_diff};
use crate::git::hunk::{find_hunk_index, generate_hunk_patch_from_hunks, generate_lines_patch, hunk_id};
use crate::git::log::{get_commit_detail, get_commit_log};
use crate::git::repository::GitRepository;
use crate::types::{RepositoryStatus, ResourceGroupKind};

use super::policy::{
  OperationRoute, classify_discard, confirmation_required, derive_actions, discard_confirmation_message,
  expand_resource_paths, files_from_groups, operation_abort, operation_continue, operation_skip,
  push_needs_confirmation, reset_needs_confirmation, should_stage_all_before_commit, sync_kind_after_commit,
  unstaged_files,
};
use super::registry::{SessionAccess, SessionState};
use super::types::{
  COMMIT_LOG_PAGE, DEFAULT_REMOTE, DiffHunkPayload, DiffPayload, FileSelection, Intent, IntentOutcome, SessionActions,
  SessionPatch, SessionScm, SyncKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefreshImpact {
  StatusPaths { paths: Vec<String> },
  StatusRepository,
  Refs,
  Stashes,
  StatusAndStashes { paths: Option<Vec<String>> },
  Snapshot,
}

#[derive(Debug)]
pub enum ApplyOutput {
  Diff(DiffPayload),
  Blame(crate::types::FileBlame),
  NeedsConfirmation { action: String, message: String },
  Ack,
  Patch(SessionPatch),
  Refresh(RefreshImpact),
}

#[derive(Debug)]
pub enum FinishApply {
  Immediate(Box<IntentOutcome>),
  Refresh(RefreshImpact),
}

pub fn finish_apply(output: ApplyOutput) -> FinishApply {
  match output {
    ApplyOutput::Ack => FinishApply::Immediate(Box::new(IntentOutcome::Ack {
      session_generation: None,
      session_revision: None,
    })),
    ApplyOutput::Patch(patch) => FinishApply::Immediate(Box::new(IntentOutcome::Patch {
      patch,
      session_generation: 0,
      session_revision: 0,
    })),
    ApplyOutput::Diff(payload) => FinishApply::Immediate(Box::new(IntentOutcome::Diff {
      payload,
      session_generation: 0,
      session_revision: 0,
    })),
    ApplyOutput::Blame(payload) => FinishApply::Immediate(Box::new(IntentOutcome::Blame {
      payload,
      session_generation: 0,
      session_revision: 0,
    })),
    ApplyOutput::NeedsConfirmation { action, message } => {
      FinishApply::Immediate(Box::new(IntentOutcome::NeedsConfirmation { action, message }))
    }
    ApplyOutput::Refresh(impact) => FinishApply::Refresh(impact),
  }
}

pub fn outcome_should_bump(intent: &Intent, output: &ApplyOutput) -> bool {
  match output {
    ApplyOutput::NeedsConfirmation { .. } => false,
    ApplyOutput::Ack => matches!(intent, Intent::ClearFile),
    ApplyOutput::Patch(_) | ApplyOutput::Diff(_) | ApplyOutput::Blame(_) | ApplyOutput::Refresh(_) => true,
  }
}

pub fn stamp_outcome(outcome: IntentOutcome, session_generation: u64, session_revision: u64) -> IntentOutcome {
  match outcome {
    IntentOutcome::Patch { patch, .. } => IntentOutcome::Patch {
      patch,
      session_generation,
      session_revision,
    },
    IntentOutcome::Diff { payload, .. } => IntentOutcome::Diff {
      payload,
      session_generation,
      session_revision,
    },
    IntentOutcome::Blame { payload, .. } => IntentOutcome::Blame {
      payload,
      session_generation,
      session_revision,
    },
    IntentOutcome::Ack { .. } => IntentOutcome::Ack {
      session_generation: Some(session_generation),
      session_revision: Some(session_revision),
    },
    other => other,
  }
}

pub async fn apply_intent(
  intent: Intent,
  root: &Path,
  status: &RepositoryStatus,
  session: &mut impl SessionAccess,
) -> Result<ApplyOutput> {
  session.with_mut(|state| state.error = None)?;
  let cli = GitCli::new(root);
  match intent {
    Intent::OpenRepository { .. }
    | Intent::InitRepository { .. }
    | Intent::CloneRepository { .. }
    | Intent::RefreshStatus => Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot)),

    Intent::ClearFile => {
      session.with_mut(|state| {
        state.selection = None;
        state.diff_path = None;
      })?;
      Ok(ApplyOutput::Ack)
    }
    Intent::SetAmend { enabled } => {
      session.with_mut(|state| state.amend_mode = enabled)?;
      if enabled {
        match cli.get_last_commit_message().await {
          Ok(message) => session.with_mut(|state| state.commit_message = message)?,
          Err(err) => {
            session.with_mut(|state| state.amend_mode = false)?;
            return Err(err);
          }
        }
      }
      Ok(session.with_mut(|state| {
        ApplyOutput::Patch(SessionPatch::Scm {
          scm: session_scm(state),
          actions: session_actions(status, state),
        })
      })?)
    }
    Intent::SetCommitMessage { message } => Ok(session.with_mut(|state| {
      state.commit_message = message;
      ApplyOutput::Patch(SessionPatch::Scm {
        scm: session_scm(state),
        actions: session_actions(status, state),
      })
    })?),
    Intent::SetFileFilter { filter } => {
      session.with_mut(|state| state.file_filter = filter)?;
      Ok(ApplyOutput::Ack)
    }
    Intent::Stage { paths } => {
      let files = files_from_groups(&status.groups);
      let expanded = expand_resource_paths(&files, &paths);
      if expanded.is_empty() {
        return Ok(ApplyOutput::Ack);
      }
      cli.stage_files(&expanded).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: expanded }))
    }
    Intent::StageAll => {
      cli.stage_all().await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusRepository))
    }
    Intent::Unstage { paths } => {
      let files = files_from_groups(&status.groups);
      let expanded = expand_resource_paths(&files, &paths);
      if expanded.is_empty() {
        return Ok(ApplyOutput::Ack);
      }
      cli.unstage_files(&expanded).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: expanded }))
    }
    Intent::UnstageAll => {
      cli.unstage_all().await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusRepository))
    }
    Intent::Discard { paths, confirmed } => {
      let repository_wide = paths.is_empty();
      let files = if repository_wide {
        unstaged_files(&status.groups)
      } else {
        files_from_groups(&status.groups)
      };
      let selected = if repository_wide {
        files.iter().map(|(path, _)| path.clone()).collect()
      } else {
        paths
      };
      let plan = classify_discard(&files, &selected);
      if confirmation_required(confirmed) {
        let (message, action) = discard_confirmation_message(&plan);
        return Ok(ApplyOutput::NeedsConfirmation { action, message });
      }
      if !plan.tracked.is_empty() {
        cli.discard_changes(&plan.tracked).await?;
      }
      if !plan.untracked.is_empty() {
        trash_paths(root, &plan.untracked)?;
      }
      let mut changed = plan.tracked;
      changed.extend(plan.untracked);
      if repository_wide {
        Ok(ApplyOutput::Refresh(RefreshImpact::StatusRepository))
      } else {
        Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: changed }))
      }
    }
    Intent::Commit { confirmed } => commit_current(&cli, status, session, confirmed).await,
    Intent::CommitAndPush { confirmed } => follow_commit(&cli, status, session, confirmed, FollowUp::Push).await,
    Intent::CommitAndSync { confirmed } => follow_commit(&cli, status, session, confirmed, FollowUp::Sync).await,
    Intent::Sync => sync_current(&cli, status).await,
    Intent::Push { force, confirmed } => push_current(&cli, status, force, confirmed).await,
    Intent::Pull { rebase } => {
      let branch = current_branch(status)?;
      cli.pull(DEFAULT_REMOTE, &branch, rebase).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::Fetch { prune } => {
      cli.fetch(DEFAULT_REMOTE, prune).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::UndoCommit { confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "undoCommit".into(),
          message: "Undo last commit? Changes will be moved back to staging.".into(),
        });
      }
      cli.undo_last_commit().await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::OperationContinue => run_operation(&cli, operation_continue(status.operation_state), "continue").await,
    Intent::OperationAbort => run_operation(&cli, operation_abort(status.operation_state), "abort").await,
    Intent::OperationSkip => run_operation(&cli, operation_skip(status.operation_state), "skip").await,
    Intent::StageHunk { hunk_id } => {
      let (path, _) = session.with_mut(|state| hunk_target(state))??;
      apply_named_hunk(root, &cli, &path, &hunk_id, false, true, false).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::UnstageHunk { hunk_id } => {
      let (path, _) = session.with_mut(|state| hunk_target(state))??;
      apply_named_hunk(root, &cli, &path, &hunk_id, true, true, true).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::DiscardHunk { hunk_id, confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "discardHunk".into(),
          message: "Discard this hunk? This action is irreversible.".into(),
        });
      }
      let (path, _) = session.with_mut(|state| hunk_target(state))??;
      apply_named_hunk(root, &cli, &path, &hunk_id, false, false, true).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::StageLines {
      path,
      start,
      end,
      staged,
    } => {
      apply_line_range(root, &cli, &path, start, end, staged).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::OpenScmDiff {
      path,
      staged,
      group_kind,
    } => {
      session.with_mut(|state| {
        if let Some(group_kind) = group_kind {
          state.selection = Some(FileSelection {
            path: path.clone(),
            staged,
            group_kind,
          });
        }
        state.diff_path = Some(path.clone());
        state.diff_staged = staged;
      })?;
      let repo = GitRepository::open(root)?;
      let file = scm_file_diff(&repo, &path, staged)?;
      let merge_text = if group_kind == Some(ResourceGroupKind::Merge) && file.content.file_type == "text" {
        read_workdir_text(root, &path)
      } else {
        None
      };
      Ok(ApplyOutput::Diff(diff_payload(file, staged, merge_text)))
    }
    Intent::OpenCommitDiff { commit, path } => {
      let repo = GitRepository::open(root)?;
      Ok(ApplyOutput::Diff(diff_payload(
        commit_file_diff(&repo, &commit, &path)?,
        false,
        None,
      )))
    }
    Intent::OpenBlame { path } => Ok(ApplyOutput::Blame(get_file_blame(root, &path).await?)),

    Intent::ResolveConflict { path, contents } => {
      write_repo_file(root, &path, &contents)?;
      cli.stage_files(std::slice::from_ref(&path)).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::StashSave {
      include_untracked,
      staged_only,
      message,
    } => {
      let msg = message.as_deref();
      if staged_only {
        cli.stash_save_staged(msg).await?;
      } else if include_untracked {
        cli.stash_save_include_untracked(msg).await?;
      } else {
        cli.stash_save(msg).await?;
      }
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusAndStashes { paths: None }))
    }
    Intent::StashApply { index } => {
      cli.stash_apply(index).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusAndStashes { paths: None }))
    }
    Intent::StashPop { index } => {
      cli.stash_pop(index).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusAndStashes { paths: None }))
    }
    Intent::StashDrop { index, confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "stashDrop".into(),
          message: "Drop this stash?".into(),
        });
      }
      cli.stash_drop(index).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Stashes))
    }
    Intent::CheckoutBranch { name } => {
      cli.checkout_branch(&name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::CreateBranch { name, from } => {
      cli.create_branch(&name, from.as_deref()).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::DeleteBranch { name, force, confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "deleteBranch".into(),
          message: format!("Delete branch \"{name}\"?"),
        });
      }
      cli.delete_branch(&name, force).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::RenameBranch { old_name, new_name } => {
      cli.rename_branch(&old_name, &new_name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::MergeBranch { name } => {
      cli.merge_branch(&name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::RebaseBranch { name } => {
      cli.rebase_branch(&name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::DeleteRemoteBranch { name } => {
      cli.delete_remote_branch(DEFAULT_REMOTE, &name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::CreateTag { name, message, commit } => {
      cli.create_tag(&name, message.as_deref(), commit.as_deref()).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::DeleteTag { name, confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "deleteTag".into(),
          message: format!("Delete tag \"{name}\"?"),
        });
      }
      cli.delete_tag(&name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::PushTag { name } => {
      cli.push_tag(DEFAULT_REMOTE, &name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::DeleteRemoteTag { name } => {
      cli.delete_remote_tag(DEFAULT_REMOTE, &name).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Refs))
    }
    Intent::CherryPick { commit } => {
      cli.cherry_pick(&commit).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::Reset {
      commit,
      mode,
      confirmed,
    } => {
      if reset_needs_confirmation(&mode, confirmed) {
        let message = if mode == "hard" {
          "Hard reset discards all uncommitted changes. This action is irreversible.".into()
        } else {
          "Reset the current branch to this commit?".into()
        };
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "reset".into(),
          message,
        });
      }
      cli.reset_to_commit(&commit, &mode).await?;
      Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
    }
    Intent::LoadMoreLog => {
      let (skip, history_path) = session.with_mut(|state| (state.commit_log.len(), state.file_history_path.clone()))?;
      let more = if let Some(path) = history_path {
        get_file_log(root, &path, skip, COMMIT_LOG_PAGE)
          .await
          .unwrap_or_default()
      } else {
        let repo = GitRepository::open(root)?;
        get_commit_log(&repo, skip, COMMIT_LOG_PAGE).unwrap_or_default()
      };
      Ok(session.with_mut(|state| {
        state.commit_log.extend(more);
        ApplyOutput::Patch(SessionPatch::CommitLog {
          commit_log: state.commit_log.clone(),
        })
      })?)
    }
    Intent::OpenFileHistory { path } => {
      let commit_log = get_file_log(root, &path, 0, COMMIT_LOG_PAGE).await.unwrap_or_default();
      Ok(session.with_mut(|state| {
        state.file_history_path = Some(path.clone());
        state.commit_log = commit_log;
        state.selected_commit = None;
        state.commit_detail = None;
        ApplyOutput::Patch(SessionPatch::FileHistory {
          path: state.file_history_path.clone(),
          commit_log: state.commit_log.clone(),
        })
      })?)
    }
    Intent::ClearFileHistory => {
      let repo = GitRepository::open(root)?;
      let commit_log = get_commit_log(&repo, 0, COMMIT_LOG_PAGE).unwrap_or_default();
      Ok(session.with_mut(|state| {
        state.file_history_path = None;
        state.commit_log = commit_log;
        state.selected_commit = None;
        state.commit_detail = None;
        ApplyOutput::Patch(SessionPatch::FileHistory {
          path: None,
          commit_log: state.commit_log.clone(),
        })
      })?)
    }
    Intent::SelectCommit { id } => {
      let repo = GitRepository::open(root)?;
      let detail = get_commit_detail(&repo, &id)?;
      Ok(session.with_mut(|state| {
        state.selected_commit = Some(id.clone());
        state.commit_detail = Some(detail);
        ApplyOutput::Patch(SessionPatch::Commit {
          id: state.selected_commit.clone(),
          detail: state.commit_detail.clone(),
        })
      })?)
    }

    Intent::DeleteFile { path, confirmed } => {
      if confirmation_required(confirmed) {
        return Ok(ApplyOutput::NeedsConfirmation {
          action: "deleteFile".into(),
          message: format!("Move \"{path}\" to the trash?"),
        });
      }
      trash_paths(root, std::slice::from_ref(&path))?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths: vec![path] }))
    }
    Intent::AddToGitignore { path } => {
      add_gitignore(root, &path)?;
      Ok(ApplyOutput::Refresh(RefreshImpact::StatusPaths {
        paths: vec![".gitignore".into()],
      }))
    }
  }
}

enum FollowUp {
  Push,
  Sync,
}

async fn commit_current(
  cli: &GitCli,
  status: &RepositoryStatus,
  session: &mut impl SessionAccess,
  confirmed: bool,
) -> Result<ApplyOutput> {
  let (message, amend) = session.with_mut(|state| (state.commit_message.trim().to_string(), state.amend_mode))?;
  if message.is_empty() {
    return Err(Error::Other("Commit message is empty".into()));
  }
  if amend && confirmation_required(confirmed) {
    return Ok(ApplyOutput::NeedsConfirmation {
      action: "commit".into(),
      message: "Amend the last commit with the current message and staged changes?".into(),
    });
  }
  let staged = status
    .groups
    .iter()
    .any(|group| group.kind == ResourceGroupKind::Index && !group.files.is_empty());
  let other = status
    .groups
    .iter()
    .any(|group| group.kind != ResourceGroupKind::Index && !group.files.is_empty());
  if should_stage_all_before_commit(staged, other) {
    cli.stage_all().await?;
  }
  cli.commit(&message, amend).await?;
  session.with_mut(|state| {
    state.commit_message.clear();
    state.amend_mode = false;
  })?;
  Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
}

async fn push_current(cli: &GitCli, status: &RepositoryStatus, force: bool, confirmed: bool) -> Result<ApplyOutput> {
  if push_needs_confirmation(force, confirmed) {
    return Ok(ApplyOutput::NeedsConfirmation {
      action: "push".into(),
      message: "Force push may overwrite remote changes.".into(),
    });
  }
  let branch = current_branch(status)?;
  cli.push(DEFAULT_REMOTE, &branch, force).await?;
  Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
}

async fn follow_commit(
  cli: &GitCli,
  status: &RepositoryStatus,
  session: &mut impl SessionAccess,
  confirmed: bool,
  follow: FollowUp,
) -> Result<ApplyOutput> {
  let amend = session.with_mut(|state| state.amend_mode)?;
  let ahead = status.ahead;
  let behind = status.behind;
  match commit_current(cli, status, session, confirmed).await? {
    ApplyOutput::NeedsConfirmation { action, message } => Ok(ApplyOutput::NeedsConfirmation { action, message }),
    ApplyOutput::Diff(_) | ApplyOutput::Blame(_) => Ok(ApplyOutput::Ack),
    ApplyOutput::Refresh(_) | ApplyOutput::Ack | ApplyOutput::Patch(_) => match follow {
      FollowUp::Push => push_current(cli, status, false, true).await,
      FollowUp::Sync => run_sync_kind(cli, status, sync_kind_after_commit(ahead, behind, amend)).await,
    },
  }
}

async fn run_sync_kind(cli: &GitCli, status: &RepositoryStatus, kind: SyncKind) -> Result<ApplyOutput> {
  match kind {
    SyncKind::Fetch => {
      cli.fetch(DEFAULT_REMOTE, true).await?;
    }
    SyncKind::Pull => {
      let branch = current_branch(status)?;
      cli.pull(DEFAULT_REMOTE, &branch, false).await?;
    }
    SyncKind::Push => {
      let branch = current_branch(status)?;
      cli.push(DEFAULT_REMOTE, &branch, false).await?;
    }
    SyncKind::PullThenPush => {
      let branch = current_branch(status)?;
      cli.pull(DEFAULT_REMOTE, &branch, false).await?;
      cli.push(DEFAULT_REMOTE, &branch, false).await?;
    }
  }
  Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
}

fn add_gitignore(root: &Path, pattern: &str) -> Result<()> {
  let gitignore_path = root.join(".gitignore");
  let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
  if !content.ends_with('\n') && !content.is_empty() {
    content.push('\n');
  }
  content.push_str(pattern);
  content.push('\n');
  std::fs::write(&gitignore_path, content)?;
  Ok(())
}

async fn sync_current(cli: &GitCli, status: &RepositoryStatus) -> Result<ApplyOutput> {
  run_sync_kind(cli, status, super::policy::sync_kind(status.ahead, status.behind)).await
}

fn current_branch(status: &RepositoryStatus) -> Result<String> {
  status
    .head_branch
    .clone()
    .filter(|name| !name.is_empty() && !name.starts_with('('))
    .ok_or_else(|| Error::Other("No branch to sync".into()))
}

async fn run_operation(cli: &GitCli, route: Option<OperationRoute>, verb: &str) -> Result<ApplyOutput> {
  let Some(route) = route else {
    return Err(Error::Other(format!("No operation to {verb}")));
  };
  match route {
    OperationRoute::MergeContinue => cli.merge_continue().await?,
    OperationRoute::MergeAbort => cli.merge_abort().await?,
    OperationRoute::RebaseContinue => cli.rebase_continue().await?,
    OperationRoute::RebaseAbort => cli.rebase_abort().await?,
    OperationRoute::RebaseSkip => cli.rebase_skip().await?,
    OperationRoute::CherryPickContinue => cli.cherry_pick_continue().await?,
    OperationRoute::CherryPickAbort => cli.cherry_pick_abort().await?,
    OperationRoute::RevertContinue => cli.revert_continue().await?,
    OperationRoute::RevertAbort => cli.revert_abort().await?,
  }
  Ok(ApplyOutput::Refresh(RefreshImpact::Snapshot))
}

fn hunk_target(state: &SessionState) -> Result<(String, bool)> {
  if let Some(path) = &state.diff_path {
    return Ok((path.clone(), state.diff_staged));
  }
  state
    .selection
    .as_ref()
    .map(|selection| (selection.path.clone(), selection.staged))
    .ok_or_else(|| Error::Other("No file selected".into()))
}

async fn apply_named_hunk(
  root: &Path,
  cli: &GitCli,
  path: &str,
  id: &str,
  staged: bool,
  cached: bool,
  reverse: bool,
) -> Result<()> {
  let repo = GitRepository::open(root)?;
  let file = scm_file_diff(&repo, path, staged)?;
  let index = find_hunk_index(&file.hunks, id).ok_or_else(|| Error::Other("hunk not found".into()))?;
  let patch = generate_hunk_patch_from_hunks(path, &file.hunks, index)?;
  cli.apply_patch(&patch, cached, reverse).await
}

async fn apply_line_range(root: &Path, cli: &GitCli, path: &str, start: usize, end: usize, staged: bool) -> Result<()> {
  let repo = GitRepository::open(root)?;
  let mut file = scm_file_diff(&repo, path, staged)?;
  let original = file.hunks.clone();
  let calls = line_range_calls(&original, start, end);
  for (index, (hunk_index, line_start, line_end)) in calls.iter().enumerate() {
    let hunks = &file.hunks;
    let id = hunk_id(&original[*hunk_index]);
    let Some(current_index) = find_hunk_index(hunks, &id).or(if *hunk_index < hunks.len() {
      Some(*hunk_index)
    } else {
      None
    }) else {
      continue;
    };
    let source = generate_hunk_patch_from_hunks(path, hunks, current_index)?;
    let patch = generate_lines_patch(path, &source, 0, *line_start, *line_end)?;
    if staged {
      cli.apply_patch(&patch, true, true).await?;
    } else {
      cli.apply_patch(&patch, true, false).await?;
    }
    if index + 1 < calls.len() {
      file = scm_file_diff(&repo, path, staged)?;
    }
  }
  Ok(())
}

fn line_range_calls(hunks: &[crate::types::DiffHunk], start: usize, end: usize) -> Vec<(usize, usize, usize)> {
  let lo = start.min(end);
  let hi = start.max(end);
  let mut calls = Vec::new();
  for (hunk_index, hunk) in hunks.iter().enumerate() {
    let mut indexes = Vec::new();
    for (line_index, line) in hunk.lines.iter().enumerate() {
      let hit = match line.line_type.as_str() {
        "add" => line.new_line_number.is_some_and(|number| number >= lo && number <= hi),
        "remove" => line.old_line_number.is_some_and(|number| number >= lo && number <= hi),
        _ => line
          .new_line_number
          .or(line.old_line_number)
          .is_some_and(|number| number >= lo && number <= hi),
      };
      if hit {
        indexes.push(line_index);
      }
    }
    if let (Some(first), Some(last)) = (indexes.first(), indexes.last()) {
      calls.push((hunk_index, *first, *last));
    }
  }
  calls
}

/// The working-tree text of `path`, when it is inside the repository and valid UTF-8.
fn read_workdir_text(root: &Path, path: &str) -> Option<String> {
  let canon_root = root.canonicalize().ok()?;
  let full = root.join(path).canonicalize().ok()?;
  if !full.starts_with(&canon_root) {
    return None;
  }
  std::fs::read_to_string(full).ok()
}

fn diff_payload(file: ScmFileDiff, staged: bool, merge_text: Option<String>) -> DiffPayload {
  let is_image = file.content.file_type == "image";
  let (old_image, new_image) = if is_image {
    (file.content.original, file.content.modified)
  } else {
    (String::new(), String::new())
  };
  let content_hash = crate::content_hash::sha256_utf8(
    &[
      file.patch.as_str(),
      old_image.as_str(),
      new_image.as_str(),
      merge_text.as_deref().unwrap_or(""),
    ]
    .join("\0"),
  );
  DiffPayload {
    path: file.content.path,
    patch: file.patch,
    merge_text,
    old_image,
    new_image,
    language: file.content.original_language,
    file_type: file.content.file_type,
    hunks: file.hunks.iter().map(DiffHunkPayload::from).collect(),
    staged,
    content_hash,
  }
}

fn trash_paths(root: &Path, paths: &[String]) -> Result<()> {
  let canon_root = root
    .canonicalize()
    .map_err(|err| Error::Other(format!("Cannot resolve repository root: {err}")))?;
  for relative in paths {
    let full = root
      .join(relative)
      .canonicalize()
      .map_err(|err| Error::Other(format!("Cannot resolve file path: {err}")))?;
    if !full.starts_with(&canon_root) {
      return Err(Error::Other("Path traversal denied".into()));
    }
    trash::delete(&full).map_err(|err| Error::Other(err.to_string()))?;
  }
  Ok(())
}

fn write_repo_file(root: &Path, relative: &str, contents: &str) -> Result<()> {
  let canon_root = root
    .canonicalize()
    .map_err(|err| Error::Other(format!("Cannot resolve repository root: {err}")))?;
  let full: PathBuf = root.join(relative);
  if let Some(parent) = full.parent() {
    std::fs::create_dir_all(parent)?;
  }
  if full.exists() {
    let canon = full
      .canonicalize()
      .map_err(|err| Error::Other(format!("Cannot resolve file path: {err}")))?;
    if !canon.starts_with(&canon_root) {
      return Err(Error::Other("Path traversal denied".into()));
    }
  } else {
    let parent = full.parent().unwrap_or(&full);
    let canon_parent = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
    if !canon_parent.starts_with(&canon_root) {
      return Err(Error::Other("Path traversal denied".into()));
    }
  }
  std::fs::write(&full, contents)?;
  Ok(())
}

fn session_actions(status: &RepositoryStatus, state: &SessionState) -> SessionActions {
  derive_actions(
    &status.groups,
    &state.commit_message,
    state.amend_mode,
    status.ahead,
    status.behind,
    status.head_branch.is_some(),
    status.operation_state,
  )
}

fn session_scm(state: &SessionState) -> SessionScm {
  SessionScm {
    amend_mode: state.amend_mode,
    commit_message: state.commit_message.clone(),
    file_filter: state.file_filter.clone(),
  }
}

#[cfg(test)]
mod tests {
  use std::path::Path;

  use super::{
    ApplyOutput, FinishApply, RefreshImpact, apply_intent, diff_payload, finish_apply, outcome_should_bump,
    scm_file_diff, stamp_outcome,
  };
  use crate::git::cli::GitCli;
  use crate::git::repository::GitRepository;
  use crate::session::registry::SessionState;
  use crate::session::types::{
    DiffPayload, FileSelection, Intent, IntentOutcome, OperationActions, SessionActions, SessionPatch, SyncAction,
    SyncKind,
  };
  use crate::types::{
    FileBlame, FileEntry, FileStatus, RepoOperationState, RepositoryStatus, ResourceGroup, ResourceGroupKind,
  };

  fn empty_status(root: &str) -> RepositoryStatus {
    RepositoryStatus {
      root: root.into(),
      has_repository: true,
      head_branch: None,
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups: vec![],
      operation_state: RepoOperationState::None,
    }
  }

  fn dummy_actions() -> SessionActions {
    SessionActions {
      can_commit: false,
      commit_label: "Commit".into(),
      commit_destructive: false,
      can_stage_all: false,
      can_unstage_all: false,
      can_discard_all: false,
      discard_all_destructive: true,
      sync: SyncAction {
        enabled: true,
        kind: SyncKind::Fetch,
        destructive: false,
      },
      operation: OperationActions {
        continue_op: false,
        abort: false,
        skip: false,
        abort_destructive: true,
      },
    }
  }

  fn init_repo() -> (tempfile::TempDir, String) {
    let directory = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    let root = repo.workdir().unwrap();
    std::fs::write(root.join("README.md"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, "initial\n", &tree, &[]).unwrap();
    (directory, oid.to_string())
  }

  #[tokio::test]
  async fn set_commit_message_patches_acknowledged_message_and_actions() {
    let mut state = SessionState::default();
    let output = apply_intent(
      Intent::SetCommitMessage { message: "wip".into() },
      Path::new("/tmp"),
      &empty_status("/tmp"),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::Scm { scm, actions }) => {
        assert_eq!(scm.commit_message, "wip");
        assert!(!actions.can_commit);
      }
      other => panic!("{other:?}"),
    }
    assert_eq!(state.commit_message, "wip");
  }

  #[tokio::test]
  async fn set_file_filter_is_ack() {
    let mut state = SessionState::default();
    let output = apply_intent(
      Intent::SetFileFilter { filter: "foo".into() },
      Path::new("/tmp"),
      &empty_status("/tmp"),
      &mut state,
    )
    .await
    .unwrap();
    assert!(matches!(output, ApplyOutput::Ack));
    assert_eq!(state.file_filter, "foo");
  }

  #[tokio::test]
  async fn empty_stage_is_ack() {
    let output = apply_intent(
      Intent::Stage {
        paths: vec!["missing.rs".into()],
      },
      Path::new("/tmp"),
      &empty_status("/tmp"),
      &mut SessionState::default(),
    )
    .await
    .unwrap();
    assert!(matches!(output, ApplyOutput::Ack));
  }

  #[tokio::test]
  async fn set_amend_patches_scm_with_last_commit_message() {
    let (directory, _) = init_repo();
    let mut state = SessionState::default();
    let root = directory.path();
    let output = apply_intent(
      Intent::SetAmend { enabled: true },
      root,
      &empty_status(&root.to_string_lossy()),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::Scm { scm, actions }) => {
        assert!(scm.amend_mode);
        assert_eq!(scm.commit_message.trim(), "initial");
        assert!(actions.commit_destructive);
      }
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn load_more_log_patches_commit_log() {
    let (directory, _) = init_repo();
    let mut state = SessionState::default();
    let root = directory.path();
    let output = apply_intent(
      Intent::LoadMoreLog,
      root,
      &empty_status(&root.to_string_lossy()),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::CommitLog { commit_log }) => {
        assert_eq!(commit_log.len(), 1);
        assert_eq!(commit_log[0].message.trim(), "initial");
      }
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn open_file_history_patches_file_history() {
    let (directory, _) = init_repo();
    let mut state = SessionState {
      selected_commit: Some("abc".into()),
      ..SessionState::default()
    };
    let root = directory.path();
    let output = apply_intent(
      Intent::OpenFileHistory {
        path: "README.md".into(),
      },
      root,
      &empty_status(&root.to_string_lossy()),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::FileHistory { path, commit_log }) => {
        assert_eq!(path.as_deref(), Some("README.md"));
        assert_eq!(commit_log.len(), 1);
      }
      other => panic!("{other:?}"),
    }
    assert!(state.selected_commit.is_none());
    assert!(state.commit_detail.is_none());
  }

  #[tokio::test]
  async fn clear_file_history_patches_head_log() {
    let (directory, _) = init_repo();
    let mut state = SessionState {
      file_history_path: Some("README.md".into()),
      selected_commit: Some("abc".into()),
      ..SessionState::default()
    };
    let root = directory.path();
    let output = apply_intent(
      Intent::ClearFileHistory,
      root,
      &empty_status(&root.to_string_lossy()),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::FileHistory { path, commit_log }) => {
        assert!(path.is_none());
        assert_eq!(commit_log.len(), 1);
      }
      other => panic!("{other:?}"),
    }
    assert!(state.file_history_path.is_none());
    assert!(state.selected_commit.is_none());
    assert!(state.commit_detail.is_none());
  }

  #[tokio::test]
  async fn select_commit_patches_commit_detail() {
    let (directory, id) = init_repo();
    let mut state = SessionState::default();
    let root = directory.path();
    let output = apply_intent(
      Intent::SelectCommit { id: id.clone() },
      root,
      &empty_status(&root.to_string_lossy()),
      &mut state,
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Patch(SessionPatch::Commit { id: selected, detail }) => {
        assert_eq!(selected.as_deref(), Some(id.as_str()));
        assert!(detail.is_some());
      }
      other => panic!("{other:?}"),
    }
  }

  #[test]
  fn ack_and_patch_never_request_a_snapshot() {
    match finish_apply(ApplyOutput::Ack) {
      FinishApply::Immediate(outcome) => assert!(matches!(*outcome, IntentOutcome::Ack { .. })),
      other => panic!("{other:?}"),
    }
    match finish_apply(ApplyOutput::Patch(SessionPatch::Actions {
      actions: dummy_actions(),
    })) {
      FinishApply::Immediate(outcome) => assert!(matches!(*outcome, IntentOutcome::Patch { .. })),
      other => panic!("{other:?}"),
    }
  }

  #[test]
  fn finish_apply_status_paths_is_not_a_snapshot() {
    match finish_apply(ApplyOutput::Refresh(RefreshImpact::StatusPaths {
      paths: vec!["a.rs".into()],
    })) {
      FinishApply::Refresh(RefreshImpact::StatusPaths { paths }) => assert_eq!(paths, ["a.rs"]),
      other => panic!("{other:?}"),
    }
  }

  #[test]
  fn snapshot_refresh_stays_snapshot() {
    match finish_apply(ApplyOutput::Refresh(RefreshImpact::Snapshot)) {
      FinishApply::Refresh(RefreshImpact::Snapshot) => {}
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn a_staged_hunk_leaves_the_working_diff_and_enters_the_staged_one() {
    let (directory, _) = init_repo();
    let root = directory.path();
    let numbered: String = (1..=40).map(|n| format!("line {n}\n")).collect();
    std::fs::write(root.join("many.txt"), &numbered).unwrap();
    let cli = GitCli::new(root);
    cli.stage_files(&["many.txt".to_string()]).await.unwrap();
    cli.commit("many", false).await.unwrap();
    let changed = numbered
      .replace("line 3\n", "line three\n")
      .replace("line 38\n", "line thirty-eight\n");
    std::fs::write(root.join("many.txt"), changed).unwrap();

    let repo = GitRepository::open(root).unwrap();
    let payload = diff_payload(scm_file_diff(&repo, "many.txt", false).unwrap(), false, None);
    assert_eq!(payload.hunks.len(), 2);
    assert!(payload.patch.contains("+line thirty-eight"));

    let mut state = SessionState {
      diff_path: Some("many.txt".into()),
      ..SessionState::default()
    };
    let status = empty_status(&root.to_string_lossy());
    apply_intent(
      Intent::StageHunk {
        hunk_id: payload.hunks[1].id.clone(),
      },
      root,
      &status,
      &mut state,
    )
    .await
    .unwrap();

    let working = diff_payload(scm_file_diff(&repo, "many.txt", false).unwrap(), false, None);
    assert_eq!(working.hunks.len(), 1);
    assert!(working.patch.contains("+line three"));
    assert!(!working.patch.contains("thirty-eight"));
    let staged = diff_payload(scm_file_diff(&repo, "many.txt", true).unwrap(), true, None);
    assert_eq!(staged.hunks.len(), 1);
    assert!(staged.patch.contains("+line thirty-eight"));
    assert_ne!(working.content_hash, staged.content_hash);
  }

  #[tokio::test]
  async fn a_merge_file_carries_its_conflict_markers() {
    let (directory, _) = init_repo();
    let root = directory.path();
    let git = |args: &[&str]| {
      let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
      output.status.success()
    };
    let base = git2::Repository::open(root)
      .unwrap()
      .head()
      .unwrap()
      .shorthand()
      .unwrap()
      .to_string();
    assert!(git(&["checkout", "-q", "-b", "other"]));
    std::fs::write(root.join("README.md"), "other\n").unwrap();
    assert!(git(&["commit", "-qam", "other"]));
    assert!(git(&["checkout", "-q", &base]));
    std::fs::write(root.join("README.md"), "mine\n").unwrap();
    assert!(git(&["commit", "-qam", "mine"]));
    assert!(!git(&["merge", "other"]), "the merge must conflict");

    let status = empty_status(&root.to_string_lossy());
    let mut state = SessionState::default();
    let open = |group_kind| Intent::OpenScmDiff {
      path: "README.md".into(),
      staged: false,
      group_kind: Some(group_kind),
    };
    let ApplyOutput::Diff(merge) = apply_intent(open(ResourceGroupKind::Merge), root, &status, &mut state)
      .await
      .unwrap()
    else {
      panic!("expected a diff");
    };
    let text = merge.merge_text.expect("a merge file carries its working text");
    assert!(text.contains("<<<<<<<") && text.contains("mine") && text.contains("other"));

    let ApplyOutput::Diff(working) = apply_intent(open(ResourceGroupKind::WorkingTree), root, &status, &mut state)
      .await
      .unwrap()
    else {
      panic!("expected a diff");
    };
    assert!(working.merge_text.is_none());
  }

  #[tokio::test]
  async fn stage_non_empty_is_status_paths() {
    let (directory, _) = init_repo();
    let root = directory.path();
    std::fs::write(root.join("README.md"), "hello\nworld\n").unwrap();
    let mut status = empty_status(&root.to_string_lossy());
    status.groups = vec![ResourceGroup {
      kind: ResourceGroupKind::WorkingTree,
      label: "Changes".into(),
      files: vec![FileEntry {
        path: "README.md".into(),
        status: FileStatus::Modified,
        rename_path: None,
      }],
    }];
    let output = apply_intent(
      Intent::Stage {
        paths: vec!["README.md".into()],
      },
      root,
      &status,
      &mut SessionState::default(),
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Refresh(RefreshImpact::StatusPaths { paths }) => assert_eq!(paths, ["README.md"]),
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn reset_prompts_for_every_mode() {
    let (directory, id) = init_repo();
    let root = directory.path();
    for mode in ["soft", "mixed", "hard"] {
      let output = apply_intent(
        Intent::Reset {
          commit: id.clone(),
          mode: mode.into(),
          confirmed: false,
        },
        root,
        &empty_status(&root.to_string_lossy()),
        &mut SessionState::default(),
      )
      .await
      .unwrap();
      match output {
        ApplyOutput::NeedsConfirmation { message, .. } if mode == "hard" => {
          assert_eq!(
            message,
            "Hard reset discards all uncommitted changes. This action is irreversible."
          );
        }
        ApplyOutput::NeedsConfirmation { message, .. } => {
          assert_eq!(message, "Reset the current branch to this commit?");
        }
        other => panic!("{mode}: {other:?}"),
      }
    }
  }

  #[tokio::test]
  async fn create_branch_is_snapshot() {
    let (directory, _) = init_repo();
    let root = directory.path();
    let output = apply_intent(
      Intent::CreateBranch {
        name: "feat".into(),
        from: None,
      },
      root,
      &empty_status(&root.to_string_lossy()),
      &mut SessionState::default(),
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Refresh(RefreshImpact::Snapshot) => {}
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn stash_drop_is_stashes() {
    let (directory, _) = init_repo();
    let root = directory.path();
    std::fs::write(root.join("README.md"), "hello\nstash me\n").unwrap();
    GitCli::new(root).stash_save(Some("wip")).await.unwrap();
    let output = apply_intent(
      Intent::StashDrop {
        index: 0,
        confirmed: true,
      },
      root,
      &empty_status(&root.to_string_lossy()),
      &mut SessionState::default(),
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Refresh(RefreshImpact::Stashes) => {}
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn open_repository_is_snapshot() {
    let output = apply_intent(
      Intent::OpenRepository {
        path: "/tmp/repo".into(),
      },
      Path::new("/tmp"),
      &empty_status("/tmp"),
      &mut SessionState::default(),
    )
    .await
    .unwrap();
    match output {
      ApplyOutput::Refresh(RefreshImpact::Snapshot) => {}
      other => panic!("{other:?}"),
    }
  }

  #[test]
  fn patch_outcome_stamps_session_revision() {
    let FinishApply::Immediate(outcome) = finish_apply(ApplyOutput::Patch(SessionPatch::Actions {
      actions: dummy_actions(),
    })) else {
      panic!("expected immediate patch");
    };
    match stamp_outcome(*outcome, 3, 11) {
      IntentOutcome::Patch {
        session_generation: 3,
        session_revision: 11,
        ..
      } => {}
      other => panic!("{other:?}"),
    }
  }

  #[test]
  fn non_patch_outcomes_keep_their_shape() {
    assert!(matches!(
      stamp_outcome(
        IntentOutcome::NeedsConfirmation {
          action: "deleteFile".into(),
          message: "Move?".into(),
        },
        1,
        3,
      ),
      IntentOutcome::NeedsConfirmation { .. }
    ));
  }

  fn dummy_diff() -> DiffPayload {
    DiffPayload {
      path: "a.rs".into(),
      patch: String::new(),
      merge_text: None,
      old_image: String::new(),
      new_image: String::new(),
      language: None,
      file_type: "text".into(),
      hunks: vec![],
      staged: false,
      content_hash: String::new(),
    }
  }

  #[test]
  fn stamp_diff_and_blame_carry_cursors() {
    match stamp_outcome(
      IntentOutcome::Diff {
        payload: dummy_diff(),
        session_generation: 0,
        session_revision: 0,
      },
      2,
      5,
    ) {
      IntentOutcome::Diff {
        session_generation: 2,
        session_revision: 5,
        ..
      } => {}
      other => panic!("{other:?}"),
    }
    match stamp_outcome(
      IntentOutcome::Blame {
        payload: FileBlame {
          path: "a.rs".into(),
          line_groups: vec![],
        },
        session_generation: 0,
        session_revision: 0,
      },
      2,
      5,
    ) {
      IntentOutcome::Blame {
        session_generation: 2,
        session_revision: 5,
        ..
      } => {}
      other => panic!("{other:?}"),
    }
  }

  #[tokio::test]
  async fn needs_confirmation_is_not_stamped_and_does_not_bump() {
    let mut state = SessionState {
      session_generation: 2,
      session_revision: 7,
      ..SessionState::default()
    };
    let intent = Intent::DeleteFile {
      path: "gone.rs".into(),
      confirmed: false,
    };
    let output = apply_intent(intent.clone(), Path::new("/tmp"), &empty_status("/tmp"), &mut state)
      .await
      .unwrap();
    assert_eq!(state.session_revision, 7);
    assert!(!outcome_should_bump(&intent, &output));
    let FinishApply::Immediate(outcome) = finish_apply(output) else {
      panic!("expected immediate confirmation");
    };
    let stamped = stamp_outcome(*outcome, 2, 8);
    match &stamped {
      IntentOutcome::NeedsConfirmation { action, .. } => assert_eq!(action, "deleteFile"),
      other => panic!("{other:?}"),
    }
    let json = serde_json::to_string(&stamped).unwrap();
    assert!(!json.contains("sessionRevision"), "{json}");
    assert!(!json.contains("sessionGeneration"), "{json}");
  }

  #[tokio::test]
  async fn empty_stage_ack_omits_cursors() {
    let mut state = SessionState::default();
    let intent = Intent::Stage { paths: vec![] };
    let output = apply_intent(intent.clone(), Path::new("/tmp"), &empty_status("/tmp"), &mut state)
      .await
      .unwrap();
    assert!(!outcome_should_bump(&intent, &output));
    let FinishApply::Immediate(outcome) = finish_apply(output) else {
      panic!("expected immediate ack");
    };
    let json = serde_json::to_string(&*outcome).unwrap();
    assert!(json.contains("\"kind\":\"ack\""), "{json}");
    assert!(!json.contains("sessionRevision"), "{json}");
    assert!(!json.contains("sessionGeneration"), "{json}");
  }

  #[tokio::test]
  async fn clear_file_ack_stamps_after_bump() {
    let mut state = SessionState {
      session_generation: 1,
      session_revision: 2,
      selection: Some(FileSelection {
        path: "a.rs".into(),
        staged: false,
        group_kind: ResourceGroupKind::WorkingTree,
      }),
      ..SessionState::default()
    };
    let intent = Intent::ClearFile;
    let output = apply_intent(intent.clone(), Path::new("/tmp"), &empty_status("/tmp"), &mut state)
      .await
      .unwrap();
    assert!(state.selection.is_none());
    assert!(outcome_should_bump(&intent, &output));
    let generation = state.session_generation;
    let revision = state.bump_revision();
    let FinishApply::Immediate(outcome) = finish_apply(output) else {
      panic!("expected immediate ack");
    };
    match stamp_outcome(*outcome, generation, revision) {
      IntentOutcome::Ack {
        session_generation: Some(1),
        session_revision: Some(3),
      } => {}
      other => panic!("{other:?}"),
    }
  }
}
