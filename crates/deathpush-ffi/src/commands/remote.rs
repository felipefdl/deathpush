use deathpush_core::error::Error;

use deathpush_core::types::RepositoryStatus;

use crate::session::{make_cli, get_root, manager, refresh_status};

#[uniffi::export]
pub fn push(
  session_id: String,
  remote: String,
  branch: String,
  force: bool,
) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.push(&remote, &branch, force))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn pull(
  session_id: String,
  remote: String,
  branch: String,
  rebase: bool,
) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.pull(&remote, &branch, rebase))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn fetch(session_id: String, remote: String, prune: bool) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.fetch(&remote, prune))?;
  refresh_status(&session_id)
}
