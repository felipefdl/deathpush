use notify::{Event, EventKind, RecursiveMode, Watcher, event::CreateKind, event::RemoveKind};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::types::{PathChangeKind, PathChangeScope};

pub struct WatcherHandle {
  stop_tx: mpsc::Sender<()>,
}

impl Drop for WatcherHandle {
  fn drop(&mut self) {
    let _ = self.stop_tx.send(());
  }
}

#[cfg(test)]
impl WatcherHandle {
  pub(crate) fn for_test() -> Self {
    let (stop_tx, _) = mpsc::channel();
    Self { stop_tx }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedPath {
  pub relative: String,
  pub kind: PathChangeKind,
  pub scope: PathChangeScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherMessage {
  Path(ClassifiedPath),
  Overflow,
  Wake,
}

pub fn classify_path(root: &Path, path: &Path, kind: EventKind) -> Option<ClassifiedPath> {
  let relative = path.strip_prefix(root).ok()?;
  let relative = relative.to_string_lossy().replace('\\', "/");
  if relative.is_empty() {
    return Some(ClassifiedPath {
      relative,
      kind: PathChangeKind::Structural,
      scope: PathChangeScope::Repository,
    });
  }

  if let Some(git_rest) = relative.strip_prefix(".git/") {
    if git_rest.contains("index.lock")
      || git_rest.starts_with("objects/")
      || git_rest.starts_with("logs/")
      || git_rest.contains(".watchman-cookie-")
    {
      return None;
    }
    return Some(ClassifiedPath {
      relative,
      kind: PathChangeKind::Git,
      scope: PathChangeScope::Repository,
    });
  }

  let scope = match kind {
    EventKind::Remove(RemoveKind::Folder | RemoveKind::Any) | EventKind::Create(CreateKind::Folder) => {
      PathChangeScope::Subtree
    }
    _ => PathChangeScope::Exact,
  };

  Some(ClassifiedPath {
    relative,
    kind: PathChangeKind::Content,
    scope,
  })
}

pub fn should_watch_path(repo: &git2::Repository, relative: &str) -> bool {
  let path = Path::new(relative);
  if let Ok(index) = repo.index()
    && index.get_path(path, 0).is_some()
  {
    return true;
  }
  if has_tracked_descendant(repo, relative) {
    return true;
  }
  match repo.is_path_ignored(path) {
    Ok(ignored) => !ignored,
    Err(_) => true,
  }
}

fn has_tracked_descendant(repo: &git2::Repository, relative: &str) -> bool {
  let Ok(index) = repo.index() else {
    return false;
  };
  let prefix = relative.trim_end_matches('/');
  if prefix.is_empty() {
    return true;
  }
  let with_slash = format!("{prefix}/");
  index.iter().any(|entry| {
    let path = String::from_utf8_lossy(&entry.path);
    path == prefix || path.starts_with(&with_slash)
  })
}

pub fn start_watcher(
  repo_root: &Path,
  sink: mpsc::SyncSender<WatcherMessage>,
  overflow: Arc<AtomicBool>,
) -> notify::Result<WatcherHandle> {
  let (stop_tx, stop_rx) = mpsc::channel();
  let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
  let mut watcher = notify::recommended_watcher(tx)?;
  watcher.watch(repo_root, RecursiveMode::Recursive)?;
  let root = repo_root.to_path_buf();

  std::thread::spawn(move || {
    let _watcher = watcher;
    loop {
      match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(Ok(event)) => {
          if event.kind.is_access() {
            continue;
          }
          for path in event.paths {
            let Some(classified) = classify_path(&root, &path, event.kind) else {
              continue;
            };
            if !send_classified(&sink, &overflow, classified) {
              return;
            }
          }
        }
        Ok(Err(_)) => {}
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
      }
      if stop_rx.try_recv().is_ok() {
        break;
      }
    }
  });

  Ok(WatcherHandle { stop_tx })
}

pub fn send_classified(
  sink: &mpsc::SyncSender<WatcherMessage>,
  overflow: &AtomicBool,
  classified: ClassifiedPath,
) -> bool {
  match sink.try_send(WatcherMessage::Path(classified)) {
    Ok(()) => true,
    Err(mpsc::TrySendError::Full(_)) => {
      overflow.store(true, Ordering::SeqCst);
      true
    }
    Err(mpsc::TrySendError::Disconnected(_)) => false,
  }
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use notify::{EventKind, event::ModifyKind};
  use tempfile::TempDir;

  use super::{classify_path, should_watch_path};

  fn commit_forced(repo: &git2::Repository, relative: &str, contents: &str) {
    let root = repo.workdir().unwrap();
    if let Some(parent) = Path::new(relative).parent() {
      std::fs::create_dir_all(root.join(parent)).unwrap();
    }
    std::fs::write(root.join(relative), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_all([relative], git2::IndexAddOption::FORCE, None).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let parents: Vec<git2::Commit> = match repo.head() {
      Ok(head) => vec![head.peel_to_commit().unwrap()],
      Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo
      .commit(Some("HEAD"), &sig, &sig, "test", &tree, &parent_refs)
      .unwrap();
  }

  fn init_repo() -> (TempDir, git2::Repository) {
    let directory = TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_forced(&repo, "README.md", "hello\n");
    (directory, repo)
  }

  #[test]
  fn classify_keeps_workdir_file_and_drops_git_objects() {
    let root = PathBuf::from("/repo");
    let work = classify_path(&root, &root.join("src/file.ts"), EventKind::Modify(ModifyKind::Any)).unwrap();
    assert_eq!(work.relative, "src/file.ts");

    assert!(
      classify_path(
        &root,
        &root.join(".git/objects/ab/cdef"),
        EventKind::Modify(ModifyKind::Any),
      )
      .is_none()
    );
  }

  #[test]
  fn ignore_filter_skips_ignored_untracked_without_tracked_descendant() {
    let (_dir, repo) = init_repo();
    let root = repo.workdir().unwrap();
    std::fs::write(root.join(".gitignore"), "vendor/\n").unwrap();
    commit_forced(&repo, ".gitignore", "vendor/\n");
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(root.join("vendor/noise.txt"), "noise\n").unwrap();

    assert!(!should_watch_path(&repo, "vendor/noise.txt"));
  }

  #[test]
  fn ignore_filter_keeps_tracked_descendant_in_ignored_tree() {
    let (_dir, repo) = init_repo();
    let root = repo.workdir().unwrap();
    std::fs::write(root.join(".gitignore"), "vendor/\n").unwrap();
    commit_forced(&repo, ".gitignore", "vendor/\n");
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(root.join("vendor/noise.txt"), "noise\n").unwrap();
    commit_forced(&repo, "vendor/kept.txt", "keep\n");

    assert!(should_watch_path(&repo, "vendor/kept.txt"));
    assert!(should_watch_path(&repo, "vendor"));
    assert!(!should_watch_path(&repo, "vendor/noise.txt"));
  }

  #[test]
  fn ignore_filter_does_not_hardcode_node_modules() {
    let (_dir, repo) = init_repo();
    let root = repo.workdir().unwrap();
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::write(root.join("node_modules/pkg/index.js"), "module.exports = 1\n").unwrap();

    assert!(should_watch_path(&repo, "node_modules/pkg/index.js"));
  }

  #[test]
  fn full_channel_sets_overflow_flag_without_sending_overflow_message() {
    use super::{ClassifiedPath, WatcherMessage, send_classified};
    use crate::types::{PathChangeKind, PathChangeScope};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let (tx, rx) = mpsc::sync_channel(1);
    let overflow = AtomicBool::new(false);
    let path = ClassifiedPath {
      relative: "a.rs".into(),
      kind: PathChangeKind::Content,
      scope: PathChangeScope::Exact,
    };
    assert!(send_classified(&tx, &overflow, path.clone()));
    assert!(send_classified(&tx, &overflow, path));
    assert!(overflow.load(Ordering::SeqCst));
    assert!(matches!(rx.try_recv(), Ok(WatcherMessage::Path(_))));
    assert!(rx.try_recv().is_err());
  }
}
