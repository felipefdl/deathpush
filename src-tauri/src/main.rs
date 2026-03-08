#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(target_os = "linux")]
  {
    // Work around WebKitGTK GPU rendering failures on systems with
    // incomplete or missing DRI/EGL drivers (e.g. ARM boards, VMs,
    // certain NVIDIA setups). Without these, WebKitGTK may show a
    // white screen with "Could not connect to localhost".
    // SAFETY: called before any threads are spawned (single-threaded main).
    unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
  }

  deathpush_lib::run()
}
