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

#[allow(dead_code)]
pub fn sha256_utf8(text: &str) -> String {
  deathpush_core::content_hash::sha256_utf8(text)
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
}
