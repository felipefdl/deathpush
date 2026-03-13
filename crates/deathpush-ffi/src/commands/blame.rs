use deathpush_core::error::Error;
use deathpush_core::git::blame;
use deathpush_core::types::{CommitEntry, FileBlame, LastCommitInfo};

use crate::session::{get_root, manager};

#[uniffi::export]
pub fn get_file_blame(session_id: String, path: String) -> Result<FileBlame, Error> {
  let root = get_root(&session_id)?;
  manager().runtime.block_on(blame::get_file_blame(&root, &path))
}

#[uniffi::export]
pub fn get_file_log(session_id: String, path: String, skip: u32, limit: u32) -> Result<Vec<CommitEntry>, Error> {
  let root = get_root(&session_id)?;
  manager()
    .runtime
    .block_on(blame::get_file_log(&root, &path, skip as usize, limit as usize))
}

#[uniffi::export]
pub fn get_last_commit_info(session_id: String) -> Result<LastCommitInfo, Error> {
  let root = get_root(&session_id)?;
  manager().runtime.block_on(blame::get_last_commit_info(&root))
}
