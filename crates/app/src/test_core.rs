use deathpush_core::Core;
use gpui_kit::TestAppContext;

/// Drain gpui work, then shut Core's runtime down before `TestAppContext` drops.
///
/// Tokio workers (and spawn_blocking) wake gpui JoinHandles from `deathpush-core`
/// threads. Allow parking so those wakes are not recorded as test-scheduler
/// non-determinism while we wait for them to finish.
pub fn park_and_shutdown(cx: &TestAppContext, core: &Core) {
  cx.executor().allow_parking();
  cx.run_until_parked();
  core.shutdown();
}
