use gpui_kit::*;

use crate::shell::Shell;

const INSTALL_MESSAGE: &str = "Install dp and deathpush commands to /usr/local/bin so you can open repositories from any terminal.\n\nExamples:\n  dp .\n  deathpush ~/projects/my-repo";

/// Install or uninstall per the native-menus spec, with system dialogs for every step.
pub fn run(shell: &mut Shell, window: &mut Window, cx: &mut Context<Shell>) {
  let core = shell.core.clone();
  let installed = core
    .check_cli_installed()
    .map(|status| status.installed)
    .unwrap_or(false);
  let answer = if installed {
    window.prompt(
      PromptLevel::Warning,
      "Command Line Tool",
      Some("Command line tools 'dp' and 'deathpush' are already installed. Would you like to uninstall them?"),
      &["Uninstall", "Cancel"],
      cx,
    )
  } else {
    window.prompt(
      PromptLevel::Warning,
      "Install Command Line Tool",
      Some(INSTALL_MESSAGE),
      &["Install", "Cancel"],
      cx,
    )
  };
  cx.spawn_in(window, async move |this, cx| {
    let Ok(0) = answer.await else {
      return;
    };
    let work_core = core.clone();
    let result = cx
      .background_spawn(async move {
        if installed {
          work_core.uninstall_cli()
        } else {
          work_core.install_cli()
        }
      })
      .await;
    let _ = this.update_in(cx, |this, window, cx| match result {
      Ok(()) => {
        let message = if installed {
          "Command line tools have been uninstalled."
        } else {
          "Commands dp and deathpush installed successfully. Restart your terminal to start using them."
        };
        drop(window.prompt(PromptLevel::Info, "Command Line Tool", Some(message), &["OK"], cx));
        this.set_cli_installed(!installed, window, cx);
      }
      Err(err) => {
        if !err.to_string().to_lowercase().contains("cancel") {
          this.show_toast(err.to_string(), cx);
        }
      }
    });
  })
  .detach();
}
