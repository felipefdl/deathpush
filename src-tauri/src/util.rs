#![allow(unused_mut)]

/// Create a `tokio::process::Command` that hides the console window on Windows.
pub fn async_command<S: AsRef<std::ffi::OsStr>>(program: S) -> tokio::process::Command {
  let mut cmd = tokio::process::Command::new(program);
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
  }
  cmd
}

/// Create a `std::process::Command` that hides the console window on Windows.
pub fn sync_command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
  let mut cmd = std::process::Command::new(program);
  #[cfg(windows)]
  {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
  }
  cmd
}
