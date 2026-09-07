//! PDF pane state and chrome. Core rasterizes a page on a background thread; this holds the
//! result, the page cursor, and the footer that walks through the document.

use std::sync::Arc;

use deathpush_core::PdfPageImage;
use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Disableable, Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::states::message_with_open;
use super::view::FileViewer;
use crate::theme::hsla;

/// Rasterization target for the longest page edge. The pane scales the result to fit, so more
/// pixels only cost VRAM.
const PAGE_EDGE: u32 = 2048;

#[derive(Debug)]
pub enum PdfLoad {
  Pending,
  Ready(Arc<RenderImage>),
  Failed(String),
}

/// A page the viewer still has to ask core for.
pub struct PdfRequest {
  pub path: String,
  pub page: usize,
  pub max_edge: u32,
  pub generation: u64,
}

/// A page already converted for the atlas: index, document length, and the texture data.
pub type PreparedPage = (usize, usize, Arc<RenderImage>);

pub struct PdfPane {
  path: Option<String>,
  hash: Option<String>,
  page: usize,
  page_count: usize,
  generation: u64,
  load: PdfLoad,
}

impl Default for PdfPane {
  fn default() -> Self {
    Self::new()
  }
}

impl PdfPane {
  pub fn new() -> Self {
    Self {
      path: None,
      hash: None,
      page: 0,
      page_count: 0,
      generation: 0,
      load: PdfLoad::Pending,
    }
  }

  /// Point the pane at a document. Returns a request when the first page must be rasterized.
  pub fn sync(&mut self, path: &str, hash: &str) -> Option<PdfRequest> {
    if self.path.as_deref() == Some(path) && self.hash.as_deref() == Some(hash) {
      return None;
    }
    self.path = Some(path.to_string());
    self.hash = Some(hash.to_string());
    self.page = 0;
    self.page_count = 0;
    Some(self.request())
  }

  pub fn go_to(&mut self, page: usize) -> Option<PdfRequest> {
    if page == self.page || (self.page_count > 0 && page >= self.page_count) {
      return None;
    }
    self.page = page;
    Some(self.request())
  }

  /// Land a rasterized page. Stale answers (an older generation) are dropped.
  pub fn apply_prepared(&mut self, generation: u64, result: Result<PreparedPage, String>) {
    if generation != self.generation {
      return;
    }
    match result {
      Ok((page, page_count, image)) => {
        self.page_count = page_count;
        self.page = page;
        self.load = PdfLoad::Ready(image);
      }
      Err(message) => self.load = PdfLoad::Failed(message),
    }
  }

  /// Forget the document. The generation keeps counting up, so a page still being rasterized for
  /// the previous document cannot land here as an answer for the next one. Called from render, so
  /// an already empty pane costs nothing.
  pub fn reset(&mut self) {
    if self.path.is_none() && matches!(self.load, PdfLoad::Pending) {
      return;
    }
    self.path = None;
    self.hash = None;
    self.page = 0;
    self.page_count = 0;
    self.generation += 1;
    self.load = PdfLoad::Pending;
  }

  #[cfg(test)]
  pub(crate) fn load(&self) -> &PdfLoad {
    &self.load
  }

  #[cfg(test)]
  pub(crate) fn page_count(&self) -> usize {
    self.page_count
  }

  pub fn render(&self, palette: UiPalette, view: WeakEntity<FileViewer>) -> AnyElement {
    let body = match &self.load {
      PdfLoad::Pending => div().flex_1().min_h_0().into_any_element(),
      PdfLoad::Ready(image) => div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p_3()
        .child(img(image.clone()).object_fit(ObjectFit::Contain).w_full().h_full())
        .into_any_element(),
      PdfLoad::Failed(message) => {
        message_with_open(message.clone(), "icons/triangle-alert.svg", view.clone(), palette).into_any_element()
      }
    };

    div()
      .size_full()
      .flex()
      .flex_col()
      .child(body)
      .when(self.page_count > 1, |el| el.child(self.footer(palette, view)))
      .into_any_element()
  }

  fn request(&mut self) -> PdfRequest {
    self.generation += 1;
    self.load = PdfLoad::Pending;
    PdfRequest {
      path: self.path.clone().unwrap_or_default(),
      page: self.page,
      max_edge: PAGE_EDGE,
      generation: self.generation,
    }
  }

  fn footer(&self, palette: UiPalette, view: WeakEntity<FileViewer>) -> impl IntoElement {
    let page = self.page;
    let last = self.page_count.saturating_sub(1);
    div()
      .h(px(30.0))
      .flex_shrink_0()
      .flex()
      .items_center()
      .justify_center()
      .gap_2()
      .border_t_1()
      .border_color(hsla(palette.border))
      .child(page_button(
        "pdf-prev",
        "icons/chevron-left.svg",
        page > 0,
        page.saturating_sub(1),
        view.clone(),
      ))
      .child(
        div()
          .text_size(px(12.0))
          .text_color(hsla(palette.muted_foreground))
          .child(format!("Page {} of {}", page + 1, self.page_count)),
      )
      .child(page_button(
        "pdf-next",
        "icons/chevron-right.svg",
        page < last,
        page + 1,
        view,
      ))
  }
}

fn page_button(
  id: &'static str,
  icon: &'static str,
  enabled: bool,
  target: usize,
  view: WeakEntity<FileViewer>,
) -> Button {
  Button::new(id)
    .ghost()
    .xsmall()
    .w(px(22.0))
    .h(px(22.0))
    .icon(Icon::empty().path(icon))
    .disabled(!enabled)
    .on_click(move |_, _, cx| {
      let _ = view.update(cx, |this, cx| this.pdf_go_to(target, cx));
    })
}

/// GPUI stores texture data as BGRA and core hands over RGBA, so the buffer is swapped in place.
/// Runs on the background executor: a 2048 px page is 16 MB.
pub fn to_render_image(page: PdfPageImage) -> Arc<RenderImage> {
  let mut buffer = page.rgba;
  for pixel in buffer.chunks_exact_mut(4) {
    pixel.swap(0, 2);
  }
  let buffer = image::RgbaImage::from_raw(page.width, page.height, buffer).expect("core sizes the buffer");
  Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn opening_a_document_asks_for_the_first_page_once() {
    let mut pane = PdfPane::new();
    let request = pane.sync("docs/manual.pdf", "h1").expect("first open requests a page");
    assert_eq!(request.page, 0);
    assert!(
      pane.sync("docs/manual.pdf", "h1").is_none(),
      "no rerender while unchanged"
    );
    assert!(pane.sync("docs/manual.pdf", "h2").is_some(), "an edited file reloads");
  }

  fn prepared(page: usize, page_count: usize) -> PreparedPage {
    let image = to_render_image(PdfPageImage {
      page,
      page_count,
      width: 2,
      height: 1,
      rgba: vec![0u8; 8],
    });
    (page, page_count, image)
  }

  #[test]
  fn page_navigation_respects_the_document_bounds() {
    let mut pane = PdfPane::new();
    let first = pane.sync("docs/manual.pdf", "h1").expect("request");
    pane.apply_prepared(first.generation, Ok(prepared(0, 3)));

    assert!(pane.go_to(0).is_none(), "the current page is not re-requested");
    assert!(pane.go_to(3).is_none(), "past the last page is refused");
    let next = pane.go_to(1).expect("page two is requested");
    assert_eq!(next.page, 1);

    // A late answer from the previous request must not overwrite the newer page.
    pane.apply_prepared(first.generation, Ok(prepared(0, 3)));
    assert!(matches!(pane.load, PdfLoad::Pending));

    pane.apply_prepared(next.generation, Ok(prepared(1, 3)));
    assert!(matches!(pane.load, PdfLoad::Ready(_)));
  }

  #[test]
  fn a_reset_pane_refuses_the_previous_documents_page() {
    let mut pane = PdfPane::new();
    let stale = pane.sync("docs/first.pdf", "h1").expect("request");
    pane.reset();

    let fresh = pane.sync("docs/second.pdf", "h2").expect("request");
    assert_ne!(
      stale.generation, fresh.generation,
      "a reset must not hand the next document the generation the previous one is still using"
    );
    pane.apply_prepared(stale.generation, Ok(prepared(0, 9)));
    assert!(matches!(pane.load, PdfLoad::Pending), "the stale page must be dropped");
    assert_eq!(pane.page_count, 0);

    pane.apply_prepared(fresh.generation, Ok(prepared(0, 2)));
    assert!(matches!(pane.load, PdfLoad::Ready(_)));
    assert_eq!(pane.page_count, 2);
  }
}
