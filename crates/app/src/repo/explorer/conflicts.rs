#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
  Replace,
  KeepBoth,
  Cancel,
}

pub const CONFLICT_TITLE: &str = "File Conflict";
pub const CONFLICT_REPLACE: &str = "A file with this name already exists. Do you want to replace it?";
pub const CONFLICT_KEEP_BOTH: &str = "Keep both files? A copy will be created with a new name.";

pub fn is_conflict_error(message: &str) -> bool {
  message.contains("already exists")
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn conflict_errors_are_detected() {
    assert!(is_conflict_error("\"a.txt\" already exists in destination"));
    assert!(!is_conflict_error("permission denied"));
  }
}
