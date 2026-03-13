use deathpush_core::error::Error;

use deathpush_core::git::hunk;
use deathpush_core::types::{FileDiffWithHunks, RepositoryStatus, StashEntry};

use crate::session::{make_cli, get_root, manager, refresh_status};

#[uniffi::export]
pub fn get_last_commit_message(session_id: String) -> Result<String, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.get_last_commit_message())
}

#[uniffi::export]
pub fn undo_last_commit(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.undo_last_commit())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_save(session_id: String, message: Option<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_save(message.as_deref()))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_list(session_id: String) -> Result<Vec<StashEntry>, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_list())
}

#[uniffi::export]
pub fn stash_apply(session_id: String, index: u32) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_apply(index))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_pop(session_id: String, index: u32) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_pop(index))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_drop(session_id: String, index: u32) -> Result<Vec<StashEntry>, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  let rt = &manager().runtime;
  rt.block_on(cli.stash_drop(index))?;
  rt.block_on(cli.stash_list())
}

#[uniffi::export]
pub fn stash_save_include_untracked(
  session_id: String,
  message: Option<String>,
) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_save_include_untracked(message.as_deref()))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_save_staged(session_id: String, message: Option<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.stash_save_staged(message.as_deref()))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stash_show(session_id: String, index: u32) -> Result<FileDiffWithHunks, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  let diff_output = manager().runtime.block_on(cli.stash_show(index))?;
  let hunks = hunk::parse_unified_diff(&diff_output);
  Ok(FileDiffWithHunks {
    path: format!("stash@{{{}}}", index),
    hunks,
  })
}
