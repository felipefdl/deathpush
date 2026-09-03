use gpui_kit::*;

pub fn run(shell: &mut crate::shell::Shell, _: &mut Window, cx: &mut Context<crate::shell::Shell>) {
  shell.show_toast("Coming in Task 9", cx);
}
