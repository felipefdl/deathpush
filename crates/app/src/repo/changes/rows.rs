use deathpush_core::config::settings::TreeDensity;
use deathpush_core::ops::repository::NestedRepository;
use deathpush_core::session::types::Intent;
use deathpush_core::theme::{Rgba, UiPalette};
use deathpush_core::types::{FileStatus, StashEntry};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::groups::{FileRow, GroupId};
use super::view::ChangesView;
use crate::repo::explorer::icons::{IconKind, icon_for};
use crate::theme::hsla;

pub fn status_letter(status: FileStatus) -> &'static str {
  use FileStatus::*;
  match status {
    Modified | TypeChanged | IndexModified | BothModified => "M",
    Added | IndexAdded | IntentToAdd | AddedByUs | AddedByThem | BothAdded => "A",
    Deleted | IndexDeleted | DeletedByUs | DeletedByThem | BothDeleted => "D",
    Renamed | IndexRenamed | IntentToRename | Copied | IndexCopied => "R",
    Untracked => "U",
    Ignored => "!",
  }
}

pub fn status_color(status: FileStatus, palette: &UiPalette) -> Rgba {
  use FileStatus::*;
  match status {
    Modified | TypeChanged => palette.git_modified,
    IndexModified => palette.git_staged_modified,
    IndexDeleted => palette.git_staged_deleted,
    Added | IndexAdded | Copied | IndexCopied | IntentToAdd => palette.git_added,
    Deleted => palette.git_deleted,
    Renamed | IndexRenamed | IntentToRename => palette.git_renamed,
    Untracked => palette.git_untracked,
    Ignored => palette.git_ignored,
    BothModified | BothAdded | BothDeleted | AddedByUs | AddedByThem | DeletedByUs | DeletedByThem => {
      palette.git_conflicting
    }
  }
}

pub fn is_dimmed(status: FileStatus) -> bool {
  matches!(status, FileStatus::Ignored)
}

fn row_height(density: TreeDensity) -> f32 {
  match density {
    TreeDensity::Compact => 22.0,
    TreeDensity::Default | TreeDensity::Relaxed => 28.0,
  }
}

fn split_name_dir(path: &str) -> (&str, Option<&str>) {
  match path.rfind('/') {
    Some(index) => (&path[index + 1..], Some(&path[..index])),
    None => (path, None),
  }
}

pub fn file_icon(kind: IconKind, path: &str) -> Option<&'static str> {
  let (name, _) = split_name_dir(path);
  icon_for(kind, name, false, false)
}

#[derive(Clone, Copy)]
pub struct FileRowPaint {
  pub density: TreeDensity,
  pub icons: IconKind,
}

pub fn render_file_row(
  row: &FileRow,
  selected: bool,
  paint: FileRowPaint,
  group_id: GroupId,
  index: usize,
  view: WeakEntity<ChangesView>,
  palette: &UiPalette,
) -> impl IntoElement {
  let (name, dir) = split_name_dir(&row.path);
  let letter = status_letter(row.status.clone());
  let color = status_color(row.status.clone(), palette);
  let dimmed = is_dimmed(row.status.clone());
  let click_row = row.clone();
  let menu_row = row.clone();
  let menu_view = view.clone();
  div()
    .id(SharedString::from(format!(
      "scm-file-{}-{}-{index}",
      row.group_kind as u8, row.path
    )))
    .h(px(row_height(paint.density)))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .when(selected, |el| el.bg(hsla(palette.list_active)))
    .when(!selected, |el| el.hover(|el| el.bg(hsla(palette.list_hover))))
    .when(dimmed, |el| el.opacity(0.6))
    .on_click(move |event, window, cx| {
      let _ = view.update(cx, |this, cx| {
        this.on_file_click(click_row.clone(), group_id, index, event, window, cx);
      });
    })
    .when_some(file_icon(paint.icons, &row.path), |el, path| {
      el.child(
        svg()
          .path(path)
          .size(px(16.0))
          .text_color(hsla(palette.muted_foreground)),
      )
    })
    .child(
      div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_baseline()
        .gap_1()
        .child(
          div()
            .flex_shrink_0()
            .text_size(px(13.0))
            .text_color(hsla(palette.foreground))
            .child(name.to_string()),
        )
        .when_some(dir.map(str::to_string), |el, dir| {
          el.child(
            div()
              .min_w_0()
              .flex_1()
              .overflow_hidden()
              .text_ellipsis()
              .text_size(px(11.0))
              .text_color(hsla(palette.muted_foreground))
              .child(dir),
          )
        }),
    )
    .child(
      div()
        .w(px(16.0))
        .flex_shrink_0()
        .text_size(px(11.0))
        .text_color(hsla(color))
        .child(letter),
    )
    .context_menu(move |menu, _, _| {
      let item = |label: &'static str,
                  action: fn(&mut ChangesView, &FileRow, &mut Window, &mut Context<ChangesView>)| {
        let view = menu_view.clone();
        let row = menu_row.clone();
        PopupMenuItem::new(label).on_click(move |_, window, cx| {
          let _ = view.update(cx, |this, cx| action(this, &row, window, cx));
        })
      };
      let staged = menu_row.staged;
      menu
        .item(item("Open Changes", ChangesView::menu_open_changes))
        .item(item("Open File", ChangesView::menu_open_file))
        .item(item("Show File History", ChangesView::menu_show_history))
        .separator()
        .item(if staged {
          item("Unstage Changes", ChangesView::menu_unstage)
        } else {
          item("Stage Changes", ChangesView::menu_stage)
        })
        .item(item("Discard Changes", ChangesView::menu_discard))
        .separator()
        .item(item("Copy Path", ChangesView::menu_copy_path))
        .item(item("Copy Relative Path", ChangesView::menu_copy_relative))
        .item(item("Reveal in Finder", ChangesView::menu_reveal))
        .when(!staged, |menu| {
          menu.separator().item(item("Move to Trash", ChangesView::menu_trash))
        })
    })
}

pub fn render_stash_row(stash: &StashEntry, view: WeakEntity<ChangesView>, palette: &UiPalette) -> impl IntoElement {
  let hover_group = SharedString::from(format!("stash-row-{}", stash.index));
  let index = stash.index;
  let apply = view.clone();
  let pop = view.clone();
  let drop = view;
  div()
    .id(SharedString::from(format!("scm-stash-{}", stash.index)))
    .group(hover_group.clone())
    .h(px(22.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .hover(|el| el.bg(hsla(palette.list_hover)))
    .child(
      svg()
        .path("icons/archive.svg")
        .size(px(16.0))
        .text_color(hsla(palette.muted_foreground)),
    )
    .child(
      div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(13.0))
        .child(stash.message.clone()),
    )
    .child(
      Button::new(SharedString::from(format!("stash-apply-{index}")))
        .ghost()
        .xsmall()
        .icon(Icon::empty().path("icons/check.svg"))
        .tooltip("Apply Stash")
        .invisible()
        .group_hover(hover_group.clone(), |style| style.visible())
        .on_click(move |_, window, cx| {
          let _ = apply.update(cx, |this, cx| this.send(Intent::StashApply { index }, window, cx));
        }),
    )
    .child(
      Button::new(SharedString::from(format!("stash-pop-{index}")))
        .ghost()
        .xsmall()
        .icon(Icon::empty().path("icons/arrow-up.svg"))
        .tooltip("Pop Stash")
        .invisible()
        .group_hover(hover_group.clone(), |style| style.visible())
        .on_click(move |_, window, cx| {
          let _ = pop.update(cx, |this, cx| this.send(Intent::StashPop { index }, window, cx));
        }),
    )
    .child(
      Button::new(SharedString::from(format!("stash-drop-{index}")))
        .ghost()
        .xsmall()
        .icon(Icon::empty().path("icons/trash.svg"))
        .tooltip("Drop Stash")
        .invisible()
        .group_hover(hover_group, |style| style.visible())
        .on_click(move |_, window, cx| {
          let _ = drop.update(cx, |this, cx| {
            this.send(
              Intent::StashDrop {
                index,
                confirmed: false,
              },
              window,
              cx,
            );
          });
        }),
    )
}

pub fn render_nested_row(
  repo: &NestedRepository,
  view: WeakEntity<ChangesView>,
  palette: &UiPalette,
) -> impl IntoElement {
  let path = repo.path.clone();
  div()
    .id(SharedString::from(format!("scm-nested-{}", repo.path)))
    .h(px(22.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .hover(|el| el.bg(hsla(palette.list_hover)))
    .on_click(move |_, _, cx| {
      let Some(entity) = view.upgrade() else { return };
      let Some(root) = entity.read(cx).model.read(cx).root_path() else {
        return;
      };
      crate::window::open_shell_window(Some(root.join(&path)), cx);
    })
    .child(
      svg()
        .path("icons/repo.svg")
        .size(px(16.0))
        .text_color(hsla(palette.muted_foreground)),
    )
    .child(
      div()
        .flex_1()
        .min_w_0()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(13.0))
        .child(repo.name.clone()),
    )
    .when_some(repo.branch.clone(), |el, branch| {
      el.child(
        div()
          .max_w(px(120.0))
          .overflow_hidden()
          .text_ellipsis()
          .text_size(px(11.0))
          .text_color(hsla(palette.muted_foreground))
          .child(branch),
      )
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::theme::UiPalette;
  use deathpush_core::types::FileStatus;

  #[test]
  fn scm_file_icon_follows_tree_icons() {
    assert_eq!(file_icon(IconKind::None, "src/main.rs"), None);
    assert_eq!(
      file_icon(IconKind::Standard, "src/main.rs"),
      Some("icons/file-code.svg")
    );
    assert_eq!(
      file_icon(IconKind::Complete, "src/main.rs"),
      Some("file-icons/file_type_rust.svg")
    );
    assert_eq!(file_icon(IconKind::Standard, "shot.png"), Some("icons/file-media.svg"));
  }

  #[test]
  fn status_letters_follow_the_spec() {
    use FileStatus::*;
    for s in [Modified, TypeChanged, IndexModified, BothModified] {
      assert_eq!(status_letter(s), "M");
    }
    for s in [Added, IndexAdded, IntentToAdd, AddedByUs, AddedByThem, BothAdded] {
      assert_eq!(status_letter(s), "A");
    }
    for s in [Deleted, IndexDeleted, DeletedByUs, DeletedByThem, BothDeleted] {
      assert_eq!(status_letter(s), "D");
    }
    for s in [Renamed, IndexRenamed, IntentToRename, Copied, IndexCopied] {
      assert_eq!(status_letter(s), "R");
    }
    assert_eq!(status_letter(Untracked), "U");
    assert_eq!(status_letter(Ignored), "!");
    assert!(is_dimmed(Ignored));
    assert!(!is_dimmed(Modified));
  }

  #[test]
  fn status_colors_pick_the_palette_slot() {
    let spec = deathpush_core::theme::parse_theme(r##"{"name":"t","type":"dark","colors":{}}"##).unwrap();
    let p = UiPalette::from_spec(&spec);
    assert_eq!(status_color(FileStatus::Modified, &p), p.git_modified);
    assert_eq!(status_color(FileStatus::IndexModified, &p), p.git_staged_modified);
    assert_eq!(status_color(FileStatus::IndexDeleted, &p), p.git_staged_deleted);
    assert_eq!(status_color(FileStatus::Added, &p), p.git_added);
    assert_eq!(status_color(FileStatus::Copied, &p), p.git_added);
    assert_eq!(status_color(FileStatus::Renamed, &p), p.git_renamed);
    assert_eq!(status_color(FileStatus::Untracked, &p), p.git_untracked);
    assert_eq!(status_color(FileStatus::Ignored, &p), p.git_ignored);
    assert_eq!(status_color(FileStatus::BothModified, &p), p.git_conflicting);
    assert_eq!(status_color(FileStatus::TypeChanged, &p), p.git_modified);
  }
}
