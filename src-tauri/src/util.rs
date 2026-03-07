/// Create a `tokio::process::Command` that hides the console window on Windows.
pub fn async_command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
  let cmd = tokio::process::Command::new(program);
  apply_no_window(cmd)
}

/// Create a `std::process::Command` that hides the console window on Windows.
pub fn sync_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
  let cmd = std::process::Command::new(program);
  apply_no_window_sync(cmd)
}

#[cfg(windows)]
#[allow(unused_imports)]
fn apply_no_window(mut cmd: tokio::process::Command) -> tokio::process::Command {
  use std::os::windows::process::CommandExt;
  cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
  cmd
}

#[cfg(not(windows))]
fn apply_no_window(cmd: tokio::process::Command) -> tokio::process::Command {
  cmd
}

#[cfg(windows)]
#[allow(unused_imports)]
fn apply_no_window_sync(mut cmd: std::process::Command) -> std::process::Command {
  use std::os::windows::process::CommandExt;
  cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
  cmd
}

#[cfg(not(windows))]
fn apply_no_window_sync(cmd: std::process::Command) -> std::process::Command {
  cmd
}
