use deathpush_core::ops::history::{avatar_hue, initials};
use deathpush_core::theme::{Rgba, UiPalette};
use deathpush_core::types::CommitEntry;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::theme::hsla;

pub enum Avatar {
  Remote(SharedString),
  Initials { text: String, hue: f32 },
}

pub fn avatar_for(entry: &CommitEntry) -> Avatar {
  if entry.avatar_url.is_empty() {
    Avatar::Initials {
      text: initials(&entry.author_name),
      hue: avatar_hue(&entry.author_name),
    }
  } else {
    Avatar::Remote(entry.avatar_url.clone().into())
  }
}

/// Mix the author hue into the theme background. `gpui_kit::hsla` is used because the hue
/// comes from core's `avatar_hue` (hashed author name), not a literal color.
pub fn fallback_fill(hue: f32, background: Rgba) -> Rgba {
  let rgb: gpui_kit::Rgba = gpui_kit::hsla(hue / 360., 0.55, 0.5, 1.).into();
  background.mix(
    Rgba {
      r: (rgb.r * 255.0).round() as u8,
      g: (rgb.g * 255.0).round() as u8,
      b: (rgb.b * 255.0).round() as u8,
      a: (rgb.a * 255.0).round() as u8,
    },
    0.6,
  )
}

pub fn render_avatar(entry: &CommitEntry, palette: &UiPalette) -> impl IntoElement {
  let avatar = avatar_for(entry);
  let (text, hue) = match &avatar {
    Avatar::Remote(_) => (initials(&entry.author_name), avatar_hue(&entry.author_name)),
    Avatar::Initials { text, hue } => (text.clone(), *hue),
  };
  let remote = match avatar {
    Avatar::Remote(url) => Some(url),
    Avatar::Initials { .. } => None,
  };
  let fill = fallback_fill(hue, palette.background);
  div()
    .relative()
    .size(px(24.0))
    .flex_shrink_0()
    .rounded_full()
    .overflow_hidden()
    .child(
      div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(hsla(fill))
        .child(
          div()
            .text_size(px(10.0))
            .font_weight(FontWeight::BOLD)
            .text_color(hsla(palette.foreground))
            .child(text),
        ),
    )
    .when_some(remote, |el, url| {
      el.child(
        img(url.to_string())
          .absolute()
          .inset_0()
          .size(px(24.0))
          .object_fit(ObjectFit::Cover),
      )
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  fn entry(url: &str) -> CommitEntry {
    CommitEntry {
      id: "0123456789abcdef0123456789abcdef01234567".into(),
      short_id: "0123456".into(),
      message: "subject".into(),
      author_name: "Ana Lima".into(),
      author_email: "ana@example.com".into(),
      author_date: "2026-09-01T00:00:00Z".into(),
      parent_ids: vec![],
      avatar_url: url.into(),
    }
  }

  #[test]
  fn avatar_for_prefers_remote_url() {
    let url = "https://avatars.githubusercontent.com/u/1";
    match avatar_for(&entry(url)) {
      Avatar::Remote(remote) => assert_eq!(remote.as_ref(), url),
      Avatar::Initials { .. } => panic!("expected remote avatar"),
    }
  }

  #[test]
  fn initials_fallback_hue_is_stable() {
    match avatar_for(&entry("")) {
      Avatar::Initials { text, hue } => {
        assert_eq!(text, "AL");
        assert_eq!(hue, avatar_hue("Ana Lima"));
        assert_eq!(hue, 300.0);
        assert_eq!(hue, avatar_hue("Ana Lima"));
      }
      Avatar::Remote(_) => panic!("expected initials fallback"),
    }
  }
}
