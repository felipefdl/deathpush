use gpui_kit::component::button::*;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde::Deserialize;

use super::frame::{backdrop, dialog_frame};
use crate::actions::Cancel;
use crate::theme::{ActivePalette, hsla};

pub enum LicensesEvent {
  Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LicenseRow {
  pub name: String,
  pub license: String,
  pub url: Option<String>,
}

#[derive(Deserialize)]
struct Metadata {
  packages: Vec<Package>,
  workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
  id: String,
  name: String,
  license: Option<String>,
  repository: Option<String>,
}

const METADATA: &str = include_str!(concat!(env!("OUT_DIR"), "/licenses.json"));

/// Bundled assets, then every non-workspace crate, sorted by name and deduplicated.
pub fn license_groups() -> Vec<(&'static str, Vec<LicenseRow>)> {
  let assets = vec![
    LicenseRow {
      name: "MesloLGS Nerd Font Mono".into(),
      license: "Apache-2.0".into(),
      url: Some("https://github.com/ryanoasis/nerd-fonts".into()),
    },
    LicenseRow {
      name: "Codicons".into(),
      license: "CC-BY-4.0".into(),
      url: Some("https://github.com/microsoft/vscode-codicons".into()),
    },
    LicenseRow {
      name: "tm-themes".into(),
      license: "MIT".into(),
      url: Some("https://github.com/shikijs/textmate-grammars-themes".into()),
    },
  ];
  let backend = match serde_json::from_str::<Metadata>(METADATA) {
    Ok(metadata) => {
      let mut rows: Vec<LicenseRow> = metadata
        .packages
        .into_iter()
        .filter(|package| !metadata.workspace_members.contains(&package.id))
        .map(|package| LicenseRow {
          name: package.name,
          license: package.license.unwrap_or_else(|| "Unknown".into()),
          url: package.repository,
        })
        .collect();
      rows.sort_by_key(|a| a.name.to_lowercase());
      rows.dedup_by(|a, b| a.name == b.name);
      rows
    }
    Err(_) => Vec::new(),
  };
  let mut groups = vec![("Assets", assets)];
  if !backend.is_empty() {
    groups.push(("Backend", backend));
  }
  groups
}

pub struct LicensesDialog {
  groups: Vec<(&'static str, Vec<LicenseRow>)>,
}

impl EventEmitter<LicensesEvent> for LicensesDialog {}

impl LicensesDialog {
  pub fn new() -> Self {
    Self {
      groups: license_groups(),
    }
  }
}

impl Render for LicensesDialog {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let max_height = window.bounds().size.height * 0.7;
    let mut list: Vec<AnyElement> = Vec::new();
    for (title, rows) in &self.groups {
      list.push(
        div()
          .pt_2()
          .pb_1()
          .text_size(px(11.0))
          .font_weight(FontWeight::BOLD)
          .text_color(hsla(palette.muted_foreground))
          .child(title.to_uppercase())
          .into_any_element(),
      );
      for (index, row) in rows.iter().enumerate() {
        let url = row.url.clone();
        list.push(
          div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(26.0))
            .px_1()
            .rounded_sm()
            .hover(|el| el.bg(hsla(palette.list_hover)))
            .child(
              div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.0))
                .child(row.name.clone()),
            )
            .child(
              div()
                .px_2()
                .rounded_full()
                .text_size(px(11.0))
                .bg(hsla(palette.badge))
                .text_color(hsla(palette.badge_foreground))
                .child(row.license.clone()),
            )
            .children(url.map(|href| {
              let tooltip = href.clone();
              Button::new(SharedString::from(format!("{title}-{index}")))
                .ghost()
                .xsmall()
                .icon(Icon::empty().path("icons/link-external.svg"))
                .tooltip(tooltip)
                .on_click(move |_, _, cx| cx.open_url(&href))
            }))
            .into_any_element(),
        );
      }
    }
    backdrop("licenses-backdrop", |_, _| {}, cx)
      .on_mouse_down(
        MouseButton::Left,
        cx.listener(|_, _, _, cx| cx.emit(LicensesEvent::Close)),
      )
      .child(
        dialog_frame(560.0, "Open Source Licenses", cx)
          .max_h(max_height)
          .on_action(cx.listener(|_, _: &Cancel, _, cx| cx.emit(LicensesEvent::Close)))
          .child(
            div()
              .id("licenses-list")
              .flex_1()
              .min_h_0()
              .overflow_y_scroll()
              .children(list),
          )
          .child(
            div().flex().justify_end().mt(px(12.0)).child(
              Button::new("close")
                .outline()
                .small()
                .label("Close")
                .on_click(cx.listener(|_, _, _, cx| cx.emit(LicensesEvent::Close))),
            ),
          ),
      )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn assets_group_comes_first_and_backend_is_sorted() {
    let groups = license_groups();
    assert_eq!(groups[0].0, "Assets");
    assert_eq!(groups[0].1.len(), 3);
    if let Some((_, backend)) = groups.get(1) {
      let names: Vec<String> = backend.iter().map(|row| row.name.to_lowercase()).collect();
      let mut sorted = names.clone();
      sorted.sort();
      assert_eq!(names, sorted);
      assert!(backend.iter().any(|row| row.name == "git2"));
    }
  }
}
