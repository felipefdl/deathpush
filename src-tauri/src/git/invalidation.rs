#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitInvalidation {
  Ignore,
  Status,
  Refs,
  Stash,
  Head,
}

pub fn classify_git_relative(relative: &str) -> GitInvalidation {
  let relative = relative.replace('\\', "/");
  let relative = relative.trim_start_matches('/');

  if relative.contains("index.lock") || relative.contains(".watchman-cookie-") || is_under(relative, "objects") {
    return GitInvalidation::Ignore;
  }

  if relative == "HEAD" || relative == "logs/HEAD" {
    return GitInvalidation::Head;
  }

  if relative == "refs/stash"
    || relative.starts_with("refs/stash/")
    || relative == "logs/refs/stash"
    || relative.starts_with("logs/refs/stash/")
  {
    return GitInvalidation::Stash;
  }

  if is_under(relative, "logs") {
    return GitInvalidation::Ignore;
  }

  if relative == "packed-refs"
    || relative == "refs"
    || is_under(relative, "refs/heads")
    || is_under(relative, "refs/tags")
    || is_under(relative, "refs/remotes")
  {
    return GitInvalidation::Refs;
  }

  GitInvalidation::Status
}

fn is_under(relative: &str, prefix: &str) -> bool {
  relative == prefix || relative.starts_with(&format!("{prefix}/"))
}

pub fn file_index_should_invalidate(kind: crate::types::PathChangeKind, relative: &str) -> bool {
  let relative = relative.replace('\\', "/");
  match kind {
    crate::types::PathChangeKind::Structural => true,
    crate::types::PathChangeKind::Git => {
      relative == "index" || relative == "info/exclude" || relative.ends_with("/info/exclude")
    }
    crate::types::PathChangeKind::Content => relative == ".gitignore" || relative.ends_with("/.gitignore"),
  }
}

#[cfg(test)]
mod tests {
  use super::{GitInvalidation, classify_git_relative, file_index_should_invalidate};

  #[test]
  fn packed_refs_is_refs() {
    assert_eq!(classify_git_relative("packed-refs"), GitInvalidation::Refs);
    assert_eq!(classify_git_relative("refs/heads/feature"), GitInvalidation::Refs);
    assert_eq!(classify_git_relative("refs/tags/v1"), GitInvalidation::Refs);
  }

  #[test]
  fn stash_ref_and_stash_log_are_stash() {
    assert_eq!(classify_git_relative("refs/stash"), GitInvalidation::Stash);
    assert_eq!(classify_git_relative("logs/refs/stash"), GitInvalidation::Stash);
  }

  #[test]
  fn other_logs_and_objects_are_ignored() {
    assert_eq!(classify_git_relative("logs/refs/heads/main"), GitInvalidation::Ignore);
    assert_eq!(classify_git_relative("objects/ab/cd"), GitInvalidation::Ignore);
    assert_eq!(classify_git_relative("index.lock"), GitInvalidation::Ignore);
    assert_eq!(classify_git_relative(".watchman-cookie-123"), GitInvalidation::Ignore);
  }

  #[test]
  fn head_is_head() {
    assert_eq!(classify_git_relative("HEAD"), GitInvalidation::Head);
    assert_eq!(classify_git_relative("logs/HEAD"), GitInvalidation::Head);
  }

  #[test]
  fn index_and_other_git_files_are_status() {
    assert_eq!(classify_git_relative("index"), GitInvalidation::Status);
    assert_eq!(classify_git_relative("config"), GitInvalidation::Status);
    assert_eq!(classify_git_relative("MERGE_HEAD"), GitInvalidation::Status);
  }

  #[test]
  fn content_edit_does_not_invalidate_file_index() {
    assert!(!file_index_should_invalidate(
      crate::types::PathChangeKind::Content,
      "README.md"
    ));
  }

  #[test]
  fn gitignore_and_index_invalidate_file_index() {
    assert!(file_index_should_invalidate(
      crate::types::PathChangeKind::Content,
      ".gitignore"
    ));
    assert!(file_index_should_invalidate(crate::types::PathChangeKind::Git, "index"));
    assert!(file_index_should_invalidate(
      crate::types::PathChangeKind::Git,
      "info/exclude"
    ));
    assert!(file_index_should_invalidate(
      crate::types::PathChangeKind::Structural,
      ""
    ));
  }
}
