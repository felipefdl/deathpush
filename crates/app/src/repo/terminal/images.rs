//! Kitty graphics images in a terminal pane: textures cached across frames and the geometry that
//! puts a placement on screen.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use deathpush_core::terminal::snapshot::{ImageLayer, ImagePixels, SnapshotImage};
use gpui_kit::*;
use image::imageops::FilterType;

use crate::repo::image_load::MAX_IMAGE_EDGE;

/// GPU textures of the images a pane shows, keyed by image id and generation.
///
/// libghostty gives every change of an image's pixels, an animation frame included, a new
/// generation, so a texture is built once per key and reused until the key leaves the snapshot.
#[derive(Default)]
pub struct ImageTextures {
  entries: HashMap<(u32, u64), Arc<RenderImage>>,
  #[cfg(test)]
  uploads: usize,
}

impl ImageTextures {
  /// The texture for `pixels`, built on first use.
  pub fn texture(&mut self, pixels: &ImagePixels) -> Option<Arc<RenderImage>> {
    let key = (pixels.id, pixels.generation);
    if let Some(texture) = self.entries.get(&key) {
      return Some(Arc::clone(texture));
    }
    let texture = build_texture(pixels)?;
    #[cfg(test)]
    {
      self.uploads += 1;
    }
    self.entries.insert(key, Arc::clone(&texture));
    Some(texture)
  }

  /// Forget every texture no placement in `images` uses and return them, so the caller can free
  /// their atlas tiles.
  pub fn retain_used(&mut self, images: &[SnapshotImage]) -> Vec<Arc<RenderImage>> {
    let used: HashSet<(u32, u64)> = images
      .iter()
      .map(|image| (image.pixels.id, image.pixels.generation))
      .collect();
    let mut evicted = Vec::new();
    self.entries.retain(|key, texture| {
      let keep = used.contains(key);
      if !keep {
        evicted.push(Arc::clone(texture));
      }
      keep
    });
    evicted
  }

  #[cfg(test)]
  pub(crate) fn len(&self) -> usize {
    self.entries.len()
  }

  #[cfg(test)]
  pub(crate) fn uploads(&self) -> usize {
    self.uploads
  }
}

/// RGBA pixels as the BGRA texture GPUI stores, downscaled when the atlas could not hold them.
fn build_texture(pixels: &ImagePixels) -> Option<Arc<RenderImage>> {
  let mut buffer = image::RgbaImage::from_raw(pixels.width, pixels.height, pixels.rgba.clone())?;
  let longest = pixels.width.max(pixels.height);
  if longest > MAX_IMAGE_EDGE {
    let scale = f64::from(MAX_IMAGE_EDGE) / f64::from(longest);
    let width = ((f64::from(pixels.width) * scale).round() as u32).max(1);
    let height = ((f64::from(pixels.height) * scale).round() as u32).max(1);
    buffer = image::imageops::resize(&buffer, width, height, FilterType::Triangle);
  }
  for pixel in buffer.as_chunks_mut::<4>().0 {
    pixel.swap(0, 2);
  }
  Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// One placement ready to paint.
pub struct ImageDraw {
  pub texture: Arc<RenderImage>,
  pub layer: ImageLayer,
  /// Where the visible part of the image goes.
  pub dest: Bounds<Pixels>,
  /// Where the whole image would go if nothing were cropped. Painting it through `dest` crops it to
  /// the placement's source rectangle.
  pub full: Bounds<Pixels>,
}

/// The textures for every placement of `images`, in the draw order the snapshot lists them in,
/// plus the textures that no placement uses any more.
pub fn prepare_images(
  images: &[SnapshotImage],
  cell_px: (u32, u32),
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  textures: &mut ImageTextures,
) -> (Vec<ImageDraw>, Vec<Arc<RenderImage>>) {
  let evicted = textures.retain_used(images);
  let draws = images
    .iter()
    .filter_map(|image| {
      let (dest, full) = placement_bounds(image, cell_px, origin, cell)?;
      Some(ImageDraw {
        texture: textures.texture(&image.pixels)?,
        layer: image.layer,
        dest,
        full,
      })
    })
    .collect();
  (draws, evicted)
}

/// Paint the placements of one layer, clipped to `clip`.
pub fn paint_layer(draws: &[ImageDraw], layer: ImageLayer, clip: Bounds<Pixels>, window: &mut Window) {
  for draw in draws.iter().filter(|draw| draw.layer == layer) {
    let visible = draw.dest.intersect(&clip);
    if visible.size.width <= Pixels::ZERO || visible.size.height <= Pixels::ZERO {
      continue;
    }
    if let Err(err) = window.paint_image(
      visible,
      draw.full,
      Corners::default(),
      Arc::clone(&draw.texture),
      0,
      false,
    ) {
      tracing::debug!(%err, "terminal image paint failed");
    }
  }
}

/// Where a placement lands, as (visible rectangle, whole-image rectangle). `None` for a placement
/// with no area.
///
/// libghostty measures a placement in terminal pixels: `cell_px` per cell. The pane draws cells at
/// `cell`, which is the same size before rounding, so every length scales by the ratio.
pub fn placement_bounds(
  image: &SnapshotImage,
  cell_px: (u32, u32),
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
  if image.width == 0 || image.height == 0 || image.source_width == 0 || image.source_height == 0 {
    return None;
  }
  let scale_x = cell.0.as_f32() / cell_px.0.max(1) as f32;
  let scale_y = cell.1.as_f32() / cell_px.1.max(1) as f32;
  let left = origin.x.as_f32() + image.col as f32 * cell.0.as_f32() + image.offset_x as f32 * scale_x;
  let top = origin.y.as_f32() + image.row as f32 * cell.1.as_f32() + image.offset_y as f32 * scale_y;
  let dest_width = image.width as f32 * scale_x;
  let dest_height = image.height as f32 * scale_y;
  // The source rectangle fills `dest`, so one source pixel is this many screen pixels.
  let per_source_x = dest_width / image.source_width as f32;
  let per_source_y = dest_height / image.source_height as f32;
  let full_left = left - image.source_x as f32 * per_source_x;
  let full_top = top - image.source_y as f32 * per_source_y;
  let dest = Bounds::new(point(px(left), px(top)), size(px(dest_width), px(dest_height)));
  let full = Bounds::new(
    point(px(full_left), px(full_top)),
    size(
      px(image.pixels.width as f32 * per_source_x),
      px(image.pixels.height as f32 * per_source_y),
    ),
  );
  Some((dest, full))
}

#[cfg(test)]
mod tests {
  use deathpush_core::terminal::snapshot::{ImageLayer, ImagePixels, SnapshotImage};
  use gpui_kit::{point, px, size};
  use std::sync::Arc;

  use super::{ImageTextures, placement_bounds};

  fn pixels(id: u32, generation: u64) -> Arc<ImagePixels> {
    Arc::new(ImagePixels {
      id,
      generation,
      width: 2,
      height: 1,
      rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
    })
  }

  fn placement(pixels: Arc<ImagePixels>) -> SnapshotImage {
    SnapshotImage {
      pixels,
      layer: ImageLayer::AboveText,
      z: 0,
      col: 0,
      row: 0,
      grid_cols: 1,
      grid_rows: 1,
      offset_x: 0,
      offset_y: 0,
      width: 2,
      height: 1,
      source_x: 0,
      source_y: 0,
      source_width: 2,
      source_height: 1,
    }
  }

  #[test]
  fn an_unchanged_generation_keeps_its_texture() {
    let mut textures = ImageTextures::default();
    let image = pixels(1, 10);
    let first = textures.texture(&image).unwrap();
    let again = textures.texture(&image).unwrap();
    assert!(Arc::ptr_eq(&first, &again));
    assert_eq!(textures.uploads(), 1, "the same generation uploaded twice");
  }

  #[test]
  fn a_new_generation_replaces_the_texture() {
    let mut textures = ImageTextures::default();
    let old = textures.texture(&pixels(1, 10)).unwrap();
    let fresh = textures.texture(&pixels(1, 11)).unwrap();
    assert!(!Arc::ptr_eq(&old, &fresh));
    assert_eq!(textures.uploads(), 2);
    let evicted = textures.retain_used(&[placement(pixels(1, 11))]);
    assert_eq!(evicted.len(), 1, "the replaced generation stayed cached");
    assert!(Arc::ptr_eq(&evicted[0], &old));
    assert_eq!(textures.len(), 1);
  }

  #[test]
  fn images_that_left_the_list_are_evicted() {
    let mut textures = ImageTextures::default();
    textures.texture(&pixels(1, 1)).unwrap();
    textures.texture(&pixels(2, 1)).unwrap();
    assert!(
      textures
        .retain_used(&[placement(pixels(1, 1)), placement(pixels(2, 1))])
        .is_empty()
    );
    assert_eq!(textures.retain_used(&[placement(pixels(2, 1))]).len(), 1);
    assert_eq!(textures.retain_used(&[]).len(), 1);
    assert_eq!(textures.len(), 0);
  }

  #[test]
  fn textures_are_bgra() {
    let texture = ImageTextures::default().texture(&pixels(1, 1)).unwrap();
    let bytes = texture.as_bytes(0).unwrap();
    assert_eq!(bytes, [0, 0, 255, 255, 0, 255, 0, 255]);
  }

  #[test]
  fn placements_scale_with_the_cell_size_and_crop_to_the_source() {
    let mut image = placement(pixels(1, 1));
    image.col = 3;
    image.row = -1;
    image.offset_x = 4;
    image.width = 16;
    image.height = 32;
    // Half the image, the right pixel column, drawn into 16 x 32 terminal pixels.
    image.source_x = 1;
    image.source_width = 1;
    image.source_height = 1;
    // Terminal pixels are 8 x 16 per cell; the pane draws 10 x 20 cells.
    let (dest, full) = placement_bounds(&image, (8, 16), point(px(100.0), px(50.0)), (px(10.0), px(20.0))).unwrap();
    assert_eq!(dest.origin, point(px(100.0 + 30.0 + 5.0), px(50.0 - 20.0)));
    assert_eq!(dest.size, size(px(20.0), px(40.0)));
    // The two source pixels span 40 pixels, and the first one hangs off to the left.
    assert_eq!(full.origin.x, px(100.0 + 30.0 + 5.0 - 20.0));
    assert_eq!(full.size, size(px(40.0), px(40.0)));
  }

  #[test]
  fn a_placement_without_area_is_skipped() {
    let mut image = placement(pixels(1, 1));
    image.width = 0;
    assert!(placement_bounds(&image, (8, 16), point(px(0.0), px(0.0)), (px(8.0), px(16.0))).is_none());
  }
}
