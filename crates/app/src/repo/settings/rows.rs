use std::rc::Rc;

use deathpush_core::config::settings_ui::zoom_options;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState, NumberInput};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::actions::{ColorTheme, ConfigureWorkspace};
use crate::theme::{ActivePalette, hsla};

type NumberChange = Rc<dyn Fn(f64, &mut App)>;

/// Zoom select options as `(label, level)` from core `zoom_options`.
pub(crate) fn zoom_select_options() -> Vec<(SharedString, i32)> {
  zoom_options()
    .into_iter()
    .map(|(level, label)| (SharedString::from(label), level))
    .collect()
}

pub(crate) fn color_theme_hint() -> &'static str {
  if cfg!(target_os = "macos") {
    "Cmd+K Cmd+T"
  } else {
    "Ctrl+K Ctrl+T"
  }
}

/// 11px bold uppercase muted section heading.
pub(crate) fn section_title(text: impl Into<SharedString>, cx: &App) -> impl IntoElement {
  div()
    .pt_2()
    .pb_1()
    .text_size(px(11.0))
    .font_weight(FontWeight::BOLD)
    .text_color(cx.theme().muted_foreground)
    .child(text.into().to_string().to_uppercase())
}

/// Label on the left, pill `Switch` on the right.
pub(crate) fn toggle_row(
  label: impl Into<SharedString>,
  value: bool,
  on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
  let label: SharedString = label.into();
  labeled_row(
    label.clone(),
    Switch::new(label)
      .small()
      .checked(value)
      .on_click(move |checked, window, cx| on_change(*checked, window, cx)),
  )
}

/// Label on the left, filled dropdown of `options` on the right.
pub(crate) fn select_row<T: Clone + PartialEq + 'static>(
  label: impl Into<SharedString>,
  options: Vec<(SharedString, T)>,
  value: T,
  on_change: impl Fn(T, &mut App) + 'static,
) -> impl IntoElement {
  let label: SharedString = label.into();
  let current = options
    .iter()
    .find(|(_, candidate)| *candidate == value)
    .map(|(text, _)| text.clone())
    .unwrap_or_default();
  let on_change = Rc::new(on_change);
  labeled_row(
    label.clone(),
    Button::new(label)
      .small()
      .label(current)
      .dropdown_caret(true)
      .dropdown_menu(move |mut menu, _, _| {
        for (text, option) in &options {
          let checked = *option == value;
          let option = option.clone();
          let on_change = on_change.clone();
          menu = menu.item(
            PopupMenuItem::new(text.clone())
              .checked(checked)
              .on_click(move |_, _, cx| on_change(option.clone(), cx)),
          );
        }
        menu.scrollable(true)
      }),
  )
}

/// Label on the left, `NumberInput` with steppers on the right.
pub(crate) fn number_row(
  id: &'static str,
  label: impl Into<SharedString>,
  value: f64,
  min: f64,
  max: f64,
  step: f64,
  on_change: impl Fn(f64, &mut App) + 'static,
) -> impl IntoElement {
  NumberRow {
    id: SharedString::from(id),
    label: label.into(),
    value,
    min,
    max,
    step,
    on_change: Rc::new(on_change),
  }
}

/// Label on the left, text `Input` bound to `input` on the right.
pub(crate) fn text_row(label: impl Into<SharedString>, input: &Entity<InputState>) -> impl IntoElement {
  labeled_row(label, Input::new(input).small().w(px(200.0)))
}

pub(crate) fn color_theme_button(
  label: impl Into<SharedString>,
  hint: impl Into<SharedString>,
  cx: &App,
) -> impl IntoElement {
  Button::new("color-theme")
    .outline()
    .small()
    .w_full()
    .on_click(|_, window, cx| window.dispatch_action(Box::new(ColorTheme), cx))
    .child(
      div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(div().text_size(px(13.0)).child(label.into()))
        .child(
          div()
            .text_size(px(13.0))
            .text_color(cx.theme().muted_foreground)
            .child(hint.into()),
        ),
    )
}

pub(crate) fn projects_row(summary: &str, cx: &App) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let empty = summary == "Not configured";
  labeled_row(
    "Workspace Directories",
    div()
      .flex()
      .items_center()
      .gap_2()
      .child(
        div()
          .text_size(px(13.0))
          .when(empty, |el| el.text_color(hsla(palette.muted_foreground)))
          .truncate()
          .max_w(px(280.0))
          .child(summary.to_string()),
      )
      .child(
        Button::new("configure-workspace")
          .outline()
          .small()
          .label("Configure...")
          .on_click(|_, window, cx| window.dispatch_action(Box::new(ConfigureWorkspace), cx)),
      ),
  )
}

fn labeled_row(label: impl Into<SharedString>, control: impl IntoElement) -> impl IntoElement {
  div()
    .w_full()
    .flex()
    .items_center()
    .justify_between()
    .gap_3()
    .min_h(px(28.0))
    .child(div().flex_1().min_w_0().text_size(px(13.0)).child(label.into()))
    .child(control)
}

fn format_number(value: f64, step: f64) -> String {
  if step >= 1.0 {
    format!("{}", value.round() as i64)
  } else {
    let places = (-step.log10()).ceil().clamp(0.0, 8.0) as usize;
    format!("{value:.places$}")
  }
}

#[derive(IntoElement)]
struct NumberRow {
  id: SharedString,
  label: SharedString,
  value: f64,
  min: f64,
  max: f64,
  step: f64,
  on_change: NumberChange,
}

struct NumberState {
  input: Entity<InputState>,
  last: f64,
  _subscription: Subscription,
}

impl RenderOnce for NumberRow {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let NumberRow {
      id,
      label,
      value,
      min,
      max,
      step,
      on_change,
    } = self;
    let id = SharedString::from(format!("settings-number-{id}"));
    let state = window.use_keyed_state(id, cx, {
      let on_change = on_change.clone();
      move |window, cx| {
        let input = cx.new(|cx| {
          InputState::new(window, cx)
            .default_value(format_number(value, step))
            .min(min)
            .max(max)
            .step(step)
        });
        let subscription = cx.subscribe(&input, {
          let on_change = on_change.clone();
          move |state: &mut NumberState, input, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
              return;
            }
            let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
              return;
            };
            let clamped = parsed.clamp(min, max);
            if (clamped - state.last).abs() < f64::EPSILON {
              return;
            }
            state.last = clamped;
            on_change(clamped, cx);
          }
        });
        NumberState {
          input,
          last: value,
          _subscription: subscription,
        }
      }
    });
    state.update(cx, |state, cx| {
      if (state.last - value).abs() > f64::EPSILON {
        state.last = value;
        state.input.update(cx, |input, cx| {
          input.set_value(format_number(value, step), window, cx);
        });
      }
    });
    let input = state.read(cx).input.clone();
    labeled_row(label, NumberInput::new(&input).small().w(px(88.0)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::config::settings_ui::zoom_options;

  #[test]
  fn zoom_select_options_match_core() {
    let options = zoom_select_options();
    let core = zoom_options();
    assert_eq!(options.len(), core.len());
    assert_eq!(options.len(), 15);
    for ((label, level), (core_level, core_label)) in options.iter().zip(core.iter()) {
      assert_eq!(*level, *core_level);
      assert_eq!(label.as_ref(), core_label);
    }
    let label = |level: i32| {
      options
        .iter()
        .find(|(_, candidate)| *candidate == level)
        .map(|(text, _)| text.as_ref())
        .unwrap()
    };
    assert_eq!(label(0), "100%");
    assert_eq!(label(1), "120%");
    assert_eq!(label(-1), "83%");
  }
}
