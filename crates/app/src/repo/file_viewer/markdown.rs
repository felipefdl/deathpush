//! Rendered markdown for the file viewer. The editor keeps the raw source; this is the other half
//! of the raw/preview toggle.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use gpui_kit::component::text::{SelectionFormat, TextView, TextViewState};
use gpui_kit::*;

use super::view::FileViewer;
use crate::markdown_assets::{self, AssetGrants};

pub fn is_markdown(language: Option<&str>) -> bool {
  matches!(language, Some("markdown") | Some("mdx"))
}

/// The file a preview renders: its own path and the repository it may read images from.
#[derive(Clone, Copy)]
pub struct DocumentPaths<'a> {
  pub file: &'a Path,
  pub root: &'a Path,
}

pub struct MarkdownPreview {
  id: u64,
  state: Option<Entity<TextViewState>>,
  grants: Option<Arc<AssetGrants>>,
  /// Cheap guard so the document is not reparsed on every frame.
  applied: Option<(usize, u64)>,
}

impl Default for MarkdownPreview {
  fn default() -> Self {
    Self::new()
  }
}

impl MarkdownPreview {
  pub fn new() -> Self {
    Self {
      id: AssetGrants::next_id(),
      state: None,
      grants: None,
      applied: None,
    }
  }

  /// Feed the preview the current editor text, which may contain unsaved edits.
  pub fn set_source(&mut self, source: &str, document: Option<DocumentPaths<'_>>, cx: &mut Context<FileViewer>) {
    let fingerprint = (source.len(), hash(source));
    if self.applied == Some(fingerprint) && self.state.is_some() {
      return;
    }
    self.applied = Some(fingerprint);

    let rendered = match document {
      // Image URLs become absolute and granted, so this document's pictures cannot be confused
      // with another document's.
      Some(paths) => {
        let rewritten = markdown_assets::rewrite_images(source, paths.file, paths.root);
        self.grants(cx).grant(self.id, rewritten.files);
        rewritten.source
      }
      None => source.to_string(),
    };

    match self.state.as_ref() {
      Some(state) => state.update(cx, |state, cx| state.set_text(&rendered, cx)),
      None => self.state = Some(cx.new(|cx| TextViewState::markdown(&rendered, cx))),
    }
  }

  pub fn render(&self) -> AnyElement {
    let Some(state) = self.state.as_ref() else {
      return div().size_full().into_any_element();
    };
    div()
      .flex_1()
      .min_h_0()
      .px_4()
      .py_2()
      .child(
        // `scrollable` paints its own scrollbar and needs a fixed-height parent, which the pane is.
        TextView::new(state)
          .selectable(true)
          .scrollable(true)
          // Copying out of a preview should yield markdown, not flattened text.
          .selection_format(SelectionFormat::Source)
          .on_link_click(|url, _event, _window, cx| {
            if markdown_assets::is_safe_link(url) {
              cx.open_url(url);
            }
          }),
      )
      .into_any_element()
  }

  /// Drop the parsed document and everything it was allowed to read. Called from render, so an
  /// already empty preview costs nothing.
  pub fn reset(&mut self) {
    if self.state.is_none() && self.applied.is_none() {
      return;
    }
    self.state = None;
    self.applied = None;
    if let Some(grants) = self.grants.as_ref() {
      grants.revoke(self.id);
    }
  }

  fn grants(&mut self, cx: &App) -> Arc<AssetGrants> {
    self
      .grants
      .get_or_insert_with(|| markdown_assets::grants(cx).unwrap_or_default())
      .clone()
  }
}

impl Drop for MarkdownPreview {
  fn drop(&mut self) {
    if let Some(grants) = self.grants.as_ref() {
      grants.revoke(self.id);
    }
  }
}

fn hash(source: &str) -> u64 {
  let mut hasher = DefaultHasher::new();
  source.hash(&mut hasher);
  hasher.finish()
}
