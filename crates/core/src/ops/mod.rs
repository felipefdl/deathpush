pub mod cli;
pub mod config;
pub mod explorer;
pub mod file_ops;
pub mod history;
pub mod repository;
pub mod session;
pub mod terminal;

use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::core::Core;
use crate::error::{Error, Result};
use crate::git::repository_runtime::RepositoryRuntime;
use crate::git::status::StatusScope;
use crate::relative_time::relative_time;
use crate::session::SessionId;
use crate::types::FileBlame;

fn repo_name(root: &str) -> String {
  std::path::Path::new(root)
    .file_name()
    .map(|n| n.to_string_lossy().into_owned())
    .unwrap_or_else(|| "DeathPush".into())
}

/// The OS window title for a repository window.
pub fn window_title(root: &str, head_branch: Option<&str>) -> String {
  let repo_name = repo_name(root);
  match head_branch {
    Some(branch) if !branch.is_empty() => format!("{repo_name} ({branch}) - DeathPush"),
    _ => format!("{repo_name} - DeathPush"),
  }
}

/// The in-window title for a repository window.
pub fn in_window_title(root: &str, head_branch: Option<&str>) -> String {
  let repo_name = repo_name(root);
  match head_branch {
    Some(branch) if !branch.is_empty() => format!("{repo_name} - {branch}"),
    _ => repo_name,
  }
}

/// `{author}, {relative time} - {summary}` for the blame group covering `line` (1-based), or None on uncommitted lines.
pub fn blame_status_line(blame: &FileBlame, line: usize, now: DateTime<Utc>) -> Option<String> {
  let group = blame
    .line_groups
    .iter()
    .find(|group| group.start_line <= line && line <= group.end_line)?;
  Some(format!(
    "{}, {} - {}",
    group.author_name,
    relative_time(&group.author_date, now),
    group.summary
  ))
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
  use super::{blame_status_line, in_window_title, window_title};
  use crate::relative_time::relative_time;
  use crate::types::{BlameLineGroup, FileBlame};

  #[test]
  fn blame_status_line_formats_the_covering_group() {
    let blame = FileBlame {
      path: "a.rs".into(),
      line_groups: vec![BlameLineGroup {
        commit_id: "abc".into(),
        short_id: "abc".into(),
        author_name: "Ana".into(),
        author_email: "".into(),
        author_date: "2026-09-01T00:00:00Z".into(),
        summary: "fix it".into(),
        start_line: 3,
        end_line: 5,
      }],
    };
    let now = chrono::DateTime::parse_from_rfc3339("2026-09-04T00:00:00Z")
      .unwrap()
      .with_timezone(&chrono::Utc);
    let expected = format!("Ana, {} - fix it", relative_time("2026-09-01T00:00:00Z", now));
    assert_eq!(blame_status_line(&blame, 4, now).as_deref(), Some(expected.as_str()));
    assert!(blame_status_line(&blame, 6, now).is_none());
  }

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

  #[test]
  fn in_window_title_with_branch() {
    assert_eq!(in_window_title("/tmp/deathpush", Some("main")), "deathpush - main");
  }

  #[test]
  fn in_window_title_detached() {
    assert_eq!(in_window_title("/tmp/deathpush", None), "deathpush");
    assert_eq!(in_window_title("/tmp/deathpush", Some("")), "deathpush");
  }
}
