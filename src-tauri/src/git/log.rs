use std::path::Path;

use git2::{DiffOptions, Oid, Sort};

use crate::error::{Error, Result};
use crate::git::diff::{blob_to_data_uri, detect_language, is_image_file};
use crate::git::repository::GitRepository;
use crate::types::{CommitDiffContent, CommitDetail, CommitEntry, CommitFileEntry};

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
  let diff = r.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

  let mut files: Vec<CommitFileEntry> = Vec::new();
  diff.foreach(
    &mut |delta, _| {
      let status = match delta.status() {
        git2::Delta::Added => "added",
        git2::Delta::Deleted => "deleted",
        git2::Delta::Modified => "modified",
        git2::Delta::Renamed => "renamed",
        git2::Delta::Copied => "copied",
        git2::Delta::Typechange => "typeChanged",
        _ => "modified",
      };

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

      files.push(CommitFileEntry { path, status: status.to_string(), old_path });
      true
    },
    None,
    None,
    None,
  )?;

  Ok(CommitDetail { commit: entry, files })
}

pub fn get_commit_file_diff(repo: &GitRepository, commit_id: &str, path: &str) -> Result<CommitDiffContent> {
  let r = repo.inner();
  let oid = Oid::from_str(commit_id).map_err(|e| Error::Other(format!("invalid commit id: {}", e)))?;
  let commit = r.find_commit(oid)?;
  let tree = commit.tree()?;

  if is_image_file(path) {
    let modified = read_blob_from_tree_base64(r, &tree, path).unwrap_or_default();

    let original = if commit.parent_count() > 0 {
      let parent_tree = commit.parent(0)?.tree()?;
      read_blob_from_tree_base64(r, &parent_tree, path).unwrap_or_default()
    } else {
      String::new()
    };

    return Ok(CommitDiffContent {
      path: path.to_string(),
      original,
      modified,
      language: None,
      file_type: "image".to_string(),
    });
  }

  let modified = read_blob_from_tree(r, &tree, path).unwrap_or_default();

  let original = if commit.parent_count() > 0 {
    let parent_tree = commit.parent(0)?.tree()?;
    read_blob_from_tree(r, &parent_tree, path).unwrap_or_default()
  } else {
    String::new()
  };

  let language = detect_language(path);

  Ok(CommitDiffContent {
    path: path.to_string(),
    original,
    modified,
    language,
    file_type: "text".to_string(),
  })
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

  CommitEntry { id, short_id, message, author_name, author_email, author_date, parent_ids }
}

fn format_git_time(time: &git2::Time) -> String {
  let secs = time.seconds();
  let dt = chrono::DateTime::from_timestamp(secs, 0);
  match dt {
    Some(d) => d.to_rfc3339(),
    None => secs.to_string(),
  }
}

fn read_blob_from_tree(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
  let entry = tree.get_path(Path::new(path)).ok()?;
  let blob = repo.find_blob(entry.id()).ok()?;
  String::from_utf8(blob.content().to_vec()).ok()
}

fn read_blob_from_tree_base64(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
  let entry = tree.get_path(Path::new(path)).ok()?;
  let blob = repo.find_blob(entry.id()).ok()?;
  Some(blob_to_data_uri(blob.content(), path))
}
