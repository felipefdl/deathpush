use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::status::scan_baseline;
use super::status_coordinator::{StatusCoordinator, DIRTY_CAP};
use super::watcher::{ClassifiedPath, WatcherMessage};
use crate::types::{PathChangeKind, PathChangeScope, StatusEntry, StatusKey, StatusPhase};

fn watcher_exact(relative: &str) -> WatcherMessage {
  WatcherMessage::Path(ClassifiedPath {
    relative: relative.to_string(),
    kind: PathChangeKind::Content,
    scope: PathChangeScope::Exact,
  })
}

fn entry_keys(entries: &[StatusEntry]) -> BTreeMap<StatusKey, StatusEntry> {
  entries
    .iter()
    .map(|entry| {
      (
        StatusKey {
          group: entry.group.clone(),
          path: entry.path.clone(),
        },
        entry.clone(),
      )
    })
    .collect()
}

#[test]
fn storm_converges_to_git2_baseline_after_synthetic_events() {
  const EVENT_COUNT: usize = 400;
  let directory = tempfile::TempDir::new().unwrap();
  git2::Repository::init(directory.path()).unwrap();
  let root = directory.path().to_path_buf();
  std::fs::create_dir_all(root.join("storm")).unwrap();
  let paths: Vec<String> = (0..EVENT_COUNT).map(|index| format!("storm/f{index}.txt")).collect();
  for path in &paths {
    std::fs::write(root.join(path), "x").unwrap();
  }

  let coordinator = Arc::new(StatusCoordinator::new(root.clone()));
  let overlapping_scan = Arc::new(AtomicBool::new(false));
  let saw_scan = Arc::new(AtomicBool::new(false));
  {
    let hook_coordinator = Arc::clone(&coordinator);
    let overlapping_scan = Arc::clone(&overlapping_scan);
    let saw_scan = Arc::clone(&saw_scan);
    coordinator.set_during_scan_hook_for_test(move || {
      saw_scan.store(true, Ordering::SeqCst);
      overlapping_scan.store(hook_coordinator.begin_scan_for_test(), Ordering::SeqCst);
    });
  }

  let sink = coordinator.spawn_worker();
  let mut max_dirty = 0usize;
  let mut saw_storm = false;
  for path in &paths {
    sink.send(watcher_exact(path)).unwrap();
    max_dirty = max_dirty.max(coordinator.dirty_scopes_for_test().len());
    saw_storm |= coordinator.in_storm();
    if coordinator.scan_in_flight_for_test() {
      assert!(
        !coordinator.begin_scan_for_test(),
        "a second scan started while one was already in flight"
      );
    }
  }

  let deadline = Instant::now() + Duration::from_secs(5);
  loop {
    max_dirty = max_dirty.max(coordinator.dirty_scopes_for_test().len());
    saw_storm |= coordinator.in_storm();
    if coordinator.scan_in_flight_for_test() {
      assert!(
        !coordinator.begin_scan_for_test(),
        "a second scan started while one was already in flight"
      );
    }
    let snapshot = coordinator.snapshot_cursor();
    if snapshot.phase == StatusPhase::Settled
      && snapshot.generation > 0
      && !coordinator.in_storm()
      && coordinator.dirty_scopes_for_test().is_empty()
      && !coordinator.scan_in_flight_for_test()
    {
      break;
    }
    assert!(
      Instant::now() < deadline,
      "coordinator did not settle after quiet; phase={:?} storm={} dirty={} in_flight={}",
      snapshot.phase,
      coordinator.in_storm(),
      coordinator.dirty_scopes_for_test().len(),
      coordinator.scan_in_flight_for_test()
    );
    std::thread::sleep(Duration::from_millis(20));
  }

  assert!(
    max_dirty <= DIRTY_CAP,
    "dirty set grew to {max_dirty}, cap is {DIRTY_CAP}"
  );
  assert!(saw_storm, "400 unique path events should enter storm");
  assert!(saw_scan.load(Ordering::SeqCst), "expected at least one scan in flight");
  assert!(
    !overlapping_scan.load(Ordering::SeqCst),
    "overlapping scan started during the in-flight scan"
  );

  let baseline = scan_baseline(&root).unwrap();
  let snapshot = coordinator.snapshot_cursor();
  assert_eq!(snapshot.phase, StatusPhase::Settled);
  assert_eq!(entry_keys(&snapshot.entries), entry_keys(&baseline.entries));

  drop(sink);
}
