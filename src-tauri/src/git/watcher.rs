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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchSpec {
  pub path: PathBuf,
  pub mode: RecursiveMode,
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

pub fn extra_watch_specs(repo_root: &Path) -> Vec<WatchSpec> {
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

  let mut specs = Vec::new();
  if !git_dir.starts_with(&workdir) {
    specs.push(WatchSpec {
      path: git_dir.clone(),
      mode: RecursiveMode::Recursive,
    });
  }
  if !common_dir.starts_with(&workdir) && common_dir != git_dir {
    specs.push(WatchSpec {
      path: common_dir.clone(),
      mode: RecursiveMode::NonRecursive,
    });
    if let Ok(refs) = std::fs::canonicalize(common_dir.join("refs")) {
      if refs != git_dir && refs != common_dir {
        specs.push(WatchSpec {
          path: refs,
          mode: RecursiveMode::Recursive,
        });
      }
    }
  }
  specs
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
  match crate::git::invalidation::classify_git_relative(relative) {
    crate::git::invalidation::GitInvalidation::Ignore => None,
    _ => Some(ClassifiedPath {
      relative: relative.to_string(),
      kind: PathChangeKind::Git,
      scope: PathChangeScope::Repository,
    }),
  }
}

pub fn classify_watched_path(
  workdir: &Path,
  extra_roots: &[WatchSpec],
  path: &Path,
  kind: EventKind,
) -> Option<ClassifiedPath> {
  let mut extras: Vec<&WatchSpec> = extra_roots.iter().collect();
  extras.sort_by_key(|root| std::cmp::Reverse(root.path.as_os_str().len()));
  for extra in extras {
    if let Ok(relative) = path.strip_prefix(&extra.path) {
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
  let extra_roots = extra_watch_specs(repo_root);
  for extra in &extra_roots {
    if let Err(err) = watcher.watch(&extra.path, extra.mode) {
      tracing::warn!("failed to watch extra git path {}: {err:?}", extra.path.display());
    }
  }
  let root = repo_root.to_path_buf();

  std::thread::spawn(move || {
    let _watcher = watcher;
    loop {
      match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(result) => {
          if notify_event_lost(&result) {
            if !signal_overflow(&sink, &overflow) {
              return;
            }
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

pub fn signal_overflow(sink: &mpsc::SyncSender<WatcherMessage>, overflow: &AtomicBool) -> bool {
  overflow.store(true, Ordering::SeqCst);
  match sink.try_send(WatcherMessage::Wake) {
    Ok(()) | Err(mpsc::TrySendError::Full(_)) => true,
    Err(mpsc::TrySendError::Disconnected(_)) => false,
  }
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
    Event, EventKind, RecursiveMode,
    event::{Flag, ModifyKind},
  };
  use tempfile::TempDir;

  use super::{
    WatchFilter, WatchSpec, classify_path, classify_watched_path, extra_watch_specs, notify_event_lost,
    should_watch_path,
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
  fn content_workdir_is_not_refs() {
    let root = PathBuf::from("/repo");
    let work = classify_path(&root, &root.join("src/a.ts"), EventKind::Modify(ModifyKind::Any)).unwrap();
    assert_eq!(work.kind, PathChangeKind::Content);
    assert_eq!(work.scope, PathChangeScope::Exact);
    assert_ne!(work.kind, PathChangeKind::Git);
  }

  #[test]
  fn stash_ref_and_log_are_watched_as_git() {
    let root = PathBuf::from("/repo");
    for relative in [".git/refs/stash", ".git/logs/refs/stash"] {
      let classified = classify_path(&root, &root.join(relative), EventKind::Modify(ModifyKind::Any)).unwrap();
      assert_eq!(classified.kind, PathChangeKind::Git, "{relative}");
      assert_eq!(classified.scope, PathChangeScope::Repository, "{relative}");
    }
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
  fn overflow_signal_wakes_idle_worker_or_stops_when_disconnected() {
    use super::{WatcherMessage, signal_overflow};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc;

    let overflow = AtomicBool::new(false);
    let (tx, rx) = mpsc::sync_channel(1);
    assert!(signal_overflow(&tx, &overflow));
    assert!(overflow.load(Ordering::SeqCst));
    assert!(matches!(rx.try_recv(), Ok(WatcherMessage::Wake)));

    tx.try_send(WatcherMessage::Wake).unwrap();
    overflow.store(false, Ordering::SeqCst);
    assert!(signal_overflow(&tx, &overflow));
    assert!(overflow.load(Ordering::SeqCst));
    assert!(matches!(rx.try_recv(), Ok(WatcherMessage::Wake)));
    assert!(rx.try_recv().is_err());

    drop(rx);
    overflow.store(false, Ordering::SeqCst);
    assert!(!signal_overflow(&tx, &overflow));
    assert!(overflow.load(Ordering::SeqCst));
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
  fn main_repo_has_no_extra_watch_specs() {
    let (dir, _repo) = init_repo();
    assert!(extra_watch_specs(dir.path()).is_empty());
  }

  #[test]
  fn linked_worktree_watch_specs_exclude_recursive_common_dir() {
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

    let specs = extra_watch_specs(&linked);
    let git_dir = specs
      .iter()
      .find(|spec| spec.path.to_string_lossy().contains("worktrees/linked"));
    assert!(git_dir.is_some(), "expected worktree git dir, got {specs:?}");
    assert_eq!(git_dir.unwrap().mode, RecursiveMode::Recursive);

    let common = specs.iter().find(|spec| {
      let text = spec.path.to_string_lossy();
      text.ends_with(".git") && !text.contains("worktrees")
    });
    assert!(common.is_some(), "expected common git dir, got {specs:?}");
    assert_eq!(common.unwrap().mode, RecursiveMode::NonRecursive);

    let refs = specs
      .iter()
      .find(|spec| spec.path.ends_with("refs") && !spec.path.to_string_lossy().contains("worktrees"));
    assert!(refs.is_some(), "expected common refs, got {specs:?}");
    assert_eq!(refs.unwrap().mode, RecursiveMode::Recursive);

    assert!(
      specs.iter().all(|spec| {
        let text = spec.path.to_string_lossy();
        !(spec.mode == RecursiveMode::Recursive && text.ends_with(".git") && !text.contains("worktrees"))
      }),
      "common git dir must not be watched recursively, got {specs:?}"
    );
  }

  #[test]
  fn git_dir_events_classify_as_repository_invalidation() {
    let git_dir = WatchSpec {
      path: PathBuf::from("/repo/.git/worktrees/linked"),
      mode: RecursiveMode::Recursive,
    };
    let classified = classify_watched_path(
      Path::new("/linked"),
      &[git_dir.clone()],
      &git_dir.path.join("HEAD"),
      EventKind::Modify(ModifyKind::Any),
    )
    .unwrap();
    assert_eq!(classified.kind, PathChangeKind::Git);
    assert_eq!(classified.scope, PathChangeScope::Repository);
  }

  #[test]
  fn extra_git_metadata_classifies_as_repository_git() {
    let extras = [
      WatchSpec {
        path: PathBuf::from("/repo/.git/worktrees/linked"),
        mode: RecursiveMode::Recursive,
      },
      WatchSpec {
        path: PathBuf::from("/repo/.git"),
        mode: RecursiveMode::NonRecursive,
      },
      WatchSpec {
        path: PathBuf::from("/repo/.git/refs"),
        mode: RecursiveMode::Recursive,
      },
    ];
    let workdir = Path::new("/linked");
    let kind = EventKind::Modify(ModifyKind::Any);

    for path in [
      "/repo/.git/worktrees/linked/HEAD",
      "/repo/.git/HEAD",
      "/repo/.git/packed-refs",
      "/repo/.git/refs/heads/main",
    ] {
      let classified = classify_watched_path(workdir, &extras, Path::new(path), kind).unwrap();
      assert_eq!(classified.kind, PathChangeKind::Git, "{path}");
      assert_eq!(classified.scope, PathChangeScope::Repository, "{path}");
    }
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
