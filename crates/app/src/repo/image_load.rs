//! Turns raw image bytes into something GPUI can actually paint. Its own loader uploads whatever
//! it decodes straight to the Metal atlas, which silently refuses a texture over 16384 px and
//! paints a blank box instead. Decoding here keeps every frame inside the atlas, bounds what an
//! animation may cost, and makes a truncated file a visible failure.

use std::io::Cursor;
use std::sync::Arc;

use gpui_kit::*;
use image::imageops::FilterType;
use image::{AnimationDecoder, Frame, ImageDecoder, RgbaImage};

/// Longest edge we hand to the atlas. The viewer only ever paints the image scaled to fit its
/// pane, so more pixels than this buy nothing and cost VRAM.
pub const MAX_IMAGE_EDGE: u32 = 4096;

/// Decode ceiling for one still image. The `image` crate's default 512 MiB rejects big
/// screenshots outright; this allows a 24000 x 16000 photo and still refuses a decompression bomb.
const DECODE_ALLOC_LIMIT: u64 = 1536 * 1024 * 1024;

/// Pixels an animation may keep resident once downscaled, about 256 MiB of BGRA. Beyond this the
/// remaining frames are dropped: a still first frame beats eating every gigabyte the file asks for.
const MAX_ANIMATION_PIXELS: u64 = 64 * 1024 * 1024;

/// A decoded image ready to become an `ImageSource`. Kept separate from `ImageSource` itself so
/// the work can happen on a background thread: `ImageSource` is neither `Send` nor `Sync`.
pub enum PreparedImage {
  /// Vector bytes. GPUI rasterizes an SVG at paint size, which is already atlas safe.
  Vector(Arc<Image>),
  /// Decoded, atlas-sized frames. One frame for a still image, all of them for an animation.
  Raster(Arc<RenderImage>),
}

impl PreparedImage {
  pub fn source(self) -> ImageSource {
    match self {
      PreparedImage::Vector(image) => ImageSource::Image(image),
      PreparedImage::Raster(image) => ImageSource::Render(image),
    }
  }
}

/// Prepare the bytes of `path` for `img(..)`. `None` means the file cannot be displayed.
pub fn prepare_path(path: &str, bytes: &[u8]) -> Option<PreparedImage> {
  prepare(format_for(path, bytes)?, bytes)
}

/// Prepare bytes whose format is already known, as it is for a diff's data URI.
pub fn prepare(format: ImageFormat, bytes: &[u8]) -> Option<PreparedImage> {
  if format == ImageFormat::Svg {
    return Some(PreparedImage::Vector(Arc::new(Image::from_bytes(
      format,
      bytes.to_vec(),
    ))));
  }
  let raster = raster_format(format)?;
  let frames = match decode(raster, bytes) {
    Ok(frames) => frames,
    Err(err) => {
      tracing::warn!(?format, %err, "image could not be decoded");
      return None;
    }
  };
  Some(PreparedImage::Raster(Arc::new(RenderImage::new(frames))))
}

fn decode(format: image::ImageFormat, bytes: &[u8]) -> image::ImageResult<Vec<Frame>> {
  let frames = match format {
    image::ImageFormat::Gif => animated(image::codecs::gif::GifDecoder::new(Cursor::new(bytes))?)?,
    image::ImageFormat::WebP => {
      let mut decoder = image::codecs::webp::WebPDecoder::new(Cursor::new(bytes))?;
      decoder.set_limits(limits())?;
      if decoder.has_animation() {
        animated(decoder)?
      } else {
        vec![Frame::new(still(decoder)?)]
      }
    }
    _ => {
      let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
      reader.limits(limits());
      vec![Frame::new(fit(reader.decode()?.into_rgba8()))]
    }
  };
  if frames.is_empty() {
    return Err(image::ImageError::Decoding(image::error::DecodingError::new(
      format.into(),
      "no frame in this image could be decoded",
    )));
  }
  Ok(frames)
}

fn still<'a, D: image::ImageDecoder + 'a>(decoder: D) -> image::ImageResult<RgbaImage> {
  Ok(fit(image::DynamicImage::from_decoder(decoder)?.into_rgba8()))
}

/// Every frame of an animation, downscaled to fit the atlas and stopped at the memory budget.
/// Frame delays survive so GPUI still plays the animation back.
fn animated<'a, D: AnimationDecoder<'a>>(decoder: D) -> image::ImageResult<Vec<Frame>> {
  let mut frames = Vec::new();
  let mut pixels = 0u64;
  for frame in decoder.into_frames() {
    let frame = match frame {
      Ok(frame) => frame,
      // A partly readable animation still shows what it managed to decode.
      Err(err) if !frames.is_empty() => {
        tracing::debug!(%err, "dropping an undecodable animation frame");
        break;
      }
      Err(err) => return Err(err),
    };
    let (left, top, delay) = (frame.left(), frame.top(), frame.delay());
    let buffer = fit(frame.into_buffer());
    pixels += u64::from(buffer.width()) * u64::from(buffer.height());
    frames.push(Frame::from_parts(buffer, left, top, delay));
    if pixels >= MAX_ANIMATION_PIXELS {
      tracing::debug!(frames = frames.len(), "animation hit its memory budget");
      break;
    }
  }
  Ok(frames)
}

/// Downscale to the atlas limit if needed, then swap to BGRA, which is what `RenderImage` stores.
fn fit(buffer: RgbaImage) -> RgbaImage {
  let mut buffer = downscaled(buffer);
  for pixel in buffer.chunks_exact_mut(4) {
    pixel.swap(0, 2);
  }
  buffer
}

fn downscaled(buffer: RgbaImage) -> RgbaImage {
  let (width, height) = buffer.dimensions();
  let longest = width.max(height);
  if longest <= MAX_IMAGE_EDGE {
    return buffer;
  }
  let scale = f64::from(MAX_IMAGE_EDGE) / f64::from(longest);
  let target = (
    ((f64::from(width) * scale).round() as u32).max(1),
    ((f64::from(height) * scale).round() as u32).max(1),
  );
  tracing::debug!(width, height, to = ?target, "downscaled image to fit the texture atlas");
  image::imageops::resize(&buffer, target.0, target.1, FilterType::Triangle)
}

/// Bytes safe to hand to GPUI's own image loader, which uploads whatever it decodes straight to
/// the atlas. An image already inside the limit is passed through untouched, so an animation keeps
/// animating; anything larger comes back as a downscaled PNG still.
pub fn bounded_bytes(path: &str, bytes: Vec<u8>) -> Vec<u8> {
  let Some(raster) = format_for(path, &bytes).and_then(raster_format) else {
    return bytes;
  };
  let dimensions = image::ImageReader::with_format(Cursor::new(&bytes), raster).into_dimensions();
  if let Ok((width, height)) = dimensions
    && width.max(height) <= MAX_IMAGE_EDGE
  {
    return bytes;
  }
  match reencoded(raster, &bytes) {
    Ok(encoded) => encoded,
    Err(err) => {
      tracing::warn!(path, %err, "oversized markdown image could not be downscaled");
      bytes
    }
  }
}

fn reencoded(raster: image::ImageFormat, bytes: &[u8]) -> image::ImageResult<Vec<u8>> {
  let mut reader = image::ImageReader::with_format(Cursor::new(bytes), raster);
  reader.limits(limits());
  let buffer = downscaled(reader.decode()?.into_rgba8());
  let mut encoded = Vec::new();
  image::DynamicImage::ImageRgba8(buffer).write_to(&mut Cursor::new(&mut encoded), image::ImageFormat::Png)?;
  Ok(encoded)
}

fn limits() -> image::Limits {
  let mut limits = image::Limits::no_limits();
  limits.max_alloc = Some(DECODE_ALLOC_LIMIT);
  limits
}

/// GPUI's format enum for `path`, from the extension when it is meaningful and from the bytes
/// when it is not.
fn format_for(path: &str, bytes: &[u8]) -> Option<ImageFormat> {
  let extension = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
  let mapped = match extension.as_str() {
    "png" => Some(ImageFormat::Png),
    "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
    "webp" => Some(ImageFormat::Webp),
    "gif" => Some(ImageFormat::Gif),
    "bmp" => Some(ImageFormat::Bmp),
    "ico" => Some(ImageFormat::Ico),
    "tif" | "tiff" => Some(ImageFormat::Tiff),
    "svg" | "svgz" => Some(ImageFormat::Svg),
    _ => None,
  };
  if mapped.is_some() {
    return mapped;
  }
  match image::guess_format(bytes).ok()? {
    image::ImageFormat::Png => Some(ImageFormat::Png),
    image::ImageFormat::Jpeg => Some(ImageFormat::Jpeg),
    image::ImageFormat::WebP => Some(ImageFormat::Webp),
    image::ImageFormat::Gif => Some(ImageFormat::Gif),
    image::ImageFormat::Bmp => Some(ImageFormat::Bmp),
    image::ImageFormat::Ico => Some(ImageFormat::Ico),
    image::ImageFormat::Tiff => Some(ImageFormat::Tiff),
    other => {
      tracing::warn!(path, ?other, "no decoder for this image format");
      None
    }
  }
}

fn raster_format(format: ImageFormat) -> Option<image::ImageFormat> {
  match format {
    ImageFormat::Png => Some(image::ImageFormat::Png),
    ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
    ImageFormat::Webp => Some(image::ImageFormat::WebP),
    ImageFormat::Gif => Some(image::ImageFormat::Gif),
    ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
    ImageFormat::Ico => Some(image::ImageFormat::Ico),
    ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
    // Neither `format_for` nor the diff ever produces these, and the decoder is not built in.
    ImageFormat::Pnm | ImageFormat::Svg => None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  fn png(width: u32, height: u32) -> Vec<u8> {
    let buffer = RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(buffer)
      .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
      .expect("encode png");
    bytes
  }

  /// A two frame animation, each frame wider than the atlas allows.
  fn wide_gif() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
      let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
      encoder
        .set_repeat(image::codecs::gif::Repeat::Infinite)
        .expect("repeat");
      for shade in [40u8, 200u8] {
        let buffer = RgbaImage::from_pixel(9000, 60, image::Rgba([shade, shade, shade, 255]));
        encoder
          .encode_frame(Frame::from_parts(
            buffer,
            0,
            0,
            image::Delay::from_numer_denom_ms(80, 1),
          ))
          .expect("encode frame");
      }
    }
    bytes
  }

  fn raster(prepared: PreparedImage) -> Arc<RenderImage> {
    match prepared {
      PreparedImage::Raster(image) => image,
      PreparedImage::Vector(_) => panic!("a raster format must not come back as a vector"),
    }
  }

  #[test]
  fn oversized_images_are_downscaled_to_fit_the_atlas() {
    let image = raster(prepare_path("wide.png", &png(9000, 90)).expect("prepared"));
    let size = image.size(0);
    assert_eq!(u32::from(size.width), MAX_IMAGE_EDGE);
    // 9000x90 scaled to a 4096 long edge keeps its aspect ratio.
    assert_eq!(u32::from(size.height), 41);
  }

  #[test]
  fn small_images_keep_their_own_size() {
    let image = raster(prepare_path("small.png", &png(32, 48)).expect("prepared"));
    assert_eq!(u32::from(image.size(0).width), 32);
    assert_eq!(u32::from(image.size(0).height), 48);
    assert_eq!(image.frame_count(), 1);
  }

  #[test]
  fn an_animation_keeps_every_frame_inside_the_atlas() {
    let image = raster(prepare_path("spin.gif", &wide_gif()).expect("prepared"));
    assert_eq!(image.frame_count(), 2, "animation must survive the atlas guard");
    for frame in 0..image.frame_count() {
      let size = image.size(frame);
      assert!(
        u32::from(size.width) <= MAX_IMAGE_EDGE && u32::from(size.height) <= MAX_IMAGE_EDGE,
        "frame {frame} is {size:?}, past the atlas limit"
      );
      assert_eq!(
        image.delay(frame).numer_denom_ms(),
        (80, 1),
        "frame delays must survive"
      );
    }
  }

  #[test]
  fn bytes_served_to_gpuis_own_loader_stay_inside_the_atlas() {
    let wide = png(9000, 90);
    let served = bounded_bytes("wide.png", wide.clone());
    assert_ne!(served, wide, "an oversized image must be re-encoded smaller");
    let dimensions = image::ImageReader::with_format(Cursor::new(&served), image::ImageFormat::Png)
      .into_dimensions()
      .expect("the served bytes must still be a readable png");
    assert_eq!(dimensions, (MAX_IMAGE_EDGE, 41));

    // Inside the limit nothing is touched, so an animation keeps animating in a preview.
    let animation = wide_gif();
    let small = png(32, 32);
    assert_eq!(bounded_bytes("small.png", small.clone()), small);
    assert_ne!(bounded_bytes("spin.gif", animation.clone()), animation);
  }

  #[test]
  fn truncated_and_undecodable_bytes_are_reported_as_unusable() {
    assert!(prepare_path("broken.png", b"not really a png").is_none());
    assert!(prepare_path("photo.avif", b"\x00\x01\x02\x03").is_none());
    // A valid header with the pixel data cut off: this is what used to paint an empty box.
    let mut truncated = png(64, 64);
    truncated.truncate(40);
    assert!(prepare_path("cut.png", &truncated).is_none());
  }

  #[test]
  fn extensionless_files_are_sniffed() {
    assert!(prepare_path("screenshot", &png(16, 16)).is_some());
  }

  #[test]
  fn svg_stays_a_vector() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"></svg>"#;
    assert!(matches!(prepare_path("mark.svg", svg), Some(PreparedImage::Vector(_))));
  }
}
