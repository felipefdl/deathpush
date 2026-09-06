pub fn valid_entry_name(name: &str) -> bool {
  !name.is_empty() && !name.contains('/') && !name.contains('\\') && !name.contains('\0') && name != "." && name != ".."
}

pub fn stem_range(name: &str) -> std::ops::Range<usize> {
  match name.rfind('.') {
    Some(index) if index > 0 => 0..index,
    _ => 0..name.len(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn entry_names_are_validated() {
    assert!(
      valid_entry_name("a.txt")
        && !valid_entry_name("")
        && !valid_entry_name("a/b")
        && !valid_entry_name("..")
        && !valid_entry_name(".")
    );
  }
}
