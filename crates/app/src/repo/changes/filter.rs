use std::time::Duration;

use gpui_kit::*;

pub const FILTER_DEBOUNCE_MS: u64 = 150;

#[allow(dead_code)]
pub fn matches_filter(path: &str, filter: &str) -> bool {
  filter.is_empty() || path.to_lowercase().contains(&filter.to_lowercase())
}

pub fn debounce<V: 'static>(
  cx: &mut Context<V>,
  generation: &mut u64,
  ms: u64,
  f: impl FnOnce(&mut V, &mut Context<V>) + 'static,
) {
  *generation += 1;
  cx.spawn(async move |this, cx| {
    cx.background_executor().timer(Duration::from_millis(ms)).await;
    let _ = this.update(cx, |this, cx| f(this, cx));
  })
  .detach();
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn filter_is_case_insensitive_substring() {
    assert!(matches_filter("src/Main.rs", ""));
    assert!(matches_filter("src/Main.rs", "main"));
    assert!(matches_filter("src/Main.rs", "SRC/m"));
    assert!(!matches_filter("src/Main.rs", "lib"));
  }
}
