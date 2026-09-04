use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Arc;

use deathpush_core::config::settings::{DiffIndicators, DiffLayout, LineDiffType, MONO_FONT_STACK};
use deathpush_core::diff_view::{DiffRow, DiffRows, RowKind};
use deathpush_core::theme::UiPalette;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;

use super::highlight::{Highlighted, Side};
use super::selection::{Anchor, Selection};
use crate::theme::hsla;

pub type Layouts = Rc<RefCell<HashMap<(usize, Side), TextLayout>>>;
pub type SelectFn = Rc<dyn Fn(Anchor, &mut Window, &mut App)>;
pub type HunkFn = Rc<dyn Fn(HunkOp, String, &mut Window, &mut App)>;

#[derive(Clone, Copy)]
pub enum HunkOp {
  Stage,
  Unstage,
  Discard,
}

#[derive(Clone)]
pub struct RowInteract {
  pub selection: Option<Selection>,
  pub layouts: Layouts,
  pub hunk_ids: Rc<Vec<String>>,
  pub staged: bool,
  pub merge: bool,
  pub show_hunk_actions: bool,
  pub on_mouse_down: SelectFn,
  pub on_hunk: HunkFn,
}

#[derive(Clone, Copy, Default)]
pub struct RowsMetrics {
  pub max_columns: usize,
  pub max_line_number: usize,
}

impl RowsMetrics {
  pub fn from_rows(rows: &DiffRows) -> Self {
    Self {
      max_columns: rows.max_columns(),
      max_line_number: max_line_number(rows),
    }
  }
}

pub struct RowPaint {
  pub palette: UiPalette,
  pub show_line_numbers: bool,
  pub show_background: bool,
  pub indicators: DiffIndicators,
  pub line_diff: LineDiffType,
  pub line_height: f32,
  pub font_family: SharedString,
  pub font_size: f32,
  pub number_width: f32,
  pub indicator_width: f32,
  pub highlighter: Option<Arc<Highlighted>>,
  pub theme: Arc<HighlightTheme>,
}

pub fn content_width(metrics: &RowsMetrics, paint: &RowPaint, layout: DiffLayout, advance: f32) -> f32 {
  let numbers = if paint.show_line_numbers {
    paint.number_width
  } else {
    0.0
  };
  let indicator = paint.indicator_width;
  let text = metrics.max_columns as f32 * advance;
  match layout {
    DiffLayout::Inline => numbers * 2.0 + indicator + text + 24.0,
    DiffLayout::SideBySide => (numbers + indicator + text) * 2.0 + 1.0 + 24.0,
  }
}

pub fn number_width(max_line_number: usize, advance: f32) -> f32 {
  let digits = max_line_number.max(1).to_string().len().max(1);
  digits as f32 * advance + 12.0
}

pub fn indicator_width(indicators: DiffIndicators, advance: f32) -> f32 {
  match indicators {
    DiffIndicators::None => 0.0,
    DiffIndicators::Classic => advance + 8.0,
    DiffIndicators::Bars => 11.0,
  }
}

pub fn editor_font_family(family: &str) -> SharedString {
  if family.is_empty() {
    MONO_FONT_STACK.into()
  } else {
    family.to_string().into()
  }
}

pub fn measure_advance(window: &mut Window, family: &str, font_size: f32) -> f32 {
  let font_id = window.text_system().resolve_font(&font(family));
  f32::from(window.text_system().layout_width(font_id, px(font_size), 'M'))
}

fn max_line_number(rows: &DiffRows) -> usize {
  match rows {
    DiffRows::Inline(rows) => rows
      .iter()
      .flat_map(|row| [row.old_line, row.new_line])
      .flatten()
      .max()
      .unwrap_or(1),
    DiffRows::SideBySide(rows) => rows
      .iter()
      .flat_map(|row| {
        [
          row.left.old_line,
          row.left.new_line,
          row.right.old_line,
          row.right.new_line,
        ]
      })
      .flatten()
      .max()
      .unwrap_or(1),
  }
}

enum Gutter {
  Dual,
  SingleOld,
  SingleNew,
}

pub fn render_row(rows: &DiffRows, index: usize, paint: &RowPaint, interact: &RowInteract) -> AnyElement {
  match rows {
    DiffRows::Inline(rows) => rows
      .get(index)
      .map(|row| render_inline_row(row, paint, index, interact))
      .unwrap_or_else(|| div().into_any_element()),
    DiffRows::SideBySide(rows) => rows
      .get(index)
      .map(|row| render_side_row(&row.left, &row.right, paint, index, interact))
      .unwrap_or_else(|| div().into_any_element()),
  }
}

fn render_inline_row(row: &DiffRow, paint: &RowPaint, index: usize, interact: &RowInteract) -> AnyElement {
  if row.kind == RowKind::Separator {
    return render_separator(row, paint, interact).into_any_element();
  }
  let side = match row.kind {
    RowKind::Remove => Side::Old,
    _ => Side::New,
  };
  render_cell(row, side, Gutter::Dual, paint, index, interact).into_any_element()
}

fn render_side_row(
  left: &DiffRow,
  right: &DiffRow,
  paint: &RowPaint,
  index: usize,
  interact: &RowInteract,
) -> AnyElement {
  if left.kind == RowKind::Separator {
    return render_separator(left, paint, interact).into_any_element();
  }
  div()
    .flex()
    .h(px(paint.line_height))
    .child(
      render_cell(left, Side::Old, Gutter::SingleOld, paint, index, interact)
        .flex_1()
        .min_w_0(),
    )
    .child(div().w(px(1.0)).h_full().bg(hsla(paint.palette.border)))
    .child(
      render_cell(right, Side::New, Gutter::SingleNew, paint, index, interact)
        .flex_1()
        .min_w_0(),
    )
    .into_any_element()
}

fn render_separator(row: &DiffRow, paint: &RowPaint, interact: &RowInteract) -> Div {
  let mut row_el = div()
    .h(px(paint.line_height))
    .flex()
    .items_center()
    .px_2()
    .border_t_1()
    .border_color(hsla(paint.palette.border))
    .text_size(px(11.0))
    .text_color(hsla(paint.palette.muted_foreground));
  if paint.show_background {
    row_el = row_el.bg(hsla(paint.palette.sidebar));
  }
  row_el
    .child(div().flex_1().min_w_0().child(row.text.clone()))
    .child(hunk_action_slot(row, interact))
}

fn hunk_action_slot(row: &DiffRow, interact: &RowInteract) -> Div {
  let slot = div().flex().items_center().gap_1();
  if !interact.show_hunk_actions || interact.merge {
    return slot;
  }
  let Some(hunk_id) = interact.hunk_ids.get(row.hunk).cloned() else {
    return slot;
  };
  let slot = slot.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
  if interact.staged {
    slot.child(hunk_button(
      SharedString::from(format!("hunk-unstage-{hunk_id}")),
      "icons/remove.svg",
      "Unstage Hunk",
      HunkOp::Unstage,
      hunk_id,
      interact.on_hunk.clone(),
    ))
  } else {
    slot
      .child(hunk_button(
        SharedString::from(format!("hunk-stage-{hunk_id}")),
        "icons/add.svg",
        "Stage Hunk",
        HunkOp::Stage,
        hunk_id.clone(),
        interact.on_hunk.clone(),
      ))
      .child(hunk_button(
        SharedString::from(format!("hunk-discard-{hunk_id}")),
        "icons/clear-all.svg",
        "Discard Hunk",
        HunkOp::Discard,
        hunk_id,
        interact.on_hunk.clone(),
      ))
  }
}

fn hunk_button(
  id: SharedString,
  icon: &'static str,
  tooltip: &'static str,
  op: HunkOp,
  hunk_id: String,
  on_hunk: HunkFn,
) -> impl IntoElement {
  Button::new(id)
    .ghost()
    .xsmall()
    .icon(Icon::empty().path(icon))
    .tooltip(tooltip)
    .on_click(move |_, window, cx| {
      cx.stop_propagation();
      (on_hunk)(op, hunk_id.clone(), window, cx);
    })
}

fn render_cell(
  row: &DiffRow,
  side: Side,
  gutter: Gutter,
  paint: &RowPaint,
  index: usize,
  interact: &RowInteract,
) -> Div {
  let mut cell = div()
    .flex()
    .h(px(paint.line_height))
    .items_center()
    .font_family(paint.font_family.clone())
    .text_size(px(paint.font_size));
  if let Some(color) = row_background(row.kind, paint) {
    cell = cell.bg(hsla(color));
  }
  if paint.show_line_numbers {
    match gutter {
      Gutter::Dual => {
        cell = cell
          .child(line_number(row.old_line, paint))
          .child(line_number(row.new_line, paint));
      }
      Gutter::SingleOld => {
        cell = cell.child(line_number(row.old_line, paint));
      }
      Gutter::SingleNew => {
        cell = cell.child(line_number(row.new_line, paint));
      }
    }
  }
  if paint.indicators != DiffIndicators::None {
    cell = cell.child(indicator(row.kind, paint));
  }
  let styled = StyledText::new(row.text.clone()).with_highlights(text_runs(row, side, paint));
  let next_layout = styled.layout().clone();
  let prev_layout = {
    let mut map = interact.layouts.borrow_mut();
    let prev = map.get(&(index, side)).cloned();
    map.insert((index, side), next_layout);
    prev
  };
  let quad = selection_quad(row, side, index, paint, interact, prev_layout.as_ref());
  let selectable = row.kind != RowKind::Empty;
  let mut text = div()
    .id(SharedString::from(format!("diff-text-{index}-{}", side as u8)))
    .relative()
    .size_full()
    .whitespace_nowrap();
  if selectable {
    let on_down = interact.on_mouse_down.clone();
    let layouts = interact.layouts.clone();
    let text_len = row.text.len();
    text = text.on_mouse_down(MouseButton::Left, move |event, window, cx| {
      let byte = layouts
        .borrow()
        .get(&(index, side))
        .and_then(|layout| byte_at(layout, event.position, text_len))
        .unwrap_or(0);
      (on_down)(Anchor { row: index, side, byte }, window, cx);
      cx.stop_propagation();
    });
  }
  cell.child(
    div()
      .flex_1()
      .min_w_0()
      .h_full()
      .px_1()
      .child(text.children(quad).child(styled)),
  )
}

fn selection_quad(
  row: &DiffRow,
  side: Side,
  index: usize,
  paint: &RowPaint,
  interact: &RowInteract,
  layout: Option<&TextLayout>,
) -> Option<Div> {
  let sel = interact.selection.as_ref().filter(|sel| !sel.is_empty())?;
  let range = sel.range_in(index, side, row.text.len())?;
  let layout = layout?;
  let (x0, x1) = selection_xs(layout, range)?;
  let width = (x1 - x0).max(1.0);
  Some(
    div()
      .absolute()
      .top_0()
      .left(px(x0))
      .h_full()
      .w(px(width))
      .bg(hsla(paint.palette.selection)),
  )
}

pub(crate) fn ready_bounds(layout: &TextLayout) -> Option<Bounds<Pixels>> {
  catch_unwind(AssertUnwindSafe(|| layout.bounds())).ok()
}

pub(crate) fn byte_at(layout: &TextLayout, pos: Point<Pixels>, text_len: usize) -> Option<usize> {
  catch_unwind(AssertUnwindSafe(|| match layout.index_for_position(pos) {
    Ok(i) | Err(i) => i.min(text_len).min(layout.len()),
  }))
  .ok()
}

fn selection_xs(layout: &TextLayout, range: Range<usize>) -> Option<(f32, f32)> {
  catch_unwind(AssertUnwindSafe(|| {
    let origin = layout.position_for_index(0)?;
    let start = layout.position_for_index(range.start)?;
    let end = layout
      .position_for_index(range.end)
      .or_else(|| layout.position_for_index(layout.len()))?;
    let x0 = f32::from(start.x - origin.x).max(0.0);
    let x1 = f32::from(end.x - origin.x).max(x0);
    Some((x0, x1))
  }))
  .ok()
  .flatten()
}

fn row_background(kind: RowKind, paint: &RowPaint) -> Option<deathpush_core::theme::Rgba> {
  if kind == RowKind::Empty {
    return Some(paint.palette.muted.with_alpha(30));
  }
  if !paint.show_background {
    return None;
  }
  match kind {
    RowKind::Add => Some(paint.palette.diff_inserted_line),
    RowKind::Remove => Some(paint.palette.diff_removed_line),
    RowKind::Separator => Some(paint.palette.sidebar),
    _ => None,
  }
}

fn line_number(number: Option<usize>, paint: &RowPaint) -> Div {
  div()
    .w(px(paint.number_width))
    .flex_shrink_0()
    .flex()
    .justify_end()
    .items_center()
    .pr_1()
    .text_color(hsla(paint.palette.muted_foreground))
    .child(number.map(|n| n.to_string()).unwrap_or_default())
}

fn indicator(kind: RowKind, paint: &RowPaint) -> Div {
  let slot = div().w(px(paint.indicator_width)).flex_shrink_0().h_full();
  match paint.indicators {
    DiffIndicators::None => slot,
    DiffIndicators::Classic => {
      let mark = match kind {
        RowKind::Add => "+",
        RowKind::Remove => "-",
        _ => "",
      };
      let color = match kind {
        RowKind::Add => paint.palette.gutter_added,
        RowKind::Remove => paint.palette.gutter_deleted,
        _ => paint.palette.muted_foreground,
      };
      slot
        .flex()
        .items_center()
        .justify_center()
        .text_color(hsla(color))
        .child(mark.to_string())
    }
    DiffIndicators::Bars => {
      let color = match kind {
        RowKind::Add => Some(paint.palette.gutter_added),
        RowKind::Remove => Some(paint.palette.gutter_deleted),
        _ => None,
      };
      slot.flex().items_center().justify_center().child({
        let mut bar = div().w(px(3.0)).h_full();
        if let Some(color) = color {
          bar = bar.bg(hsla(color));
        }
        bar
      })
    }
  }
}

fn text_runs(row: &DiffRow, side: Side, paint: &RowPaint) -> Vec<(Range<usize>, HighlightStyle)> {
  let highlight_side = match row.kind {
    RowKind::Remove => Side::Old,
    RowKind::Add => Side::New,
    _ => side,
  };
  let line = match highlight_side {
    Side::Old => row.old_line,
    Side::New => row.new_line,
  };
  let syntax = match (paint.highlighter.as_ref(), line) {
    (Some(highlighted), Some(number)) if number > 0 => {
      highlighted.line_styles(highlight_side, number - 1, paint.theme.as_ref())
    }
    _ => Vec::new(),
  };
  let bg = word_background(row.kind, paint);
  merge_runs(row.text.len(), syntax, &row.changed, bg)
}

fn word_background(kind: RowKind, paint: &RowPaint) -> Option<Hsla> {
  let color = match kind {
    RowKind::Add => paint.palette.diff_inserted_text,
    RowKind::Remove => paint.palette.diff_removed_text,
    _ => return None,
  };
  let color = if paint.line_diff == LineDiffType::WordAlt {
    color.with_alpha(160)
  } else {
    color
  };
  Some(hsla(color))
}

pub(crate) fn merge_runs(
  text_len: usize,
  syntax: Vec<(Range<usize>, HighlightStyle)>,
  changed: &[Range<usize>],
  bg: Option<Hsla>,
) -> Vec<(Range<usize>, HighlightStyle)> {
  if text_len == 0 {
    return Vec::new();
  }
  let mut marks = vec![0, text_len];
  for (range, _) in &syntax {
    marks.push(range.start.min(text_len));
    marks.push(range.end.min(text_len));
  }
  if bg.is_some() {
    for range in changed {
      marks.push(range.start.min(text_len));
      marks.push(range.end.min(text_len));
    }
  }
  marks.sort_unstable();
  marks.dedup();
  let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
  for window in marks.windows(2) {
    let range = window[0]..window[1];
    if range.start >= range.end {
      continue;
    }
    let mut style = syntax
      .iter()
      .rev()
      .find(|(span, _)| span.start <= range.start && span.end >= range.end)
      .map(|(_, style)| *style)
      .unwrap_or_default();
    if let Some(bg) = bg
      && changed
        .iter()
        .any(|span| span.start <= range.start && span.end >= range.end)
    {
      style.background_color = Some(bg);
    }
    if let Some((last_range, last_style)) = out.last_mut()
      && *last_style == style
      && last_range.end == range.start
    {
      last_range.end = range.end;
      continue;
    }
    out.push((range, style));
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn merge_runs_covers_the_line_without_overlap() {
    let first = HighlightStyle {
      font_weight: Some(FontWeight::BOLD),
      ..Default::default()
    };
    let second = HighlightStyle {
      font_style: Some(FontStyle::Italic),
      ..Default::default()
    };
    let syntax = vec![(0..6, first), (4..9, second)];
    let changed = vec![2..4, 5..7];
    let runs = merge_runs(10, syntax, &changed, Some(Hsla::default()));
    assert!(!runs.is_empty());
    assert_eq!(runs.first().map(|run| run.0.start), Some(0));
    assert_eq!(runs.last().map(|run| run.0.end), Some(10));
    for window in runs.windows(2) {
      assert!(window[0].0.end <= window[1].0.start, "overlap: {runs:?}");
      assert!(window[0].0.start < window[0].0.end);
    }
    assert!(runs.last().is_some_and(|run| run.0.start < run.0.end));
    let covered: usize = runs.iter().map(|run| run.0.end - run.0.start).sum();
    assert_eq!(covered, 10);
  }
}
