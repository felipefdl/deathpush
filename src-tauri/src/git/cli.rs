use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

use tauri::{AppHandle, Emitter};

use crate::error::{Error, Result};
use crate::util::async_command;
use crate::types::StashEntry;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(handle: AppHandle) {
  let _ = APP_HANDLE.set(handle);
}

#[derive(serde::Serialize, Clone)]
struct GitCommandEvent {
  command: String,
  duration_ms: u64,
  timestamp: String,
}

fn emit_git_command(args_str: &str, duration_ms: u64) {
  if let Some(app) = APP_HANDLE.get() {
    let now = chrono::Local::now();
    let _ = app.emit(
      "git:command",
      GitCommandEvent {
        command: format!("git {args_str}"),
        duration_ms,
        timestamp: now.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
      },
    );
  }
}

pub struct GitCli {
  git_path: PathBuf,
  repo_root: PathBuf,
}

impl GitCli {
  pub fn new(repo_root: &Path) -> Self {
    Self {
      git_path: PathBuf::from("git"),
      repo_root: repo_root.to_path_buf(),
    }
  }

  pub async fn run(&self, args: &[&str]) -> Result<String> {
    let args_str = args.join(" ");
    let start = Instant::now();

    let output = async_command(&self.git_path)
      .args(args)
      .current_dir(&self.repo_root)
      .output()
      .await?;

    emit_git_command(&args_str, start.elapsed().as_millis() as u64);

    if output.status.success() {
      Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr).to_string();
      Err(Error::GitCli(stderr))
    }
  }

  pub async fn stage_files(&self, paths: &[String]) -> Result<()> {
    let mut args = vec!["add", "--"];
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    args.extend(path_refs);
    self.run(&args).await?;
    Ok(())
  }

  pub async fn stage_all(&self) -> Result<()> {
    self.run(&["add", "-A"]).await?;
    Ok(())
  }

  pub async fn unstage_files(&self, paths: &[String]) -> Result<()> {
    let mut args = vec!["reset", "HEAD", "--"];
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    args.extend(path_refs);
    self.run(&args).await?;
    Ok(())
  }

  pub async fn unstage_all(&self) -> Result<()> {
    self.run(&["reset", "HEAD"]).await?;
    Ok(())
  }

  pub async fn discard_changes(&self, paths: &[String]) -> Result<()> {
    let mut args = vec!["checkout", "--"];
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    args.extend(path_refs);
    self.run(&args).await?;
    Ok(())
  }

  pub async fn commit(&self, message: &str, amend: bool) -> Result<()> {
    let mut args = vec!["commit", "-m", message];
    if amend {
      args.push("--amend");
    }
    self.run(&args).await?;
    Ok(())
  }

  pub async fn checkout_branch(&self, name: &str) -> Result<()> {
    self.run(&["switch", name]).await?;
    Ok(())
  }

  pub async fn create_branch(&self, name: &str, from: Option<&str>) -> Result<()> {
    match from {
      Some(base) => self.run(&["checkout", "-b", name, base]).await?,
      None => self.run(&["checkout", "-b", name]).await?,
    };
    Ok(())
  }

  pub async fn delete_branch(&self, name: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };
    self.run(&["branch", flag, name]).await?;
    Ok(())
  }

  pub async fn push(&self, remote: &str, branch: &str, force: bool) -> Result<()> {
    let mut args = vec!["push", remote, branch];
    if force {
      args.push("--force-with-lease");
    }
    self.run(&args).await?;
    Ok(())
  }

  pub async fn pull(&self, remote: &str, branch: &str, rebase: bool) -> Result<()> {
    let mut args = vec!["pull", remote, branch];
    if rebase {
      args.push("--rebase");
    }
    self.run(&args).await?;
    Ok(())
  }

  pub async fn fetch(&self, remote: &str, prune: bool) -> Result<()> {
    let mut args = vec!["fetch", remote];
    if prune {
      args.push("--prune");
    }
    self.run(&args).await?;
    Ok(())
  }

  pub async fn get_last_commit_message(&self) -> Result<String> {
    let output = self.run(&["log", "-1", "--format=%B"]).await?;
    Ok(output.trim().to_string())
  }

  pub async fn undo_last_commit(&self) -> Result<()> {
    self.run(&["reset", "--soft", "HEAD~1"]).await?;
    Ok(())
  }

  pub async fn stash_save(&self, message: Option<&str>) -> Result<()> {
    match message {
      Some(msg) => self.run(&["stash", "push", "-m", msg]).await?,
      None => self.run(&["stash", "push"]).await?,
    };
    Ok(())
  }

  pub async fn stash_list(&self) -> Result<Vec<StashEntry>> {
    let output = self.run(&["stash", "list", "--format=%gd|%gs"]).await?;
    let entries = output
      .lines()
      .filter(|line| !line.is_empty())
      .enumerate()
      .map(|(i, line)| {
        let message = line.split_once('|').map(|(_, msg)| msg).unwrap_or(line).to_string();
        StashEntry { index: i, message }
      })
      .collect();
    Ok(entries)
  }

  pub async fn stash_apply(&self, index: usize) -> Result<()> {
    self.run(&["stash", "apply", &format!("stash@{{{}}}", index)]).await?;
    Ok(())
  }

  pub async fn stash_pop(&self, index: usize) -> Result<()> {
    self.run(&["stash", "pop", &format!("stash@{{{}}}", index)]).await?;
    Ok(())
  }

  pub async fn stash_drop(&self, index: usize) -> Result<()> {
    self.run(&["stash", "drop", &format!("stash@{{{}}}", index)]).await?;
    Ok(())
  }

  pub async fn create_tag(&self, name: &str, message: Option<&str>, commit: Option<&str>) -> Result<()> {
    let mut args: Vec<&str> = vec!["tag"];
    if let Some(msg) = message {
      args.extend(["-a", name, "-m", msg]);
    } else {
      args.push(name);
    }
    if let Some(c) = commit {
      args.push(c);
    }
    self.run(&args).await?;
    Ok(())
  }

  pub async fn delete_tag(&self, name: &str) -> Result<()> {
    self.run(&["tag", "-d", name]).await?;
    Ok(())
  }

  pub async fn push_tag(&self, remote: &str, tag: &str) -> Result<()> {
    self.run(&["push", remote, tag]).await?;
    Ok(())
  }

  pub async fn clone_repo(url: &str, path: &Path) -> Result<()> {
    let start = Instant::now();
    let output = async_command("git")
      .args(["clone", url, &path.to_string_lossy()])
      .output()
      .await?;
    emit_git_command(&format!("clone {url} {}", path.to_string_lossy()), start.elapsed().as_millis() as u64);
    if !output.status.success() {
      let stderr = String::from_utf8_lossy(&output.stderr).to_string();
      return Err(Error::GitCli(stderr));
    }
    Ok(())
  }

  pub async fn merge_continue(&self) -> Result<()> {
    self.run(&["merge", "--continue"]).await?;
    Ok(())
  }

  pub async fn merge_abort(&self) -> Result<()> {
    self.run(&["merge", "--abort"]).await?;
    Ok(())
  }

  pub async fn rebase_continue(&self) -> Result<()> {
    self.run(&["rebase", "--continue"]).await?;
    Ok(())
  }

  pub async fn rebase_abort(&self) -> Result<()> {
    self.run(&["rebase", "--abort"]).await?;
    Ok(())
  }

  pub async fn rebase_skip(&self) -> Result<()> {
    self.run(&["rebase", "--skip"]).await?;
    Ok(())
  }

  pub async fn cherry_pick(&self, commit_id: &str) -> Result<()> {
    self.run(&["cherry-pick", commit_id]).await?;
    Ok(())
  }

  pub async fn reset_to_commit(&self, commit_id: &str, mode: &str) -> Result<()> {
    let flag = match mode {
      "hard" => "--hard",
      "mixed" => "--mixed",
      _ => "--soft",
    };
    self.run(&["reset", flag, commit_id]).await?;
    Ok(())
  }

  pub async fn get_unified_diff(&self, path: &str, staged: bool) -> Result<String> {
    if staged {
      self.run(&["diff", "--cached", "--", path]).await
    } else {
      self.run(&["diff", "--", path]).await
    }
  }

  pub async fn apply_patch(&self, patch: &str, cached: bool, reverse: bool) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut args = vec!["apply"];
    if cached {
      args.push("--cached");
    }
    if reverse {
      args.push("--reverse");
    }
    args.push("-");

    let args_str = args.join(" ");
    let start = Instant::now();

    let mut child = async_command(&self.git_path)
      .args(&args)
      .current_dir(&self.repo_root)
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
      stdin.write_all(patch.as_bytes()).await?;
    }

    let output = child.wait_with_output().await?;
    emit_git_command(&args_str, start.elapsed().as_millis() as u64);
    if !output.status.success() {
      return Err(Error::GitCli(String::from_utf8_lossy(&output.stderr).to_string()));
    }
    Ok(())
  }
}
