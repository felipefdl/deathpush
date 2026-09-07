use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use deathpush_core::config::settings::{MONO_FONT_STACK, WordWrap};
use gpui_kit::component::input::{Editor, EditorState, InputEvent, Position, TabSize};
use gpui_kit::*;

use super::autosave::{AUTOSAVE_MS, SaveState, SaveToken, should_complete_save, token_still_valid};
use super::header;
use super::markdown::{DocumentPaths, MarkdownPreview, is_markdown};
use super::pdf::{PdfPane, PdfRequest, to_render_image};
use super::states::{self, ImageLoad, ViewerKind, classify};
use crate::config::AppConfig;
use crate::repo::diff::highlight::grammar_name;
use crate::repo::image_load::prepare_path;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::{RepoEvent, RepoModel};
use crate::theme::ActivePalette;

pub struct FileViewer {
  repo: Entity<RepoModel>,
  #[allow(dead_code)]
  layout: Entity<LayoutModel>,
  editor: Entity<EditorState>,
  save: SaveState,
  save_token: Option<SaveToken>,
  /// `OpenFile::load_id` of the document on screen. It survives a rename and changes on a new
  /// open, which is what tells a moved file apart from a different file.
  loaded_id: Option<u64>,
  loaded_path: Option<String>,
  loaded_hash: Option<String>,
  loaded_language: Option<String>,
  image: ImageLoad,
  image_generation: u64,
  pdf: PdfPane,
  markdown: MarkdownPreview,
  preview: bool,
  last_cursor_line: Option<usize>,
  window_handle: AnyWindowHandle,
  focus_handle: FocusHandle,
  editor_input_sub: Option<Subscription>,
  editor_cursor_sub: Option<Subscription>,
}

/// The open document as the viewer needs it for a sync pass: identity and shape, no payload.
struct Incoming {
  id: u64,
  kind: ViewerKind,
  path: String,
  pending_line: Option<usize>,
  /// `None` while the read is still in flight.
  hash: Option<String>,
  language: Option<String>,
}

impl FileViewer {
  pub fn new(
    repo: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.subscribe(&repo, |this, _, event: &RepoEvent, cx| match event {
      RepoEvent::Saved { path, hash, generation } => this.on_saved(path, hash, *generation, cx),
      RepoEvent::Changed => cx.notify(),
      RepoEvent::Error(_) => cx.notify(),
    })
    .detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    cx.observe_global::<AppConfig>(|this, cx| {
      this.apply_editor_settings(cx);
      cx.notify();
    })
    .detach();
    let editor = Self::build_editor(None, window, cx);
    let mut this = Self {
      repo,
      layout,
      editor,
      save: SaveState {
        saved_hash: String::new(),
        dirty: false,
        generation: 0,
      },
      save_token: None,
      loaded_id: None,
      loaded_path: None,
      loaded_hash: None,
      loaded_language: None,
      image: ImageLoad::Pending,
      image_generation: 0,
      pdf: PdfPane::new(),
      markdown: MarkdownPreview::new(),
      preview: false,
      last_cursor_line: None,
      window_handle: window.window_handle(),
      focus_handle: cx.focus_handle(),
      editor_input_sub: None,
      editor_cursor_sub: None,
    };
    this.bind_editor(cx);
    this
  }

  pub(crate) fn reveal(&self, cx: &mut Context<Self>) {
    let Some(path) = self.open_path(cx) else {
      return;
    };
    self.repo.update(cx, |model, cx| model.reveal_in_file_manager(path, cx));
  }

  pub(crate) fn open_external(&self, cx: &mut Context<Self>) {
    let Some(path) = self.open_path(cx) else {
      return;
    };
    self.repo.update(cx, |model, cx| model.open_in_editor(path, cx));
  }

  fn open_path(&self, cx: &App) -> Option<String> {
    self
      .repo
      .read(cx)
      .state()
      .open_file
      .as_ref()
      .map(|open| open.path.clone())
  }

  fn build_editor(language: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Entity<EditorState> {
    let settings = &AppConfig::get(cx).settings;
    let line_number = settings.diff.show_line_numbers;
    let wrap = settings.editor.word_wrap == WordWrap::On;
    let tab = settings.editor.tab_size as usize;
    let grammar = language.and_then(grammar_name);
    cx.new(|cx| {
      let mut state = EditorState::new(window, cx)
        .line_number(line_number)
        .soft_wrap(wrap)
        .tab_size(TabSize {
          tab_size: tab,
          hard_tabs: false,
        });
      if let Some(name) = grammar {
        state = state.language(name);
      }
      state
    })
  }

  fn bind_editor(&mut self, cx: &mut Context<Self>) {
    self.editor_input_sub = Some(cx.subscribe(&self.editor, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let was_clean = !this.save.dirty;
        let generation = this.save.edited();
        this.save_token = this.loaded_path.as_ref().map(|path| SaveToken {
          path: path.clone(),
          generation,
        });
        if was_clean {
          this.repo.update(cx, |model, cx| model.mark_open_file_dirty(cx));
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
          cx.background_executor().timer(Duration::from_millis(AUTOSAVE_MS)).await;
          let _ = this.update(cx, |this, cx| this.flush_save(generation, cx));
        })
        .detach();
      }
    }));
    self.editor_cursor_sub = Some(cx.observe(&self.editor, |this, editor, cx| {
      let line = editor.read(cx).cursor_position().line as usize + 1;
      if this.last_cursor_line == Some(line) {
        return;
      }
      this.last_cursor_line = Some(line);
      let repo = this.repo.clone();
      let handle = this.window_handle;
      let _ = handle.update(cx, |_, window, cx| {
        repo.update(cx, |model, cx| model.set_cursor_line(Some(line), window, cx));
      });
    }));
  }

  fn apply_editor_settings(&self, cx: &mut Context<Self>) {
    let settings = &AppConfig::get(cx).settings;
    let line_number = settings.diff.show_line_numbers;
    let wrap = settings.editor.word_wrap == WordWrap::On;
    let tab = settings.editor.tab_size as usize;
    let editor = self.editor.clone();
    let handle = self.window_handle;
    let _ = handle.update(cx, |_, window, cx| {
      editor.update(cx, |state, cx| {
        state.set_line_number(line_number, window, cx);
        state.set_soft_wrap(wrap, window, cx);
        state.set_tab_size(
          TabSize {
            tab_size: tab,
            hard_tabs: false,
          },
          cx,
        );
      });
    });
  }

  fn flush_save(&mut self, generation: u64, cx: &mut Context<Self>) {
    let Some(token) = self
      .save_token
      .as_ref()
      .filter(|token| token.generation == generation)
      .cloned()
    else {
      return;
    };
    if !token_still_valid(&token, self.loaded_path.as_deref(), &self.save) {
      return;
    }
    let content = self.editor.read(cx).value().to_string();
    let expected = self.save.saved_hash.clone();
    self.repo.update(cx, |model, cx| {
      model.write_open_file(token.path, content, expected, generation, cx)
    });
  }

  fn on_saved(&mut self, path: &str, hash: &str, generation: u64, cx: &mut Context<Self>) {
    if !should_complete_save(
      self.loaded_path.as_deref(),
      path,
      self.save.generation,
      generation,
      self.save.dirty,
    ) {
      return;
    }
    self.save.saved(hash.to_string(), generation);
    self.loaded_hash = Some(hash.to_string());
    let repo = self.repo.clone();
    let handle = self.window_handle;
    let _ = handle.update(cx, |_, window, cx| {
      repo.update(cx, |model, cx| model.mark_open_file_saved(window, cx));
    });
    cx.notify();
  }

  fn rebuild_editor(&mut self, language: Option<&str>, window: &mut Window, cx: &mut Context<Self>) {
    if self.loaded_language.as_deref() == language && self.loaded_path.is_some() {
      return;
    }
    self.loaded_language = language.map(str::to_string);
    self.editor = Self::build_editor(language, window, cx);
    self.bind_editor(cx);
  }

  fn apply_pending_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
    if line == 0 {
      return;
    }
    self.editor.update(cx, |state, cx| {
      state.set_cursor_position(
        Position {
          line: (line - 1) as u32,
          character: 0,
        },
        window,
        cx,
      );
      state.focus(window, cx);
    });
    self.last_cursor_line = Some(line);
    self.repo.update(cx, |model, cx| {
      if let Some(open) = model.state_mut().open_file.as_mut() {
        open.pending_line = None;
      }
      model.set_cursor_line(Some(line), window, cx);
    });
  }

  fn reset_save(&mut self, hash: String) {
    self.save.saved_hash = hash;
    self.save.dirty = false;
    self.save_token = None;
  }

  /// What the model currently has open, without the payload: this runs on every frame, so the
  /// body and the raw bytes are only pulled once something actually has to be applied.
  fn incoming(&self, cx: &App) -> Option<Incoming> {
    let state = self.repo.read(cx).state();
    let open = state.open_file.as_ref()?;
    Some(Incoming {
      id: open.load_id,
      kind: classify(Some(open)),
      path: open.path.clone(),
      pending_line: open.pending_line,
      hash: open.content.as_ref().map(|content| content.content_hash.clone()),
      language: open.content.as_ref().and_then(|content| content.language.clone()),
    })
  }

  fn sync_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(open) = self.incoming(cx) else {
      if self.loaded_id.is_some() {
        self.close_document();
      }
      return;
    };

    let entering = self.loaded_id != Some(open.id);
    if entering {
      self.loaded_id = Some(open.id);
      self.loaded_path = Some(open.path.clone());
      self.loaded_hash = None;
      self.last_cursor_line = None;
      self.reset_save(String::new());
    } else if self.loaded_path.as_deref() != Some(open.path.as_str()) {
      // A rename keeps the same document: only the name moved, so the buffer and any pending
      // save follow it instead of being torn down.
      if let Some(token) = &mut self.save_token {
        token.path = open.path.clone();
      }
      self.loaded_path = Some(open.path.clone());
    }
    // Whatever this document is not, the viewer must stop holding.
    self.drop_payloads_except(open.kind);

    // No hash yet means the read is still in flight.
    let Some(hash) = open.hash else {
      return;
    };
    let fresh = entering || self.loaded_hash.as_deref() != Some(hash.as_str());
    match open.kind {
      ViewerKind::Empty | ViewerKind::Loading => {}
      ViewerKind::Image => {
        if fresh && let Some(bytes) = self.open_bytes(cx) {
          self.load_image(open.path.clone(), bytes, cx);
          self.loaded_hash = Some(hash.clone());
          self.reset_save(hash);
        }
      }
      ViewerKind::Pdf => {
        if let Some(request) = self.pdf.sync(&open.path, &hash) {
          self.request_pdf_page(request, cx);
        }
        if fresh {
          self.loaded_hash = Some(hash.clone());
          self.reset_save(hash);
        }
      }
      ViewerKind::Binary | ViewerKind::Large => {
        if fresh {
          self.loaded_hash = Some(hash.clone());
          self.reset_save(hash);
        }
      }
      ViewerKind::Text => {
        if entering || (!self.save.dirty && self.save.should_reload_external(&hash)) {
          let body = self.open_text(cx);
          self.rebuild_editor(open.language.as_deref(), window, cx);
          self.editor.update(cx, |state, cx| state.set_value(body, window, cx));
          self.loaded_hash = Some(hash.clone());
          self.reset_save(hash);
        }
        if !is_markdown(self.loaded_language.as_deref()) {
          self.preview = false;
          self.markdown.reset();
        }
        if let Some(line) = open.pending_line {
          self.apply_pending_line(line, window, cx);
        }
      }
    }
  }

  /// The text of the open file. Only called when the editor is about to be refilled.
  fn open_text(&self, cx: &App) -> String {
    self
      .repo
      .read(cx)
      .state()
      .open_file
      .as_ref()
      .and_then(|open| open.content.as_ref())
      .map(|content| content.content.clone())
      .unwrap_or_default()
  }

  fn open_bytes(&self, cx: &App) -> Option<Arc<[u8]>> {
    self
      .repo
      .read(cx)
      .state()
      .open_file
      .as_ref()
      .and_then(|open| open.content.as_ref())
      .and_then(|content| content.bytes.clone())
  }

  fn close_document(&mut self) {
    self.loaded_id = None;
    self.loaded_path = None;
    self.loaded_hash = None;
    self.loaded_language = None;
    self.last_cursor_line = None;
    self.reset_save(String::new());
    self.clear_image();
    self.pdf.reset();
    self.markdown.reset();
    self.preview = false;
  }

  /// Release the payloads that belong to a different kind of file. A pending decode is
  /// invalidated too, so a slow answer cannot paint itself over the next document.
  fn drop_payloads_except(&mut self, kind: ViewerKind) {
    if kind != ViewerKind::Image && !matches!(self.image, ImageLoad::Pending) {
      self.clear_image();
    }
    if kind != ViewerKind::Pdf {
      self.pdf.reset();
    }
    if kind != ViewerKind::Text {
      self.preview = false;
      self.markdown.reset();
    }
  }

  /// Forget the image on screen and any decode still running for it.
  fn clear_image(&mut self) {
    self.image_generation += 1;
    self.image = ImageLoad::Pending;
  }

  /// Decoding happens on the background executor: an oversized image is resized before it reaches
  /// the texture atlas, and that must never block a frame.
  fn load_image(&mut self, path: String, bytes: Arc<[u8]>, cx: &mut Context<Self>) {
    self.clear_image();
    let generation = self.image_generation;
    cx.spawn(async move |this, cx| {
      let prepared = cx.background_spawn(async move { prepare_path(&path, &bytes) }).await;
      let _ = this.update(cx, |this, cx| {
        if this.image_generation != generation {
          return;
        }
        this.image = match prepared {
          Some(prepared) => ImageLoad::Ready(prepared.source()),
          None => ImageLoad::Failed,
        };
        cx.notify();
      });
    })
    .detach();
  }

  fn request_pdf_page(&mut self, request: PdfRequest, cx: &mut Context<Self>) {
    let generation = request.generation;
    let task = self.repo.update(cx, |model, cx| {
      model.request_pdf_page(request.path, request.page, request.max_edge, cx)
    });
    cx.spawn(async move |this, cx| {
      let page = task.await;
      // The RGBA to BGRA swap is 16 MB of work; keep it off the frame.
      let converted = cx
        .background_spawn(async move { page.map(|page| (page.page, page.page_count, to_render_image(page))) })
        .await;
      let _ = this.update(cx, |this, cx| {
        this.pdf.apply_prepared(generation, converted);
        cx.notify();
      });
    })
    .detach();
  }

  pub(crate) fn pdf_go_to(&mut self, page: usize, cx: &mut Context<Self>) {
    if let Some(request) = self.pdf.go_to(page) {
      self.request_pdf_page(request, cx);
      cx.notify();
    }
  }

  /// The editor keeps its buffer and cursor while the preview is up, so toggling back lands on
  /// the same line the source was left at.
  pub(crate) fn toggle_markdown_preview(&mut self, cx: &mut Context<Self>) {
    self.preview = !self.preview;
    cx.notify();
  }

  fn editor_font(family: &str) -> SharedString {
    if family.is_empty() {
      MONO_FONT_STACK.into()
    } else {
      family.to_string().into()
    }
  }

  #[cfg(test)]
  pub(crate) fn model(&self) -> &Entity<RepoModel> {
    &self.repo
  }

  /// Where the open file lives and which repository bounds it, so a markdown preview can resolve
  /// its own images and nothing else.
  fn open_document(&self, cx: &App) -> Option<(PathBuf, PathBuf)> {
    let model = self.repo.read(cx);
    let root = model.root_path()?;
    let open = model.state().open_file.as_ref()?;
    Some((root.join(&open.path), root))
  }

  #[cfg(test)]
  pub(crate) fn editor_value(&self, cx: &App) -> String {
    self.editor.read(cx).value().to_string()
  }

  #[cfg(test)]
  pub(crate) fn image_state(&self) -> &ImageLoad {
    &self.image
  }

  #[cfg(test)]
  pub(crate) fn pdf_pane(&self) -> &PdfPane {
    &self.pdf
  }

  #[cfg(test)]
  pub(crate) fn preview_active(&self) -> bool {
    self.preview
  }
}

impl Render for FileViewer {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.sync_open_file(window, cx);
    let palette = cx.global::<ActivePalette>().0;
    let settings = &AppConfig::get(cx).settings;
    let font_family = Self::editor_font(&settings.editor.font_family);
    let font_size = settings.editor.font_size as f32;
    let line_height = settings.editor.line_height as f32;
    let (kind, path) = {
      let open = self.repo.read(cx).state().open_file.as_ref();
      (
        classify(open),
        open.map(|open| open.path.as_str()).unwrap_or("").to_string(),
      )
    };
    let weak = cx.weak_entity();
    let markdown = kind == ViewerKind::Text && is_markdown(self.loaded_language.as_deref());
    let preview = markdown && self.preview;
    let mut root = div()
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .flex_col()
      .bg(hsla_bg(&palette));
    if kind == ViewerKind::Empty {
      return root.child(states::render_empty(palette));
    }
    root = root.child(header::render_header(
      &path,
      header::HeaderState {
        dirty: self.save.dirty,
        kind,
        markdown,
        preview,
      },
      weak.clone(),
      palette,
      cx,
    ));
    if preview {
      // The preview mirrors the editor buffer, so unsaved edits show up immediately.
      let source = self.editor.read(cx).value().to_string();
      let document = self.open_document(cx);
      let paths = document.as_ref().map(|(file, root)| DocumentPaths {
        file,
        root: root.as_path(),
      });
      self.markdown.set_source(&source, paths, cx);
      return root.child(self.markdown.render());
    }
    match kind {
      ViewerKind::Empty => root,
      ViewerKind::Loading => root.child(div().flex_1().min_h_0()),
      ViewerKind::Image => root.child(states::render_image(&self.image, weak, palette)),
      ViewerKind::Pdf => root.child(self.pdf.render(palette, weak)),
      ViewerKind::Binary => root.child(states::render_binary(weak, palette)),
      ViewerKind::Large => root.child(states::render_large(weak, palette)),
      ViewerKind::Text => root.child(
        div().flex_1().min_h_0().child(
          Editor::new(&self.editor)
            .bordered(false)
            .font_family(font_family)
            .text_size(px(font_size))
            .line_height(px(line_height))
            .size_full(),
        ),
      ),
    }
  }
}

fn hsla_bg(palette: &deathpush_core::theme::UiPalette) -> Hsla {
  crate::theme::hsla(palette.background)
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  use deathpush_core::Core;
  use deathpush_core::session::types::{
    OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{FileContent, RepoOperationState, StatusPhase};
  use gpui_kit::TestAppContext;

  use super::super::pdf::PdfLoad;
  use crate::config::AppConfig;
  use crate::markdown_assets::{AssetGrants, MarkdownAssets};
  use crate::repo::image_load::MAX_IMAGE_EDGE;
  use crate::repo::layout_model::LayoutModel;
  use crate::repo::model::RepoModel;
  use crate::repo::state::OpenFile;

  fn snapshot(root: &str) -> SessionSnapshot {
    SessionSnapshot {
      session_generation: 1,
      session_revision: 1,
      status_generation: 1,
      status_revision: 1,
      repo: SessionRepo {
        root: root.into(),
        has_repository: true,
        head_branch: Some("main".into()),
        head_commit: Some("abc".into()),
        ahead: 0,
        behind: 0,
        operation_state: RepoOperationState::None,
        phase: StatusPhase::Settled,
      },
      groups: vec![],
      selection: SessionSelection::default(),
      scm: SessionScm::default(),
      actions: SessionActions {
        can_commit: false,
        commit_label: "Commit".into(),
        commit_destructive: false,
        can_stage_all: false,
        can_unstage_all: false,
        can_discard_all: false,
        discard_all_destructive: false,
        sync: SyncAction {
          enabled: false,
          kind: SyncKind::Fetch,
          destructive: false,
        },
        operation: OperationActions {
          continue_op: false,
          abort: false,
          skip: false,
          abort_destructive: false,
        },
      },
      last_commit: None,
      branches: vec![],
      stashes: vec![],
      tags: vec![],
      commit_log: vec![],
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  #[gpui_kit::test]
  fn injected_text_file_fills_the_editor(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let layout_dir = config_dir.path().to_path_buf();
    let root = layout_dir.to_string_lossy().into_owned();
    let body = "fn main() {}\n";
    let window = cx.add_window({
      let core = core.clone();
      let snapshot = snapshot(&root);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        FileViewer::new(model, layout, window, cx)
      }
    });
    window
      .update(cx, |viewer, window, cx| {
        viewer.model().update(cx, |model, cx| {
          model.state_mut().open_file = Some(OpenFile {
            path: "src/main.rs".into(),
            content: Some(FileContent {
              path: "src/main.rs".into(),
              content: body.into(),
              bytes: None,
              language: Some("rust".into()),
              file_type: "text".into(),
              content_hash: "h".into(),
            }),
            pending_line: None,
            load_id: 1,
            dirty: false,
          });
          cx.notify();
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |viewer, _, cx| {
        assert_eq!(
          classify(viewer.model().read(cx).state().open_file.as_ref()),
          ViewerKind::Text
        );
        assert_eq!(viewer.editor_value(cx), body);
      })
      .unwrap();

    crate::test_core::park_and_shutdown(cx, &core);
  }

  /// A PNG over the 5 MiB text budget and wider than the texture atlas allows: the pair of walls
  /// that used to make big screenshots unviewable.
  fn oversized_png() -> Vec<u8> {
    let mut buffer = image::RgbaImage::new(6000, 400);
    for (x, y, pixel) in buffer.enumerate_pixels_mut() {
      // Noise, so the encoder cannot compress the file back under the budget.
      let seed = x.wrapping_mul(2_654_435_761).wrapping_add(y.wrapping_mul(40_503));
      *pixel = image::Rgba([(seed >> 3) as u8, (seed >> 11) as u8, (seed >> 19) as u8, 255]);
    }
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(buffer)
      .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
      .expect("encode png");
    bytes
  }

  /// A two page PDF, each page holding one filled rectangle.
  fn two_page_pdf() -> Vec<u8> {
    let stream = "0 0 0 rg\n72 72 200 200 re\nf\n";
    let objects = [
      "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
      "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_string(),
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << >> >>".to_string(),
      format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
      "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 6 0 R /Resources << >> >>".to_string(),
      format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
    ];
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

  #[gpui_kit::test]
  fn images_pdfs_and_markdown_reach_their_own_surfaces(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let repo_dir = tempfile::TempDir::new().unwrap();
    let png = oversized_png();
    assert!(png.len() > 5 * 1024 * 1024, "fixture must exceed the text budget");
    std::fs::write(repo_dir.path().join("shot.png"), &png).unwrap();
    std::fs::write(repo_dir.path().join("manual.pdf"), two_page_pdf()).unwrap();
    std::fs::write(repo_dir.path().join("README.md"), "# Title\n\n![shot](shot.png)\n").unwrap();

    let grants = Arc::new(AssetGrants::default());
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      // What `main` installs alongside the http client, so a preview can register its images.
      cx.set_global(MarkdownAssets(grants.clone()));
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let root = repo_dir.path().to_string_lossy().into_owned();
    cx.executor().allow_parking();
    // Core's operations need its own tokio runtime; the test executor is not a reactor.
    core
      .runtime_handle()
      .block_on(core.session_intent(
        session,
        deathpush_core::session::types::Intent::OpenRepository { path: root.clone() },
      ))
      .expect("session opens the fixture directory");

    let window = cx.add_window({
      let core = core.clone();
      let snapshot = snapshot(&root);
      let layout_dir = config_dir.path().to_path_buf();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        FileViewer::new(model, layout, window, cx)
      }
    });
    let handle = AnyWindowHandle::from(window);

    open(cx, &window, handle, "shot.png", |viewer, _| {
      matches!(viewer.image_state(), ImageLoad::Ready(_) | ImageLoad::Failed)
    });
    window
      .update(cx, |viewer, _, cx| {
        assert_eq!(
          classify(viewer.model().read(cx).state().open_file.as_ref()),
          ViewerKind::Image,
          "an image past the text budget must stay an image"
        );
        match viewer.image_state() {
          ImageLoad::Ready(ImageSource::Render(image)) => {
            let size = image.size(0);
            assert!(u32::from(size.width) <= MAX_IMAGE_EDGE, "{size:?} overflows the atlas");
          }
          other => panic!("expected a downscaled image, got {}", describe(other)),
        }
      })
      .unwrap();

    open(cx, &window, handle, "manual.pdf", |viewer, _| {
      !matches!(viewer.pdf_pane().load(), PdfLoad::Pending)
    });
    window
      .update(cx, |viewer, _, cx| {
        assert_eq!(
          classify(viewer.model().read(cx).state().open_file.as_ref()),
          ViewerKind::Pdf
        );
        assert!(
          matches!(viewer.pdf_pane().load(), PdfLoad::Ready(_)),
          "the pdf page must rasterize, got {:?}",
          viewer.pdf_pane().load()
        );
        assert_eq!(viewer.pdf_pane().page_count(), 2);
        assert!(
          matches!(viewer.image_state(), ImageLoad::Pending),
          "opening a pdf must release the previous image"
        );
      })
      .unwrap();

    // The second page replaces the first, and the first document's answer cannot come back.
    window.update(cx, |viewer, _, cx| viewer.pdf_go_to(1, cx)).unwrap();
    settle(cx, &window, handle, |viewer, _| {
      matches!(viewer.pdf_pane().load(), PdfLoad::Ready(_))
    });

    open(cx, &window, handle, "README.md", |viewer, cx| {
      viewer.editor_value(cx).starts_with("# Title")
    });
    window
      .update(cx, |viewer, _, cx| {
        assert!(
          matches!(viewer.pdf_pane().load(), PdfLoad::Pending),
          "opening a text file must release the pdf"
        );
        assert!(!viewer.preview_active());
        viewer.toggle_markdown_preview(cx);
        assert!(viewer.preview_active());
      })
      .unwrap();
    settle(cx, &window, handle, |viewer, _| viewer.preview_active());
    assert!(
      grants.allows_file(&repo_dir.path().canonicalize().unwrap().join("shot.png")),
      "the preview must grant its own local image, and only its own"
    );
    window
      .update(cx, |viewer, _, cx| {
        assert_eq!(
          viewer.editor_value(cx),
          "# Title\n\n![shot](shot.png)\n",
          "the raw buffer must survive the preview toggle"
        );
        viewer.toggle_markdown_preview(cx);
      })
      .unwrap();
    // Leaving the document releases the grant with it.
    window
      .update(cx, |viewer, _, cx| {
        viewer.model().update(cx, |model, cx| model.close_file(cx));
      })
      .unwrap();
    settle(cx, &window, handle, |viewer, cx| {
      viewer.model().read(cx).state().open_file.is_none()
    });
    assert!(
      !grants.allows_file(&repo_dir.path().canonicalize().unwrap().join("shot.png")),
      "a closed document must not keep filesystem access"
    );

    crate::test_core::park_and_shutdown(cx, &core);
  }

  /// Ask the model for a file, then pump frames until `ready` sees the surface settle.
  fn open(
    cx: &mut TestAppContext,
    window: &WindowHandle<FileViewer>,
    handle: AnyWindowHandle,
    path: &str,
    ready: impl Fn(&FileViewer, &App) -> bool,
  ) {
    let path = path.to_string();
    window
      .update(cx, move |viewer, _, cx| {
        viewer.model().update(cx, |model, cx| model.open_file(&path, None, cx));
      })
      .unwrap();
    settle(cx, window, handle, ready);
  }

  /// Draw and drain until the condition holds. Every surface here finishes on a background task
  /// whose result only lands on the next frame, so the loop is a real barrier, not a fixed pump.
  fn settle(
    cx: &mut TestAppContext,
    window: &WindowHandle<FileViewer>,
    handle: AnyWindowHandle,
    ready: impl Fn(&FileViewer, &App) -> bool,
  ) {
    for _ in 0..50 {
      cx.run_until_parked();
      handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)).unwrap();
      cx.run_until_parked();
      if window.update(cx, |viewer, _, cx| ready(viewer, cx)).unwrap() {
        return;
      }
    }
    panic!("the viewer never reached the expected state");
  }

  fn describe(load: &ImageLoad) -> &'static str {
    match load {
      ImageLoad::Pending => "pending",
      ImageLoad::Ready(_) => "an image that skipped the downscale",
      ImageLoad::Failed => "a failure",
    }
  }
}
