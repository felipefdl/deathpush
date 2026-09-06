use crate::core::Core;
use crate::error::Result;
use crate::util::async_command_ready;

impl Core {
  pub async fn get_git_config(&self, key: &str) -> Result<String> {
    let output = async_command_ready("git")
      .await
      .args(["config", "--global", "--get", key])
      .output()
      .await
      .map_err(|e| crate::error::Error::Other(format!("Failed to run git config: {e}")))?;

    if !output.status.success() {
      return Ok(String::new());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
  }

  pub async fn set_git_config(&self, key: &str, value: &str) -> Result<()> {
    let output = async_command_ready("git")
      .await
      .args(["config", "--global", key, value])
      .output()
      .await
      .map_err(|e| crate::error::Error::Other(format!("Failed to run git config: {e}")))?;

    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr);
      return Err(crate::error::Error::Other(format!("git config failed: {stderr}")));
    }

    Ok(())
  }
}
