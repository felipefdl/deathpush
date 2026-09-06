use git2::{DiffOptions, Oid, Sort};

use crate::error::{Error, Result};
use crate::git::diff::detect_renames;
use crate::git::repository::GitRepository;
use crate::types::{CommitDetail, CommitEntry, CommitFileEntry, FileStatus, LastCommitInfo};

pub fn compute_avatar_url(email: &str) -> String {
  let email_lower = email.trim().to_lowercase();
  // GitHub noreply: {id}+{username}@users.noreply.github.com or {username}@users.noreply.github.com
  if email_lower.ends_with("@users.noreply.github.com") {
    let local = email_lower.split('@').next().unwrap_or("");
    let username = if let Some(pos) = local.find('+') {
      &local[pos + 1..]
    } else {
      local
    };
    if !username.is_empty() {
      return format!("https://github.com/{username}.png?size=48");
    }
  }
  let hash = md5::compute(email_lower.as_bytes());
  format!("https://www.gravatar.com/avatar/{hash:x}?s=48&d=404")
}

pub fn get_commit_log(repo: &GitRepository, skip: usize, limit: usize) -> Result<Vec<CommitEntry>> {
  let r = repo.inner();
  let mut revwalk = r.revwalk()?;
  revwalk.set_sorting(Sort::TIME)?;
  revwalk.push_head()?;

  let entries: Vec<CommitEntry> = revwalk
    .skip(skip)
    .take(limit)
    .filter_map(|oid| oid.ok())
    .filter_map(|oid| {
      let commit = r.find_commit(oid).ok()?;
      Some(commit_to_entry(&commit))
    })
    .collect();

  Ok(entries)
}

pub fn last_commit_info(repo: &GitRepository) -> Option<LastCommitInfo> {
  let commit = repo.inner().head().ok()?.peel_to_commit().ok()?;
  let entry = commit_to_entry(&commit);
  Some(LastCommitInfo {
    short_id: entry.short_id,
    message: entry.message.lines().next().unwrap_or("").to_string(),
    author_date: entry.author_date,
  })
}
pub fn get_commit_detail(repo: &GitRepository, commit_id: &str) -> Result<CommitDetail> {
  let r = repo.inner();
  let oid = Oid::from_str(commit_id).map_err(|e| Error::Other(format!("invalid commit id: {}", e)))?;
  let commit = r.find_commit(oid)?;
  let entry = commit_to_entry(&commit);

  let tree = commit.tree()?;
  let parent_tree = if commit.parent_count() > 0 {
    Some(commit.parent(0)?.tree()?)
  } else {
    None
  };

  let mut diff_opts = DiffOptions::new();
  let mut diff = r.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;
  detect_renames(&mut diff)?;

  let mut files: Vec<CommitFileEntry> = Vec::new();
  diff.foreach(
    &mut |delta, _| {
      let path = delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

      let old_path = if delta.status() == git2::Delta::Renamed {
        delta.old_file().path().map(|p| p.to_string_lossy().to_string())
      } else {
        None
      };

      files.push(CommitFileEntry {
        path,
        status: commit_file_status(delta.status()),
        old_path,
      });
      true
    },
    None,
    None,
    None,
  )?;

  Ok(CommitDetail { commit: entry, files })
}

pub fn commit_file_status(status: git2::Delta) -> FileStatus {
  match status {
    git2::Delta::Added => FileStatus::Added,
    git2::Delta::Deleted => FileStatus::Deleted,
    git2::Delta::Modified => FileStatus::Modified,
    git2::Delta::Renamed => FileStatus::Renamed,
    git2::Delta::Copied => FileStatus::Copied,
    git2::Delta::Typechange => FileStatus::TypeChanged,
    _ => FileStatus::Modified,
  }
}

fn commit_to_entry(commit: &git2::Commit) -> CommitEntry {
  let id = commit.id().to_string();
  let short_id = id[..7.min(id.len())].to_string();
  let message = commit.message().unwrap_or("").to_string();
  let author = commit.author();
  let author_name = author.name().unwrap_or("").to_string();
  let author_email = author.email().unwrap_or("").to_string();
  let time = commit.time();
  let author_date = format_git_time(&time);
  let parent_ids: Vec<String> = (0..commit.parent_count())
    .filter_map(|i| commit.parent_id(i).ok())
    .map(|oid| oid.to_string())
    .collect();

  let avatar_url = compute_avatar_url(&author_email);
  CommitEntry {
    id,
    short_id,
    message,
    author_name,
    author_email,
    author_date,
    parent_ids,
    avatar_url,
  }
}

fn format_git_time(time: &git2::Time) -> String {
  let secs = time.seconds();
  let dt = chrono::DateTime::from_timestamp(secs, 0);
  match dt {
    Some(d) => d.to_rfc3339(),
    None => secs.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_avatar_url_github_noreply_with_id() {
    let url = compute_avatar_url("12345+user@users.noreply.github.com");
    assert_eq!(url, "https://github.com/user.png?size=48");
  }

  #[test]
  fn test_avatar_url_github_noreply_without_id() {
    let url = compute_avatar_url("user@users.noreply.github.com");
    assert_eq!(url, "https://github.com/user.png?size=48");
  }

  #[test]
  fn test_avatar_url_regular_email() {
    let url = compute_avatar_url("test@example.com");
    let expected_hash = md5::compute(b"test@example.com");
    assert_eq!(
      url,
      format!("https://www.gravatar.com/avatar/{expected_hash:x}?s=48&d=404")
    );
  }

  #[test]
  fn test_avatar_url_case_insensitive() {
    let url_upper = compute_avatar_url("Test@Example.COM");
    let url_lower = compute_avatar_url("test@example.com");
    assert_eq!(url_upper, url_lower);
  }

  #[test]
  fn test_avatar_url_trims_whitespace() {
    let url_padded = compute_avatar_url("  test@example.com  ");
    let url_clean = compute_avatar_url("test@example.com");
    assert_eq!(url_padded, url_clean);
  }

  #[test]
  fn commit_detail_detects_renames() {
    use std::path::Path;

    use crate::git::diff::commit_file_diff;
    use crate::git::repository::GitRepository;

    let directory = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    let root = repo.workdir().unwrap();
    std::fs::write(root.join("old.txt"), "rename me\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("old.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "add\n", &tree, &[]).unwrap();

    std::fs::rename(root.join("old.txt"), root.join("new.txt")).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old.txt")).unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let oid = repo
      .commit(Some("HEAD"), &sig, &sig, "rename\n", &tree, &[&parent])
      .unwrap();

    let git = GitRepository::open(directory.path()).unwrap();
    let detail = get_commit_detail(&git, &oid.to_string()).unwrap();
    assert_eq!(detail.files.len(), 1);
    assert_eq!(detail.files[0].path, "new.txt");
    assert_eq!(detail.files[0].old_path.as_deref(), Some("old.txt"));
    assert_eq!(detail.files[0].status, FileStatus::Renamed);

    let diff = commit_file_diff(&git, &oid.to_string(), "new.txt").unwrap();
    assert_eq!(diff.content.path, "new.txt");
    assert_eq!(diff.content.original, "rename me\n");
    assert_eq!(diff.content.modified, "rename me\n");
  }

  #[test]
  fn commit_file_status_maps_git_deltas() {
    assert_eq!(commit_file_status(git2::Delta::Added), FileStatus::Added);
    assert_eq!(commit_file_status(git2::Delta::Deleted), FileStatus::Deleted);
    assert_eq!(commit_file_status(git2::Delta::Modified), FileStatus::Modified);
    assert_eq!(commit_file_status(git2::Delta::Renamed), FileStatus::Renamed);
    assert_eq!(commit_file_status(git2::Delta::Copied), FileStatus::Copied);
    assert_eq!(commit_file_status(git2::Delta::Typechange), FileStatus::TypeChanged);
    assert_eq!(commit_file_status(git2::Delta::Unmodified), FileStatus::Modified);
  }
}
