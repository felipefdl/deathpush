#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  #[cfg(target_os = "linux")]
  {
    // Work around WebKitGTK GPU rendering failures on systems with incomplete
    // or missing DRI/EGL drivers (e.g. ARM, VMs, NVIDIA). The env var must be
    // present before dynamic library initializers run, so set_var() in main()
    // is too late. Instead, re-exec the binary with the var already in the
    // environment -- exec() replaces the process in-place, so the new image
    // sees it from the very start. Skipped if the user already exported it.
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
      use std::os::unix::process::CommandExt;
      let err = std::process::Command::new(std::env::current_exe().expect("failed to resolve exe path"))
        .args(std::env::args_os().skip(1))
        .env("WEBKIT_DISABLE_DMABUF_RENDERER", "1")
        .exec();
      eprintln!("failed to re-exec: {err}");
      std::process::exit(1);
    }
  }

  deathpush_lib::run()
}
