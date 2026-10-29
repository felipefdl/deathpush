use super::*;
use core::prelude::v1::test;
use gpui_kit::base::TextSelectionLayer;
use gpui_kit::component::diff::{DiffFile, DiffMode};

struct Host {
  state: Entity<SplitState>,
  width: Pixels,
  actions: Option<HunkRenderer>,
  _config: tempfile::TempDir,
}

impl Render for Host {
  fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
    div()
      .relative()
      .w(self.width)
      .h(px(240.))
      .flex()
      .flex_col()
      .child(TextSelectionLayer)
      .child(SplitDiff::new(&self.state, self.actions.clone()))
  }
}

fn draw(cx: &mut VisualTestContext) {
  cx.run_until_parked();
  cx.update(|window, cx| {
    let _ = window.draw(cx);
  });
}

fn host<'a>(
  patch: &str,
  width: f32,
  cx: &'a mut TestAppContext,
) -> (Entity<SplitState>, Entity<Host>, &'a mut VisualTestContext) {
  let config = tempfile::TempDir::new().unwrap();
  cx.update(|cx| {
    gpui_kit::init(cx);
    AppConfig::init_at(config.path().to_path_buf(), cx);
    crate::theme::init(cx);
    AppConfig::update(cx, |config| {
      config.settings.editor.word_wrap = WordWrap::Off;
      config.settings.diff.hunk_separators = HunkSeparators::Simple;
      config.settings.diff.show_line_numbers = true;
    });
  });
  let files = DiffFile::parse(patch).unwrap();
  let diff = cx.new(|cx| {
    DiffState::new(files, cx)
      .with_mode(DiffMode::Split)
      .with_context_lines(None)
  });
  let state = cx.new(|cx| SplitState::new(diff, cx));
  let document = Document::parse("example.txt", patch).unwrap();
  state.update(cx, |state, cx| state.set_document(Some(document), "text".into(), cx));
  let (view, cx) = cx.add_window_view({
    let state = state.clone();
    move |_, cx| {
      cx.observe(&state, |_, _, cx| cx.notify()).detach();
      cx.observe_global::<AppConfig>(|_, cx| cx.notify()).detach();
      Host {
        state,
        width: px(width),
        actions: None,
        _config: config,
      }
    }
  });
  for _ in 0..3 {
    draw(cx);
  }
  (state, view, cx)
}

fn patch(old: &[String], new: &[String]) -> String {
  let mut patch = format!(
    "diff --git a/example.txt b/example.txt\n--- a/example.txt\n+++ b/example.txt\n@@ -1,{} +1,{} @@\n",
    old.len(),
    new.len()
  );
  for line in old {
    patch.push('-');
    patch.push_str(line);
    patch.push('\n');
  }
  for line in new {
    patch.push('+');
    patch.push_str(line);
    patch.push('\n');
  }
  patch
}

fn columns(state: &Entity<SplitState>, cx: &mut VisualTestContext) -> [Bounds<Pixels>; 2] {
  cx.update(|_, cx| {
    let state = state.read(cx);
    let geometry = state.geometry.borrow();
    let columns = geometry.columns.map(Option::unwrap);
    let viewport = geometry.bounds;
    assert!(viewport.size.width > px(0.));
    for column in columns {
      assert!(
        column.left() >= viewport.left(),
        "column begins left of the viewport: {column:?}"
      );
      assert!(column.left() < viewport.right(), "column begins offscreen: {column:?}");
      assert!(
        column.right() <= viewport.right() + px(0.5),
        "column extends past the viewport: {column:?}"
      );
      assert!(column.size.width <= viewport.size.width / 2. + px(0.5));
    }
    assert!((columns[0].right() - columns[1].left()).abs() < px(0.5));
    columns
  })
}

fn wheel(side: usize, dx: f32, dy: f32, state: &Entity<SplitState>, cx: &mut VisualTestContext) {
  let position = cx.update(|_, cx| state.read(cx).horizontal[side].viewport_bounds().center());
  cx.simulate_event(ScrollEvent {
    position,
    delta: ScrollDelta::Pixels(point(px(dx), px(dy))),
    ..Default::default()
  });
  draw(cx);
}

fn long_patch() -> String {
  patch(
    &["a".repeat(1200), "old tail".into()],
    &["b".repeat(1200), "new tail".into()],
  )
}

#[gpui_kit::test]
fn long_lines_keep_both_column_starts_visible_while_each_side_scrolls(cx: &mut TestAppContext) {
  let patch = long_patch();
  let (state, _view, cx) = host(&patch, 640., cx);
  let before = columns(&state, cx);
  let original_start = cx.update(|_, cx| {
    state
      .read(cx)
      .geometry
      .borrow()
      .runs
      .iter()
      .find(|run| run.side == 0 && run.line == 0 && run.chunk.start == 0)
      .unwrap()
      .bounds
      .left()
  });
  wheel(0, -600., 0., &state, cx);
  cx.update(|_, cx| {
    let state = state.read(cx);
    assert!(state.horizontal[0].offset().x < px(-500.));
    assert_eq!(state.horizontal[1].offset().x, px(0.));
    let geometry = state.geometry.borrow();
    let run = geometry
      .runs
      .iter()
      .find(|run| run.side == 0 && run.line == 0 && run.chunk.start == 0)
      .unwrap();
    assert!(
      run.bounds.left() < original_start - px(500.),
      "the code must actually scroll, not just the thumb"
    );
    let old = geometry.runs.iter().find(|run| run.side == 0 && run.line == 1).unwrap();
    let new = geometry.runs.iter().find(|run| run.side == 1 && run.line == 1).unwrap();
    assert_eq!(old.bounds.top(), new.bounds.top());
  });
  let after_old = columns(&state, cx);
  assert_eq!(
    before.map(|column| column.left()),
    after_old.map(|column| column.left())
  );
  let old_offset = cx.update(|_, cx| state.read(cx).horizontal[0].offset().x);
  wheel(1, -900., 0., &state, cx);
  cx.update(|_, cx| {
    assert_eq!(state.read(cx).horizontal[0].offset().x, old_offset);
    assert!(state.read(cx).horizontal[1].offset().x < px(-800.));
  });
  assert_eq!(
    before.map(|column| column.left()),
    columns(&state, cx).map(|column| column.left())
  );
}

#[gpui_kit::test]
fn long_lines_stay_reachable_after_the_window_narrows(cx: &mut TestAppContext) {
  let patch = long_patch();
  let (state, view, cx) = host(&patch, 640., cx);
  view.update(cx, |host, cx| {
    host.width = px(240.);
    cx.notify();
  });
  draw(cx);
  let narrow = columns(&state, cx);
  assert!((narrow[0].size.width - px(120.)).abs() < px(0.5));
  wheel(0, -100_000., 0., &state, cx);
  wheel(1, -100_000., 0., &state, cx);
  assert_eq!(
    narrow.map(|column| column.left()),
    columns(&state, cx).map(|column| column.left())
  );
  cx.update(|_, cx| {
    let state = state.read(cx);
    let geometry = state.geometry.borrow();
    for side in 0..2 {
      let last = geometry
        .runs
        .iter()
        .rfind(|run| run.side == side && run.line == 0)
        .unwrap();
      let viewport = state.horizontal[side].viewport_bounds();
      assert!(last.bounds.right() <= viewport.right() + px(1.));
      assert!(
        last.bounds.right() > viewport.left(),
        "the final characters must be reachable"
      );
    }
  });
}

fn caret(
  state: &Entity<SplitState>,
  side: usize,
  line: usize,
  source_byte: usize,
  cx: &mut VisualTestContext,
) -> Point<Pixels> {
  cx.update(|_, cx| {
    let state = state.read(cx);
    let document = state.document.as_ref().unwrap();
    let display_byte = document.lines[side][line].display_offset(source_byte);
    let geometry = state.geometry.borrow();
    let run = geometry
      .runs
      .iter()
      .find(|run| {
        run.side == side && run.line == line && run.chunk.start <= display_byte && display_byte <= run.chunk.end
      })
      .unwrap();
    let position = run.layout.position_for_index(display_byte - run.chunk.start).unwrap();
    point(position.x + px(0.1), position.y + run.layout.line_height() / 2.)
  })
}

fn drag(from: Point<Pixels>, to: Point<Pixels>, cx: &mut VisualTestContext) {
  cx.simulate_event(MouseDownEvent {
    position: from,
    button: MouseButton::Left,
    click_count: 1,
    first_mouse: false,
    modifiers: Default::default(),
  });
  cx.simulate_event(MouseMoveEvent {
    position: to,
    pressed_button: Some(MouseButton::Left),
    modifiers: Default::default(),
  });
  cx.simulate_event(MouseUpEvent {
    position: to,
    button: MouseButton::Left,
    click_count: 1,
    modifiers: Default::default(),
  });
  draw(cx);
}

#[gpui_kit::test]
fn selection_after_horizontal_scroll_copies_exact_source_across_chunks_and_virtualized_rows(cx: &mut TestAppContext) {
  let first = format!("{}\tα\tEND{}", "a".repeat(1018), "z".repeat(180));
  let old = std::iter::once(first.clone())
    .chain((1..200).map(|ix| format!("old row {ix}")))
    .collect::<Vec<_>>();
  let new = (0..200).map(|ix| format!("new row {ix}")).collect::<Vec<_>>();
  let (state, _, cx) = host(&patch(&old, &new), 640., cx);
  let start_byte = 1018;
  let end_byte = 1025;
  let start = caret(&state, 0, 0, start_byte, cx);
  cx.update(|_, cx| {
    let scroll = &state.read(cx).horizontal[0];
    scroll.scroll_by(scroll.viewport_bounds().left() + px(20.) - start.x);
  });
  draw(cx);
  let from = caret(&state, 0, 0, start_byte, cx);
  let to = caret(&state, 0, 0, end_byte, cx);
  drag(from, to, cx);
  let expected = &first[start_byte..end_byte];
  cx.update(|window, cx| {
    assert_eq!(state.read(cx).selected_text(cx), expected);
    assert_eq!(TextSelection::selected_text(window, cx), expected);
  });
  state.update(cx, |state, cx| {
    state.list.scroll_to(ListOffset {
      item_ix: 150,
      offset_in_item: px(0.),
    });
    cx.notify();
  });
  draw(cx);
  cx.update(|window, cx| {
    {
      let state = state.read(cx);
      let geometry = state.geometry.borrow();
      assert!(
        geometry.runs.iter().all(|run| run.line > 0),
        "the selected row must be recycled"
      );
      assert!(geometry.runs.len() < 100, "large diffs must remain virtualized");
      assert_eq!(state.selected_text(cx), expected);
    }
    assert_eq!(TextSelection::selected_text(window, cx), expected);
  });
}

#[gpui_kit::test]
fn selection_in_modified_column_never_hits_original_text_extending_behind_it(cx: &mut TestAppContext) {
  let (state, _, cx) = host(&patch(&["a".repeat(1200)], &["modified text".into()]), 480., cx);
  let from = caret(&state, 1, 0, 0, cx);
  let to = caret(&state, 1, 0, 8, cx);
  drag(from, to, cx);
  cx.update(|window, cx| {
    assert_eq!(state.read(cx).selected_text(cx), "modified");
    assert_eq!(TextSelection::selected_text(window, cx), "modified");
  });
}

#[gpui_kit::test]
fn gutter_selection_copy_preserves_tabs_unicode_crlf_and_reversed_ranges(cx: &mut TestAppContext) {
  let old = vec!["\tα old\r".into(), "\tβ tail\r".into()];
  let new = vec!["\tα new\r".into(), "\tβ new tail\r".into()];
  let (state, _, cx) = host(&patch(&old, &new), 480., cx);
  cx.update(|window, cx| {
    state.update(cx, |state, cx| {
      state.select_line(0, 2, false, window, cx);
      state.select_line(0, 1, true, window, cx);
      state.dragging_lines = false;
      assert_eq!(state.selected_text(cx), "\tα old\r\n\tβ tail\r\n");
    })
  });
  draw(cx);
  cx.update(|window, cx| assert_eq!(TextSelection::selected_text(window, cx), "\tα old\r\n\tβ tail\r\n"));
}

#[gpui_kit::test]
fn wrapping_is_opt_in_and_both_sides_keep_the_same_vertical_row_positions(cx: &mut TestAppContext) {
  let (state, _, cx) = host(
    &patch(
      &["a ".repeat(100), "old tail".into()],
      &["short".into(), "new tail".into()],
    ),
    480.,
    cx,
  );
  let unwrapped = cx.update(|_, cx| {
    let state = state.read(cx);
    let geometry = state.geometry.borrow();
    geometry
      .runs
      .iter()
      .find(|run| run.side == 0 && run.line == 0)
      .unwrap()
      .bounds
      .size
      .height
  });
  cx.update(|_, cx| AppConfig::update(cx, |config| config.settings.editor.word_wrap = WordWrap::On));
  draw(cx);
  cx.update(|_, cx| {
    let state = state.read(cx);
    let geometry = state.geometry.borrow();
    let first = geometry.runs.iter().find(|run| run.side == 0 && run.line == 0).unwrap();
    assert!(first.bounds.size.height > unwrapped);
    let old = geometry.runs.iter().find(|run| run.side == 0 && run.line == 1).unwrap();
    let new = geometry.runs.iter().find(|run| run.side == 1 && run.line == 1).unwrap();
    assert_eq!(old.bounds.top(), new.bounds.top());
    assert_eq!(state.horizontal[0].offset().x, px(0.));
  });
  columns(&state, cx);
}

#[gpui_kit::test]
fn hunk_toolbar_does_not_follow_horizontal_scroll_and_navigation_uses_shared_rows(cx: &mut TestAppContext) {
  let patch = format!(
    "diff --git a/example.txt b/example.txt\n--- a/example.txt\n+++ b/example.txt\n@@ -1,2 +1,2 @@\n-{}\n+{}\n context\n@@ -200,2 +200,2 @@\n-old\n+new\n tail\n",
    "a".repeat(1200),
    "b".repeat(1200)
  );
  let (state, view, cx) = host(&patch, 480., cx);
  let clicks = Rc::new(Cell::new(0));
  view.update(cx, |host, cx| {
    let clicks = clicks.clone();
    host.actions = Some(Rc::new(move |_, _, _| {
      let clicks = clicks.clone();
      div()
        .id("hunk-action-test")
        .h(px(24.))
        .w(px(40.))
        .child("Stage")
        .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
        .into_any_element()
    }));
    cx.notify();
  });
  draw(cx);
  wheel(0, -1000., 0., &state, cx);
  let position = cx.update(|_, cx| {
    let state = state.read(cx);
    let geometry = state.geometry.borrow();
    let first = geometry.runs.iter().find(|run| run.line == 0).unwrap();
    point(geometry.bounds.left() + px(24.), first.bounds.bottom() + px(24.))
  });
  cx.simulate_event(MouseDownEvent {
    position,
    button: MouseButton::Left,
    click_count: 1,
    first_mouse: false,
    modifiers: Default::default(),
  });
  cx.simulate_event(MouseUpEvent {
    position,
    button: MouseButton::Left,
    click_count: 1,
    modifiers: Default::default(),
  });
  draw(cx);
  assert_eq!(clicks.get(), 1, "hunk action must remain in the fixed viewport");
  state.update(cx, |state, cx| {
    state.list.scroll_to(ListOffset {
      item_ix: state.document.as_ref().unwrap().changes[0],
      offset_in_item: px(0.),
    });
    state.move_change(true, cx);
    assert_eq!(
      state.list.logical_scroll_top().item_ix,
      state.document.as_ref().unwrap().changes[1]
    );
    state.move_change(false, cx);
    assert_eq!(
      state.list.logical_scroll_top().item_ix,
      state.document.as_ref().unwrap().changes[0]
    );
  });
}

#[gpui_kit::test]
fn double_and_triple_click_keep_native_word_and_line_selection(cx: &mut TestAppContext) {
  let (state, _, cx) = host(&patch(&["old text".into()], &["alpha\tbeta".into()]), 480., cx);
  for (click_count, expected) in [(2, "beta"), (3, "alpha\tbeta")] {
    let position = caret(&state, 1, 0, 8, cx);
    cx.simulate_event(MouseDownEvent {
      position,
      button: MouseButton::Left,
      click_count,
      first_mouse: false,
      modifiers: Default::default(),
    });
    cx.simulate_event(MouseUpEvent {
      position,
      button: MouseButton::Left,
      click_count,
      modifiers: Default::default(),
    });
    draw(cx);
    cx.update(|window, cx| assert_eq!(TextSelection::selected_text(window, cx), expected));
  }
}
