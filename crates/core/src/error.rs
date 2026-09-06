use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("Git error: {0}")]
  Git(#[from] git2::Error),

  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("Git CLI failed: {0}")]
  GitCli(String),

  #[error("No repository open")]
  NoRepository,

  #[error("{0}")]
  Other(String),
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_repository_display() {
    let err = Error::NoRepository;
    assert_eq!(err.to_string(), "No repository open");
  }

  #[test]
  fn git_cli_display() {
    let err = Error::GitCli("msg".into());
    assert_eq!(err.to_string(), "Git CLI failed: msg");
  }

  #[test]
  fn other_display() {
    let err = Error::Other("custom".into());
    assert_eq!(err.to_string(), "custom");
  }

  #[test]
  fn error_serializes_to_string() {
    let err = Error::NoRepository;
    let json = serde_json::to_string(&err).unwrap();
    assert_eq!(json, "\"No repository open\"");
  }

  #[test]
  fn io_error_display_contains_io_error() {
    let err = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "nope"));
    assert!(err.to_string().contains("IO error"));
  }
}
