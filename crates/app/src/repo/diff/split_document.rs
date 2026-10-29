use std::ops::Range;

use deathpush_core::config::settings::LineDiffType;
use git2::{Diff, DiffLineType, Patch};
use gpui_kit::component::Rope;
use gpui_kit::component::highlighter::{HighlightTheme, SyntaxHighlighter};
use gpui_kit::*;
use similar::{ChangeTag, TextDiff};

const MAX_PRESENTATION_LINE_BYTES: usize = 1000;
const MAX_DISPLAY_CHUNK_BYTES: usize = 1024;
const MIN_INLINE_SIMILARITY: f32 = 0.25;

/// Only the source supplied by the patch, concatenated in hunk order on each side.
/// Missing context is represented by hunk metadata, never invented source bytes.
#[derive(Debug)]
pub(super) struct Document {
  pub path: SharedString,
  pub lines: [Vec<Line>; 2],
  pub source: [String; 2],
  pub rows: Vec<Row>,
  pub hunks: Vec<Hunk>,
  pub changes: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Row {
  Separator(usize),
  Code { lines: [Option<usize>; 2], changed: bool },
}

#[derive(Debug)]
pub(super) struct Hunk {
  pub label: SharedString,
  pub hidden_before: usize,
  /// The first code row, immediately after this hunk's separator.
  pub first_row: usize,
  pub old_start: usize,
  pub old_lines: usize,
  pub new_start: usize,
  pub new_lines: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Line {
  pub number: usize,
  pub source: Range<usize>,
  pub content_end: usize,
  pub display: SharedString,
  pub chunks: Vec<(Range<usize>, SharedString)>,
  tabs: Vec<DisplayTab>,
}

#[derive(Clone, Debug)]
struct DisplayTab {
  source: usize,
  display: usize,
  width: usize,
}

impl Line {
  fn new(number: usize, source_start: usize, raw: &str) -> Self {
    let content = match raw.strip_suffix('\n') {
      Some(content) => content.strip_suffix('\r').unwrap_or(content),
      None => raw,
    };
    let mut tabs = Vec::new();
    let display: SharedString = if content.contains('\t') {
      let mut display = String::with_capacity(content.len());
      let mut column = 0;
      for (source, ch) in content.char_indices() {
        if ch == '\t' {
          let width = 4 - column % 4;
          tabs.push(DisplayTab {
            source,
            display: display.len(),
            width,
          });
          display.push_str(&"    "[..width]);
          column += width;
        } else {
          display.push(ch);
          column += 1;
        }
      }
      display.into()
    } else {
      content.to_owned().into()
    };
    let chunks = if display.len() <= MAX_DISPLAY_CHUNK_BYTES {
      vec![(0..display.len(), display.clone())]
    } else {
      let mut chunks = Vec::with_capacity(display.len().div_ceil(MAX_DISPLAY_CHUNK_BYTES));
      let mut start = 0;
      while start < display.len() {
        let mut end = (start + MAX_DISPLAY_CHUNK_BYTES).min(display.len());
        while !display.is_char_boundary(end) {
          end -= 1;
        }
        chunks.push((start..end, SharedString::from(display[start..end].to_owned())));
        start = end;
      }
      chunks
    };
    Self {
      number,
      source: source_start..source_start + raw.len(),
      content_end: source_start + content.len(),
      display,
      chunks,
      tabs,
    }
  }

  /// Relative source bytes; positions inside an expanded tab map to its start.
  pub(super) fn source_offset(&self, display_byte: usize) -> usize {
    let mut display_byte = display_byte.min(self.display.len());
    while !self.display.is_char_boundary(display_byte) {
      display_byte -= 1;
    }
    let preceding = self.tabs.partition_point(|tab| tab.display <= display_byte);
    let Some(tab) = preceding.checked_sub(1).map(|index| &self.tabs[index]) else {
      return display_byte;
    };
    if display_byte < tab.display + tab.width {
      tab.source
    } else {
      display_byte - (tab.display + tab.width - tab.source - 1)
    }
  }

  /// Relative display bytes; line-ending offsets clamp to the visible content end.
  pub(super) fn display_offset(&self, source_byte: usize) -> usize {
    let source_byte = source_byte.min(self.content_end - self.source.start);
    let preceding = self.tabs.partition_point(|tab| tab.source < source_byte);
    let expansion = preceding.checked_sub(1).map_or(0, |index| {
      let tab = &self.tabs[index];
      tab.display + tab.width - tab.source - 1
    });
    let mut display_byte = source_byte + expansion;
    while !self.display.is_char_boundary(display_byte) {
      display_byte -= 1;
    }
    display_byte
  }
}

impl Document {
  pub(super) fn parse(path: &str, patch: &str) -> Result<Self, git2::Error> {
    let diff = Diff::from_buffer(patch.as_bytes())?;
    if diff.deltas().len() != 1 {
      return Err(git2::Error::from_str("A split document requires exactly one file"));
    }
    let patch =
      Patch::from_diff(&diff, 0)?.ok_or_else(|| git2::Error::from_str("A split document requires a text patch"))?;
    let mut document = Self {
      path: path.to_owned().into(),
      lines: Default::default(),
      source: Default::default(),
      rows: Vec::new(),
      hunks: Vec::with_capacity(patch.num_hunks()),
      changes: Vec::new(),
    };
    let mut previous_old_end = 1;
    for hunk_index in 0..patch.num_hunks() {
      let (hunk, line_count) = patch.hunk(hunk_index)?;
      let old_start = hunk.old_start() as usize;
      let old_lines = hunk.old_lines() as usize;
      let new_start = hunk.new_start() as usize;
      let new_lines = hunk.new_lines() as usize;
      // A zero-count side names the line before the insertion point.
      let first_old = old_start + usize::from(old_lines == 0);
      let header = String::from_utf8_lossy(hunk.header());
      document.hunks.push(Hunk {
        label: header.trim_end_matches(['\r', '\n']).to_owned().into(),
        hidden_before: first_old.saturating_sub(previous_old_end),
        first_row: document.rows.len() + 1,
        old_start,
        old_lines,
        new_start,
        new_lines,
      });
      document.rows.push(Row::Separator(hunk_index));
      previous_old_end = first_old + old_lines;
      let mut changed_start = None;
      let mut lines = (0..line_count)
        .map(|line_index| patch.line_in_hunk(hunk_index, line_index))
        .peekable();
      while let Some(line) = lines.next() {
        let line = line?;
        let origin = line.origin_value();
        if !matches!(
          origin,
          DiffLineType::Context | DiffLineType::Addition | DiffLineType::Deletion
        ) {
          continue;
        }
        let raw = std::str::from_utf8(line.content())
          .map_err(|_| git2::Error::from_str("A split document requires UTF-8 source"))?;
        // libgit2 exposes the marker as a separate line and retains the patch's
        // framing LF on the preceding content. A trailing CR without that LF is data.
        let no_newline = lines.peek().is_some_and(|next| {
          next.as_ref().is_ok_and(|next| {
            matches!(
              next.origin_value(),
              DiffLineType::ContextEOFNL | DiffLineType::AddEOFNL | DiffLineType::DeleteEOFNL
            )
          })
        });
        let raw = if no_newline {
          raw.strip_suffix('\n').unwrap_or(raw)
        } else {
          raw
        };
        let number = |value: Option<u32>| {
          value
            .map(|value| value as usize)
            .ok_or_else(|| git2::Error::from_str("Missing source line number in patch"))
        };
        match origin {
          DiffLineType::Context => {
            document.flush_changes(&mut changed_start);
            let original = document.push_line(0, number(line.old_lineno())?, raw);
            // Context shares its immutable display strings, even when the two
            // source buffers have different offsets and line numbers.
            let mut modified = document.lines[0][original].clone();
            let modified_start = document.source[1].len();
            modified.number = number(line.new_lineno())?;
            modified.content_end = modified_start + modified.content_end - modified.source.start;
            modified.source = modified_start..modified_start + raw.len();
            let modified_index = document.lines[1].len();
            document.source[1].push_str(raw);
            document.lines[1].push(modified);
            document.rows.push(Row::Code {
              lines: [Some(original), Some(modified_index)],
              changed: false,
            });
          }
          DiffLineType::Addition | DiffLineType::Deletion => {
            changed_start.get_or_insert_with(|| document.lines.each_ref().map(Vec::len));
            let (side, line_number) = if origin == DiffLineType::Deletion {
              (0, number(line.old_lineno())?)
            } else {
              (1, number(line.new_lineno())?)
            };
            document.push_line(side, line_number, raw);
          }
          _ => unreachable!(),
        }
      }
      document.flush_changes(&mut changed_start);
    }
    Ok(document)
  }

  fn push_line(&mut self, side: usize, number: usize, raw: &str) -> usize {
    let index = self.lines[side].len();
    self.lines[side].push(Line::new(number, self.source[side].len(), raw));
    self.source[side].push_str(raw);
    index
  }

  fn flush_changes(&mut self, start: &mut Option<[usize; 2]>) {
    let Some(start) = start.take() else { return };
    let lengths = [self.lines[0].len() - start[0], self.lines[1].len() - start[1]];
    self.changes.push(self.rows.len());
    for offset in 0..lengths[0].max(lengths[1]) {
      self.rows.push(Row::Code {
        lines: [
          (offset < lengths[0]).then_some(start[0] + offset),
          (offset < lengths[1]).then_some(start[1] + offset),
        ],
        changed: true,
      });
    }
  }
}

type StyledRuns = Vec<(Range<usize>, HighlightStyle)>;

#[derive(Default)]
pub(super) struct Presentation {
  pub lines: [Vec<StyledRuns>; 2],
  pub inline: [Vec<Vec<Range<usize>>>; 2],
}

impl Presentation {
  /// Blocking preparation, independent of GPUI entities and window state.
  pub(super) fn prepare(document: &Document, language: &str, unit: LineDiffType, theme: &HighlightTheme) -> Self {
    let mut presentation = Self {
      lines: document.lines.each_ref().map(|lines| vec![Vec::new(); lines.len()]),
      inline: document.lines.each_ref().map(|lines| vec![Vec::new(); lines.len()]),
    };
    if !matches!(language, "" | "text" | "plain") {
      let mut starts = [0, 0];
      for hunk in &document.hunks {
        for (side, count) in [hunk.old_lines, hunk.new_lines].into_iter().enumerate() {
          let end = starts[side] + count;
          syntax_lines(
            document,
            side,
            starts[side]..end,
            language,
            theme,
            &mut presentation.lines[side],
          );
          starts[side] = end;
        }
      }
    }
    if unit != LineDiffType::None {
      for row in &document.rows {
        let Row::Code {
          lines: [Some(old), Some(new)],
          changed: true,
        } = row
        else {
          continue;
        };
        let original = &document.lines[0][*old];
        let modified = &document.lines[1][*new];
        let runs = changed_runs(
          &document.source[0][original.source.start..original.content_end],
          &document.source[1][modified.source.start..modified.content_end],
          unit,
        );
        for (side, (index, mut runs)) in [*old, *new].into_iter().zip(runs).enumerate() {
          let line = &document.lines[side][index];
          for range in &mut runs {
            *range = line.display_offset(range.start)..line.display_offset(range.end);
          }
          presentation.inline[side][index] = runs;
        }
      }
    }
    presentation
  }
}

fn syntax_lines(
  document: &Document,
  side: usize,
  range: Range<usize>,
  language: &str,
  theme: &HighlightTheme,
  output: &mut [Vec<(Range<usize>, HighlightStyle)>],
) {
  let lines = &document.lines[side];
  let mut next = range.start;
  while next < range.end {
    if lines[next].content_end - lines[next].source.start > MAX_PRESENTATION_LINE_BYTES {
      next += 1;
      continue;
    }
    let first = next;
    next += 1;
    while next < range.end && lines[next].content_end - lines[next].source.start <= MAX_PRESENTATION_LINE_BYTES {
      next += 1;
    }
    // Never feed an oversized line to the parser. The remaining contiguous
    // fragments keep multiline context, but never bridge unavailable hunk text.
    let base = lines[first].source.start;
    let text = &document.source[side][base..lines[next - 1].source.end];
    if text.is_empty() {
      continue;
    }
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &Rope::from_str(text), None);
    let mut line_index = first;
    // styles resolves overlapping captures in native precedence order once.
    for (capture, style) in highlighter.styles(&(0..text.len()), theme) {
      if style == HighlightStyle::default() {
        continue;
      }
      let capture = base + capture.start..base + capture.end;
      while line_index < next && lines[line_index].source.end <= capture.start {
        line_index += 1;
      }
      for index in line_index..next {
        let line = &lines[index];
        if line.source.start >= capture.end {
          break;
        }
        let start = capture.start.max(line.source.start);
        let end = capture.end.min(line.content_end);
        if start < end {
          output[index].push((
            line.display_offset(start - line.source.start)..line.display_offset(end - line.source.start),
            style,
          ));
        }
      }
    }
  }
}

fn changed_runs(old: &str, new: &str, unit: LineDiffType) -> [Vec<Range<usize>>; 2] {
  if old == new || old.len() > MAX_PRESENTATION_LINE_BYTES || new.len() > MAX_PRESENTATION_LINE_BYTES {
    return Default::default();
  }
  let diff = match unit {
    LineDiffType::Word => TextDiff::from_unicode_words(old, new),
    LineDiffType::Char => TextDiff::from_graphemes(old, new),
    LineDiffType::None => return Default::default(),
  };
  if diff.ratio() < MIN_INLINE_SIMILARITY {
    return Default::default();
  }
  let mut runs: [Vec<Range<usize>>; 2] = Default::default();
  let mut offsets = [0, 0];
  for change in diff.iter_all_changes() {
    let length = change.value().len();
    let side = match change.tag() {
      ChangeTag::Equal => {
        offsets[0] += length;
        offsets[1] += length;
        continue;
      }
      ChangeTag::Delete => 0,
      ChangeTag::Insert => 1,
    };
    let range = offsets[side]..offsets[side] + length;
    let text = if side == 0 { old } else { new };
    if let Some(last) = runs[side].last_mut()
      && (last.end == range.start || unit == LineDiffType::Word && text[last.end..range.start].trim().is_empty())
    {
      last.end = range.end;
    } else {
      runs[side].push(range);
    }
    offsets[side] += length;
  }
  runs
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  const HEADER: &str = "diff --git a/example.rs b/example.rs\n--- a/example.rs\n+++ b/example.rs\n";

  fn parse(body: &str) -> Document {
    Document::parse("example.rs", &format!("{HEADER}{body}")).unwrap()
  }

  fn presentation(document: &Document, unit: LineDiffType) -> Presentation {
    Presentation::prepare(document, "text", unit, &HighlightTheme::default_dark())
  }

  #[test]
  fn pairs_changes_by_run_and_preserves_hunk_coordinates() {
    let document = parse(
      "@@ -3,5 +3,5 @@ first\n shared\n-old α\n-old β\n+new α\n between\n+extra\n tail\n@@ -12,1 +12,2 @@ second\n keep\n+later\n",
    );
    assert_eq!(document.path.as_str(), "example.rs");
    assert_eq!(document.changes, [2, 5, 9]);
    assert_eq!(
      document.rows,
      [
        Row::Separator(0),
        Row::Code {
          lines: [Some(0), Some(0)],
          changed: false
        },
        Row::Code {
          lines: [Some(1), Some(1)],
          changed: true
        },
        Row::Code {
          lines: [Some(2), None],
          changed: true
        },
        Row::Code {
          lines: [Some(3), Some(2)],
          changed: false
        },
        Row::Code {
          lines: [None, Some(3)],
          changed: true
        },
        Row::Code {
          lines: [Some(4), Some(4)],
          changed: false
        },
        Row::Separator(1),
        Row::Code {
          lines: [Some(5), Some(5)],
          changed: false
        },
        Row::Code {
          lines: [None, Some(6)],
          changed: true
        },
      ],
    );
    let numbers = |side: usize| document.lines[side].iter().map(|line| line.number).collect::<Vec<_>>();
    assert_eq!(numbers(0), [3, 4, 5, 6, 7, 12]);
    assert_eq!(numbers(1), [3, 4, 5, 6, 7, 12, 13]);
    let coordinates = |hunk: &Hunk| {
      (
        hunk.first_row,
        hunk.hidden_before,
        hunk.old_start,
        hunk.old_lines,
        hunk.new_start,
        hunk.new_lines,
      )
    };
    assert_eq!(coordinates(&document.hunks[0]), (1, 2, 3, 5, 3, 5));
    assert_eq!(coordinates(&document.hunks[1]), (8, 4, 12, 1, 12, 2));
    assert_eq!(document.hunks[1].label.as_str(), "@@ -12,1 +12,2 @@ second");
    assert_eq!(document.source[0], "shared\nold α\nold β\nbetween\ntail\nkeep\n");
    assert_eq!(document.source[1], "shared\nnew α\nbetween\nextra\ntail\nkeep\nlater\n");
  }

  #[test]
  fn empty_hunk_sides_use_insertion_points_without_pairing_across_hunks() {
    let document = parse("@@ -5,0 +6,1 @@\n+insert\n@@ -9,1 +10,0 @@\n-delete\n");
    assert_eq!(document.changes, [1, 3]);
    assert_eq!(document.hunks[0].hidden_before, 5);
    assert_eq!(document.hunks[1].hidden_before, 3);
    assert_eq!(
      document.rows[1],
      Row::Code {
        lines: [None, Some(0)],
        changed: true
      }
    );
    assert_eq!(
      document.rows[3],
      Row::Code {
        lines: [Some(0), None],
        changed: true
      }
    );
    assert_eq!(document.lines[0][0].number, 9);
    assert_eq!(document.lines[1][0].number, 6);
  }

  #[test]
  fn rejects_invalid_or_multiple_file_patches() {
    assert!(Document::parse("example.rs", "not a patch").is_err());
    let patch = format!("{HEADER}@@ -1 +1 @@\n-old\n+new\n");
    let other = patch.replace("example.rs", "other.rs");
    assert!(Document::parse("example.rs", &format!("{patch}{other}")).is_err());
  }

  #[test]
  fn source_ranges_preserve_crlf_and_context_without_a_final_newline() {
    let document = parse("@@ -1,3 +1,3 @@\n same\r\n-old\r\n+new\r\n last\n\\ No newline at end of file\n");
    assert_eq!(document.source[0], "same\r\nold\r\nlast");
    assert_eq!(document.source[1], "same\r\nnew\r\nlast");
    for side in 0..2 {
      let raw = document.lines[side]
        .iter()
        .map(|line| &document.source[side][line.source.clone()])
        .collect::<String>();
      assert_eq!(raw, document.source[side]);
      assert_eq!(document.lines[side][0].display.as_str(), "same");
      assert_eq!(document.lines[side][0].content_end, 4);
      assert_eq!(document.lines[side][0].source, 0..6);
      assert_eq!(document.lines[side][2].display.as_str(), "last");
    }
  }

  #[test]
  fn newline_markers_apply_to_the_preceding_side_and_preserve_bare_carriage_returns() {
    for (body, old, new) in [
      ("-old\n\\ No newline at end of file\n+new\n", "old", "new\n"),
      ("-old\n+new\n\\ No newline at end of file\n", "old\n", "new"),
      (
        "-old\r\n\\ No newline at end of file\n+new\r\n\\ No newline at end of file\n",
        "old\r",
        "new\r",
      ),
    ] {
      let document = parse(&format!("@@ -1 +1 @@\n{body}"));
      assert_eq!(document.source, [old, new]);
      assert_eq!(document.lines[0][0].source, 0..old.len());
      assert_eq!(document.lines[1][0].source, 0..new.len());
    }
  }

  #[test]
  fn utf8_tab_offsets_round_trip_at_every_source_character_boundary() {
    for raw in ["é\t猫\t🙂\r\n", "\t\té\t", "中é", "", "trailing\r"] {
      let line = Line::new(8, 37, raw);
      let content = &raw[..line.content_end - line.source.start];
      for source in (0..=content.len()).filter(|offset| content.is_char_boundary(*offset)) {
        let display = line.display_offset(source);
        assert!(line.display.is_char_boundary(display));
        assert_eq!(line.source_offset(display), source);
      }
      for display in (0..=line.display.len()).filter(|offset| line.display.is_char_boundary(*offset)) {
        let source = line.source_offset(display);
        assert!(content.is_char_boundary(source));
        assert!(line.display_offset(source) <= display);
      }
      assert_eq!(line.source_offset(usize::MAX), content.len());
      assert_eq!(line.display_offset(usize::MAX), line.display.len());
    }
    let line = Line::new(1, 0, "é\t猫\t🙂\r\n");
    assert_eq!(line.display.as_str(), "é   猫   🙂");
    assert_eq!(line.source_offset(3), 2);
    assert_eq!(line.source_offset(5), 3);
    assert_eq!(line.source_offset(1), 0);
    assert_eq!(line.display_offset(4), 5);
  }

  #[test]
  fn display_chunks_are_bounded_utf8_and_cover_the_line_exactly() {
    let line = Line::new(1, 0, &format!("{}\té\r\n", "猫".repeat(900)));
    let mut end = 0;
    for (range, text) in &line.chunks {
      assert_eq!(range.start, end);
      assert!(range.len() <= MAX_DISPLAY_CHUNK_BYTES);
      assert_eq!(text.as_str(), &line.display[range.clone()]);
      end = range.end;
    }
    assert_eq!(end, line.display.len());
    let empty = Line::new(1, 0, "\r\n");
    assert_eq!(empty.chunks.len(), 1);
    assert_eq!(empty.chunks[0].0, 0..0);
    assert!(empty.chunks[0].1.is_empty());
  }

  #[test]
  fn inline_settings_use_words_or_graphemes_and_map_to_expanded_tabs() {
    let document = parse("@@ -1 +1 @@\n-\tlet colour = 1;\n+\tlet color = 1;\n");
    let words = presentation(&document, LineDiffType::Word);
    assert_eq!(words.inline[0][0], vec![8..14]);
    assert_eq!(words.inline[1][0], vec![8..13]);
    let chars = presentation(&document, LineDiffType::Char);
    assert_eq!(chars.inline[0][0], vec![12..13]);
    assert!(chars.inline[1][0].is_empty());
    let none = presentation(&document, LineDiffType::None);
    assert!(none.inline.iter().flatten().all(Vec::is_empty));
    assert!(none.lines.iter().flatten().all(Vec::is_empty));

    let old = "let emoji = 👨‍👩‍👧;";
    let new = "let emoji = 👨‍👩‍👦;";
    let runs = changed_runs(old, new, LineDiffType::Char);
    assert_eq!(&old[runs[0][0].clone()], "👨‍👩‍👧");
    assert_eq!(&new[runs[1][0].clone()], "👨‍👩‍👦");
    assert_eq!(changed_runs("a b c", "a x y c", LineDiffType::Word)[1], vec![2..5]);
    assert!(
      changed_runs("alpha", "zzzzzzzzz", LineDiffType::Word)
        .iter()
        .all(Vec::is_empty)
    );
  }

  #[test]
  fn inline_limits_count_source_bytes_and_only_compare_paired_changes() {
    let prefix = "a".repeat(994);
    let runs = changed_runs(
      &format!("{prefix}colour"),
      &format!("{prefix}color"),
      LineDiffType::Char,
    );
    assert_eq!(runs[0], vec![998..999]);
    assert!(
      changed_runs(
        &format!("{prefix}xcolour"),
        &format!("{prefix}xcolor"),
        LineDiffType::Char
      )
      .iter()
      .all(Vec::is_empty)
    );
    let tabs = "\t".repeat(250);
    let document = parse(&format!("@@ -1 +1 @@\n-{tabs}old\n+{tabs}new\n"));
    assert_eq!(
      presentation(&document, LineDiffType::Char).inline[0][0],
      vec![1000..1003]
    );
    let unpaired = parse("@@ -1,2 +1,2 @@\n-let old = 1;\n same\n+let new = 1;\n");
    assert!(
      presentation(&unpaired, LineDiffType::Word)
        .inline
        .iter()
        .flatten()
        .all(Vec::is_empty)
    );
  }

  #[test]
  fn syntax_is_nonoverlapping_in_display_coordinates_and_isolated_per_hunk() {
    let document = parse(
      "@@ -1,2 +1,2 @@\n-\tlet old = 1;\n+\tlet new = 2;\n /* open\n@@ -10,2 +10,2 @@\n close */\n-\tfn old() {}\n+\tfn new() {}\n",
    );
    let theme = HighlightTheme::default_dark();
    let prepared = Presentation::prepare(&document, "rust", LineDiffType::None, &theme);
    let keyword = theme.style.syntax.style("keyword").unwrap();
    assert!(
      prepared.lines[1][0]
        .iter()
        .any(|(range, style)| *range == (4..7) && *style == keyword)
    );
    let function_line = &document.lines[1][3];
    assert!(
      prepared.lines[1][3]
        .iter()
        .any(|(range, _)| &function_line.display[range.clone()] == "new")
    );
    for (side, per_line) in prepared.lines.iter().enumerate() {
      for (index, spans) in per_line.iter().enumerate() {
        let text = &document.lines[side][index].display;
        let mut end = 0;
        for (range, _) in spans {
          assert!(end <= range.start && range.start < range.end && range.end <= text.len());
          assert!(text.is_char_boundary(range.start) && text.is_char_boundary(range.end));
          end = range.end;
        }
      }
    }
  }

  #[test]
  fn oversized_source_lines_do_not_enter_syntax_preparation() {
    let long = format!("/* {}", "x".repeat(MAX_PRESENTATION_LINE_BYTES));
    let document = parse(&format!("@@ -1,2 +1,2 @@\n {long}\n-let old = 1;\n+let new = 2;\n"));
    let theme = HighlightTheme::default_dark();
    let prepared = Presentation::prepare(&document, "rust", LineDiffType::None, &theme);
    for side in 0..2 {
      assert!(prepared.lines[side][0].is_empty());
      assert!(!prepared.lines[side][1].is_empty());
    }
  }

  #[test]
  fn documents_and_presentations_can_be_shared_with_background_workers() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Document>();
    assert_send_sync::<Presentation>();
  }
}
