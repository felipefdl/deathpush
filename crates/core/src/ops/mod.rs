pub mod cli;
pub mod config;
pub mod explorer;
pub mod file_ops;
pub mod repository;
pub mod session;
pub mod terminal;

use std::path::PathBuf;

use crate::core::Core;
use crate::error::{Error, Result};
use crate::git::repository_runtime::RepositoryRuntime;
use crate::git::status::StatusScope;
use crate::session::SessionId;

/// The OS window title for a repository window.
pub fn window_title(root: &str, head_branch: Option<&str>) -> String {
  let repo_name = std::path::Path::new(root)
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "DeathPush".into());
  match head_branch {
    Some(branch) if !branch.is_empty() => format!("{repo_name} ({branch}) - DeathPush"),
    _ => format!("{repo_name} - DeathPush"),
  }
}

impl Core {
  pub fn repo_root(&self, id: SessionId) -> Result<PathBuf> {
    self
      .lock_repos()
      .get(&id)
      .and_then(|state| state.cli_root.clone())
      .ok_or(Error::NoRepository)
  }

  pub(crate) fn invalidate_status(&self, id: SessionId) -> Result<()> {
    self.invalidate_status_with(id, |runtime| {
      runtime.invalidate(StatusScope::Repository);
      Ok(())
    })
  }

  pub(crate) fn invalidate_status_paths(&self, id: SessionId, paths: &[String]) -> Result<()> {
    self.invalidate_status_with(id, |runtime| {
      runtime.invalidate_paths(paths);
      Ok(())
    })
  }

  fn invalidate_status_with(
    &self,
    id: SessionId,
    invalidate: impl FnOnce(&RepositoryRuntime) -> Result<()>,
  ) -> Result<()> {
    let runtime = self.runtimes.runtime_for_session(id).ok_or(Error::NoRepository)?;
    invalidate(&runtime)?;
    let repo = runtime.open_repository()?;
    let mut repos = self.lock_repos();
    let state = repos.get_mut(&id).ok_or(Error::NoRepository)?;
    state.repo = Some(repo);
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::window_title;

  #[test]
  fn title_with_branch() {
    assert_eq!(
      window_title("/tmp/deathpush", Some("main")),
      "deathpush (main) - DeathPush"
    );
  }

  #[test]
  fn title_detached() {
    assert_eq!(window_title("/tmp/deathpush", None), "deathpush - DeathPush");
    assert_eq!(window_title("/tmp/deathpush", Some("")), "deathpush - DeathPush");
  }
}
