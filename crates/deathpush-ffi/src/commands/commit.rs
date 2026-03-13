use deathpush_core::error::Error;

use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::types::RepositoryStatus;

use crate::session::{make_cli, get_root, manager};

#[uniffi::export]
pub fn commit(session_id: String, message: String, amend: bool) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.commit(&message, amend))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}
