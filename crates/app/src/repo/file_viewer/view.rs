use std::sync::Arc;
use std::time::Duration;

use deathpush_core::config::settings::{MONO_FONT_STACK, WordWrap};
use gpui_kit::component::input::{Editor, EditorState, InputEvent, Position, TabSize};
use gpui_kit::*;

use super::autosave::{AUTOSAVE_MS, SaveState};
use super::header;
use super::states::{self, ViewerKind, classify};
use crate::config::AppConfig;
use crate::repo::diff::highlight::grammar_name;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::ActivePalette;

pub struct FileViewer {
  repo: Entity<RepoModel>,
  #[allow(dead_code)]
  layout: Entity<LayoutModel>,
  editor: Entity<EditorState>,
  save: SaveState,
  loaded_path: Option<String>,
  loaded_hash: Option<String>,
  loaded_language: Option<String>,
  image: Option<Arc<Image>>,
  pending_save_generation: u64,
  last_cursor_line: Option<usize>,
  window_handle: AnyWindowHandle,
  focus_handle: FocusHandle,
  editor_input_sub: Option<Subscription>,
  editor_cursor_sub: Option<Subscription>,
}

impl FileViewer {
  pub fn new(
    repo: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&repo, |_, _, cx| cx.notify()).detach();
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
      loaded_path: None,
      loaded_hash: None,
      loaded_language: None,
      image: None,
      pending_save_generation: 0,
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
        let generation = this.save.edited();
        this.pending_save_generation = generation;
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
    if !self.save.should_save(generation) {
      return;
    }
    let content = self.editor.read(cx).value().to_string();
    let expected = self.save.saved_hash.clone();
    self
      .repo
      .update(cx, |model, cx| model.write_open_file(content, expected, cx));
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
    self.save = SaveState {
      saved_hash: hash,
      dirty: false,
      generation: 0,
    };
    self.pending_save_generation = 0;
  }

  fn sync_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let open = self.repo.read(cx).state().open_file.clone();
    let kind = classify(open.as_ref());
    let Some(open) = open else {
      if self.loaded_path.is_some() {
        self.loaded_path = None;
        self.loaded_hash = None;
        self.loaded_language = None;
        self.image = None;
        self.reset_save(String::new());
        self.last_cursor_line = None;
      }
      return;
    };
    let new_path = self.loaded_path.as_deref() != Some(open.path.as_str());
    let Some(content) = open.content.clone() else {
      if new_path {
        self.loaded_path = Some(open.path);
        self.loaded_hash = None;
        self.image = None;
        self.reset_save(String::new());
      }
      return;
    };

    if kind == ViewerKind::Image {
      if new_path || self.loaded_hash.as_deref() != Some(content.content_hash.as_str()) {
        self.image = states::decode_image(&content.content);
        self.loaded_path = Some(open.path);
        self.loaded_hash = Some(content.content_hash.clone());
        self.reset_save(content.content_hash);
      }
      return;
    }

    if kind != ViewerKind::Text {
      if new_path {
        self.loaded_path = Some(open.path);
        self.loaded_hash = Some(content.content_hash.clone());
        self.image = None;
        self.reset_save(content.content_hash);
      }
      return;
    }

    if new_path {
      self.rebuild_editor(content.language.as_deref(), window, cx);
      self.editor.update(cx, |state, cx| {
        state.set_value(content.content.clone(), window, cx);
      });
      self.reset_save(content.content_hash.clone());
      self.loaded_path = Some(open.path);
      self.loaded_hash = Some(content.content_hash);
      self.image = None;
      if let Some(line) = open.pending_line {
        self.apply_pending_line(line, window, cx);
      }
      return;
    }

    if self.save.dirty {
      self
        .save
        .saved(content.content_hash.clone(), self.pending_save_generation);
      if !self.save.dirty {
        self.loaded_hash = Some(content.content_hash);
      }
      return;
    }

    if self.save.should_reload_external(&content.content_hash) {
      self.rebuild_editor(content.language.as_deref(), window, cx);
      self.editor.update(cx, |state, cx| {
        state.set_value(content.content.clone(), window, cx);
      });
      self.reset_save(content.content_hash.clone());
      self.loaded_hash = Some(content.content_hash);
      if let Some(line) = open.pending_line {
        self.apply_pending_line(line, window, cx);
      }
    }
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

  #[cfg(test)]
  pub(crate) fn editor_value(&self, cx: &App) -> String {
    self.editor.read(cx).value().to_string()
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
    let open = self.repo.read(cx).state().open_file.clone();
    let kind = classify(open.as_ref());
    let path = open.as_ref().map(|open| open.path.clone()).unwrap_or_default();
    let weak = cx.weak_entity();
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
      self.save.dirty,
      kind,
      weak.clone(),
      palette,
      cx,
    ));
    match kind {
      ViewerKind::Empty => root,
      ViewerKind::Loading => root.child(div().flex_1().min_h_0()),
      ViewerKind::Image => root.child(states::render_image(self.image.clone())),
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

  use crate::config::AppConfig;
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
        viewer.model().update(cx, |model, _| {
          model.state_mut().open_file = Some(OpenFile {
            path: "src/main.rs".into(),
            content: Some(FileContent {
              path: "src/main.rs".into(),
              content: body.into(),
              language: Some("rust".into()),
              file_type: "text".into(),
              content_hash: "h".into(),
            }),
            pending_line: None,
            load_id: 1,
          });
        });
        window.refresh();
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |viewer, _, cx| {
        assert_eq!(
          classify(viewer.model().read(cx).state().open_file.as_ref()),
          ViewerKind::Text
        );
        assert_eq!(viewer.editor_value(cx), body);
      })
      .unwrap();
  }
}
