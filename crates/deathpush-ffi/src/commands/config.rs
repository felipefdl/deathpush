use deathpush_core::error::Error;
use deathpush_core::util::async_command;

use crate::session::manager;

#[uniffi::export]
pub fn get_git_config(key: String) -> Result<String, Error> {
  manager().runtime.block_on(async {
    let output = async_command("git")
      .args(["config", "--global", "--get", &key])
      .output()
      .await
      .map_err(|e| Error::other(format!("Failed to run git config: {e}")))?;

    if !output.status.success() {
      return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
  })
}

#[uniffi::export]
pub fn set_git_config(key: String, value: String) -> Result<(), Error> {
  manager().runtime.block_on(async {
    let output = async_command("git")
      .args(["config", "--global", &key, &value])
      .output()
      .await
      .map_err(|e| Error::other(format!("Failed to run git config: {e}")))?;

    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      return Err(Error::other(format!("git config failed: {stderr}")));
    }

    Ok(())
  })
}
