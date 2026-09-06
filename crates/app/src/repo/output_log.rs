use deathpush_core::git::cli::GitCommandEvent;
use gpui_kit::*;

const MAX_LINES: usize = 500;

/// The Output tab: one line per git command core ran in this window's process.
#[derive(Default)]
pub struct OutputLog {
  lines: Vec<GitCommandEvent>,
}

/// `{timestamp} [info] > {command} [{duration} ms]`, per docs/specs/terminal.md.
pub fn format_line(event: &GitCommandEvent) -> String {
  format!(
    "{} [info] > {} [{} ms]",
    event.timestamp, event.command, event.duration_ms
  )
}

impl OutputLog {
  pub fn push(&mut self, event: GitCommandEvent, cx: &mut Context<Self>) {
    self.lines.push(event);
    if self.lines.len() > MAX_LINES {
      let excess = self.lines.len() - MAX_LINES;
      self.lines.drain(..excess);
    }
    cx.notify();
  }

  pub fn lines(&self) -> &[GitCommandEvent] {
    &self.lines
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn line_format_matches_the_terminal_spec() {
    let event = GitCommandEvent {
      command: "git status".into(),
      duration_ms: 12,
      timestamp: "2026-09-03 12:00:00.000".into(),
    };
    assert_eq!(
      format_line(&event),
      "2026-09-03 12:00:00.000 [info] > git status [12 ms]"
    );
  }
}
