use crate::error::Result;
use crate::git::repository::GitRepository;
use crate::types::TagEntry;

pub fn list_tags(repo: &GitRepository) -> Result<Vec<TagEntry>> {
  let r = repo.inner();
  let tag_names = r.tag_names(None)?;
  let mut entries = Vec::new();

  for name in tag_names.iter().flatten() {
    let ref_name = format!("refs/tags/{}", name);
    let Ok(reference) = r.find_reference(&ref_name) else {
      continue;
    };

    let (is_annotated, message, target_id) = if let Ok(tag) = reference.peel_to_tag() {
      let msg = tag.message().map(|m| m.trim().to_string());
      let tid = tag.target_id().to_string();
      (true, msg, tid)
    } else if let Ok(commit) = reference.peel_to_commit() {
      (false, None, commit.id().to_string())
    } else {
      let oid = reference.target().map(|o| o.to_string()).unwrap_or_default();
      (false, None, oid)
    };

    entries.push(TagEntry {
      name: name.to_string(),
      message,
      target_id,
      is_annotated,
    });
  }

  entries.sort_by(|a, b| a.name.cmp(&b.name));
  Ok(entries)
}
