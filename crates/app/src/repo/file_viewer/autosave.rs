pub const AUTOSAVE_MS: u64 = 1000;
pub const LARGE_FILE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveState {
  pub saved_hash: String,
  pub dirty: bool,
  pub generation: u64,
}

impl SaveState {
  pub fn edited(&mut self) -> u64 {
    self.dirty = true;
    self.generation = self.generation.saturating_add(1);
    self.generation
  }

  pub fn should_save(&self, generation: u64) -> bool {
    self.dirty && self.generation == generation
  }

  pub fn saved(&mut self, new_hash: String, generation: u64) {
    if self.generation == generation {
      self.dirty = false;
      self.saved_hash = new_hash;
    }
  }

  pub fn should_reload_external(&self, incoming_hash: &str) -> bool {
    !self.dirty && self.saved_hash != incoming_hash
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveToken {
  pub path: String,
  pub generation: u64,
}

pub fn token_still_valid(token: &SaveToken, current_path: Option<&str>, save: &SaveState) -> bool {
  current_path == Some(token.path.as_str()) && save.should_save(token.generation)
}

pub fn should_complete_save(
  current_path: Option<&str>,
  event_path: &str,
  current_generation: u64,
  event_generation: u64,
  dirty: bool,
) -> bool {
  dirty && current_path == Some(event_path) && current_generation == event_generation
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn save_state_tracks_generations() {
    let mut s = SaveState {
      saved_hash: "h0".into(),
      dirty: false,
      generation: 0,
    };
    let g1 = s.edited();
    let g2 = s.edited();
    assert!(!s.should_save(g1) && s.should_save(g2));
    s.saved("h1".into(), g1);
    assert!(s.dirty, "an older generation does not clear dirty");
    s.saved("h2".into(), g2);
    assert!(!s.dirty && s.saved_hash == "h2");
    assert!(s.should_reload_external("h3") && !s.should_reload_external("h2"));
    s.edited();
    assert!(!s.should_reload_external("h3"), "no reload while a save is pending");
  }

  #[test]
  fn timer_token_for_a_does_not_save_after_switch_to_b() {
    let token = SaveToken {
      path: "a.rs".into(),
      generation: 1,
    };
    let mut save = SaveState {
      saved_hash: "ha".into(),
      dirty: true,
      generation: 1,
    };
    assert!(token_still_valid(&token, Some("a.rs"), &save));
    save.dirty = false;
    save.saved_hash = "hb".into();
    assert!(
      !token_still_valid(&token, Some("b.rs"), &save),
      "path switch drops the token even when generation is unchanged"
    );
    let g2 = save.edited();
    assert_eq!(g2, 2, "generation stays monotonic across files");
    assert!(!token_still_valid(&token, Some("b.rs"), &save));
    assert!(!save.should_save(token.generation));
  }

  #[test]
  fn completes_save_only_for_current_generation_and_path() {
    assert!(should_complete_save(Some("a.rs"), "a.rs", 2, 2, true));
    assert!(
      !should_complete_save(Some("a.rs"), "a.rs", 2, 1, true),
      "older generation is ignored"
    );
    assert!(
      !should_complete_save(Some("a.rs"), "a.rs", 2, 2, false),
      "already completed"
    );
    assert!(!should_complete_save(Some("b.rs"), "a.rs", 2, 2, true));
  }
}
