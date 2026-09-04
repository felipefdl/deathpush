use deathpush_core::session::types::DiffPayload;
use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::Button;
use gpui_kit::*;

use super::panel::DiffPanel;
use crate::theme::hsla;

pub const LARGE_FILE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
  Empty,
  Text,
  Image,
  Binary,
  Large,
}

pub fn classify(payload: Option<&DiffPayload>) -> DiffKind {
  let Some(payload) = payload else {
    return DiffKind::Empty;
  };
  match payload.file_type.as_str() {
    "image" => DiffKind::Image,
    "binary" => DiffKind::Binary,
    _ if payload.original.len() > LARGE_FILE_BYTES || payload.modified.len() > LARGE_FILE_BYTES => DiffKind::Large,
    _ => DiffKind::Text,
  }
}

pub fn render_empty(palette: UiPalette) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_2()
    .child(
      svg()
        .path("brand/deathpush.svg")
        .size(px(48.0))
        .text_color(hsla(palette.mark))
        .opacity(0.12),
    )
    .child(
      div()
        .text_size(px(13.0))
        .text_color(hsla(palette.foreground))
        .opacity(0.18)
        .child("Select a file to view changes"),
    )
}

pub fn render_binary(view: WeakEntity<DiffPanel>, palette: UiPalette) -> impl IntoElement {
  message_with_open("Binary file cannot be displayed", view, palette)
}

pub fn render_large(view: WeakEntity<DiffPanel>, palette: UiPalette) -> impl IntoElement {
  message_with_open("File is too large to display (over 5 MB)", view, palette)
}

pub fn render_image(payload: &DiffPayload, palette: UiPalette) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_row()
    .gap_2()
    .p_3()
    .child(image_pane(&payload.original, payload.presence.old_exists, palette))
    .child(image_pane(&payload.modified, payload.presence.new_exists, palette))
}

fn message_with_open(message: &'static str, view: WeakEntity<DiffPanel>, palette: UiPalette) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_3()
    .child(
      div()
        .text_size(px(13.0))
        .text_color(hsla(palette.muted_foreground))
        .child(message),
    )
    .child(
      Button::new("diff-open-external")
        .outline()
        .label("Open in External Editor")
        .on_click(move |_, _, cx| {
          let _ = view.update(cx, |this, cx| this.open_selected_in_editor(cx));
        }),
    )
}

fn image_pane(uri: &str, exists: bool, palette: UiPalette) -> Div {
  let empty = div()
    .flex_1()
    .min_w_0()
    .min_h(px(120.0))
    .bg(hsla(palette.muted.with_alpha(30)));
  if !exists || uri.is_empty() {
    return empty;
  }
  match decode_data_uri(uri) {
    Some((format, bytes)) => div().flex_1().min_w_0().flex().items_center().justify_center().child(
      img(std::sync::Arc::new(Image::from_bytes(format, bytes)))
        .object_fit(ObjectFit::Contain)
        .max_h(px(480.0))
        .w_full(),
    ),
    None => empty,
  }
}

fn decode_data_uri(uri: &str) -> Option<(ImageFormat, Vec<u8>)> {
  let rest = uri.strip_prefix("data:")?;
  let (meta, data) = rest.split_once(',')?;
  let mime = meta.split(';').next()?.trim();
  let format = match mime {
    "image/png" => ImageFormat::Png,
    "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
    "image/webp" => ImageFormat::Webp,
    "image/gif" => ImageFormat::Gif,
    "image/svg+xml" => ImageFormat::Svg,
    _ => return None,
  };
  use base64::Engine as _;
  let bytes = base64::engine::general_purpose::STANDARD.decode(data.as_bytes()).ok()?;
  Some((format, bytes))
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::session::types::DiffPresence;

  fn text_payload(modified: &str) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      original: String::new(),
      modified: modified.to_string(),
      language: Some("rust".into()),
      file_type: "text".into(),
      hunks: vec![],
      presence: DiffPresence {
        old_exists: true,
        new_exists: true,
      },
      editable: true,
      enable_line_selection: true,
      staged: false,
      content_hash: "h".into(),
    }
  }

  #[test]
  fn classify_kinds() {
    assert_eq!(classify(None), DiffKind::Empty);
    let mut p = text_payload("x");
    assert_eq!(classify(Some(&p)), DiffKind::Text);
    p.file_type = "image".into();
    assert_eq!(classify(Some(&p)), DiffKind::Image);
    p.file_type = "binary".into();
    assert_eq!(classify(Some(&p)), DiffKind::Binary);
    p.file_type = "text".into();
    p.modified = "x".repeat(LARGE_FILE_BYTES + 1);
    assert_eq!(classify(Some(&p)), DiffKind::Large);
  }
}
