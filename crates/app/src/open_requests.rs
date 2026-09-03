use std::path::PathBuf;

use deathpush_core::deep_link::repository_path_from_url;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

/// Something the OS asked the app to open: a repository from a deep link, or a plain reopen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenRequest {
  Repository(PathBuf),
  NewWindow,
}

/// The sender side lives in the platform callbacks; the receiver is drained on the main thread.
pub struct OpenRequests {
  pub tx: UnboundedSender<OpenRequest>,
  pub rx: Option<UnboundedReceiver<OpenRequest>>,
}

impl OpenRequests {
  pub fn new() -> Self {
    let (tx, rx) = unbounded();
    Self { tx, rx: Some(rx) }
  }

  pub fn from_urls(urls: &[String]) -> Vec<OpenRequest> {
    urls
      .iter()
      .filter_map(|url| repository_path_from_url(url))
      .map(OpenRequest::Repository)
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn urls_become_repository_requests_only_when_the_directory_exists() {
    let dir = tempfile::TempDir::new().unwrap();
    let good = format!("deathpush://{}", dir.path().display());
    let requests = OpenRequests::from_urls(&[good, "deathpush:///nope/missing".into(), "https://x".into()]);
    assert_eq!(requests, vec![OpenRequest::Repository(dir.path().to_path_buf())]);
  }
}
