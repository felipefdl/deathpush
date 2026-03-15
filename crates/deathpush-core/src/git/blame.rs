use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::git::log::compute_avatar_url;
use crate::types::{BlameLineGroup, CommitEntry, FileBlame, LastCommitInfo};
use crate::util::async_command;

pub async fn get_file_blame(repo_root: &Path, path: &str) -> Result<FileBlame> {
  let output = async_command("git")
    .args(["blame", "--porcelain", "--", path])
    .current_dir(repo_root)
    .output()
    .await?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::GitCli { message: stderr });
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let line_groups = parse_porcelain_blame(&stdout);

  Ok(FileBlame {
    path: path.to_string(),
    line_groups,
  })
}

fn parse_porcelain_blame(output: &str) -> Vec<BlameLineGroup> {
  let mut commits: HashMap<String, (String, String, String, String)> = HashMap::new();
  let mut raw_entries: Vec<(String, u32)> = Vec::new();

  let lines: Vec<&str> = output.lines().collect();
  let mut i = 0;

  while i < lines.len() {
    let line = lines[i];
    if line.is_empty() {
      i += 1;
      continue;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
      i += 1;
      continue;
    }

    let commit_id = parts[0].to_string();
    let result_line: u32 = parts[2].parse().unwrap_or(1);

    if !commits.contains_key(&commit_id) {
      let mut author_name = String::new();
      let mut author_email = String::new();
      let mut author_date = String::new();
      let mut summary = String::new();

      i += 1;
      while i < lines.len() {
        let header = lines[i];
        if header.starts_with('\t') {
          break;
        }
        if let Some(val) = header.strip_prefix("author ") {
          author_name = val.to_string();
        } else if let Some(val) = header.strip_prefix("author-mail ") {
          author_email = val.trim_matches(|c| c == '<' || c == '>').to_string();
        } else if let Some(val) = header.strip_prefix("author-time ") {
          if let Ok(ts) = val.parse::<i64>() {
            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default().to_rfc3339();
            author_date = dt;
          }
        } else if let Some(val) = header.strip_prefix("summary ") {
          summary = val.to_string();
        }
        i += 1;
      }

      commits.insert(commit_id.clone(), (author_name, author_email, author_date, summary));
    } else {
      i += 1;
      while i < lines.len() && !lines[i].starts_with('\t') {
        i += 1;
      }
    }

    // Skip the tab-prefixed content line
    if i < lines.len() && lines[i].starts_with('\t') {
      i += 1;
    }

    raw_entries.push((commit_id, result_line));
  }

  // Merge adjacent lines with the same commit
  let mut groups: Vec<BlameLineGroup> = Vec::new();

  for (commit_id, line_num) in &raw_entries {
    let short_id = if commit_id.len() >= 7 {
      commit_id[..7].to_string()
    } else {
      commit_id.clone()
    };

    let (author_name, author_email, author_date, summary) = commits.get(commit_id).cloned().unwrap_or_default();

    if let Some(last) = groups.last_mut() {
      if last.commit_id == *commit_id && last.end_line + 1 == *line_num {
        last.end_line = *line_num;
        continue;
      }
    }

    groups.push(BlameLineGroup {
      commit_id: commit_id.clone(),
      short_id,
      author_name,
      author_email,
      author_date,
      summary,
      start_line: *line_num,
      end_line: *line_num,
    });
  }

  groups
}

pub async fn get_file_log(repo_root: &Path, path: &str, skip: usize, limit: usize) -> Result<Vec<CommitEntry>> {
  let output = async_command("git")
    .args([
      "log",
      "--follow",
      "--format=%H%n%h%n%s%n%an%n%ae%n%aI%n%P%n---",
      &format!("--skip={skip}"),
      &format!("--max-count={limit}"),
      "--",
      path,
    ])
    .current_dir(repo_root)
    .output()
    .await?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::GitCli { message: stderr });
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut entries = Vec::new();

  for block in stdout.split("---\n") {
    let block = block.trim();
    if block.is_empty() {
      continue;
    }
    let lines: Vec<&str> = block.lines().collect();
    if lines.len() < 6 {
      continue;
    }

    let parent_ids: Vec<String> = if lines.len() > 6 && !lines[6].is_empty() {
      lines[6].split_whitespace().map(|s| s.to_string()).collect()
    } else {
      Vec::new()
    };

    let author_email = lines[4].to_string();
    let avatar_url = compute_avatar_url(&author_email);
    entries.push(CommitEntry {
      id: lines[0].to_string(),
      short_id: lines[1].to_string(),
      message: lines[2].to_string(),
      author_name: lines[3].to_string(),
      author_email,
      author_date: lines[5].to_string(),
      parent_ids,
      avatar_url,
    });
  }

  Ok(entries)
}

pub async fn get_last_commit_info(repo_root: &Path) -> Result<LastCommitInfo> {
  let output = async_command("git")
    .args(["log", "-1", "--format=%h|%s|%aI"])
    .current_dir(repo_root)
    .output()
    .await?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::GitCli { message: stderr });
  }

  let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
  let parts: Vec<&str> = stdout.splitn(3, '|').collect();

  if parts.len() < 3 {
    return Err(Error::Other {
      message: "Failed to parse last commit info".to_string(),
    });
  }

  Ok(LastCommitInfo {
    short_id: parts[0].to_string(),
    message: parts[1].to_string(),
    author_date: parts[2].to_string(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_single_commit_adjacent_lines_merged() {
    let output = "\
abc1234567890 1 1 3
author John Doe
author-mail <john@example.com>
author-time 1700000000
author-tz +0000
committer John Doe
committer-mail <john@example.com>
committer-time 1700000000
committer-tz +0000
summary Initial commit
filename test.txt
\tline one
abc1234567890 2 2
\tline two
abc1234567890 3 3
\tline three
";

    let groups = parse_porcelain_blame(output);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].commit_id, "abc1234567890");
    assert_eq!(groups[0].short_id, "abc1234");
    assert_eq!(groups[0].author_name, "John Doe");
    assert_eq!(groups[0].author_email, "john@example.com");
    assert_eq!(groups[0].summary, "Initial commit");
    assert_eq!(groups[0].start_line, 1);
    assert_eq!(groups[0].end_line, 3);
  }

  #[test]
  fn test_multiple_commits_separate_groups() {
    let output = "\
aaa1234567890 1 1 1
author Alice
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary First
filename test.txt
\tline one
bbb1234567890 2 2 1
author Bob
author-mail <bob@example.com>
author-time 1700001000
author-tz +0000
committer Bob
committer-mail <bob@example.com>
committer-time 1700001000
committer-tz +0000
summary Second
filename test.txt
\tline two
";

    let groups = parse_porcelain_blame(output);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].author_name, "Alice");
    assert_eq!(groups[0].summary, "First");
    assert_eq!(groups[0].start_line, 1);
    assert_eq!(groups[0].end_line, 1);
    assert_eq!(groups[1].author_name, "Bob");
    assert_eq!(groups[1].summary, "Second");
    assert_eq!(groups[1].start_line, 2);
    assert_eq!(groups[1].end_line, 2);
  }

  #[test]
  fn test_empty_output() {
    let groups = parse_porcelain_blame("");
    assert!(groups.is_empty());
  }

  #[test]
  fn test_same_commit_non_adjacent_no_false_merge() {
    let output = "\
aaa1234567890 1 1 1
author Alice
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary First
filename test.txt
\tline one
bbb1234567890 2 2 1
author Bob
author-mail <bob@example.com>
author-time 1700001000
author-tz +0000
committer Bob
committer-mail <bob@example.com>
committer-time 1700001000
committer-tz +0000
summary Second
filename test.txt
\tline two
aaa1234567890 3 3
\tline three
";

    let groups = parse_porcelain_blame(output);
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0].commit_id, "aaa1234567890");
    assert_eq!(groups[0].start_line, 1);
    assert_eq!(groups[0].end_line, 1);
    assert_eq!(groups[1].commit_id, "bbb1234567890");
    assert_eq!(groups[1].start_line, 2);
    assert_eq!(groups[1].end_line, 2);
    assert_eq!(groups[2].commit_id, "aaa1234567890");
    assert_eq!(groups[2].start_line, 3);
    assert_eq!(groups[2].end_line, 3);
  }
}
