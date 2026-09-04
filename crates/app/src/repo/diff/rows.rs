use std::ops::Range;
use std::sync::Arc;

use deathpush_core::config::settings::{DiffIndicators, DiffLayout, LineDiffType, MONO_FONT_STACK};
use deathpush_core::diff_view::{DiffRow, DiffRows, RowKind};
use deathpush_core::theme::UiPalette;
use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::*;

use super::highlight::{Highlighted, Side};
use crate::theme::hsla;

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

pub fn content_width(rows: &DiffRows, paint: &RowPaint, layout: DiffLayout, advance: f32) -> f32 {
  let numbers = if paint.show_line_numbers {
    paint.number_width
  } else {
    0.0
  };
  let indicator = paint.indicator_width;
  let text = rows.max_columns() as f32 * advance;
  match layout {
    DiffLayout::Inline => numbers * 2.0 + indicator + text + 24.0,
    DiffLayout::SideBySide => (numbers + indicator + text) * 2.0 + 1.0 + 24.0,
  }
}

pub fn number_width(rows: &DiffRows, advance: f32) -> f32 {
  let max_line = max_line_number(rows).max(1);
  let digits = max_line.to_string().len().max(1);
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

pub fn render_row(rows: &DiffRows, index: usize, paint: &RowPaint) -> AnyElement {
  match rows {
    DiffRows::Inline(rows) => rows
      .get(index)
      .map(|row| render_inline_row(row, paint))
      .unwrap_or_else(|| div().into_any_element()),
    DiffRows::SideBySide(rows) => rows
      .get(index)
      .map(|row| render_side_row(&row.left, &row.right, paint))
      .unwrap_or_else(|| div().into_any_element()),
  }
}

fn render_inline_row(row: &DiffRow, paint: &RowPaint) -> AnyElement {
  if row.kind == RowKind::Separator {
    return render_separator(row, paint).into_any_element();
  }
  let side = match row.kind {
    RowKind::Remove => Side::Old,
    _ => Side::New,
  };
  render_cell(row, side, Gutter::Dual, paint).into_any_element()
}

fn render_side_row(left: &DiffRow, right: &DiffRow, paint: &RowPaint) -> AnyElement {
  if left.kind == RowKind::Separator {
    return render_separator(left, paint).into_any_element();
  }
  div()
    .flex()
    .h(px(paint.line_height))
    .child(
      render_cell(left, Side::Old, Gutter::SingleOld, paint)
        .flex_1()
        .min_w_0(),
    )
    .child(div().w(px(1.0)).h_full().bg(hsla(paint.palette.border)))
    .child(
      render_cell(right, Side::New, Gutter::SingleNew, paint)
        .flex_1()
        .min_w_0(),
    )
    .into_any_element()
}

fn render_separator(row: &DiffRow, paint: &RowPaint) -> Div {
  let mut row_el = div()
    .h(px(22.0))
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
    .child(div())
}

fn render_cell(row: &DiffRow, side: Side, gutter: Gutter, paint: &RowPaint) -> Div {
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
  cell.child(
    div()
      .flex_1()
      .min_w_0()
      .px_1()
      .whitespace_nowrap()
      .child(StyledText::new(row.text.clone()).with_highlights(text_runs(row, side, paint))),
  )
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

fn merge_runs(
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
