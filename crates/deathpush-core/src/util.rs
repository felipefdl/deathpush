/// Create a `tokio::process::Command` with the resolved shell environment
/// and hidden console window on Windows.
pub fn async_command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
  let mut cmd = tokio::process::Command::new(program);
  if let Some(env) = crate::shell_env::get() {
    cmd.env_clear();
    cmd.envs(env.iter());
  }
  apply_no_window(cmd)
}

/// Create a `std::process::Command` with the resolved shell environment
/// and hidden console window on Windows.
pub fn sync_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
  let mut cmd = std::process::Command::new(program);
  if let Some(env) = crate::shell_env::get() {
    cmd.env_clear();
    cmd.envs(env.iter());
  }
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
