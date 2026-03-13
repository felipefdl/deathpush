use deathpush_core::error::Error;
use deathpush_core::git::cli::GitCli;
use deathpush_core::git::hunk;
use deathpush_core::types::{FileDiffWithHunks, RepositoryStatus};

use crate::session::{get_root, manager, refresh_status};

#[uniffi::export]
pub fn stage_files(session_id: String, paths: Vec<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.stage_files(&paths))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stage_all(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.stage_all())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn unstage_files(session_id: String, paths: Vec<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.unstage_files(&paths))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn unstage_all(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.unstage_all())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn discard_changes(session_id: String, paths: Vec<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.discard_changes(&paths))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn get_file_hunks(session_id: String, path: String, staged: bool) -> Result<FileDiffWithHunks, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  let diff_output = manager().runtime.block_on(cli.get_unified_diff(&path, staged))?;
  let hunks = hunk::parse_unified_diff(&diff_output);
  Ok(FileDiffWithHunks { path, hunks })
}

#[uniffi::export]
pub fn stage_hunk(
  session_id: String,
  path: String,
  hunk_index: u32,
  staged: bool,
) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  let rt = &manager().runtime;

  let diff_output = rt.block_on(cli.get_unified_diff(&path, staged))?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index as usize)?;

  if staged {
    rt.block_on(cli.apply_patch(&patch, true, true))?;
  } else {
    rt.block_on(cli.apply_patch(&patch, true, false))?;
  }

  refresh_status(&session_id)
}

#[uniffi::export]
pub fn discard_hunk(session_id: String, path: String, hunk_index: u32) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  let rt = &manager().runtime;

  let diff_output = rt.block_on(cli.get_unified_diff(&path, false))?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index as usize)?;
  rt.block_on(cli.apply_patch(&patch, false, true))?;

  refresh_status(&session_id)
}

#[uniffi::export]
pub fn stage_lines(
  session_id: String,
  path: String,
  hunk_index: u32,
  line_start: u32,
  line_end: u32,
  staged: bool,
) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  let rt = &manager().runtime;

  let diff_output = rt.block_on(cli.get_unified_diff(&path, staged))?;
  let patch = hunk::generate_lines_patch(
    &path,
    &diff_output,
    hunk_index as usize,
    line_start as usize,
    line_end as usize,
  )?;

  if staged {
    rt.block_on(cli.apply_patch(&patch, true, true))?;
  } else {
    rt.block_on(cli.apply_patch(&patch, true, false))?;
  }

  refresh_status(&session_id)
}
