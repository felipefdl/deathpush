use std::path::{Path, PathBuf};

use git2::{ErrorCode, Repository};

use crate::error::{Error, Result};

pub struct GitRepository {
  repo: Repository,
  root: PathBuf,
}

/// `Repository::discover` reports a missing repository and a missing path with the same code.
/// An existing directory with no Git repository in it or above it is reported separately so
/// callers can offer to initialize one.
fn discover_error(err: git2::Error, path: &Path) -> Error {
  if err.code() == ErrorCode::NotFound && path.is_dir() {
    Error::NotARepository {
      path: path.display().to_string(),
    }
  } else {
    Error::Git(err)
  }
}

impl GitRepository {
  pub fn open(path: &Path) -> Result<Self> {
    let repo = Repository::discover(path).map_err(|err| discover_error(err, path))?;
    let root = repo
      .workdir()
      .ok_or_else(|| Error::Other("bare repository not supported".into()))?
      .to_path_buf();
    Ok(Self { repo, root })
  }

  /// `Ok(None)` when `path` is a folder with no Git repository in it or above it.
  pub fn open_optional(path: &Path) -> Result<Option<Self>> {
    match Self::open(path) {
      Ok(repo) => Ok(Some(repo)),
      Err(Error::NotARepository { .. }) => Ok(None),
      Err(err) => Err(err),
    }
  }

  pub fn root(&self) -> &Path {
    &self.root
  }

  pub fn inner(&self) -> &Repository {
    &self.repo
  }

  pub fn head_branch(&self) -> Option<String> {
    let head = self.repo.head().ok()?;
    if head.is_branch() {
      head.shorthand().ok().map(|s| s.to_string())
    } else {
      let commit = head.peel_to_commit().ok()?;
      Some(format!("({})", &commit.id().to_string()[..7]))
    }
  }

  pub fn head_commit_id(&self) -> Option<String> {
    let head = self.repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    Some(commit.id().to_string())
  }

  pub fn ahead_behind(&self) -> (usize, usize) {
    let Ok(head) = self.repo.head() else {
      return (0, 0);
    };
    if !head.is_branch() {
      return (0, 0);
    }

    let local_oid = match head.target() {
      Some(oid) => oid,
      None => return (0, 0),
    };

    let branch_name = match head.shorthand() {
      Ok(name) => name.to_string(),
      Err(_) => return (0, 0),
    };

    let upstream_name = format!("refs/remotes/origin/{}", branch_name);
    let upstream_ref = match self.repo.find_reference(&upstream_name) {
      Ok(r) => r,
      Err(_) => return (0, 0),
    };

    let upstream_oid = match upstream_ref.target() {
      Some(oid) => oid,
      None => return (0, 0),
    };

    self.repo.graph_ahead_behind(local_oid, upstream_oid).unwrap_or((0, 0))
  }
}
