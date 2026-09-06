use std::path::PathBuf;

#[cfg(not(windows))]
use percent_encoding::percent_decode_str;

/// `deathpush:///Users/x/repo` or `deathpush://Users/x/repo` to `/Users/x/repo`, only when that directory exists.
pub fn repository_path_from_url(raw: &str) -> Option<PathBuf> {
  #[cfg(windows)]
  let candidate = {
    let (scheme, path) = raw.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("deathpush") {
      return None;
    }
    url::Url::parse(&format!("file:{path}")).ok()?.to_file_path().ok()?
  };
  #[cfg(not(windows))]
  let candidate = {
    let url = url::Url::parse(raw).ok()?;
    if url.scheme() != "deathpush" {
      return None;
    }
    let host = url.host_str().unwrap_or("");
    let path = percent_decode_str(url.path()).decode_utf8_lossy().into_owned();
    let joined = if host.is_empty() {
      path
    } else {
      format!("/{host}{path}")
    };
    PathBuf::from(joined)
  };
  candidate.is_dir().then_some(candidate)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn triple_slash_form_resolves_an_existing_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let url = format!("deathpush://{}", dir.path().display());
    assert_eq!(repository_path_from_url(&url).unwrap(), dir.path());
    let triple = format!(
      "deathpush:///{}",
      dir.path().display().to_string().trim_start_matches('/')
    );
    assert_eq!(repository_path_from_url(&triple).unwrap(), dir.path());
  }

  #[test]
  fn percent_encoded_segments_decode() {
    let dir = tempfile::TempDir::new().unwrap();
    let spaced = dir.path().join("my repo");
    std::fs::create_dir(&spaced).unwrap();
    let url = format!("deathpush://{}/my%20repo", dir.path().display());
    assert_eq!(repository_path_from_url(&url).unwrap(), spaced);
  }

  #[test]
  fn other_schemes_and_missing_directories_are_rejected() {
    assert!(repository_path_from_url("https://example.com/x").is_none());
    assert!(repository_path_from_url("deathpush:///definitely/missing/dir").is_none());
    assert!(repository_path_from_url("not a url").is_none());
  }
}
