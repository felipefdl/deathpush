use deathpush_core::session::types::DiffPayload;
use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::Button;
use gpui_kit::*;

use super::panel::DiffPanel;
use crate::repo::image_load::prepare;
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
    _ if payload.patch.len() + payload.merge_text.as_ref().map_or(0, String::len) > LARGE_FILE_BYTES => DiffKind::Large,
    _ => DiffKind::Text,
  }
}

pub fn render_empty(has_repository: bool, palette: UiPalette) -> impl IntoElement {
  let message = if has_repository {
    "Select a file to view changes"
  } else {
    "Diffs need a Git repository"
  };
  div()
    .size_full()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_3()
    .child(
      svg()
        .path("brand/deathpush.svg")
        .size(px(80.0))
        .text_color(hsla(palette.mark))
        .opacity(0.07),
    )
    .child(
      div()
        .text_size(px(13.0))
        .text_color(hsla(palette.foreground))
        .opacity(0.4)
        .child(message),
    )
}

pub fn render_binary(view: WeakEntity<DiffPanel>, palette: UiPalette) -> impl IntoElement {
  message_with_open("Binary file cannot be displayed", view, palette)
}

pub fn render_large(view: WeakEntity<DiffPanel>, palette: UiPalette) -> impl IntoElement {
  message_with_open("File is too large to display (over 5 MB)", view, palette)
}

pub fn render_image(old: Option<ImageSource>, new: Option<ImageSource>, palette: UiPalette) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_row()
    .gap_2()
    .p_3()
    .child(image_pane("Original", old, palette))
    .child(image_pane("Modified", new, palette))
}

pub(crate) fn decode_images(payload: &DiffPayload) -> (Option<ImageSource>, Option<ImageSource>) {
  (decode_to_image(&payload.old_image), decode_to_image(&payload.new_image))
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

fn image_pane(label: &'static str, image: Option<ImageSource>, palette: UiPalette) -> Div {
  let body = match image {
    Some(image) => div()
      .flex_1()
      .min_h_0()
      .flex()
      .items_center()
      .justify_center()
      .child(img(image).object_fit(ObjectFit::Contain).max_h(px(480.0)).w_full()),
    None => div().flex_1().min_h(px(120.0)).bg(hsla(palette.muted.with_alpha(30))),
  };
  div()
    .flex_1()
    .min_w_0()
    .flex()
    .flex_col()
    .gap_2()
    .child(
      div()
        .flex_shrink_0()
        .text_size(px(11.0))
        .text_color(hsla(palette.muted_foreground))
        .child(label),
    )
    .child(body)
}

/// A diff side is a data URI. Both sides go through the viewer's image pipeline, so a wide
/// screenshot is downscaled once per selection instead of overflowing the texture atlas.
fn decode_to_image(uri: &str) -> Option<ImageSource> {
  if uri.is_empty() {
    return None;
  }
  let (format, bytes) = decode_data_uri(uri)?;
  Some(prepare(format, &bytes)?.source())
}

pub(crate) fn decode_data_uri(uri: &str) -> Option<(ImageFormat, Vec<u8>)> {
  let rest = uri.strip_prefix("data:")?;
  let (meta, data) = rest.split_once(',')?;
  let mime = meta.split(';').next()?.trim();
  let format = match mime {
    "image/png" => ImageFormat::Png,
    "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
    "image/webp" => ImageFormat::Webp,
    "image/gif" => ImageFormat::Gif,
    "image/bmp" => ImageFormat::Bmp,
    "image/x-icon" | "image/ico" => ImageFormat::Ico,
    "image/tiff" => ImageFormat::Tiff,
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

  fn text_payload(patch: &str) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      patch: patch.to_string(),
      merge_text: None,
      old_image: String::new(),
      new_image: String::new(),
      language: Some("rust".into()),
      file_type: "text".into(),
      hunks: vec![],
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
    p.patch = "x".repeat(LARGE_FILE_BYTES + 1);
    assert_eq!(classify(Some(&p)), DiffKind::Large);
  }

  #[test]
  fn every_image_mime_core_emits_can_be_decoded() {
    // The diff panel used to accept five mime types while core classified ten extensions as
    // images, so a bmp, ico, or tiff diff showed an empty grey box.
    for mime in [
      "image/png",
      "image/jpeg",
      "image/webp",
      "image/gif",
      "image/bmp",
      "image/x-icon",
      "image/tiff",
      "image/svg+xml",
    ] {
      let uri = format!("data:{mime};base64,QUJD");
      assert!(decode_data_uri(&uri).is_some(), "{mime} must decode");
    }
    assert!(decode_data_uri("data:application/octet-stream;base64,QUJD").is_none());
  }
}
