//! PDF page rasterization. Pure CPU, no GPU and no native dependency: `hayro` renders a page into
//! an RGBA8 buffer that the UI layer uploads as a texture.

use std::path::Path;
use std::time::Instant;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::{DecryptionError, LoadPdfError, Pdf};
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};

use crate::error::{Error, Result};

/// Longest edge we ever rasterize. GPUI's Metal atlas refuses a texture over 16384 px in either
/// direction, and a page that big would cost a gigabyte of VRAM for nothing.
const MAX_RENDER_EDGE: u32 = 8192;
const MIN_RENDER_EDGE: u32 = 256;
/// A business card sized page still deserves a readable raster, but not an absurd one.
const MAX_SCALE: f32 = 6.0;

/// One rasterized page, ready to be uploaded as a texture.
pub struct PdfPageImage {
  /// Zero-based page index.
  pub page: usize,
  pub page_count: usize,
  pub width: u32,
  pub height: u32,
  /// Row-major RGBA8, opaque (the page is composited over white).
  pub rgba: Vec<u8>,
}

impl std::fmt::Debug for PdfPageImage {
  /// Never the pixels: a page is megabytes of them.
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PdfPageImage")
      .field("page", &self.page)
      .field("page_count", &self.page_count)
      .field("width", &self.width)
      .field("height", &self.height)
      .field("bytes", &self.rgba.len())
      .finish()
  }
}

/// Rasterize a single page. `max_edge` is the longest rendered edge in device pixels.
pub(crate) fn render_page(file: &Path, page: usize, max_edge: u32) -> Result<PdfPageImage> {
  let started = Instant::now();
  let data = std::fs::read(file)?;
  let pdf = Pdf::new(data).map_err(load_error)?;
  let pages = pdf.pages();
  let page_count = pages.len();
  let target = pages
    .get(page)
    .ok_or_else(|| Error::Other(format!("This PDF has no page {}", page + 1)))?;

  let budget = max_edge.clamp(MIN_RENDER_EDGE, MAX_RENDER_EDGE) as f32;
  let (base_width, base_height) = target.render_dimensions();
  if !usable(base_width) || !usable(base_height) {
    return Err(Error::Other("This PDF page has no usable dimensions".into()));
  }
  // No lower clamp: a monstrous page must scale all the way down to the budget instead of being
  // pinned at some minimum factor that blows past it.
  let scale = (budget / base_width.max(base_height)).min(MAX_SCALE);
  let (want_width, want_height) = ((base_width * scale).round(), (base_height * scale).round());
  if want_width < 1.0 || want_height < 1.0 {
    return Err(Error::Other("This PDF page is too thin to rasterize".into()));
  }
  if want_width > MAX_RENDER_EDGE as f32 || want_height > MAX_RENDER_EDGE as f32 {
    return Err(Error::Other("This PDF page is too large to rasterize".into()));
  }

  let pixmap_settings = PixmapSettings {
    x_scale: scale,
    y_scale: scale,
    bg_color: WHITE,
  };
  let pixmap = render(
    target,
    &RenderCache::new(),
    &InterpreterSettings::default(),
    &RenderSettings::default(),
    &pixmap_settings,
  );
  let (width, height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
  if width == 0 || height == 0 {
    return Err(Error::Other("This PDF page rendered empty".into()));
  }
  // One copy out of the pixmap. The page is composited over opaque white, so premultiplied and
  // straight alpha agree and the bytes need no per-pixel conversion here.
  let rgba = pixmap.data_as_u8_slice().to_vec();

  tracing::debug!(
    page = page + 1,
    page_count,
    width,
    height,
    scale,
    elapsed_ms = started.elapsed().as_millis(),
    "rasterized pdf page"
  );
  Ok(PdfPageImage {
    page,
    page_count,
    width,
    height,
    rgba,
  })
}

fn usable(edge: f32) -> bool {
  edge.is_finite() && edge > 0.0
}

fn load_error(err: LoadPdfError) -> Error {
  Error::Other(
    match err {
      LoadPdfError::Decryption(DecryptionError::PasswordProtected) => "This PDF is password protected",
      LoadPdfError::Decryption(DecryptionError::UnsupportedAlgorithm) => {
        "This PDF uses an encryption algorithm DeathPush cannot read"
      }
      LoadPdfError::Decryption(_) => "This PDF has broken encryption",
      LoadPdfError::Invalid => "This PDF is malformed",
    }
    .to_string(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A minimal PDF built from `objects`, with real `/Length` values for the content stream.
  fn pdf(objects: &[String]) -> Vec<u8> {
    let mut body = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
      offsets.push(body.len());
      body.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, object));
    }
    let xref_at = body.len();
    body.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in &offsets {
      body.push_str(&format!("{offset:010} 00000 n \n"));
    }
    body.push_str(&format!(
      "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
      objects.len() + 1,
      xref_at
    ));
    body.into_bytes()
  }

  fn content_stream(operators: &str) -> String {
    format!("<< /Length {} >>\nstream\n{operators}endstream", operators.len())
  }

  /// One-page PDF, US Letter, with a black rectangle in the lower-left corner.
  fn fixture() -> Vec<u8> {
    pdf(&[
      "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
      "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>".to_string(),
      content_stream("0 0 0 rg\n72 72 200 200 re\nf\n"),
    ])
  }

  /// A page 40 times wider than any real paper size, which is what used to slip past the scale
  /// clamp and ask for a texture the atlas refuses.
  fn panorama() -> Vec<u8> {
    pdf(&[
      "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
      "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 500000 2000] /Contents 4 0 R /Resources << >> >>".to_string(),
      content_stream("0 0 0 rg\n0 0 1000 1000 re\nf\n"),
    ])
  }

  fn write(name: &str, bytes: Vec<u8>) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    (dir, path)
  }

  #[test]
  fn renders_a_single_page_to_opaque_pixels() {
    let (_dir, path) = write("fixture.pdf", fixture());

    let image = render_page(&path, 0, 1024).expect("render");
    assert_eq!(image.page_count, 1);
    assert_eq!(image.page, 0);
    assert!(image.width > 0 && image.height > 0);
    assert_eq!(image.rgba.len(), image.width as usize * image.height as usize * 4);
    assert!(image.rgba.chunks(4).all(|px| px[3] == 255), "page must be opaque");
    assert!(
      image.rgba.chunks(4).any(|px| px[0] < 32 && px[1] < 32 && px[2] < 32),
      "the drawn rectangle must actually be rasterized"
    );
  }

  #[test]
  fn rejects_a_page_past_the_end() {
    let (_dir, path) = write("fixture.pdf", fixture());

    assert!(render_page(&path, 4, 1024).is_err());
  }

  #[test]
  fn caps_the_rendered_edge() {
    let (_dir, path) = write("fixture.pdf", fixture());

    let image = render_page(&path, 0, 64_000).expect("render");
    assert!(image.width <= MAX_RENDER_EDGE && image.height <= MAX_RENDER_EDGE);
  }

  #[test]
  fn a_giant_page_stays_inside_the_requested_budget() {
    let (_dir, path) = write("panorama.pdf", panorama());

    let image = render_page(&path, 0, 2048).expect("render");
    assert!(
      image.width <= 2048 && image.height <= 2048,
      "a 500000 pt wide page must scale down to the budget, got {}x{}",
      image.width,
      image.height
    );
    assert_eq!(image.rgba.len(), image.width as usize * image.height as usize * 4);
  }

  #[test]
  fn a_malformed_file_says_so() {
    let (_dir, path) = write("broken.pdf", b"%PDF-1.7\nnot really a pdf".to_vec());

    let err = render_page(&path, 0, 1024).expect_err("a malformed pdf must not render");
    assert!(err.to_string().contains("malformed"), "unhelpful message: {err}");
  }
}
