use deathpush_core::error::Error;

use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::tag as git_tag;
use deathpush_core::types::TagEntry;

use crate::session::{make_cli, get_root, manager};

#[uniffi::export]
pub fn list_tags(session_id: String) -> Result<Vec<TagEntry>, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  git_tag::list_tags(repo)
}

#[uniffi::export]
pub fn create_tag(
  session_id: String,
  name: String,
  message: Option<String>,
  target: Option<String>,
) -> Result<Vec<TagEntry>, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager()
    .runtime
    .block_on(cli.create_tag(&name, message.as_deref(), target.as_deref()))?;

  let repo = GitRepository::open(&root)?;
  let tags = git_tag::list_tags(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(tags)
}

#[uniffi::export]
pub fn delete_tag(session_id: String, name: String) -> Result<Vec<TagEntry>, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.delete_tag(&name))?;

  let repo = GitRepository::open(&root)?;
  let tags = git_tag::list_tags(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(tags)
}

#[uniffi::export]
pub fn push_tag(session_id: String, remote: String, tag: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.push_tag(&remote, &tag))?;
  Ok(())
}

#[uniffi::export]
pub fn delete_remote_tag(session_id: String, remote: String, name: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.delete_remote_tag(&remote, &name))?;
  Ok(())
}
