use deathpush_core::config::settings::BellStyle;

/// `Sound` has no platform beep in gpui, so it flashes like `Visual` and `Both`.
pub fn bell_flashes(style: BellStyle) -> bool {
  !matches!(style, BellStyle::Off)
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn bell_flashes_for_every_style_except_off() {
    assert!(!bell_flashes(BellStyle::Off));
    assert!(bell_flashes(BellStyle::Sound));
    assert!(bell_flashes(BellStyle::Visual));
    assert!(bell_flashes(BellStyle::Both));
  }
}
