use deathpush_core::config::settings::{CursorInactiveStyle, CursorStyle, MONO_FONT_STACK};
use deathpush_core::terminal::pane::{
  KeyInput, KeyMods, MouseAction, MouseButton as TermMouse, MouseInput, PaneCommand,
};
use deathpush_core::terminal::snapshot::{PaneSnapshot, Rgb, SnapshotCell};
use deathpush_core::theme::Rgba;
use gpui_kit::*;

use super::pane_view::PaneView;
use crate::config::AppConfig;
use crate::theme::{ActivePalette, hsla};

/// Paint settings resolved from app config and the active palette.
///
/// Font family, size, weights, line height, letter spacing, and saturation are cache-sensitive:
/// changing any of them invalidates shaped glyph runs.
#[derive(Clone)]
pub struct TerminalPaint {
  pub family: SharedString,
  pub font_size: f32,
  pub line_height: f32,
  pub letter_spacing: f32,
  pub weight: FontWeight,
  pub weight_bold: FontWeight,
  pub cursor_style: CursorStyle,
  pub cursor_inactive_style: CursorInactiveStyle,
  pub cursor_blink: bool,
  pub cursor_width: f32,
  pub saturation: f32,
  pub selection: Rgba,
  pub cursor: Rgba,
  pub background: Rgba,
}

/// Custom element that paints a [`PaneView`] snapshot and routes input.
pub struct TerminalElement {
  pub view: Entity<PaneView>,
  pub settings: TerminalPaint,
}

/// Resolve [`TerminalPaint`] from the current app config and [`ActivePalette`].
pub fn paint_from_app(cx: &App) -> TerminalPaint {
  let settings = &AppConfig::get(cx).settings.terminal;
  let palette = cx.global::<ActivePalette>().0;
  let family = if settings.font_family.is_empty() {
    MONO_FONT_STACK
  } else {
    settings.font_family.as_str()
  };
  TerminalPaint {
    family: family.into(),
    font_size: settings.font_size as f32,
    line_height: settings.line_height,
    letter_spacing: settings.letter_spacing,
    weight: parse_weight(&settings.font_weight),
    weight_bold: parse_weight(&settings.font_weight_bold),
    cursor_style: settings.cursor_style,
    cursor_inactive_style: settings.cursor_inactive_style,
    cursor_blink: settings.cursor_blink,
    cursor_width: settings.cursor_width as f32,
    saturation: settings.color_saturation,
    selection: palette.selection,
    cursor: palette.terminal_cursor,
    background: palette.terminal_background,
  }
}

/// Scale an RGB color's HSL saturation by `factor`, clamped to `0..=1`.
pub fn saturate(color: Rgba, factor: f32) -> Rgba {
  let (h, s, l) = rgb_to_hsl(color.r, color.g, color.b);
  let s = (s * factor).clamp(0.0, 1.0);
  let (r, g, b) = hsl_to_rgb(h, s, l);
  Rgba { r, g, b, a: color.a }
}

/// Cell size in pixels: width is the `M` advance plus `letter_spacing`, height is `size * line_height`.
pub fn cell_size(window: &Window, family: &str, size: f32, line_height: f32, letter_spacing: f32) -> (Pixels, Pixels) {
  let font_size = px(size);
  let run = TextRun {
    len: 1,
    font: Font {
      family: family.into(),
      features: FontFeatures::default(),
      fallbacks: None,
      weight: FontWeight::NORMAL,
      style: FontStyle::Normal,
    },
    color: white(),
    background_color: None,
    underline: None,
    strikethrough: None,
  };
  let line = window.text_system().shape_line("M".into(), font_size, &[run], None);
  let width = line.width();
  let cell_w = if width > Pixels::ZERO {
    width + px(letter_spacing)
  } else {
    px(size * 0.6 + letter_spacing)
  };
  (cell_w.max(px(1.0)), px((size * line_height).max(1.0)))
}

/// Convert a window point to a 0-based cell, clamped to `[0, cols)` × `[0, rows)`.
pub fn cell_at(
  point: Point<Pixels>,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
) -> (u16, u16) {
  let (cell_w, cell_h) = cell;
  if cell_w <= Pixels::ZERO || cell_h <= Pixels::ZERO {
    return (0, 0);
  }
  let x = ((point.x - origin.x) / cell_w).floor();
  let y = ((point.y - origin.y) / cell_h).floor();
  (
    x.clamp(0.0, f32::from(cols.saturating_sub(1))) as u16,
    y.clamp(0.0, f32::from(rows.saturating_sub(1))) as u16,
  )
}

/// Layout metrics from [`TerminalElement`] prepaint: hitbox, origin, cell size, and grid.
pub struct PaintState {
  hitbox: Hitbox,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
}

/// Cached shaped runs for one snapshot sequence and paint inputs.
///
/// Reused while the snapshot sequence and cache-sensitive paint settings are unchanged, so cursor
/// blink and selection changes do not reshape. Nonzero letter-spacing caches one shaped line per
/// cell under the same key.
#[derive(Default)]
pub struct PaintCache {
  key: Option<PaintCacheKey>,
  rows: Vec<CachedRow>,
  #[cfg(test)]
  shape_calls: usize,
}

#[cfg(test)]
impl PaintCache {
  pub(crate) fn shape_calls(&self) -> usize {
    self.shape_calls
  }
}

#[derive(Clone, PartialEq)]
struct PaintCacheKey {
  seq: u64,
  family: SharedString,
  font_size: f32,
  letter_spacing: f32,
  saturation: f32,
  weight: FontWeight,
  weight_bold: FontWeight,
  cell_w: f32,
  cell_h: f32,
  cols: u16,
  rows: u16,
}

struct CachedRow {
  runs: Vec<CachedRun>,
}

struct CachedRun {
  start_x: u16,
  cells: Vec<String>,
  line: Option<ShapedLine>,
  cell_lines: Vec<Option<ShapedLine>>,
}

impl IntoElement for TerminalElement {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl Element for TerminalElement {
  type RequestLayoutState = ();
  type PrepaintState = PaintState;

  fn id(&self) -> Option<ElementId> {
    None
  }

  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }

  fn request_layout(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, Self::RequestLayoutState) {
    let mut style = Style::default();
    style.size.width = relative(1.).into();
    style.size.height = relative(1.).into();
    (window.request_layout(style, [], cx), ())
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    window: &mut Window,
    cx: &mut App,
  ) -> Self::PrepaintState {
    let settings = &self.settings;
    let (mut cell_w, mut cell_h) = cell_size(
      window,
      &settings.family,
      settings.font_size,
      settings.line_height,
      settings.letter_spacing,
    );
    cell_w = cell_w.max(px(1.0));
    cell_h = cell_h.max(px(1.0));
    let cell_w_px = cell_w.as_f32().round().max(1.0) as u32;
    let cell_h_px = cell_h.as_f32().round().max(1.0) as u32;
    let schedule = self.view.update(cx, |this, _cx| {
      this.set_cell((cell_w, cell_h));
      let Some((cols, rows)) = grid_from_bounds(bounds.size.width, bounds.size.height, cell_w, cell_h) else {
        return false;
      };
      this.needs_resize(cols, rows, cell_w_px, cell_h_px) && this.queue_resize(cols, rows, cell_w_px, cell_h_px)
    });
    if schedule {
      let view = self.view.clone();
      window.defer(cx, move |_window, cx| {
        view.update(cx, |this, cx| this.flush_resize(cx));
      });
    }
    let (cols, rows) = grid_from_bounds(bounds.size.width, bounds.size.height, cell_w, cell_h).unwrap_or((1, 1));
    PaintState {
      hitbox: window.insert_hitbox(bounds, HitboxBehavior::Normal),
      origin: bounds.origin,
      cell: (cell_w, cell_h),
      cols,
      rows,
    }
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector_id: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _request_layout: &mut Self::RequestLayoutState,
    prepaint: &mut Self::PrepaintState,
    window: &mut Window,
    cx: &mut App,
  ) {
    let settings = self.settings.clone();
    let sat = settings.saturation;
    let snapshot = self.view.read(cx).snapshot();
    let bg = snapshot
      .as_ref()
      .map(|snap| paint_rgb(snap.background, sat))
      .unwrap_or_else(|| hsla(saturate(settings.background, sat)));
    window.paint_quad(fill(bounds, bg));
    if let Some(snap) = snapshot.as_deref() {
      paint_cells(snap, &settings, prepaint.origin, prepaint.cell, window, cx);
      paint_selection(
        self.view.read(cx).selection_range(),
        snap,
        settings.selection,
        prepaint.origin,
        prepaint.cell,
        window,
      );
      self.view.update(cx, |this, cx| {
        paint_rows(
          snap,
          &settings,
          prepaint.origin,
          prepaint.cell,
          this.paint_cache(),
          window,
          cx,
        );
      });
      paint_cursor(
        snap,
        &settings,
        self.view.read(cx).active(),
        self.view.read(cx).blink_on(),
        self.view.read(cx).focus_handle().is_focused(window),
        prepaint.origin,
        prepaint.cell,
        window,
        cx,
      );
      if let Some(marked) = self.view.read(cx).marked_text().map(str::to_string) {
        paint_marked(&marked, snap, &settings, prepaint.origin, prepaint.cell, window, cx);
      }
    }
    if self.view.read(cx).flashing() {
      let color = cx.global::<ActivePalette>().0.foreground.with_alpha(38);
      window.paint_quad(fill(bounds, hsla(color)));
    }

    let view = self.view.clone();
    let hitbox = prepaint.hitbox.clone();
    let origin = prepaint.origin;
    let cell = prepaint.cell;
    let cols = prepaint.cols;
    let rows = prepaint.rows;
    let focused = view.read(cx).focus_handle().is_focused(window);
    if focused {
      window.handle_input(
        view.read(cx).focus_handle(),
        ElementInputHandler::new(bounds, view.clone()),
        cx,
      );
    }
    window.on_mouse_event({
      let view = view.clone();
      let hitbox = hitbox.clone();
      move |event: &MouseDownEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble && hitbox.is_hovered(window) {
          on_mouse_down(&view, event, origin, cell, cols, rows, window, cx);
        }
      }
    });
    window.on_mouse_event({
      let view = view.clone();
      let hitbox = hitbox.clone();
      move |event: &MouseMoveEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble && (hitbox.is_hovered(window) || view.read(cx).mouse_captured()) {
          on_mouse_move(&view, event, origin, cell, cols, rows, cx);
        }
      }
    });
    window.on_mouse_event({
      let view = view.clone();
      let hitbox = hitbox.clone();
      move |event: &MouseUpEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble && (hitbox.is_hovered(window) || view.read(cx).mouse_captured()) {
          on_mouse_up(&view, event, origin, cell, cols, rows, cx);
        }
      }
    });
    window.on_mouse_event({
      let view = view.clone();
      move |event: &ScrollWheelEvent, phase, window, cx| {
        if phase == DispatchPhase::Bubble && hitbox.should_handle_scroll(window) {
          on_scroll(&view, event, origin, cell, cols, rows, cx);
        }
      }
    });
  }
}

/// Grid size for a pane bound. `None` when the bound has no area.
pub fn grid_from_bounds(width: Pixels, height: Pixels, cell_w: Pixels, cell_h: Pixels) -> Option<(u16, u16)> {
  if width <= Pixels::ZERO || height <= Pixels::ZERO {
    return None;
  }
  Some((
    ((width / cell_w.max(px(1.0))).floor() as u16).max(1),
    ((height / cell_h.max(px(1.0))).floor() as u16).max(1),
  ))
}

/// Clamp one selection anchor to the last valid column and row of `cols`×`rows`.
pub fn clamp_sel_anchor(cell: (u16, u16), cols: u16, rows: u16) -> (u16, u16) {
  (cell.0.min(cols.saturating_sub(1)), cell.1.min(rows.saturating_sub(1)))
}

/// Clamp both selection endpoints when the grid shrinks.
pub fn clamp_selection(
  selection: Option<((u16, u16), (u16, u16))>,
  cols: u16,
  rows: u16,
) -> Option<((u16, u16), (u16, u16))> {
  selection.map(|(start, end)| (clamp_sel_anchor(start, cols, rows), clamp_sel_anchor(end, cols, rows)))
}

fn paint_cells(
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  window: &mut Window,
  _cx: &mut App,
) {
  let sat = settings.saturation;
  let default_bg = snap.background;
  for y in 0..snap.rows {
    for x in 0..snap.cols {
      let Some(cell_data) = snap.cell(x, y) else {
        continue;
      };
      let (_, bg) = effective_colors(cell_data, snap);
      if bg == default_bg && !cell_data.inverse && cell_data.bg.is_none() {
        continue;
      }
      let pos = point(origin.x + cell.0 * usize::from(x), origin.y + cell.1 * usize::from(y));
      window.paint_quad(fill(Bounds::new(pos, size(cell.0, cell.1)), paint_rgb(bg, sat)));
    }
  }
}

fn paint_selection(
  selection: Option<((u16, u16), (u16, u16))>,
  snap: &PaneSnapshot,
  color: Rgba,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  window: &mut Window,
) {
  let Some((start, end)) = selection else {
    return;
  };
  let (start, end) = order_cells(start, end);
  let fill_color = hsla(color.with_alpha(128));
  for y in start.1..=end.1 {
    if y >= snap.rows {
      break;
    }
    let x0 = if y == start.1 { start.0 } else { 0 };
    let x1 = if y == end.1 {
      end.0.min(snap.cols.saturating_sub(1))
    } else {
      snap.cols.saturating_sub(1)
    };
    if x0 > x1 {
      continue;
    }
    let pos = point(origin.x + cell.0 * usize::from(x0), origin.y + cell.1 * usize::from(y));
    let width = cell.0 * usize::from(x1.saturating_sub(x0) + 1);
    window.paint_quad(fill(Bounds::new(pos, size(width, cell.1)), fill_color));
  }
}

struct RowRun {
  start_x: u16,
  cells: Vec<String>,
  style: RunStyle,
}

#[allow(clippy::too_many_arguments)]
fn paint_rows(
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cache: &mut PaintCache,
  window: &mut Window,
  cx: &mut App,
) {
  rebuild_cache(cache, snap, settings, cell, window);
  let spaced = settings.letter_spacing != 0.0;
  for (y, row) in cache.rows.iter().enumerate() {
    let y_pos = origin.y + cell.1 * y;
    for run in &row.runs {
      paint_cached_run(run, origin.x, y_pos, cell.0, cell.1, spaced, window, cx);
    }
  }
}

fn cache_key(snap: &PaneSnapshot, settings: &TerminalPaint, cell: (Pixels, Pixels)) -> PaintCacheKey {
  PaintCacheKey {
    seq: snap.seq,
    family: settings.family.clone(),
    font_size: settings.font_size,
    letter_spacing: settings.letter_spacing,
    saturation: settings.saturation,
    weight: settings.weight,
    weight_bold: settings.weight_bold,
    cell_w: cell.0.as_f32(),
    cell_h: cell.1.as_f32(),
    cols: snap.cols,
    rows: snap.rows,
  }
}

fn rebuild_cache(
  cache: &mut PaintCache,
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  cell: (Pixels, Pixels),
  window: &Window,
) {
  let key = cache_key(snap, settings, cell);
  if cache.key.as_ref() == Some(&key) {
    return;
  }
  let spaced = settings.letter_spacing != 0.0;
  let font_size = px(settings.font_size);
  let mut rows = Vec::with_capacity(usize::from(snap.rows));
  for y in 0..snap.rows {
    let mut runs = Vec::new();
    for run in row_runs(snap, y, settings) {
      let empty = run.cells.iter().all(String::is_empty);
      let (line, cell_lines) = if empty {
        (None, Vec::new())
      } else if spaced {
        let cell_lines = run
          .cells
          .iter()
          .map(|text| {
            if text.is_empty() {
              None
            } else {
              Some(shape_cached(
                cache,
                window,
                text.clone().into(),
                font_size,
                run.style,
                settings,
              ))
            }
          })
          .collect();
        (None, cell_lines)
      } else {
        let text = run.cells.concat();
        if text.is_empty() {
          (None, Vec::new())
        } else {
          (
            Some(shape_cached(cache, window, text.into(), font_size, run.style, settings)),
            Vec::new(),
          )
        }
      };
      runs.push(CachedRun {
        start_x: run.start_x,
        cells: run.cells,
        line,
        cell_lines,
      });
    }
    rows.push(CachedRow { runs });
  }
  cache.key = Some(key);
  cache.rows = rows;
}

fn shape_cached(
  cache: &mut PaintCache,
  window: &Window,
  text: SharedString,
  font_size: Pixels,
  style: RunStyle,
  settings: &TerminalPaint,
) -> ShapedLine {
  #[cfg(test)]
  {
    cache.shape_calls += 1;
  }
  #[cfg(not(test))]
  {
    let _ = cache;
  }
  let shaped = text_run(text.len(), style, settings);
  window.text_system().shape_line(text, font_size, &[shaped], None)
}

#[allow(clippy::too_many_arguments)]
fn paint_cached_run(
  run: &CachedRun,
  origin_x: Pixels,
  y_pos: Pixels,
  cell_w: Pixels,
  cell_h: Pixels,
  spaced: bool,
  window: &mut Window,
  cx: &mut App,
) {
  if run.cells.iter().all(|text| text.is_empty()) {
    return;
  }
  if spaced {
    for (index, line) in run.cell_lines.iter().enumerate() {
      let Some(line) = line else {
        continue;
      };
      let pos = point(origin_x + cell_w * (usize::from(run.start_x) + index), y_pos);
      let _ = line.paint(pos, cell_h, TextAlign::Left, None, window, cx);
    }
    return;
  }
  let Some(line) = run.line.as_ref() else {
    return;
  };
  let pos = point(origin_x + cell_w * usize::from(run.start_x), y_pos);
  let _ = line.paint(pos, cell_h, TextAlign::Left, None, window, cx);
}

fn row_runs(snap: &PaneSnapshot, y: u16, settings: &TerminalPaint) -> Vec<RowRun> {
  let mut runs = Vec::new();
  let mut current: Option<RowRun> = None;
  for x in 0..snap.cols {
    let cell = snap.cell(x, y).cloned().unwrap_or_default();
    let style = run_style(&cell, snap, settings);
    match current {
      Some(ref mut run) if run.style == style => run.cells.push(cell.text),
      Some(run) => {
        runs.push(run);
        current = Some(RowRun {
          start_x: x,
          cells: vec![cell.text],
          style,
        });
      }
      None => {
        current = Some(RowRun {
          start_x: x,
          cells: vec![cell.text],
          style,
        });
      }
    }
  }
  if let Some(run) = current {
    runs.push(run);
  }
  runs
}

fn paint_marked(
  text: &str,
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  window: &mut Window,
  cx: &mut App,
) {
  if text.is_empty() {
    return;
  }
  let Some(cursor) = snap.cursor.as_ref() else {
    return;
  };
  let style = RunStyle {
    fg: paint_rgb(snap.foreground, settings.saturation),
    bg: paint_rgb(snap.background, settings.saturation),
    bold: false,
    italic: false,
    underline: true,
    strikethrough: false,
  };
  let run = text_run(text.len(), style, settings);
  let line = window
    .text_system()
    .shape_line(text.to_string().into(), px(settings.font_size), &[run], None);
  let pos = point(
    origin.x + cell.0 * usize::from(cursor.x),
    origin.y + cell.1 * usize::from(cursor.y),
  );
  let _ = line.paint(pos, cell.1, TextAlign::Left, None, window, cx);
}

#[allow(clippy::too_many_arguments)]
fn paint_cursor(
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  active: bool,
  blink_on: bool,
  focused: bool,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  window: &mut Window,
  cx: &mut App,
) {
  let Some(cursor) = snap.cursor.as_ref() else {
    return;
  };
  if !cursor.visible {
    return;
  }
  if active && settings.cursor_blink && focused && !blink_on {
    return;
  }
  let pos = point(
    origin.x + cell.0 * usize::from(cursor.x),
    origin.y + cell.1 * usize::from(cursor.y),
  );
  let bounds = Bounds::new(pos, size(cell.0, cell.1));
  let color = snap
    .cursor_color
    .map(|rgb| paint_rgb(rgb, settings.saturation))
    .unwrap_or_else(|| hsla(saturate(settings.cursor, settings.saturation)));
  if active {
    match settings.cursor_style {
      CursorStyle::Block => paint_block_cursor(snap, settings, cursor.x, cursor.y, bounds, color, window, cx),
      CursorStyle::Underline => paint_underline_cursor(bounds, color, window),
      CursorStyle::Bar => paint_bar_cursor(bounds, px(settings.cursor_width.max(1.0)), color, window),
    }
  } else {
    match settings.cursor_inactive_style {
      CursorInactiveStyle::None => {}
      CursorInactiveStyle::Block => paint_block_cursor(snap, settings, cursor.x, cursor.y, bounds, color, window, cx),
      CursorInactiveStyle::Underline => paint_underline_cursor(bounds, color, window),
      CursorInactiveStyle::Bar => paint_bar_cursor(bounds, px(settings.cursor_width.max(1.0)), color, window),
      CursorInactiveStyle::Outline => {
        window.paint_quad(outline(bounds, color, BorderStyle::default()));
      }
    }
  }
}

#[allow(clippy::too_many_arguments)]
fn paint_block_cursor(
  snap: &PaneSnapshot,
  settings: &TerminalPaint,
  x: u16,
  y: u16,
  bounds: Bounds<Pixels>,
  color: Hsla,
  window: &mut Window,
  cx: &mut App,
) {
  window.paint_quad(fill(bounds, color));
  let Some(cell) = snap.cell(x, y) else {
    return;
  };
  let bg = paint_rgb(snap.background, settings.saturation);
  let run = TextRun {
    len: cell.text.len(),
    font: cell_font(settings, cell.bold, cell.italic),
    color: bg,
    background_color: None,
    underline: None,
    strikethrough: None,
  };
  let line = window
    .text_system()
    .shape_line(cell.text.clone().into(), px(settings.font_size), &[run], None);
  let _ = line.paint(bounds.origin, bounds.size.height, TextAlign::Left, None, window, cx);
}

fn paint_underline_cursor(bounds: Bounds<Pixels>, color: Hsla, window: &mut Window) {
  let bar = Bounds::new(
    point(bounds.origin.x, bounds.origin.y + bounds.size.height - px(2.0)),
    size(bounds.size.width, px(2.0)),
  );
  window.paint_quad(fill(bar, color));
}

fn paint_bar_cursor(bounds: Bounds<Pixels>, width: Pixels, color: Hsla, window: &mut Window) {
  window.paint_quad(fill(
    Bounds::new(bounds.origin, size(width.max(px(1.0)), bounds.size.height)),
    color,
  ));
}

#[derive(Clone, Copy, PartialEq)]
struct RunStyle {
  fg: Hsla,
  bg: Hsla,
  bold: bool,
  italic: bool,
  underline: bool,
  strikethrough: bool,
}

fn run_style(cell: &SnapshotCell, snap: &PaneSnapshot, settings: &TerminalPaint) -> RunStyle {
  let (fg, bg) = effective_colors(cell, snap);
  let mut fg = paint_rgb(fg, settings.saturation);
  if cell.faint {
    fg.a *= 0.5;
  }
  RunStyle {
    fg,
    bg: paint_rgb(bg, settings.saturation),
    bold: cell.bold,
    italic: cell.italic,
    underline: cell.underline,
    strikethrough: cell.strikethrough,
  }
}

fn text_run(len: usize, style: RunStyle, settings: &TerminalPaint) -> TextRun {
  TextRun {
    len,
    font: cell_font(settings, style.bold, style.italic),
    color: style.fg,
    background_color: None,
    underline: style.underline.then_some(UnderlineStyle {
      thickness: px(1.0),
      color: Some(style.fg),
      wavy: false,
    }),
    strikethrough: style.strikethrough.then_some(StrikethroughStyle {
      thickness: px(1.0),
      color: Some(style.fg),
    }),
  }
}

fn cell_font(settings: &TerminalPaint, bold: bool, italic: bool) -> Font {
  Font {
    family: settings.family.clone(),
    features: FontFeatures::default(),
    fallbacks: None,
    weight: if bold { settings.weight_bold } else { settings.weight },
    style: if italic { FontStyle::Italic } else { FontStyle::Normal },
  }
}

fn effective_colors(cell: &SnapshotCell, snap: &PaneSnapshot) -> (Rgb, Rgb) {
  let mut fg = cell.fg.unwrap_or(snap.foreground);
  let mut bg = cell.bg.unwrap_or(snap.background);
  if cell.inverse {
    std::mem::swap(&mut fg, &mut bg);
  }
  (fg, bg)
}

fn paint_rgb(rgb: Rgb, sat: f32) -> Hsla {
  hsla(saturate(Rgba::rgb(rgb.0, rgb.1, rgb.2), sat))
}

fn order_cells(start: (u16, u16), end: (u16, u16)) -> ((u16, u16), (u16, u16)) {
  if reading_pos(start) <= reading_pos(end) {
    (start, end)
  } else {
    (end, start)
  }
}

fn reading_pos(cell: (u16, u16)) -> u32 {
  (u32::from(cell.1) << 16) | u32::from(cell.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyRoute {
  Ignore,
  Paste,
  CopyOrSend,
  Send,
}

fn classify_key(keystroke: &Keystroke) -> KeyRoute {
  let mods = &keystroke.modifiers;
  let key = keystroke.key.as_str();
  if cfg!(target_os = "macos") {
    if !mods.platform {
      return KeyRoute::Send;
    }
    if key == "v" && !mods.shift && !mods.control && !mods.alt && !mods.function {
      return KeyRoute::Paste;
    }
    if key == "c" && !mods.shift && !mods.control && !mods.alt && !mods.function {
      return KeyRoute::CopyOrSend;
    }
    KeyRoute::Ignore
  } else if mods.platform {
    KeyRoute::Ignore
  } else if mods.control && !mods.shift && !mods.alt && !mods.function && key == "v" {
    KeyRoute::Paste
  } else if mods.control && !mods.shift && !mods.alt && !mods.function && key == "c" {
    KeyRoute::CopyOrSend
  } else {
    KeyRoute::Send
  }
}

/// Printable unmodified keys go through the IME input handler; named keys and chords stay on the key path.
pub fn key_uses_ime(keystroke: &Keystroke) -> bool {
  let mods = &keystroke.modifiers;
  if mods.control || mods.alt || mods.platform || mods.function {
    return false;
  }
  if keystroke.key_char.as_deref().is_none_or(str::is_empty) {
    return false;
  }
  !matches!(
    keystroke.key.as_str(),
    "enter"
      | "tab"
      | "escape"
      | "backspace"
      | "delete"
      | "up"
      | "down"
      | "left"
      | "right"
      | "home"
      | "end"
      | "pageup"
      | "pagedown"
      | "insert"
      | "f1"
      | "f2"
      | "f3"
      | "f4"
      | "f5"
      | "f6"
      | "f7"
      | "f8"
      | "f9"
      | "f10"
      | "f11"
      | "f12"
      | "f13"
      | "f14"
      | "f15"
      | "f16"
      | "f17"
      | "f18"
      | "f19"
      | "f20"
      | "f21"
      | "f22"
      | "f23"
      | "f24"
  )
}

pub(crate) fn on_key_down(view: &Entity<PaneView>, event: &KeyDownEvent, cx: &mut App) {
  match classify_key(&event.keystroke) {
    KeyRoute::Ignore => {}
    KeyRoute::Paste => {
      if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
        view.update(cx, |this, _| {
          this.send(PaneCommand::Paste(text));
        });
      }
      cx.stop_propagation();
    }
    KeyRoute::CopyOrSend => {
      let copied = view.update(cx, |this, cx| this.copy_selection(cx).is_some());
      view.update(cx, |this, _| {
        if copied {
          this.note_copy_consumed(event.keystroke.key.clone());
        } else {
          this.note_sent_key(event.keystroke.key.clone(), mods_from(&event.keystroke.modifiers));
        }
      });
      if !copied {
        send_key(view, &event.keystroke, true, cx);
      }
      cx.stop_propagation();
    }
    KeyRoute::Send => {
      if key_uses_ime(&event.keystroke) {
        return;
      }
      view.update(cx, |this, _| {
        this.note_sent_key(event.keystroke.key.clone(), mods_from(&event.keystroke.modifiers));
      });
      send_key(view, &event.keystroke, true, cx);
      cx.stop_propagation();
    }
  }
}

pub(crate) fn on_key_up(view: &Entity<PaneView>, event: &KeyUpEvent, cx: &mut App) {
  let key = event.keystroke.key.as_str();
  let mods = view.update(cx, |this, _| {
    if this.take_copy_consumed(key) {
      None
    } else {
      this.take_sent_key(key)
    }
  });
  if let Some(mods) = mods {
    view.update(cx, |this, _| {
      this.send(PaneCommand::Key(KeyInput {
        key: event.keystroke.key.clone(),
        text: None,
        mods,
        press: false,
      }));
    });
  }
}

fn send_key(view: &Entity<PaneView>, keystroke: &Keystroke, press: bool, cx: &mut App) {
  let input = KeyInput {
    key: keystroke.key.clone(),
    text: if press { keystroke.key_char.clone() } else { None },
    mods: mods_from(&keystroke.modifiers),
    press,
  };
  view.update(cx, |this, _| {
    this.send(PaneCommand::Key(input));
  });
}

fn tracking_button(button: MouseButton) -> Option<TermMouse> {
  match button {
    MouseButton::Left => Some(TermMouse::Left),
    MouseButton::Right => Some(TermMouse::Right),
    MouseButton::Middle => Some(TermMouse::Middle),
    _ => None,
  }
}

#[allow(clippy::too_many_arguments)]
fn on_mouse_down(
  view: &Entity<PaneView>,
  event: &MouseDownEvent,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
  window: &mut Window,
  cx: &mut App,
) {
  let (x, y) = cell_at(event.position, origin, cell, cols, rows);
  view.update(cx, |this, cx| {
    this.focus_handle().focus(window, cx);
    let tracking = this.mouse_tracking();
    let force_sel = AppConfig::get(cx).settings.terminal.mac_option_click_forces_selection && event.modifiers.alt;
    match event.button {
      MouseButton::Left => {
        if tracking && !force_sel {
          this.set_forwarded_button(Some(TermMouse::Left));
          this.send(PaneCommand::Mouse(MouseInput {
            action: MouseAction::Press,
            button: Some(TermMouse::Left),
            x,
            y,
            mods: mods_from(&event.modifiers),
          }));
        } else {
          this.begin_selection((x, y), cx);
        }
      }
      MouseButton::Right => {
        if tracking && !force_sel {
          this.set_forwarded_button(Some(TermMouse::Right));
          this.send(PaneCommand::Mouse(MouseInput {
            action: MouseAction::Press,
            button: Some(TermMouse::Right),
            x,
            y,
            mods: mods_from(&event.modifiers),
          }));
        } else if AppConfig::get(cx).settings.terminal.right_click_selects_word {
          this.select_word_at(x, y, cx);
        }
      }
      MouseButton::Middle if tracking => {
        this.set_forwarded_button(Some(TermMouse::Middle));
        this.send(PaneCommand::Mouse(MouseInput {
          action: MouseAction::Press,
          button: Some(TermMouse::Middle),
          x,
          y,
          mods: mods_from(&event.modifiers),
        }));
      }
      _ => {}
    }
  });
  cx.stop_propagation();
}

fn on_mouse_move(
  view: &Entity<PaneView>,
  event: &MouseMoveEvent,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
  cx: &mut App,
) {
  let (x, y) = cell_at(event.position, origin, cell, cols, rows);
  view.update(cx, |this, cx| {
    if this.dragging() {
      this.extend_selection((x, y), cx);
    } else if let Some(button) = this.forwarded_button() {
      this.send(PaneCommand::Mouse(MouseInput {
        action: MouseAction::Motion,
        button: Some(button),
        x,
        y,
        mods: mods_from(&event.modifiers),
      }));
    } else if this.mouse_tracking() {
      this.send(PaneCommand::Mouse(MouseInput {
        action: MouseAction::Motion,
        button: event.pressed_button.and_then(tracking_button),
        x,
        y,
        mods: mods_from(&event.modifiers),
      }));
    }
  });
}

fn on_mouse_up(
  view: &Entity<PaneView>,
  event: &MouseUpEvent,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
  cx: &mut App,
) {
  let (x, y) = cell_at(event.position, origin, cell, cols, rows);
  view.update(cx, |this, cx| {
    if this.dragging() {
      this.end_selection((x, y), cx);
      if AppConfig::get(cx).settings.terminal.copy_on_select {
        this.copy_selection(cx);
      }
    } else if this.forwarded_button().is_some() {
      this.send(PaneCommand::Mouse(MouseInput {
        action: MouseAction::Release,
        button: tracking_button(event.button),
        x,
        y,
        mods: mods_from(&event.modifiers),
      }));
      this.set_forwarded_button(None);
    }
  });
}

/// Convert a GPUI scroll delta into whole cell lines, accumulating sub-cell remainders.
///
/// Positive GPUI vertical delta is toward the top of the view.
pub fn wheel_lines(delta: ScrollDelta, cell_h: f32, accum: &mut f32) -> isize {
  let add = match delta {
    ScrollDelta::Lines(delta) => delta.y,
    ScrollDelta::Pixels(delta) => delta.y.as_f32() / cell_h.max(1.0),
  };
  *accum += add;
  let lines = accum.trunc() as isize;
  *accum -= lines as f32;
  lines
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WheelCommand {
  Scroll(isize),
  Track { button: TermMouse, count: u32 },
}

fn wheel_command(gpui_lines: isize, tracking: bool) -> Option<WheelCommand> {
  if gpui_lines == 0 {
    return None;
  }
  if tracking {
    Some(WheelCommand::Track {
      button: if gpui_lines > 0 {
        TermMouse::WheelUp
      } else {
        TermMouse::WheelDown
      },
      count: gpui_lines.unsigned_abs() as u32,
    })
  } else {
    Some(WheelCommand::Scroll(-gpui_lines))
  }
}

fn on_scroll(
  view: &Entity<PaneView>,
  event: &ScrollWheelEvent,
  origin: Point<Pixels>,
  cell: (Pixels, Pixels),
  cols: u16,
  rows: u16,
  cx: &mut App,
) {
  let gpui_lines = view.update(cx, |this, _| {
    wheel_lines(event.delta, cell.1.as_f32(), this.wheel_accum())
  });
  let Some(command) = wheel_command(gpui_lines, view.read(cx).mouse_tracking()) else {
    return;
  };
  let (x, y) = cell_at(event.position, origin, cell, cols, rows);
  view.update(cx, |this, _| match command {
    WheelCommand::Scroll(delta) => this.send(PaneCommand::Scroll(delta)),
    WheelCommand::Track { button, count } => {
      for _ in 0..count {
        this.send(PaneCommand::Mouse(MouseInput {
          action: MouseAction::Press,
          button: Some(button),
          x,
          y,
          mods: mods_from(&event.modifiers),
        }));
      }
    }
  });
  cx.stop_propagation();
}

fn mods_from(mods: &Modifiers) -> KeyMods {
  KeyMods {
    shift: mods.shift,
    alt: mods.alt,
    ctrl: mods.control,
    super_: mods.platform,
  }
}

fn parse_weight(value: &str) -> FontWeight {
  match value.trim().to_ascii_lowercase().as_str() {
    "thin" => FontWeight::THIN,
    "extralight" | "extra-light" | "ultralight" => FontWeight::EXTRA_LIGHT,
    "light" => FontWeight::LIGHT,
    "normal" | "regular" => FontWeight::NORMAL,
    "medium" => FontWeight::MEDIUM,
    "semibold" | "semi-bold" | "demibold" => FontWeight::SEMIBOLD,
    "bold" => FontWeight::BOLD,
    "extrabold" | "extra-bold" | "ultrabold" => FontWeight::EXTRA_BOLD,
    "black" | "heavy" => FontWeight::BLACK,
    other => other.parse::<f32>().map(FontWeight).unwrap_or(FontWeight::NORMAL),
  }
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
  let r = f32::from(r) / 255.0;
  let g = f32::from(g) / 255.0;
  let b = f32::from(b) / 255.0;
  let max = r.max(g).max(b);
  let min = r.min(g).min(b);
  let l = (max + min) / 2.0;
  if (max - min).abs() < f32::EPSILON {
    return (0.0, 0.0, l);
  }
  let d = max - min;
  let s = d / (1.0 - (2.0 * l - 1.0).abs());
  let h = if (max - r).abs() < f32::EPSILON {
    (g - b) / d
  } else if (max - g).abs() < f32::EPSILON {
    (b - r) / d + 2.0
  } else {
    (r - g) / d + 4.0
  };
  (h.rem_euclid(6.0) * 60.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let hp = h / 60.0;
  let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
  let (r1, g1, b1) = if (0.0..1.0).contains(&hp) {
    (c, x, 0.0)
  } else if (1.0..2.0).contains(&hp) {
    (x, c, 0.0)
  } else if (2.0..3.0).contains(&hp) {
    (0.0, c, x)
  } else if (3.0..4.0).contains(&hp) {
    (0.0, x, c)
  } else if (4.0..5.0).contains(&hp) {
    (x, 0.0, c)
  } else {
    (c, 0.0, x)
  };
  let m = l - c / 2.0;
  (
    ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  struct CellSizeProbe;

  impl Render for CellSizeProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      div()
    }
  }

  fn keystroke(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
    Keystroke {
      key: key.into(),
      key_char: key_char.map(str::to_string),
      modifiers,
    }
  }

  #[test]
  fn saturate_scales_saturation_and_clamps() {
    let red = Rgba::rgb(255, 0, 0);
    assert_eq!(saturate(red, 1.0), red);
    assert_eq!(saturate(red, 0.5), Rgba::rgb(191, 64, 64));
    assert_eq!(saturate(red, 10.0), red);
    assert_eq!(saturate(red, 0.0), Rgba::rgb(128, 128, 128));
    let gray = Rgba::rgb(128, 128, 128);
    assert_eq!(saturate(gray, 2.0), gray);
  }

  #[test]
  fn cell_at_clamps_to_grid() {
    let origin = point(px(10.0), px(20.0));
    let cell = (px(8.0), px(16.0));
    assert_eq!(cell_at(point(px(10.0), px(20.0)), origin, cell, 80, 24), (0, 0));
    assert_eq!(cell_at(point(px(18.0), px(20.0)), origin, cell, 80, 24), (1, 0));
    assert_eq!(cell_at(point(px(10.0), px(36.0)), origin, cell, 80, 24), (0, 1));
    assert_eq!(cell_at(point(px(9.0), px(19.0)), origin, cell, 80, 24), (0, 0));
    assert_eq!(
      cell_at(point(px(10_000.0), px(10_000.0)), origin, cell, 80, 24),
      (79, 23)
    );
    assert_eq!(cell_at(point(px(10.0), px(20.0)), origin, cell, 0, 0), (0, 0));
  }

  #[test]
  fn resize_skips_zero_size_bounds() {
    assert_eq!(grid_from_bounds(px(0.0), px(100.0), px(8.0), px(16.0)), None);
    assert_eq!(grid_from_bounds(px(80.0), px(0.0), px(8.0), px(16.0)), None);
    assert_eq!(grid_from_bounds(px(80.0), px(48.0), px(8.0), px(16.0)), Some((10, 3)));
  }

  #[test]
  fn wheel_converts_both_directions_and_tracking_modes() {
    assert_eq!(wheel_command(3, false), Some(WheelCommand::Scroll(-3)));
    assert_eq!(wheel_command(-2, false), Some(WheelCommand::Scroll(2)));
    assert_eq!(
      wheel_command(3, true),
      Some(WheelCommand::Track {
        button: TermMouse::WheelUp,
        count: 3
      })
    );
    assert_eq!(
      wheel_command(-2, true),
      Some(WheelCommand::Track {
        button: TermMouse::WheelDown,
        count: 2
      })
    );
    assert_eq!(wheel_command(0, false), None);

    let mut accum = 0.0;
    assert_eq!(
      wheel_lines(ScrollDelta::Pixels(point(px(0.0), px(8.0))), 16.0, &mut accum),
      0
    );
    assert_eq!(accum, 0.5);
    assert_eq!(
      wheel_lines(ScrollDelta::Pixels(point(px(0.0), px(8.0))), 16.0, &mut accum),
      1
    );
    assert_eq!(accum, 0.0);

    let mut accum = 0.0;
    assert_eq!(
      wheel_lines(ScrollDelta::Pixels(point(px(0.0), px(-40.0))), 16.0, &mut accum),
      -2
    );
    assert!((accum + 0.5).abs() < f32::EPSILON);

    let mut accum = 0.0;
    assert_eq!(wheel_lines(ScrollDelta::Lines(point(0.0, 2.7)), 16.0, &mut accum), 2);
    assert!((accum - 0.7).abs() < 1e-5);
    assert_eq!(wheel_lines(ScrollDelta::Lines(point(0.0, -0.2)), 16.0, &mut accum), 0);
  }

  #[test]
  fn printable_keys_use_ime_named_and_chords_use_key_path() {
    assert!(key_uses_ime(&keystroke("a", Some("a"), Modifiers::default())));
    assert!(key_uses_ime(&keystroke(
      "a",
      Some("A"),
      Modifiers {
        shift: true,
        ..Modifiers::default()
      }
    )));
    assert!(key_uses_ime(&keystroke("space", Some(" "), Modifiers::default())));
    assert!(!key_uses_ime(&keystroke("enter", Some("\n"), Modifiers::default())));
    assert!(!key_uses_ime(&keystroke("tab", Some("\t"), Modifiers::default())));
    assert!(!key_uses_ime(&keystroke("up", None, Modifiers::default())));
    assert!(!key_uses_ime(&keystroke(
      "c",
      None,
      Modifiers {
        control: true,
        ..Modifiers::default()
      }
    )));
    assert!(!key_uses_ime(&keystroke(
      "v",
      None,
      Modifiers {
        platform: true,
        ..Modifiers::default()
      }
    )));
    assert_eq!(
      classify_key(&keystroke("enter", None, Modifiers::default())),
      KeyRoute::Send
    );
  }

  #[gpui_kit::test]
  fn cell_size_measures_font_line_height_and_letter_spacing(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let window = cx.add_window(|_, _cx| CellSizeProbe);
    window
      .update(cx, |_, window, _| {
        let family = MONO_FONT_STACK;
        let (w0, h0) = cell_size(window, family, 13.0, 1.2, 0.0);
        let (w1, h1) = cell_size(window, family, 13.0, 1.2, 3.0);
        assert_eq!(h0, px(13.0 * 1.2));
        assert_eq!(h1, h0);
        assert_eq!(w1, w0 + px(3.0));
        let (w2, h2) = cell_size(window, family, 26.0, 2.0, 0.0);
        assert_eq!(h2, px(52.0));
        assert!(w2 > w0);
      })
      .unwrap();
  }
}
