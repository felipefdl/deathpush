use serde::{Deserialize, Serialize};

pub const MAX_RECENTS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
  pub path: String,
  pub name: String,
  pub last_opened: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Recents {
  pub projects: Vec<RecentProject>,
}

pub fn normalize_path(path: &str) -> String {
  let trimmed = path.trim_end_matches(['/', '\\']);
  if trimmed.is_empty() {
    path.to_string()
  } else {
    trimmed.to_string()
  }
}

pub fn name_from_path(path: &str) -> String {
  let normalized = normalize_path(path);
  normalized
    .rsplit(['/', '\\'])
    .next()
    .filter(|name| !name.is_empty())
    .unwrap_or(&normalized)
    .to_string()
}

impl Recents {
  /// Newest first, capped at `MAX_RECENTS`. `now` is an RFC 3339 timestamp.
  pub fn add(&mut self, path: &str, branch: Option<String>, now: &str) {
    let normalized = normalize_path(path);
    self.projects.retain(|project| project.path != normalized);
    self.projects.insert(
      0,
      RecentProject {
        name: name_from_path(&normalized),
        path: normalized,
        last_opened: now.to_string(),
        branch,
      },
    );
    self.projects.truncate(MAX_RECENTS);
  }

  pub fn remove(&mut self, path: &str) {
    let normalized = normalize_path(path);
    self.projects.retain(|project| project.path != normalized);
  }

  /// Newest first regardless of file order.
  pub fn sorted(&self) -> Vec<RecentProject> {
    let mut projects = self.projects.clone();
    projects.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    projects
  }

  /// Indices of projects whose name or path contains `query`, case-insensitive, newest first.
  pub fn filter(&self, query: &str) -> Vec<usize> {
    let needle = query.trim().to_lowercase();
    self
      .sorted()
      .iter()
      .enumerate()
      .filter(|(_, project)| {
        needle.is_empty()
          || project.name.to_lowercase().contains(&needle)
          || project.path.to_lowercase().contains(&needle)
      })
      .map(|(index, _)| index)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn add_moves_existing_to_front_and_caps_at_twenty() {
    let mut recents = Recents::default();
    for i in 0..25 {
      recents.add(&format!("/repos/p{i}/"), None, &format!("2026-09-03T00:00:{i:02}Z"));
    }
    assert_eq!(recents.projects.len(), MAX_RECENTS);
    assert_eq!(recents.projects[0].path, "/repos/p24");
    recents.add("/repos/p10", Some("main".into()), "2026-09-04T00:00:00Z");
    assert_eq!(recents.projects[0].path, "/repos/p10");
    assert_eq!(recents.projects[0].branch.as_deref(), Some("main"));
    assert_eq!(recents.projects.iter().filter(|p| p.path == "/repos/p10").count(), 1);
  }

  #[test]
  fn name_comes_from_the_last_segment() {
    assert_eq!(name_from_path("/Users/x/deathpush/"), "deathpush");
    assert_eq!(name_from_path("C:\\repos\\app"), "app");
  }

  #[test]
  fn filter_matches_name_or_path_case_insensitively() {
    let mut recents = Recents::default();
    recents.add("/work/Alpha", None, "2026-09-03T00:00:01Z");
    recents.add("/home/beta", None, "2026-09-03T00:00:02Z");
    assert_eq!(recents.filter("ALPHA"), vec![1]);
    assert_eq!(recents.filter("/home"), vec![0]);
    assert_eq!(recents.filter(""), vec![0, 1]);
  }

  #[test]
  fn remove_drops_by_normalized_path() {
    let mut recents = Recents::default();
    recents.add("/a", None, "2026-09-03T00:00:01Z");
    recents.remove("/a/");
    assert!(recents.projects.is_empty());
  }
}
