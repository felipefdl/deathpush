use notify::{
  Event, EventKind, RecursiveMode, Watcher,
  event::{CreateKind, RemoveKind},
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Default)]
pub struct WatchFilter {
  tracked: HashSet<String>,
  prefixes: HashSet<String>,
}

impl WatchFilter {
  pub fn from_repo(repo: &git2::Repository) -> Self {
    let mut filter = Self::default();
    let Ok(index) = repo.index() else {
      return filter;
    };
    for entry in index.iter() {
      let path = String::from_utf8_lossy(&entry.path).replace('\\', "/");
      filter.tracked.insert(path.clone());
      let mut rest = path.as_str();
      while let Some((parent, _)) = rest.rsplit_once('/') {
        if !filter.prefixes.insert(parent.to_string()) {
          break;
        }
        rest = parent;
      }
    }
    filter
  }

  pub fn should_watch(&self, repo: Option<&git2::Repository>, relative: &str, subtree: bool) -> bool {
    let relative = relative.replace('\\', "/");
    if self.tracked.contains(&relative) {
      return true;
    }
    if subtree {
      let prefix = relative.trim_end_matches('/');
      if prefix.is_empty() || self.prefixes.contains(prefix) || self.tracked.contains(prefix) {
        return true;
      }
    }
    let Some(repo) = repo else {
      return true;
    };
    match repo.is_path_ignored(Path::new(&relative)) {
      Ok(ignored) => !ignored,
      Err(_) => true,
    }
  }
}

#[derive(Default)]
pub struct WatchCache {
  repo: Option<git2::Repository>,
  filter: WatchFilter,
}

impl WatchCache {
  pub fn invalidate(&mut self) {
    self.repo = None;
    self.filter = WatchFilter::default();
  }

  pub fn refresh(&mut self, root: &Path) {
    self.repo = git2::Repository::open(root).ok();
    self.filter = match &self.repo {
      Some(repo) => WatchFilter::from_repo(repo),
      None => WatchFilter::default(),
    };
  }

  pub fn should_watch(&mut self, root: &Path, relative: &str, subtree: bool) -> bool {
    if self.repo.is_none() {
      self.refresh(root);
    }
    self.filter.should_watch(self.repo.as_ref(), relative, subtree)
  }
}

pub fn extra_watch_roots(repo_root: &Path) -> Vec<PathBuf> {
  let Ok(repo) = git2::Repository::open(repo_root) else {
    return Vec::new();
  };
  let Some(workdir) = repo.workdir() else {
    return Vec::new();
  };
  let Ok(workdir) = std::fs::canonicalize(workdir) else {
    return Vec::new();
  };
  let Ok(git_dir) = std::fs::canonicalize(repo.path()) else {
    return Vec::new();
  };
  let Ok(common_dir) = std::fs::canonicalize(repo.commondir()) else {
    return Vec::new();
  };

  let mut roots = Vec::new();
  if !git_dir.starts_with(&workdir) {
    roots.push(git_dir);
  }
  if !common_dir.starts_with(&workdir) && !roots.iter().any(|root| root == &common_dir) {
    roots.push(common_dir);
  }
  roots
}

pub fn notify_event_lost(result: &notify::Result<Event>) -> bool {
  match result {
    Ok(event) => event.need_rescan(),
    Err(err) => matches!(
      err.kind,
      notify::ErrorKind::Io(_) | notify::ErrorKind::MaxFilesWatch | notify::ErrorKind::Generic(_)
    ),
  }
}

fn classify_git_relative(relative: &str) -> Option<ClassifiedPath> {
  if relative.contains("index.lock")
    || relative.starts_with("objects/")
    || relative.contains("/objects/")
    || relative.starts_with("logs/")
    || relative.contains("/logs/")
    || relative.contains(".watchman-cookie-")
  {
    return None;
  }
  Some(ClassifiedPath {
    relative: relative.to_string(),
    kind: PathChangeKind::Git,
    scope: PathChangeScope::Repository,
  })
}

pub fn classify_watched_path(
  workdir: &Path,
  extra_roots: &[PathBuf],
  path: &Path,
  kind: EventKind,
) -> Option<ClassifiedPath> {
  let mut extras: Vec<&PathBuf> = extra_roots.iter().collect();
  extras.sort_by_key(|root| std::cmp::Reverse(root.as_os_str().len()));
  for extra in extras {
    if let Ok(relative) = path.strip_prefix(extra) {
      let relative = relative.to_string_lossy().replace('\\', "/");
      return classify_git_relative(&relative);
    }
  }
  classify_path(workdir, path, kind)
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
    return classify_git_relative(git_rest);
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

#[cfg(test)]
pub fn should_watch_path(repo: &git2::Repository, relative: &str, scope: PathChangeScope) -> bool {
  WatchFilter::from_repo(repo).should_watch(Some(repo), relative, scope == PathChangeScope::Subtree)
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
  let extra_roots = extra_watch_roots(repo_root);
  for extra in &extra_roots {
    if let Err(err) = watcher.watch(extra, RecursiveMode::Recursive) {
      tracing::warn!("failed to watch extra git path {}: {err:?}", extra.display());
    }
  }
  let root = repo_root.to_path_buf();

  std::thread::spawn(move || {
    let _watcher = watcher;
    loop {
      match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(result) => {
          if notify_event_lost(&result) {
            overflow.store(true, Ordering::SeqCst);
            continue;
          }
          let Ok(event) = result else {
            continue;
          };
          if event.kind.is_access() {
            continue;
          }
          for path in event.paths {
            let Some(classified) = classify_watched_path(&root, &extra_roots, &path, event.kind) else {
              continue;
            };
            if !send_classified(&sink, &overflow, classified) {
              return;
            }
          }
        }
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

  use notify::{
    Event, EventKind,
    event::{Flag, ModifyKind},
  };
  use tempfile::TempDir;

  use super::{
    WatchFilter, classify_path, classify_watched_path, extra_watch_roots, notify_event_lost, should_watch_path,
  };
  use crate::types::{PathChangeKind, PathChangeScope};

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

    assert!(!should_watch_path(&repo, "vendor/noise.txt", PathChangeScope::Exact));
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

    assert!(should_watch_path(&repo, "vendor/kept.txt", PathChangeScope::Exact));
    assert!(should_watch_path(&repo, "vendor", PathChangeScope::Subtree));
    assert!(!should_watch_path(&repo, "vendor", PathChangeScope::Exact));
    assert!(!should_watch_path(&repo, "vendor/noise.txt", PathChangeScope::Exact));
  }

  #[test]
  fn ignore_filter_does_not_hardcode_node_modules() {
    let (_dir, repo) = init_repo();
    let root = repo.workdir().unwrap();
    std::fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
    std::fs::write(root.join("node_modules/pkg/index.js"), "module.exports = 1\n").unwrap();

    assert!(should_watch_path(
      &repo,
      "node_modules/pkg/index.js",
      PathChangeScope::Exact
    ));
  }

  #[test]
  fn watch_filter_limits_descendant_lookup_to_subtree_scope() {
    let (_dir, repo) = init_repo();
    let root = repo.workdir().unwrap();
    std::fs::write(root.join(".gitignore"), "vendor/\n").unwrap();
    commit_forced(&repo, ".gitignore", "vendor/\n");
    commit_forced(&repo, "vendor/kept.txt", "keep\n");
    let filter = WatchFilter::from_repo(&repo);
    assert!(filter.should_watch(Some(&repo), "vendor", true));
    assert!(!filter.should_watch(Some(&repo), "vendor", false));
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

  #[test]
  fn main_repo_has_no_extra_watch_roots() {
    let (dir, _repo) = init_repo();
    assert!(extra_watch_roots(dir.path()).is_empty());
  }

  #[test]
  fn linked_worktree_watch_roots_include_git_dir_and_common_dir() {
    let directory = TempDir::new().unwrap();
    let main = directory.path().join("main");
    let linked = directory.path().join("linked");
    std::fs::create_dir(&main).unwrap();
    let repo = git2::Repository::init(&main).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_forced(&repo, "README.md", "hello\n");
    repo.worktree("linked", &linked, None).unwrap();

    let extra = extra_watch_roots(&linked);
    assert!(
      extra
        .iter()
        .any(|root| root.to_string_lossy().contains("worktrees/linked")),
      "expected worktree git dir, got {extra:?}"
    );
    assert!(
      extra.iter().any(|root| {
        let text = root.to_string_lossy();
        text.ends_with(".git") && !text.contains("worktrees")
      }),
      "expected common git dir, got {extra:?}"
    );
  }

  #[test]
  fn git_dir_events_classify_as_repository_invalidation() {
    let git_dir = PathBuf::from("/repo/.git/worktrees/linked");
    let classified = classify_watched_path(
      Path::new("/linked"),
      &[git_dir.clone()],
      &git_dir.join("HEAD"),
      EventKind::Modify(ModifyKind::Any),
    )
    .unwrap();
    assert_eq!(classified.kind, PathChangeKind::Git);
    assert_eq!(classified.scope, PathChangeScope::Repository);
  }

  #[test]
  fn rescan_and_loss_errors_promote_to_overflow() {
    let rescan = Event::new(EventKind::Other).set_flag(Flag::Rescan);
    assert!(notify_event_lost(&Ok(rescan)));

    let io = notify::Error::new(notify::ErrorKind::Io(std::io::Error::other("lost")));
    assert!(notify_event_lost(&Err(io)));

    let max = notify::Error::new(notify::ErrorKind::MaxFilesWatch);
    assert!(notify_event_lost(&Err(max)));

    let generic = notify::Error::new(notify::ErrorKind::Generic("backend lost events".into()));
    assert!(notify_event_lost(&Err(generic)));

    let benign = Event::new(EventKind::Modify(ModifyKind::Any));
    assert!(!notify_event_lost(&Ok(benign)));
  }
}
