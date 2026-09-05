/// Git `user.name` / `user.email` as shown on the Settings page.
pub(crate) struct GitIdentity {
  pub name: String,
  pub email: String,
  pub(crate) name_gen: u64,
  pub(crate) email_gen: u64,
  pub(crate) name_done_gen: u64,
  pub(crate) email_done_gen: u64,
  pub(crate) name_inflight: Option<u64>,
  pub(crate) email_inflight: Option<u64>,
  pub(crate) name_ready_gen: u64,
  pub(crate) email_ready_gen: u64,
}

/// Quiet period before writing a Git identity field.
pub(crate) const IDENTITY_DEBOUNCE_MS: u64 = 500;

impl GitIdentity {
  pub(crate) fn new() -> Self {
    Self {
      name: String::new(),
      email: String::new(),
      name_gen: 0,
      email_gen: 0,
      name_done_gen: 0,
      email_done_gen: 0,
      name_inflight: None,
      email_inflight: None,
      name_ready_gen: 0,
      email_ready_gen: 0,
    }
  }

  pub(crate) fn name_pending(&self) -> bool {
    self.name_inflight.is_some() || self.name_done_gen < self.name_gen
  }

  pub(crate) fn email_pending(&self) -> bool {
    self.email_inflight.is_some() || self.email_done_gen < self.email_gen
  }
}

/// Write the field when it is a non-empty change from the last saved value.
pub(crate) fn should_save(previous: &str, current: &str) -> bool {
  !current.is_empty() && current != previous
}

/// Skip a loaded Git identity value while the user is editing or a save is in flight.
pub(crate) fn should_apply_loaded(pending: bool, focused: bool) -> bool {
  !pending && !focused
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

  #[test]
  fn identity_skips_loaded_values_while_editing() {
    assert!(should_apply_loaded(false, false));
    assert!(!should_apply_loaded(true, false));
    assert!(!should_apply_loaded(false, true));
    assert!(!should_apply_loaded(true, true));
  }
}
