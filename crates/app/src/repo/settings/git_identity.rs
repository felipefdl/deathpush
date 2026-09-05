pub struct GitIdentity {
  pub name: String,
  pub email: String,
  pub(crate) name_gen: u64,
  pub(crate) email_gen: u64,
}

pub const IDENTITY_DEBOUNCE_MS: u64 = 500;

impl GitIdentity {
  pub fn new() -> Self {
    Self {
      name: String::new(),
      email: String::new(),
      name_gen: 0,
      email_gen: 0,
    }
  }
}

pub fn should_save(previous: &str, current: &str) -> bool {
  !current.is_empty() && current != previous
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn identity_should_save_rules() {
    assert!(should_save("", "Ada"));
    assert!(should_save("Ada", "Grace"));
    assert!(!should_save("Ada", "Ada"));
    assert!(!should_save("Ada", ""));
    assert!(!should_save("", ""));
  }
}
