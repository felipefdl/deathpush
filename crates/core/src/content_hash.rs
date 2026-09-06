use sha2::{Digest, Sha256};

pub fn sha256_utf8(text: &str) -> String {
  format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[cfg(test)]
mod tests {
  use super::sha256_utf8;

  #[test]
  fn sha256_utf8_matches_web_crypto_hello_newline() {
    assert_eq!(
      sha256_utf8("hello\n"),
      "5891b5b522d5df086d0ff0b110fbd9d21bb4fc7163af34d08286a2e846f6be03"
    );
  }

  #[test]
  fn sha256_utf8_hashes_utf8_not_latin1() {
    assert_eq!(
      sha256_utf8("café"),
      "850f7dc43910ff890f8879c0ed26fe697c93a067ad93a7d50f466a7028a9bf4e"
    );
  }

  #[test]
  fn sha256_utf8_is_not_a_git_blob_oid() {
    let text = "hello\n";
    let git_blob = format!("blob {}\0{text}", text.len());
    assert_ne!(sha256_utf8(text), sha256_utf8(&git_blob));
  }

  #[test]
  fn sha256_utf8_empty_matches_web_crypto() {
    assert_eq!(
      sha256_utf8(""),
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
  }
}
