use crate::error::Error;

#[tauri::command]
pub async fn get_git_config(key: String) -> Result<String, Error> {
  let output = tokio::process::Command::new("git")
    .args(["config", "--global", "--get", &key])
    .output()
    .await
    .map_err(|e| Error::Other(format!("Failed to run git config: {e}")))?;

  if !output.status.success() {
    return Ok(String::new());
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub async fn set_git_config(key: String, value: String) -> Result<(), Error> {
  let output = tokio::process::Command::new("git")
    .args(["config", "--global", &key, &value])
    .output()
    .await
    .map_err(|e| Error::Other(format!("Failed to run git config: {e}")))?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    return Err(Error::Other(format!("git config failed: {stderr}")));
  }

  Ok(())
}
