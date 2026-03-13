use git2::BranchType;

use crate::error::Result;
use crate::git::repository::GitRepository;
use crate::types::BranchEntry;

pub fn list_branches(repo: &GitRepository) -> Result<Vec<BranchEntry>> {
  let r = repo.inner();
  let mut entries = Vec::new();

  for branch_result in r.branches(None)? {
    let (branch, branch_type) = branch_result?;
    let name = match branch.name()? {
      Some(n) => n.to_string(),
      None => continue,
    };

    let is_head = branch.is_head();
    let is_remote = branch_type == BranchType::Remote;

    let upstream = branch
      .upstream()
      .ok()
      .and_then(|u| u.name().ok().flatten().map(|s| s.to_string()));

    let (ahead, behind) = if !is_remote {
      if let (Some(local_oid), Ok(upstream_branch)) = (branch.get().target(), branch.upstream()) {
        if let Some(upstream_oid) = upstream_branch.get().target() {
          r.graph_ahead_behind(local_oid, upstream_oid).unwrap_or((0, 0))
        } else {
          (0, 0)
        }
      } else {
        (0, 0)
      }
    } else {
      (0, 0)
    };

    entries.push(BranchEntry {
      name,
      is_head,
      is_remote,
      upstream,
      ahead: ahead as u32,
      behind: behind as u32,
    });
  }

  // Sort: current branch first, then local branches, then remote
  entries.sort_by(|a, b| {
    b.is_head
      .cmp(&a.is_head)
      .then(a.is_remote.cmp(&b.is_remote))
      .then(a.name.cmp(&b.name))
  });

  Ok(entries)
}
