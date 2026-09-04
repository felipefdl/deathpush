use deathpush_core::theme::{Rgba, UiPalette};
use deathpush_core::types::FileStatus;

#[allow(dead_code)]
pub fn status_letter(status: FileStatus) -> &'static str {
  use FileStatus::*;
  match status {
    Modified | TypeChanged | IndexModified | BothModified => "M",
    Added | IndexAdded | IntentToAdd | AddedByUs | AddedByThem | BothAdded => "A",
    Deleted | IndexDeleted | DeletedByUs | DeletedByThem | BothDeleted => "D",
    Renamed | IndexRenamed | IntentToRename | Copied | IndexCopied => "R",
    Untracked => "U",
    Ignored => "!",
  }
}

#[allow(dead_code)]
pub fn status_color(status: FileStatus, palette: &UiPalette) -> Rgba {
  use FileStatus::*;
  match status {
    Modified | TypeChanged => palette.git_modified,
    IndexModified => palette.git_staged_modified,
    IndexDeleted => palette.git_staged_deleted,
    Added | IndexAdded | Copied | IndexCopied | IntentToAdd => palette.git_added,
    Deleted => palette.git_deleted,
    Renamed | IndexRenamed | IntentToRename => palette.git_renamed,
    Untracked => palette.git_untracked,
    Ignored => palette.git_ignored,
    BothModified | BothAdded | BothDeleted | AddedByUs | AddedByThem | DeletedByUs | DeletedByThem => {
      palette.git_conflicting
    }
  }
}

#[allow(dead_code)]
pub fn is_dimmed(status: FileStatus) -> bool {
  matches!(status, FileStatus::Ignored)
}

#[cfg(test)]
mod tests {
  use super::*;
  use deathpush_core::theme::UiPalette;
  use deathpush_core::types::FileStatus;

  #[test]
  fn status_letters_follow_the_spec() {
    use FileStatus::*;
    for s in [Modified, TypeChanged, IndexModified, BothModified] {
      assert_eq!(status_letter(s), "M");
    }
    for s in [Added, IndexAdded, IntentToAdd, AddedByUs, AddedByThem, BothAdded] {
      assert_eq!(status_letter(s), "A");
    }
    for s in [Deleted, IndexDeleted, DeletedByUs, DeletedByThem, BothDeleted] {
      assert_eq!(status_letter(s), "D");
    }
    for s in [Renamed, IndexRenamed, IntentToRename, Copied, IndexCopied] {
      assert_eq!(status_letter(s), "R");
    }
    assert_eq!(status_letter(Untracked), "U");
    assert_eq!(status_letter(Ignored), "!");
    assert!(is_dimmed(Ignored));
    assert!(!is_dimmed(Modified));
  }

  #[test]
  fn status_colors_pick_the_palette_slot() {
    let spec = deathpush_core::theme::parse_theme(r##"{"name":"t","type":"dark","colors":{}}"##).unwrap();
    let p = UiPalette::from_spec(&spec);
    assert_eq!(status_color(FileStatus::Modified, &p), p.git_modified);
    assert_eq!(status_color(FileStatus::IndexModified, &p), p.git_staged_modified);
    assert_eq!(status_color(FileStatus::IndexDeleted, &p), p.git_staged_deleted);
    assert_eq!(status_color(FileStatus::Added, &p), p.git_added);
    assert_eq!(status_color(FileStatus::Copied, &p), p.git_added);
    assert_eq!(status_color(FileStatus::Renamed, &p), p.git_renamed);
    assert_eq!(status_color(FileStatus::Untracked, &p), p.git_untracked);
    assert_eq!(status_color(FileStatus::Ignored, &p), p.git_ignored);
    assert_eq!(status_color(FileStatus::BothModified, &p), p.git_conflicting);
    assert_eq!(status_color(FileStatus::TypeChanged, &p), p.git_modified);
  }
}
