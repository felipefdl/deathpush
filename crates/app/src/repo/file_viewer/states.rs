use std::sync::Arc;

use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::Button;
use gpui_kit::*;

use super::autosave::LARGE_FILE_BYTES;
use super::view::FileViewer;
use crate::repo::diff::states::decode_data_uri;
use crate::repo::state::OpenFile;
use crate::theme::hsla;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerKind {
  Empty,
  Loading,
  Text,
  Image,
  Binary,
  Large,
}

pub fn classify(open: Option<&OpenFile>) -> ViewerKind {
  let Some(open) = open else {
    return ViewerKind::Empty;
  };
  let Some(content) = open.content.as_ref() else {
    return ViewerKind::Loading;
  };
  match content.file_type.as_str() {
    "image" => ViewerKind::Image,
    "binary" => ViewerKind::Binary,
    "large" => ViewerKind::Large,
    _ if content.content.len() > LARGE_FILE_BYTES => ViewerKind::Large,
    _ => ViewerKind::Text,
  }
}

pub fn decode_image(uri: &str) -> Option<Arc<Image>> {
  if uri.is_empty() {
    return None;
  }
  let (format, bytes) = decode_data_uri(uri)?;
  Some(Arc::new(Image::from_bytes(format, bytes)))
}

pub fn render_empty(palette: UiPalette) -> impl IntoElement {
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
        .child("Select a file to view its contents"),
    )
}

pub fn render_image(image: Option<Arc<Image>>) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .items_center()
    .justify_center()
    .p_3()
    .child(match image {
      Some(image) => img(image)
        .object_fit(ObjectFit::Contain)
        .w_full()
        .h_full()
        .into_any_element(),
      None => div().into_any_element(),
    })
}

pub fn render_binary(view: WeakEntity<FileViewer>, palette: UiPalette) -> impl IntoElement {
  message_with_open("Binary file cannot be displayed", "icons/binary.svg", view, palette)
}

pub fn render_large(view: WeakEntity<FileViewer>, palette: UiPalette) -> impl IntoElement {
  message_with_open(
    "File is too large to display (over 5 MB)",
    "icons/triangle-alert.svg",
    view,
    palette,
  )
}

fn message_with_open(
  message: &'static str,
  icon: &'static str,
  view: WeakEntity<FileViewer>,
  palette: UiPalette,
) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_3()
    .child(
      svg()
        .path(icon)
        .size(px(32.0))
        .text_color(hsla(palette.foreground))
        .opacity(0.4),
    )
    .child(
      div()
        .text_size(px(13.0))
        .text_color(hsla(palette.foreground))
        .opacity(0.7)
        .child(message),
    )
    .child(
      Button::new("file-open-external")
        .outline()
        .label("Open in External Editor")
        .on_click(move |_, _, cx| {
          let _ = view.update(cx, |this, cx| this.open_external(cx));
        }),
    )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::types::FileContent;

  fn file(file_type: &str, content: &str) -> OpenFile {
    OpenFile {
      path: "src/main.rs".into(),
      content: Some(FileContent {
        path: "src/main.rs".into(),
        content: content.to_string(),
        language: Some("rust".into()),
        file_type: file_type.into(),
        content_hash: "h".into(),
      }),
      pending_line: None,
      load_id: 1,
      dirty: false,
    }
  }

  #[test]
  fn classify_viewer_kinds() {
    assert_eq!(classify(None), ViewerKind::Empty);
    let loading = OpenFile {
      path: "src/main.rs".into(),
      content: None,
      pending_line: None,
      load_id: 1,
      dirty: false,
    };
    assert_eq!(classify(Some(&loading)), ViewerKind::Loading);
    assert_eq!(classify(Some(&file("text", "x"))), ViewerKind::Text);
    assert_eq!(
      classify(Some(&file("image", "data:image/png;base64,AA=="))),
      ViewerKind::Image
    );
    assert_eq!(classify(Some(&file("binary", ""))), ViewerKind::Binary);
    assert_eq!(
      classify(Some(&file("text", &"x".repeat(LARGE_FILE_BYTES + 1)))),
      ViewerKind::Large
    );
  }
}
