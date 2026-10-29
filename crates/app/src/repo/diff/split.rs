use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use deathpush_core::config::settings::{DiffIndicators, HunkSeparators, LineDiffType, WordWrap};
use gpui_kit::base::{
  ScrollableMask, Scrollbar, ScrollbarHandle, ScrollbarMode, TextSelection, TextSelectionContentKey,
  TextSelectionEvent, TextSelectionHandle, TextSelectionRegistration, TextSelectionRun,
};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::diff::{DiffLinePosition, DiffLineRange, DiffSide, DiffState};
use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::input::{Copy, SelectAll};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::split_document::{Document, Hunk, Presentation, Row};
use crate::config::AppConfig;

actions!(
  bounded_split,
  [
    Up, Down, Left, Right, PageUp, PageDown, First, Last, ExtendUp, ExtendDown
  ]
);

struct Bindings;
impl Global for Bindings {}

fn init(cx: &mut App) {
  if cx.has_global::<Bindings>() {
    return;
  }
  cx.set_global(Bindings);
  cx.bind_keys([
    KeyBinding::new("up", Up, Some("BoundedSplit")),
    KeyBinding::new("down", Down, Some("BoundedSplit")),
    KeyBinding::new("left", Left, Some("BoundedSplit")),
    KeyBinding::new("right", Right, Some("BoundedSplit")),
    KeyBinding::new("pageup", PageUp, Some("BoundedSplit")),
    KeyBinding::new("pagedown", PageDown, Some("BoundedSplit")),
    KeyBinding::new("home", First, Some("BoundedSplit")),
    KeyBinding::new("end", Last, Some("BoundedSplit")),
    KeyBinding::new("shift-up", ExtendUp, Some("BoundedSplit")),
    KeyBinding::new("shift-down", ExtendDown, Some("BoundedSplit")),
    KeyBinding::new(
      if cx.platform_info().is_mac() { "cmd-c" } else { "ctrl-c" },
      Copy,
      Some("BoundedSplit"),
    ),
    KeyBinding::new(
      if cx.platform_info().is_mac() { "cmd-a" } else { "ctrl-a" },
      SelectAll,
      Some("BoundedSplit"),
    ),
  ]);
}

/// Horizontal state belongs to a viewport half, never to the full virtual list.
/// Both the scrollbar and wheel mask use this same clamped source offset.
#[derive(Clone, Default)]
struct SideScroll(Rc<Cell<Horizontal>>);

#[derive(Clone, Copy, Default)]
struct Horizontal {
  viewport: Bounds<Pixels>,
  width: Pixels,
  offset: Pixels,
}

impl SideScroll {
  fn configure(&self, viewport: Bounds<Pixels>, width: Pixels) {
    self.0.set(Horizontal {
      viewport,
      width,
      ..self.0.get()
    });
    self.set_offset(self.offset());
  }

  fn scroll_by(&self, delta: Pixels) {
    self.set_offset(point(self.offset().x + delta, px(0.)));
  }
}

impl ScrollbarHandle for SideScroll {
  fn viewport_bounds(&self) -> Bounds<Pixels> {
    self.0.get().viewport
  }

  fn offset(&self) -> Point<Pixels> {
    point(self.0.get().offset, px(0.))
  }

  fn set_offset(&self, offset: Point<Pixels>) {
    let mut state = self.0.get();
    state.offset = offset
      .x
      .clamp(-(state.width - state.viewport.size.width).max(px(0.)), px(0.));
    self.0.set(state);
  }

  fn content_size(&self) -> Size<Pixels> {
    let state = self.0.get();
    size(state.width.max(state.viewport.size.width), state.viewport.size.height)
  }
}

fn side(index: usize) -> DiffSide {
  if index == 0 {
    DiffSide::Original
  } else {
    DiffSide::Modified
  }
}

fn key(side: usize, offset: usize) -> u64 {
  ((side as u64) << 32) | offset as u64
}

fn decode_key(key: u64) -> (usize, usize) {
  ((key >> 32) as usize, (key & u32::MAX as u64) as usize)
}

struct CodeRun {
  side: usize,
  line: usize,
  chunk: Range<usize>,
  bounds: Bounds<Pixels>,
  text: SharedString,
  order: u64,
  layout: TextLayout,
}

#[derive(Default)]
struct Geometry {
  bounds: Bounds<Pixels>,
  columns: [Option<Bounds<Pixels>>; 2],
  scroll: Point<Pixels>,
  runs: Vec<CodeRun>,
  multi_click: Cell<Option<(Point<Pixels>, usize, bool)>>,
}

impl Geometry {
  fn run_at(&self, position: Point<Pixels>, side: Option<usize>) -> Option<&CodeRun> {
    let side = side.or_else(|| {
      self
        .columns
        .iter()
        .position(|column| column.is_some_and(|bounds| position.x >= bounds.left() && position.x < bounds.right()))
    });
    self
      .runs
      .iter()
      .filter(|run| side.is_none_or(|side| side == run.side))
      .min_by(|a, b| {
        let distance = |run: &CodeRun| {
          let outside_row = position.y < run.bounds.top() || position.y >= run.bounds.bottom();
          let dy: f32 = (position.y - run.bounds.center().y).abs().into();
          let dx: f32 = (run.bounds.left() - position.x)
            .max(position.x - run.bounds.right())
            .max(px(0.))
            .into();
          (outside_row, dy, dx)
        };
        let (outside_a, dy_a, dx_a) = distance(a);
        let (outside_b, dy_b, dx_b) = distance(b);
        outside_a
          .cmp(&outside_b)
          .then_with(|| dy_a.total_cmp(&dy_b))
          .then_with(|| dx_a.total_cmp(&dx_b))
      })
  }

  fn key_at(&self, position: Point<Pixels>, selected_side: Option<usize>, document: &Document) -> Option<u64> {
    if let Some((position, clicks, cursor)) = self.multi_click.get() {
      let run = self.run_at(position, selected_side)?;
      let line = &document.lines[run.side][run.line];
      let offset = run.chunk.start + run.layout.index_for_position(position).unwrap_or_else(|ix| ix);
      let range = if clicks >= 3 {
        0..line.display.len()
      } else {
        word_range(&line.display, offset)
      };
      self.multi_click.set((!cursor).then_some((position, clicks, true)));
      return Some(key(
        run.side,
        line.source.start + line.source_offset(if cursor { range.end } else { range.start }),
      ));
    }
    let position = position + self.bounds.origin + self.scroll;
    let run = self.run_at(position, selected_side)?;
    let line = &document.lines[run.side][run.line];
    let offset = run.chunk.start + run.layout.index_for_position(position).unwrap_or_else(|ix| ix);
    Some(key(run.side, line.source.start + line.source_offset(offset)))
  }
}

fn word_range(text: &str, offset: usize) -> Range<usize> {
  let mut offset = offset.min(text.len());
  while !text.is_char_boundary(offset) {
    offset -= 1;
  }
  let Some(character) = text[offset..].chars().next() else {
    return offset..offset;
  };
  // Match Base's word units across our shaping chunks; its boundary helper is private.
  let class = |c: char| {
    if c == '_'
      || c.is_ascii_alphanumeric()
      || matches!(c, '\u{00C0}'..='\u{024F}' | '\u{0400}'..='\u{04FF}' | '\u{1E00}'..='\u{1EFF}' | '\u{0300}'..='\u{036F}')
    {
      1
    } else if c.is_whitespace() {
      2
    } else {
      0
    }
  };
  let kind = class(character);
  let start = text[..offset]
    .chars()
    .rev()
    .take(128)
    .take_while(|c| kind != 0 && class(*c) == kind)
    .fold(offset, |offset, c| offset - c.len_utf8());
  let end = offset + character.len_utf8();
  let end = text[end..]
    .chars()
    .take(128)
    .take_while(|c| kind != 0 && class(*c) == kind)
    .fold(end, |offset, c| offset + c.len_utf8());
  start..end
}

fn selected_source(selection: &TextSelectionHandle, cx: &App) -> Option<(usize, Range<usize>)> {
  let snapshot = selection.snapshot(cx)?;
  let entity = Some(selection.entity_id());
  if snapshot.anchor().entity_id() != entity || snapshot.cursor().entity_id() != entity {
    return None;
  }
  let (side, anchor) = decode_key(snapshot.anchor().content_key()?.value());
  let (other, cursor) = decode_key(snapshot.cursor().content_key()?.value());
  (side == other && anchor != cursor).then_some((side, anchor.min(cursor)..anchor.max(cursor)))
}

pub(super) struct SplitState {
  document: Option<Arc<Document>>,
  language: SharedString,
  diff: Entity<DiffState>,
  list: ListState,
  horizontal: [SideScroll; 2],
  focus: FocusHandle,
  selection: TextSelectionHandle,
  geometry: Rc<RefCell<Geometry>>,
  active_side: usize,
  dragging_lines: bool,
  clear_on_render: bool,
  presentation: Arc<Presentation>,
  presentation_key: Option<(LineDiffType, Arc<HighlightTheme>)>,
  preparation: Task<()>,
  generation: usize,
  layout_key: Option<(Pixels, SharedString, bool, bool, DiffIndicators, HunkSeparators, bool)>,
  widths: [Pixels; 2],
  _selection_subscription: Subscription,
}

impl SplitState {
  pub(super) fn new(diff: Entity<DiffState>, cx: &mut Context<Self>) -> Self {
    init(cx);
    let focus = cx.focus_handle().tab_stop(true);
    let selection = TextSelectionHandle::new("", cx);
    let geometry = Rc::new(RefCell::new(Geometry::default()));
    let weak = cx.weak_entity();
    selection.resolve_content_key_with(
      {
        let weak = weak.clone();
        let geometry = geometry.clone();
        move |position, cx| {
          let state = weak.upgrade()?;
          let state = state.read(cx);
          let document = state.document.as_ref()?;
          let snapshot = state.selection.snapshot(cx);
          let anchor_side = snapshot
            .filter(|snapshot| snapshot.anchor().entity_id() == Some(state.selection.entity_id()))
            .and_then(|snapshot| snapshot.anchor().content_key())
            .map(|key| decode_key(key.value()).0);
          geometry
            .borrow()
            .key_at(position, anchor_side, document)
            .map(TextSelectionContentKey::new)
        }
      },
      cx,
    );
    selection.copy_with(
      {
        let weak = weak.clone();
        move |cx| {
          weak
            .upgrade()
            .map_or_else(String::new, |state| state.read(cx).selected_text(cx))
        }
      },
      cx,
    );
    selection.focus_with(
      {
        let focus = focus.clone();
        move |window, cx| window.focus(&focus, cx)
      },
      cx,
    );
    let subscription = selection.subscribe(
      move |event, cx| {
        let _ = weak.update(cx, |state, cx| {
          if !state.selection.has_local_selection(cx) {
            if matches!(
              event,
              TextSelectionEvent::SelectionChanged(Some(_)) | TextSelectionEvent::Cleared
            ) {
              state.diff.update(cx, |diff, cx| diff.set_selected_lines(None, cx));
              state.dragging_lines = false;
            }
            if let Some(anchor) = state
              .selection
              .snapshot(cx)
              .and_then(|snapshot| snapshot.anchor().content_key())
            {
              state.active_side = decode_key(anchor.value()).0;
            }
          }
          cx.notify();
        });
      },
      cx,
    );
    Self {
      document: None,
      language: "text".into(),
      diff,
      list: ListState::new(0, ListAlignment::Top, px(300.)),
      horizontal: Default::default(),
      focus,
      selection,
      geometry,
      active_side: 1,
      dragging_lines: false,
      clear_on_render: false,
      presentation: Arc::new(Presentation::default()),
      presentation_key: None,
      preparation: Task::ready(()),
      generation: 0,
      layout_key: None,
      widths: [px(0.); 2],
      _selection_subscription: subscription,
    }
  }

  pub(super) fn set_document(&mut self, document: Option<Document>, language: SharedString, cx: &mut Context<Self>) {
    self.document = document.map(Arc::new);
    self.language = language;
    self.generation += 1;
    self.preparation = Task::ready(());
    self.presentation = Arc::new(Presentation::default());
    self.presentation_key = None;
    self.layout_key = None;
    self
      .list
      .reset_with_uniform_height(self.document.as_ref().map_or(0, |doc| doc.rows.len()), px(22.));
    self
      .horizontal
      .iter()
      .for_each(|scroll| scroll.0.set(Horizontal::default()));
    self.geometry.borrow_mut().runs.clear();
    self.dragging_lines = false;
    self.clear_on_render = self.selection.has_local_selection(cx) || self.selection.snapshot(cx).is_some();
    cx.notify();
  }

  pub(super) fn has_document(&self) -> bool {
    self.document.as_ref().is_some_and(|doc| !doc.rows.is_empty())
  }

  #[cfg(test)]
  pub(super) fn focus_handle(&self) -> FocusHandle {
    self.focus.clone()
  }

  pub(super) fn top_line(&self) -> Option<DiffLinePosition> {
    let doc = self.document.as_ref()?;
    doc.rows[self.list.logical_scroll_top().item_ix.min(doc.rows.len())..]
      .iter()
      .find_map(|row| {
        let Row::Code { lines, .. } = row else { return None };
        let index = if lines[1].is_some() { 1 } else { 0 };
        let line = &doc.lines[index][lines[index]?];
        Some(DiffLinePosition::new(doc.path.clone(), side(index), line.number))
      })
  }

  pub(super) fn move_change(&self, forward: bool, cx: &mut Context<Self>) {
    let Some(doc) = &self.document else { return };
    let top = self.list.logical_scroll_top().item_ix;
    let next = if forward {
      doc
        .changes
        .iter()
        .copied()
        .find(|row| *row > top)
        .or_else(|| doc.changes.first().copied())
    } else {
      doc
        .changes
        .iter()
        .copied()
        .rev()
        .find(|row| *row < top)
        .or_else(|| doc.changes.last().copied())
    };
    if let Some(item_ix) = next {
      self.list.scroll_to(ListOffset {
        item_ix,
        offset_in_item: px(0.),
      });
      cx.notify();
    }
  }

  pub(super) fn clear_selection(&mut self, cx: &mut Context<Self>) {
    self.selection.set_local_selection(false, cx);
    self.dragging_lines = false;
    cx.notify();
  }

  fn selected_text(&self, cx: &App) -> String {
    if self.selection.has_local_selection(cx) {
      return self.diff.read(cx).selected_text(cx);
    }
    let Some((side, range)) = selected_source(&self.selection, cx) else {
      return String::new();
    };
    self
      .document
      .as_ref()
      .and_then(|doc| doc.source[side].get(range))
      .unwrap_or("")
      .to_string()
  }

  fn select_line(
    &mut self,
    side_index: usize,
    number: usize,
    extend: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let Some(doc) = &self.document else { return };
    let previous = self
      .diff
      .read(cx)
      .selected_lines()
      .filter(|range| range.side() == side(side_index));
    let start = if extend {
      previous.as_ref().map_or(number, DiffLineRange::start)
    } else {
      number
    };
    let range = DiffLineRange::new(doc.path.clone(), side(side_index), start, number);
    TextSelection::clear(window, cx);
    self
      .diff
      .update(cx, |diff, cx| diff.set_selected_lines(Some(range), cx));
    self.selection.set_local_selection(true, cx);
    self.active_side = side_index;
    self.dragging_lines = true;
    window.focus(&self.focus, cx);
    cx.stop_propagation();
    cx.notify();
  }

  fn extend_lines(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
    let Some(doc) = &self.document else { return };
    let index = self.active_side;
    let lines = &doc.lines[index];
    if lines.is_empty() {
      return;
    }
    let selection = self
      .diff
      .read(cx)
      .selected_lines()
      .filter(|range| range.side() == side(index));
    let cursor = selection.as_ref().map_or_else(
      || self.top_line().map_or(lines[0].number, |position| position.line()),
      DiffLineRange::end,
    );
    let ix = lines.partition_point(|line| line.number < cursor).min(lines.len() - 1);
    let ix = if forward {
      (ix + 1).min(lines.len() - 1)
    } else {
      ix.saturating_sub(1)
    };
    let number = lines[ix].number;
    let row = doc
      .rows
      .iter()
      .position(|row| matches!(row, Row::Code { lines, .. } if lines[index] == Some(ix)));
    self.select_line(index, number, true, window, cx);
    self.dragging_lines = false;
    if let Some(row) = row {
      self.list.scroll_to_reveal_item(row);
    }
  }

  fn ensure_presentation(&mut self, cx: &mut Context<Self>) {
    let unit = AppConfig::get(cx).settings.diff.line_diff_type;
    let theme = cx.theme().highlight_theme.clone();
    if self
      .presentation_key
      .as_ref()
      .is_some_and(|(previous, old_theme)| *previous == unit && Arc::ptr_eq(old_theme, &theme))
    {
      return;
    }
    let Some(document) = self.document.clone() else { return };
    self.presentation_key = Some((unit, theme.clone()));
    self.generation += 1;
    let generation = self.generation;
    let language = self.language.clone();
    let task = cx
      .background_executor()
      .spawn(async move { Presentation::prepare(&document, &language, unit, &theme) });
    self.preparation = cx.spawn(async move |entity, cx| {
      let prepared = task.await;
      let _ = entity.update(cx, |state, cx| {
        if state.generation == generation {
          state.presentation = Arc::new(prepared);
          cx.notify();
        }
      });
    });
  }
}

type HunkRenderer = Rc<dyn Fn(&Hunk, &mut Window, &mut App) -> AnyElement>;

#[derive(IntoElement)]
pub(super) struct SplitDiff {
  state: Entity<SplitState>,
  actions: Option<HunkRenderer>,
}

impl SplitDiff {
  pub(super) fn new(state: &Entity<SplitState>, actions: Option<HunkRenderer>) -> Self {
    Self {
      state: state.clone(),
      actions,
    }
  }
}

impl RenderOnce for SplitDiff {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    self
      .state
      .update(cx, |state, cx| render_split(state, self.actions, window, cx))
  }
}

struct Paint {
  document: Arc<Document>,
  presentation: Arc<Presentation>,
  state: Entity<SplitState>,
  geometry: Rc<RefCell<Geometry>>,
  horizontal: [SideScroll; 2],
  selected: Option<(usize, Range<usize>)>,
  selected_lines: Option<DiffLineRange>,
  row_height: Pixels,
  gutter: Pixels,
  line_numbers: bool,
  background: bool,
  indicators: DiffIndicators,
  separators: HunkSeparators,
  wrap: bool,
  actions: Option<HunkRenderer>,
}

fn render_split(
  state: &mut SplitState,
  actions: Option<HunkRenderer>,
  window: &mut Window,
  cx: &mut Context<SplitState>,
) -> AnyElement {
  if state.clear_on_render {
    state.clear_on_render = false;
    TextSelection::clear(window, cx);
    state.selection.set_local_selection(false, cx);
  }
  state.ensure_presentation(cx);
  let Some(document) = state.document.clone() else {
    return div().into_any_element();
  };
  let settings = &AppConfig::get(cx).settings;
  let line_numbers = settings.diff.show_line_numbers;
  let indicators = settings.diff.diff_indicators;
  let separators = settings.diff.hunk_separators;
  let wrap = settings.editor.word_wrap == WordWrap::On;
  let background = settings.diff.show_background;
  let family = cx.theme().mono_font_family.clone();
  let font_size = window.rem_size() * (f32::from(cx.theme().mono_font_size) / 16.);
  let row_height = font_size * 1.6;
  let digits = document
    .lines
    .iter()
    .filter_map(|lines| lines.last())
    .map(|line| line.number.ilog10() + 1)
    .max()
    .unwrap_or(1);
  let gutter = if line_numbers {
    font_size * 0.7 * digits as f32 + window.rem_size()
  } else {
    px(0.)
  };
  let layout_key = (
    font_size,
    family.clone(),
    wrap,
    line_numbers,
    indicators,
    separators,
    actions.is_some(),
  );
  if state.layout_key.as_ref() != Some(&layout_key) {
    let font_changed = state
      .layout_key
      .as_ref()
      .is_none_or(|previous| previous.0 != font_size || previous.1 != family);
    if font_changed {
      state.widths = measure_widths(&document, font_size, &family, window, cx);
    }
    for (index, width) in state.widths.iter().enumerate() {
      let scroll = &state.horizontal[index];
      scroll.0.set(Horizontal {
        width: *width,
        ..scroll.0.get()
      });
    }
    let anchor = state.list.logical_scroll_top();
    state.list.reset_with_uniform_height(document.rows.len(), row_height);
    state.list.scroll_to(anchor);
    state.layout_key = Some(layout_key);
  }
  let paint = Rc::new(Paint {
    document,
    presentation: state.presentation.clone(),
    state: cx.entity(),
    geometry: state.geometry.clone(),
    horizontal: state.horizontal.clone(),
    selected: selected_source(&state.selection, cx),
    selected_lines: state.diff.read(cx).selected_lines(),
    row_height,
    gutter,
    line_numbers,
    background,
    indicators,
    separators,
    wrap,
    actions,
  });
  let scrollbar = state.list.clone();
  let horizontal = state.horizontal.clone();
  let indicator_width = if indicators == DiffIndicators::None {
    window.rem_size() / 2.
  } else {
    window.rem_size()
  };
  let leading = gutter + indicator_width + window.rem_size() / 2.;
  let trailing = window.rem_size() / 2.;
  let rows = paint.clone();
  let body = div()
    .id("bounded-split-body")
    .relative()
    .size_full()
    .min_w_0()
    .min_h_0()
    .overflow_hidden()
    .font_family(family)
    .text_size(font_size)
    .line_height(row_height)
    .text_color(cx.theme().foreground)
    .child(SelectionLayer {
      child: list(scrollbar.clone(), move |ix, window, cx| {
        render_row(ix, &rows, window, cx)
      })
      .size_full()
      .pb_4()
      .into_any_element(),
      geometry: state.geometry.clone(),
      selection: state.selection.clone(),
      list: scrollbar.clone(),
      horizontal: horizontal.clone(),
      leading,
      trailing,
      wrap,
    })
    .children(
      (!wrap)
        .then(|| {
          (0..2)
            .map(|index| {
              div()
                .absolute()
                .top_0()
                .left(relative(index as f32 / 2.))
                .w(relative(0.5))
                .h_full()
                .child(ScrollableMask::new(Axis::Horizontal, &horizontal[index]).id(("split-wheel", index)))
                .child(
                  Scrollbar::horizontal(&horizontal[index])
                    .id(("split-horizontal", index))
                    .mode(ScrollbarMode::Always),
                )
            })
            .collect::<Vec<_>>()
        })
        .into_iter()
        .flatten(),
    )
    .child(Scrollbar::vertical(&scrollbar))
    .on_mouse_up(
      MouseButton::Left,
      cx.listener(|state, _, _, _| state.dragging_lines = false),
    )
    .on_mouse_up_out(
      MouseButton::Left,
      cx.listener(|state, _, _, _| state.dragging_lines = false),
    )
    .on_action(cx.listener(|state, _: &Up, _, cx| {
      state.list.scroll_by(-px(24.));
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &Down, _, cx| {
      state.list.scroll_by(px(24.));
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &Left, _, cx| {
      state.horizontal[state.active_side].scroll_by(px(40.));
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &Right, _, cx| {
      state.horizontal[state.active_side].scroll_by(-px(40.));
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &PageUp, _, cx| {
      state.list.scroll_by(-state.list.viewport_bounds().size.height);
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &PageDown, _, cx| {
      state.list.scroll_by(state.list.viewport_bounds().size.height);
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &First, _, cx| {
      state.list.scroll_to(ListOffset::default());
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &Last, _, cx| {
      state.list.scroll_to_end();
      cx.notify();
    }))
    .on_action(cx.listener(|state, _: &ExtendUp, window, cx| state.extend_lines(false, window, cx)))
    .on_action(cx.listener(|state, _: &ExtendDown, window, cx| state.extend_lines(true, window, cx)))
    .on_action(cx.listener(|state, _: &SelectAll, window, cx| {
      let Some(doc) = &state.document else { return };
      let lines = &doc.lines[state.active_side];
      let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return;
      };
      let (start, end, side) = (first.number, last.number, state.active_side);
      state.select_line(side, start, false, window, cx);
      state.select_line(side, end, true, window, cx);
      state.dragging_lines = false;
    }))
    .on_action(cx.listener(|state, _: &Copy, _, cx| {
      let text = state.selected_text(cx);
      if text.is_empty() {
        cx.propagate();
      } else {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
      }
    }))
    .track_focus(&state.focus)
    // `Diff` is the context the app keymap binds F7 and Shift+F7 to, the same one gpui-kit's inline `Diff` sets.
    .key_context("BoundedSplit Diff")
    .role(gpui_kit::accesskit::Role::Document)
    .aria_label("Side by side diff");
  div()
    .flex_1()
    .min_w_0()
    .min_h_0()
    .size_full()
    .child(body)
    .into_any_element()
}

fn measure_widths(
  document: &Document,
  font_size: Pixels,
  family: &SharedString,
  window: &mut Window,
  cx: &App,
) -> [Pixels; 2] {
  let font = Font {
    family: family.clone(),
    ..Default::default()
  };
  std::array::from_fn(|side| {
    let mut candidates = Vec::with_capacity(9);
    for line in &document.lines[side] {
      let width = line.display.len();
      let index = candidates.partition_point(|(size, _)| *size >= width);
      if index < 8 {
        candidates.insert(index, (width, line));
        candidates.truncate(8);
      }
    }
    candidates
      .into_iter()
      .map(|(_, line)| {
        line
          .chunks
          .iter()
          .map(|(_, text)| {
            window
              .text_system()
              .shape_line(
                text.clone(),
                font_size,
                &[TextRun {
                  len: text.len(),
                  font: font.clone(),
                  color: cx.theme().foreground,
                  background_color: None,
                  underline: None,
                  strikethrough: None,
                }],
                None,
              )
              .width
          })
          .fold(px(0.), |sum, width| sum + width)
      })
      .fold(px(0.), Pixels::max)
      + px(16.)
  })
}

fn render_row(ix: usize, paint: &Rc<Paint>, window: &mut Window, cx: &mut App) -> AnyElement {
  match &paint.document.rows[ix] {
    Row::Separator(hunk) => {
      let hunk = &paint.document.hunks[*hunk];
      let row = div()
        .w_full()
        .px_3()
        .bg(cx.theme().muted.opacity(0.35))
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .overflow_hidden();
      match paint.separators {
        HunkSeparators::Simple => row.h_1().into_any_element(),
        HunkSeparators::Metadata => row.h_6().child(hunk.label.clone()).into_any_element(),
        HunkSeparators::LineInfo if hunk.hidden_before > 0 => row
          .h_6()
          .child(format!("{} hidden lines", hunk.hidden_before))
          .into_any_element(),
        HunkSeparators::LineInfo => div().into_any_element(),
      }
    }
    Row::Code { lines, changed } => {
      let mut row = div().id(("split-row", ix)).w_full().min_w_0().child(
        div()
          .flex()
          .items_stretch()
          .w_full()
          .min_w_0()
          .children((0..2).map(|side| render_cell(side, lines[side], lines[1 - side], *changed, paint, cx))),
      );
      if let Some(render) = &paint.actions {
        let hunk = paint.document.hunks.partition_point(|hunk| hunk.first_row < ix);
        if paint.document.hunks.get(hunk).is_some_and(|hunk| hunk.first_row == ix) {
          row = row.child(
            div()
              .w_full()
              .min_w_0()
              .px_3()
              .py_3()
              .border_y_1()
              .border_color(cx.theme().border)
              .bg(cx.theme().muted.opacity(0.18))
              .child(render(&paint.document.hunks[hunk], window, cx)),
          );
        }
      }
      row.into_any_element()
    }
  }
}

fn render_cell(
  side_index: usize,
  line: Option<usize>,
  other: Option<usize>,
  changed: bool,
  paint: &Rc<Paint>,
  cx: &mut App,
) -> AnyElement {
  let cell = div()
    .w(relative(0.5))
    .flex_shrink_0()
    .min_w_0()
    .overflow_hidden()
    .when(side_index == 0, |cell| {
      cell.border_r_1().border_color(cx.theme().border)
    });
  let column = |child: AnyElement| {
    Column {
      child,
      side: side_index,
      geometry: paint.geometry.clone(),
    }
    .into_any_element()
  };
  let Some(ix) = line else {
    return column(
      cell
        .bg(cx.theme().muted.opacity(0.3))
        .min_h(paint.row_height)
        .into_any_element(),
    );
  };
  let line = &paint.document.lines[side_index][ix];
  let status = if side_index == 0 {
    cx.theme().danger
  } else {
    cx.theme().success
  };
  let selected = paint.selected_lines.as_ref().is_some_and(|range| {
    range.side() == side(side_index)
      && range.start().min(range.end()) <= line.number
      && line.number <= range.start().max(range.end())
  });
  let syntax = paint.presentation.lines[side_index]
    .get(ix)
    .into_iter()
    .flatten()
    .cloned();
  let inline = paint.presentation.inline[side_index]
    .get(ix)
    .into_iter()
    .flatten()
    .cloned()
    .map(|range| {
      (
        range,
        HighlightStyle {
          background_color: Some(status.opacity(0.3)),
          ..Default::default()
        },
      )
    });
  let highlights = combine_highlights(syntax, inline);
  let selection = paint
    .selected
    .as_ref()
    .filter(|(side, _)| *side == side_index)
    .and_then(|(_, range)| {
      let start = range.start.max(line.source.start);
      let end = range.end.min(line.content_end);
      (start < end).then(|| {
        (
          line.display_offset(start - line.source.start)..line.display_offset(end - line.source.start),
          HighlightStyle {
            background_color: Some(cx.theme().selection),
            ..Default::default()
          },
        )
      })
    });
  let highlights: Vec<_> = combine_highlights(highlights, selection).collect();
  let render_text = |chunk_ix: usize, range: Range<usize>, text: SharedString| {
    let highlights = highlights
      .iter()
      .filter_map(|(highlight, style)| {
        let start = highlight.start.max(range.start);
        let end = highlight.end.min(range.end);
        (start < end).then(|| (start - range.start..end - range.start, *style))
      })
      .collect::<Vec<_>>();
    let order = key(side_index, line.source.start + line.source_offset(range.start));
    SourceText {
      id: ("split-text", chunk_ix).into(),
      source: text.clone(),
      text: StyledText::new(text).with_highlights(highlights),
      geometry: paint.geometry.clone(),
      side: side_index,
      line: ix,
      chunk: range,
      order,
    }
  };
  let text = div()
    .when(!paint.wrap, |text| text.flex().flex_shrink_0())
    .when(paint.wrap, |text| text.w_full())
    .map(|text| {
      if paint.wrap {
        text.child(render_text(0, 0..line.display.len(), line.display.clone()))
      } else {
        text.children(
          line
            .chunks
            .iter()
            .enumerate()
            .map(|(ix, (range, text))| render_text(ix, range.clone(), text.clone())),
        )
      }
    });
  let indicator = match paint.indicators {
    DiffIndicators::Classic => div().w_4().flex_shrink_0().text_color(status).child(if changed {
      if side_index == 0 { "−" } else { "+" }
    } else {
      " "
    }),
    DiffIndicators::Bars => div()
      .w_4()
      .flex_shrink_0()
      .when(changed, |marker| marker.child(div().w(px(3.)).h_full().bg(status))),
    DiffIndicators::None => div().w_2().flex_shrink_0(),
  };
  let number = line.number;
  let down = paint.state.clone();
  let moved = paint.state.clone();
  let row = div()
    .id(("split-source", side_index))
    .flex()
    .items_stretch()
    .w_full()
    .min_w_0()
    .px_2()
    .when(paint.wrap, |row| row.min_h(paint.row_height).whitespace_normal())
    .when(!paint.wrap, |row| row.h(paint.row_height).whitespace_nowrap())
    .when(changed && paint.background, |row| row.bg(status.opacity(0.12)))
    .when(selected, |row| row.bg(cx.theme().selection))
    .when(paint.line_numbers, |row| {
      row.child(
        div()
          .id("gutter")
          .w(paint.gutter)
          .h(paint.row_height)
          .flex_shrink_0()
          .pr_1()
          .text_right()
          .text_color(cx.theme().muted_foreground)
          .child(number.to_string())
          .role(gpui_kit::accesskit::Role::Button)
          .aria_label(format!(
            "Select {} line {number}",
            if side_index == 0 { "original" } else { "modified" }
          ))
          .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            down.update(cx, |state, cx| {
              state.select_line(side_index, number, event.modifiers.shift, window, cx)
            })
          })
          .on_mouse_move(move |event, window, cx| {
            if event.pressed_button == Some(MouseButton::Left)
              && moved.read(cx).dragging_lines
              && moved.read(cx).active_side == side_index
            {
              moved.update(cx, |state, cx| state.select_line(side_index, number, true, window, cx));
            }
          }),
      )
    })
    .child(indicator)
    .child(
      div()
        .id("source")
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .role(gpui_kit::accesskit::Role::Label)
        .aria_label(line.display.clone())
        .aria_description(format!(
          "{} line {number}",
          if side_index == 0 { "Original" } else { "Modified" }
        ))
        .child(
          div()
            .relative()
            .when(!paint.wrap, |text| {
              text
                .left(paint.horizontal[side_index].offset().x)
                .w(paint.horizontal[side_index].content_size().width)
                .flex()
            })
            .when(paint.wrap, |text| text.w_full())
            .child(text),
        ),
    );
  let ending = &paint.document.source[side_index][line.content_end..line.source.end];
  let ending_changed = changed
    && other.is_some_and(|other| {
      let other_line = &paint.document.lines[1 - side_index][other];
      line.display == other_line.display
        && ending != &paint.document.source[1 - side_index][other_line.content_end..other_line.source.end]
    });
  column(
    cell
      .child(row)
      .when(ending.is_empty() || ending_changed, |cell| {
        cell.child(
          div()
            .h_6()
            .px_3()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(if ending.is_empty() {
              "\\ No newline at end of file"
            } else if ending == "\r\n" {
              "\\ CRLF"
            } else {
              "\\ LF"
            }),
        )
      })
      .into_any_element(),
  )
}

/// Records the actual laid-out column, not its potentially enormous text bounds.
struct Column {
  child: AnyElement,
  side: usize,
  geometry: Rc<RefCell<Geometry>>,
}

impl IntoElement for Column {
  type Element = Self;
  fn into_element(self) -> Self {
    self
  }
}

impl Element for Column {
  type RequestLayoutState = ();
  type PrepaintState = ();
  fn id(&self) -> Option<ElementId> {
    None
  }
  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }
  fn request_layout(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, ()) {
    (self.child.request_layout(window, cx), ())
  }
  fn prepaint(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    self.geometry.borrow_mut().columns[self.side] = Some(bounds);
    self.child.prepaint(window, cx);
  }
  fn paint(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    _: Bounds<Pixels>,
    _: &mut (),
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    self.child.paint(window, cx);
  }
}

struct SourceText {
  id: ElementId,
  text: StyledText,
  source: SharedString,
  order: u64,
  geometry: Rc<RefCell<Geometry>>,
  side: usize,
  line: usize,
  chunk: Range<usize>,
}

impl IntoElement for SourceText {
  type Element = Self;
  fn into_element(self) -> Self {
    self
  }
}

impl Element for SourceText {
  type RequestLayoutState = ();
  type PrepaintState = ();
  fn id(&self) -> Option<ElementId> {
    Some(self.id.clone())
  }
  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }
  fn request_layout(
    &mut self,
    id: Option<&GlobalElementId>,
    inspector: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, ()) {
    self.text.request_layout(id, inspector, window, cx)
  }
  fn prepaint(
    &mut self,
    id: Option<&GlobalElementId>,
    inspector: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    self.text.prepaint(id, inspector, bounds, &mut (), window, cx);
    self.geometry.borrow_mut().runs.push(CodeRun {
      side: self.side,
      line: self.line,
      chunk: self.chunk.clone(),
      bounds,
      layout: self.text.layout().clone(),
      text: self.source.clone(),
      order: self.order,
    });
  }
  fn paint(
    &mut self,
    id: Option<&GlobalElementId>,
    inspector: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _: &mut (),
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    self.text.paint(id, inspector, bounds, &mut (), &mut (), window, cx);
  }
}

/// One participant covers both virtual columns, so dragging across the divider
/// remains anchored to the source side where the selection started.
struct SelectionLayer {
  child: AnyElement,
  geometry: Rc<RefCell<Geometry>>,
  selection: TextSelectionHandle,
  list: ListState,
  horizontal: [SideScroll; 2],
  leading: Pixels,
  trailing: Pixels,
  wrap: bool,
}

impl IntoElement for SelectionLayer {
  type Element = Self;
  fn into_element(self) -> Self {
    self
  }
}

impl Element for SelectionLayer {
  type RequestLayoutState = ();
  type PrepaintState = ();
  fn id(&self) -> Option<ElementId> {
    Some("bounded-split-selection".into())
  }
  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }
  fn request_layout(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, ()) {
    (self.child.request_layout(window, cx), ())
  }
  fn prepaint(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    bounds: Bounds<Pixels>,
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    {
      let mut geometry = self.geometry.borrow_mut();
      if self.wrap && geometry.bounds.size.width != bounds.size.width {
        self.list.remeasure();
      }
      geometry.bounds = bounds;
      geometry.runs.clear();
      geometry.columns = [None, None];
    }
    for index in 0..2 {
      let width = bounds.size.width / 2.;
      self.horizontal[index].configure(
        Bounds::new(
          point(bounds.left() + width * index as f32 + self.leading, bounds.top()),
          size(
            (width - self.leading - self.trailing - if index == 0 { px(1.) } else { px(0.) }).max(px(0.)),
            bounds.size.height,
          ),
        ),
        if self.wrap {
          px(0.)
        } else {
          self.horizontal[index].0.get().width
        },
      );
    }
    let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
    self.child.prepaint(window, cx);
    if !self.wrap {
      let mut first: [Option<(usize, Pixels)>; 2] = [None, None];
      let mut widths = self.horizontal.each_ref().map(|scroll| scroll.0.get().width);
      for run in &self.geometry.borrow().runs {
        let entry = &mut first[run.side];
        if entry.is_none_or(|(line, _)| line != run.line) {
          *entry = Some((run.line, run.bounds.left()));
        }
        if let Some((_, left)) = entry {
          widths[run.side] = widths[run.side].max(run.bounds.right() - *left + px(16.));
        }
      }
      let mut changed = false;
      for (index, width) in widths.into_iter().enumerate() {
        let scroll = &self.horizontal[index];
        if width > scroll.0.get().width {
          scroll.0.set(Horizontal {
            width,
            ..scroll.0.get()
          });
          changed = true;
        }
      }
      if changed {
        window.on_next_frame(|window, _| window.refresh());
      }
    }
    let scroll = ScrollbarHandle::offset(&self.list);
    self.geometry.borrow_mut().scroll = point(px(0.), scroll.y);
    let text_bounds = self
      .geometry
      .borrow()
      .runs
      .iter()
      .map(|run| run.bounds.intersect(&self.horizontal[run.side].viewport_bounds()))
      .collect();
    self.selection.register(
      TextSelectionRegistration::new(hitbox, bounds)
        .with_scroll_offset(point(px(0.), scroll.y))
        .with_text_bounds(text_bounds)
        .with_rendered_element(&self.selection, window, cx),
      window,
      cx,
    );
  }
  fn paint(
    &mut self,
    _: Option<&GlobalElementId>,
    _: Option<&InspectorElementId>,
    _: Bounds<Pixels>,
    _: &mut (),
    _: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) {
    // The copy callback reads stable source keys, not this visible-only projection.
    let runs = self
      .geometry
      .borrow()
      .runs
      .iter()
      .map(|run| TextSelectionRun::new(run.text.clone(), run.layout.clone(), run.bounds).with_document_order(run.order))
      .collect::<Vec<_>>();
    self.selection.update_runs(&runs, cx);
    window.on_mouse_event({
      let geometry = self.geometry.clone();
      move |event: &MouseDownEvent, phase, _, _| {
        if phase.capture() && event.button == MouseButton::Left {
          geometry.borrow().multi_click.set((event.click_count >= 2).then_some((
            event.position,
            event.click_count,
            false,
          )));
        }
      }
    });
    window.on_mouse_event({
      let geometry = self.geometry.clone();
      move |_: &MouseMoveEvent, phase, _, _| {
        if phase.capture() {
          geometry.borrow().multi_click.set(None);
        }
      }
    });
    window.on_mouse_event({
      let geometry = self.geometry.clone();
      move |_: &MouseUpEvent, phase, _, _| {
        if phase.capture() {
          geometry.borrow().multi_click.set(None);
        }
      }
    });
    self.child.paint(window, cx);
  }
}

#[cfg(test)]
#[path = "split_tests.rs"]
mod tests;
