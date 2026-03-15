use serde::Serialize;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum Error {
  #[error("Git error: {message}")]
  Git { message: String },

  #[error("IO error: {message}")]
  Io { message: String },

  #[error("Git CLI failed: {message}")]
  GitCli { message: String },

  #[error("No repository open")]
  NoRepository,

  #[error("{message}")]
  Other { message: String },
}

impl From<git2::Error> for Error {
  fn from(e: git2::Error) -> Self {
    Error::Git { message: e.to_string() }
  }
}

impl From<std::io::Error> for Error {
  fn from(e: std::io::Error) -> Self {
    Error::Io { message: e.to_string() }
  }
}

impl Serialize for Error {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    serializer.serialize_str(&self.to_string())
  }
}

impl Error {
  pub fn other(message: impl Into<String>) -> Self {
    Error::Other {
      message: message.into(),
    }
  }

  pub fn git_cli(message: impl Into<String>) -> Self {
    Error::GitCli {
      message: message.into(),
    }
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
    let err = Error::GitCli { message: "msg".into() };
    assert_eq!(err.to_string(), "Git CLI failed: msg");
  }

  #[test]
  fn other_display() {
    let err = Error::Other {
      message: "custom".into(),
    };
    assert_eq!(err.to_string(), "custom");
  }

  #[test]
  fn error_serializes_to_string() {
    let err = Error::NoRepository;
    let json = serde_json::to_string(&err).unwrap();
    assert_eq!(json, "\"No repository open\"");
  }

  #[test]
  fn io_error_converts() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "nope");
    let err: Error = io_err.into();
    assert!(err.to_string().contains("IO error"));
  }

  #[test]
  fn git_error_converts() {
    let git_err = git2::Error::from_str("bad ref");
    let err: Error = git_err.into();
    assert!(err.to_string().contains("Git error"));
  }
}
