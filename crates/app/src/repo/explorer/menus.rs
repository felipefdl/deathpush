#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemMenu {
  NewFile,
  NewFolder,
  OpenInEditor,
  Rename,
  Duplicate,
  Cut,
  Copy,
  Paste,
  RevealInFinder,
  CopyPath,
  CopyRelativePath,
  MoveToTrash,
  AddToGitignore,
}

impl ItemMenu {
  pub const ORDER: [ItemMenu; 13] = [
    ItemMenu::NewFile,
    ItemMenu::NewFolder,
    ItemMenu::OpenInEditor,
    ItemMenu::Rename,
    ItemMenu::Duplicate,
    ItemMenu::Cut,
    ItemMenu::Copy,
    ItemMenu::Paste,
    ItemMenu::RevealInFinder,
    ItemMenu::CopyPath,
    ItemMenu::CopyRelativePath,
    ItemMenu::MoveToTrash,
    ItemMenu::AddToGitignore,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Self::NewFile => "New File...",
      Self::NewFolder => "New Folder...",
      Self::OpenInEditor => "Open in Editor",
      Self::Rename => "Rename",
      Self::Duplicate => "Duplicate",
      Self::Cut => "Cut",
      Self::Copy => "Copy",
      Self::Paste => "Paste",
      Self::RevealInFinder => "Reveal in Finder",
      Self::CopyPath => "Copy Path",
      Self::CopyRelativePath => "Copy Relative Path",
      Self::MoveToTrash => "Move to Trash",
      Self::AddToGitignore => "Add to .gitignore",
    }
  }

  pub fn enabled(self, is_directory: bool, has_mark: bool) -> bool {
    match self {
      Self::OpenInEditor => !is_directory,
      Self::Paste => has_mark,
      _ => true,
    }
  }
}

pub fn blank_menu_items(has_mark: bool) -> Vec<ItemMenu> {
  let mut items = vec![ItemMenu::NewFile, ItemMenu::NewFolder];
  if has_mark {
    items.push(ItemMenu::Paste);
  }
  items
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn item_menu_order_and_labels() {
    let labels: Vec<&str> = ItemMenu::ORDER.iter().map(|i| i.label()).collect();
    assert_eq!(
      labels,
      vec![
        "New File...",
        "New Folder...",
        "Open in Editor",
        "Rename",
        "Duplicate",
        "Cut",
        "Copy",
        "Paste",
        "Reveal in Finder",
        "Copy Path",
        "Copy Relative Path",
        "Move to Trash",
        "Add to .gitignore"
      ]
    );
    assert!(!ItemMenu::OpenInEditor.enabled(true, false));
    assert!(!ItemMenu::Paste.enabled(false, false) && ItemMenu::Paste.enabled(true, true));
    assert_eq!(blank_menu_items(false), vec![ItemMenu::NewFile, ItemMenu::NewFolder]);
    assert_eq!(
      blank_menu_items(true),
      vec![ItemMenu::NewFile, ItemMenu::NewFolder, ItemMenu::Paste]
    );
  }
}
