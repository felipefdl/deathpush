#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(target_os = "linux")]
  {
    // Work around WebKitGTK GPU rendering failures on systems with
    // incomplete or missing DRI/EGL drivers (e.g. ARM, VMs, NVIDIA).
    // Must be set before WebKitGTK initializes.
    // SAFETY: single-threaded, called before any other code runs.
    unsafe {
      std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
  }

  deathpush_lib::run()
}
