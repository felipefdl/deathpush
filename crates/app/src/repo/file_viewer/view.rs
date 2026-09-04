use std::sync::Arc;
use std::time::Duration;

use deathpush_core::config::settings::{MONO_FONT_STACK, WordWrap};
use gpui_kit::component::input::{Editor, EditorState, InputEvent, Position, TabSize};
use gpui_kit::*;

use super::autosave::{AUTOSAVE_MS, SaveState, SaveToken, should_complete_save, token_still_valid};
use super::header;
use super::states::{self, ViewerKind, classify};
use crate::config::AppConfig;
use crate::repo::diff::highlight::grammar_name;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::{RepoEvent, RepoModel};
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
    cx.subscribe(&repo, |this, _, event: &RepoEvent, cx| match event {
      RepoEvent::Saved { path, hash, generation } => this.on_saved(path, hash, *generation, cx),
      RepoEvent::Changed | RepoEvent::Error(_) => cx.notify(),
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
      loaded_path: None,
      loaded_hash: None,
      loaded_language: None,
      image: None,
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
        let path = this.loaded_path.clone();
        if was_clean {
          this.repo.update(cx, |model, cx| model.mark_open_file_dirty(cx));
        }
        cx.notify();
        cx.spawn(async move |this, cx| {
          cx.background_executor().timer(Duration::from_millis(AUTOSAVE_MS)).await;
          let _ = this.update(cx, |this, cx| {
            let Some(path) = path else {
              return;
            };
            this.flush_save(&path, generation, cx);
          });
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

  fn flush_save(&mut self, path: &str, generation: u64, cx: &mut Context<Self>) {
    let token = SaveToken {
      path: path.to_string(),
      generation,
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
  }

  fn sync_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    struct Snap {
      kind: ViewerKind,
      path: String,
      pending_line: Option<usize>,
      new_path: bool,
      hash: String,
      language: Option<String>,
      body: Option<String>,
    }

    enum Prep {
      Empty,
      Loading { path: String, new_path: bool },
      Ready(Snap),
    }

    let prep = {
      let state = self.repo.read(cx).state();
      match state.open_file.as_ref() {
        None => Prep::Empty,
        Some(open) if open.content.is_none() => Prep::Loading {
          path: open.path.clone(),
          new_path: self.loaded_path.as_deref() != Some(open.path.as_str()),
        },
        Some(open) => {
          let content = open.content.as_ref().expect("checked");
          let kind = classify(Some(open));
          let new_path = self.loaded_path.as_deref() != Some(open.path.as_str());
          let apply_image = kind == ViewerKind::Image
            && (new_path || self.loaded_hash.as_deref() != Some(content.content_hash.as_str()));
          let apply_text = kind == ViewerKind::Text
            && (new_path || (!self.save.dirty && self.save.should_reload_external(&content.content_hash)));
          Prep::Ready(Snap {
            kind,
            path: open.path.clone(),
            pending_line: open.pending_line,
            new_path,
            hash: content.content_hash.clone(),
            language: content.language.clone(),
            body: (apply_image || apply_text).then(|| content.content.clone()),
          })
        }
      }
    };

    match prep {
      Prep::Empty => {
        if self.loaded_path.is_some() {
          self.loaded_path = None;
          self.loaded_hash = None;
          self.loaded_language = None;
          self.image = None;
          self.reset_save(String::new());
          self.last_cursor_line = None;
        }
      }
      Prep::Loading { path, new_path } => {
        if new_path {
          self.loaded_path = Some(path);
          self.loaded_hash = None;
          self.image = None;
          self.reset_save(String::new());
        }
      }
      Prep::Ready(snap) => match snap.kind {
        ViewerKind::Empty | ViewerKind::Loading => {}
        ViewerKind::Image => {
          if let Some(uri) = snap.body {
            self.image = states::decode_image(&uri);
            self.loaded_path = Some(snap.path);
            self.loaded_hash = Some(snap.hash.clone());
            self.reset_save(snap.hash);
          }
        }
        ViewerKind::Binary | ViewerKind::Large => {
          if snap.new_path {
            self.loaded_path = Some(snap.path);
            self.loaded_hash = Some(snap.hash.clone());
            self.image = None;
            self.reset_save(snap.hash);
          }
        }
        ViewerKind::Text => {
          if let Some(body) = snap.body {
            self.rebuild_editor(snap.language.as_deref(), window, cx);
            self.editor.update(cx, |state, cx| {
              state.set_value(body, window, cx);
            });
            self.reset_save(snap.hash.clone());
            self.loaded_path = Some(snap.path);
            self.loaded_hash = Some(snap.hash);
            self.image = None;
          }
          if let Some(line) = snap.pending_line {
            self.apply_pending_line(line, window, cx);
          }
        }
      },
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
    let (kind, path) = {
      let open = self.repo.read(cx).state().open_file.as_ref();
      (
        classify(open),
        open.map(|open| open.path.as_str()).unwrap_or("").to_string(),
      )
    };
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
            dirty: false,
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
