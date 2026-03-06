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
