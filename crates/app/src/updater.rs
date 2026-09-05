use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use cargo_packager_updater::{Config, Update, UpdaterBuilder};
use gpui_kit::*;

/// Manifest URL for the latest packaged release.
pub(crate) const UPDATER_ENDPOINT: &str = "https://github.com/felipefdl/deathpush/releases/latest/download/latest.json";
/// Minisign public key that verifies updater bundles.
pub(crate) const UPDATER_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDJGNjkyMjhEQkE4NUJEMDAKUldRQXZZVzZqU0pwTHh2VGlkTzg4UHNxdGFyb09BOEJDRzl5UnNrYmRzVWJXMGVJbGZXZnRHMk8K";

pub(crate) const CHECK_DELAY: Duration = Duration::from_secs(2);
pub(crate) const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Result of asking the updater whether a newer package exists.
#[derive(Clone)]
pub(crate) enum UpdateCheck {
  UpToDate,
  Available {
    version: String,
    update: Option<Box<Update>>,
  },
  Failed(String),
}

enum UpdateStatus {
  Idle,
  Available {
    version: String,
    update: Option<Box<Update>>,
  },
  Downloading {
    version: String,
    percent: u8,
    update: Option<Box<Update>>,
  },
}

/// Check and install operations. Tests replace this with a fake.
pub(crate) trait UpdaterOps: Send + Sync {
  fn check(&self, current: &str) -> UpdateCheck;
  fn install(&self, update: Option<Box<Update>>, on_progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), String>;
}

struct LiveOps;

impl UpdaterOps for LiveOps {
  fn check(&self, current: &str) -> UpdateCheck {
    check_sync(current)
  }

  fn install(&self, update: Option<Box<Update>>, on_progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), String> {
    install_sync(update, on_progress)
  }
}

/// In-flight check flag, last result, and download progress shared by every window.
pub struct UpdaterState {
  started: bool,
  status: UpdateStatus,
  ops: Arc<dyn UpdaterOps>,
}

impl Default for UpdaterState {
  fn default() -> Self {
    Self {
      started: false,
      status: UpdateStatus::Idle,
      ops: Arc::new(LiveOps),
    }
  }
}

impl Global for UpdaterState {}

impl UpdaterState {
  fn ops(&self) -> Arc<dyn UpdaterOps> {
    self.ops.clone()
  }

  /// True when this call is the one that should start the network check.
  pub(crate) fn begin_check(&mut self) -> bool {
    if self.started {
      return false;
    }
    self.started = true;
    true
  }

  pub(crate) fn apply_check(&mut self, result: UpdateCheck) -> Option<String> {
    match result {
      UpdateCheck::Available { version, update } => {
        self.status = UpdateStatus::Available { version, update };
        None
      }
      UpdateCheck::UpToDate => None,
      UpdateCheck::Failed(err) => Some(err),
    }
  }

  pub(crate) fn begin_install(&mut self) -> Option<Option<Box<Update>>> {
    match &self.status {
      UpdateStatus::Available { version, update } => {
        let version = version.clone();
        let update = update.clone();
        self.status = UpdateStatus::Downloading {
          version,
          percent: 0,
          update: update.clone(),
        };
        Some(update)
      }
      _ => None,
    }
  }

  pub(crate) fn set_percent(&mut self, percent: u8) {
    if let UpdateStatus::Downloading { percent: slot, .. } = &mut self.status {
      *slot = percent;
    }
  }

  pub(crate) fn finish_install(&mut self, result: Result<(), String>) -> Option<String> {
    match result {
      Ok(()) => None,
      Err(err) => {
        if let UpdateStatus::Downloading { version, update, .. } =
          std::mem::replace(&mut self.status, UpdateStatus::Idle)
        {
          self.status = UpdateStatus::Available { version, update };
        }
        Some(err)
      }
    }
  }

  /// Footer label and disabled flag when an update exists.
  pub(crate) fn button(&self) -> Option<(String, bool)> {
    match &self.status {
      UpdateStatus::Idle => None,
      UpdateStatus::Available { version, .. } => Some((update_button_label(version), false)),
      UpdateStatus::Downloading { percent, .. } => Some((updating_button_label(*percent), true)),
    }
  }

  #[cfg(test)]
  pub(crate) fn set_ops(&mut self, ops: Arc<dyn UpdaterOps>) {
    self.ops = ops;
  }
}

pub(crate) fn should_check_on_launch(debug: bool) -> bool {
  !debug
}

pub(crate) fn update_button_label(version: &str) -> String {
  format!("Update to v{version}")
}

pub(crate) fn updating_button_label(percent: u8) -> String {
  format!("Updating {percent}%")
}

pub(crate) fn download_percent(received: u64, total: Option<u64>) -> u8 {
  match total {
    Some(total) if total > 0 => ((received.saturating_mul(100)) / total).min(100) as u8,
    _ => 0,
  }
}

fn updater_config() -> Result<Config, String> {
  let endpoint = UPDATER_ENDPOINT
    .parse()
    .map_err(|err: url::ParseError| err.to_string())?;
  Ok(Config {
    endpoints: vec![endpoint],
    pubkey: UPDATER_PUBKEY.into(),
    windows: None,
  })
}

fn check_sync(current: &str) -> UpdateCheck {
  let version = match current.parse::<cargo_packager_updater::semver::Version>() {
    Ok(version) => version,
    Err(err) => return UpdateCheck::Failed(err.to_string()),
  };
  let config = match updater_config() {
    Ok(config) => config,
    Err(err) => return UpdateCheck::Failed(err),
  };
  match UpdaterBuilder::new(version, config).timeout(CHECK_TIMEOUT).build() {
    Ok(updater) => match updater.check() {
      Ok(Some(update)) => UpdateCheck::Available {
        version: update.version.clone(),
        update: Some(Box::new(update)),
      },
      Ok(None) => UpdateCheck::UpToDate,
      Err(err) => UpdateCheck::Failed(err.to_string()),
    },
    Err(err) => UpdateCheck::Failed(err.to_string()),
  }
}

fn relaunch(update: &Update) -> Result<(), String> {
  #[cfg(windows)]
  {
    let _ = update;
    Ok(())
  }
  #[cfg(target_os = "macos")]
  {
    std::process::Command::new("open")
      .arg("-n")
      .arg(&update.extract_path)
      .spawn()
      .map_err(|err| err.to_string())?;
    std::process::exit(0);
  }
  #[cfg(not(any(windows, target_os = "macos")))]
  {
    std::process::Command::new(&update.extract_path)
      .spawn()
      .map_err(|err| err.to_string())?;
    std::process::exit(0);
  }
}

fn install_sync(update: Option<Box<Update>>, on_progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), String> {
  let Some(mut update) = update.map(|update| *update) else {
    return Err("No update to install".into());
  };
  update.timeout = Some(DOWNLOAD_TIMEOUT);
  let received = AtomicU64::new(0);
  update
    .download_and_install_extended(
      |chunk, total| {
        let previous = received.fetch_add(chunk as u64, Ordering::Relaxed);
        on_progress(download_percent(previous.saturating_add(chunk as u64), total));
      },
      || {},
    )
    .map_err(|err| err.to_string())?;
  relaunch(&update)
}

pub(crate) fn take_ops(cx: &mut App) -> Arc<dyn UpdaterOps> {
  cx.default_global::<UpdaterState>().ops()
}

#[cfg(test)]
pub(crate) struct FakeOps {
  pub check: UpdateCheck,
  pub percents: Vec<u8>,
  pub install: Result<(), String>,
  pub checks: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl FakeOps {
  pub fn available() -> Self {
    Self {
      check: UpdateCheck::Available {
        version: "0.5.0".into(),
        update: None,
      },
      percents: vec![40],
      install: Ok(()),
      checks: std::sync::atomic::AtomicUsize::new(0),
    }
  }
}

#[cfg(test)]
impl UpdaterOps for FakeOps {
  fn check(&self, _current: &str) -> UpdateCheck {
    self.checks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    self.check.clone()
  }

  fn install(&self, _update: Option<Box<Update>>, on_progress: &(dyn Fn(u8) + Send + Sync)) -> Result<(), String> {
    for percent in &self.percents {
      on_progress(*percent);
    }
    self.install.clone()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use cargo_packager_updater::{RemoteRelease, RemoteReleaseData};
  use core::prelude::v1::test;

  #[test]
  fn should_check_on_launch_skips_debug() {
    assert!(!should_check_on_launch(true));
    assert!(should_check_on_launch(false));
  }

  #[test]
  fn update_button_labels_match_spec() {
    assert_eq!(update_button_label("0.5.0"), "Update to v0.5.0");
    assert_eq!(updating_button_label(0), "Updating 0%");
    assert_eq!(updating_button_label(42), "Updating 42%");
    assert_eq!(updating_button_label(100), "Updating 100%");
  }

  #[test]
  fn updater_timeouts_are_finite() {
    assert_eq!(CHECK_TIMEOUT, Duration::from_secs(30));
    assert_eq!(DOWNLOAD_TIMEOUT, Duration::from_secs(10 * 60));
  }

  #[test]
  fn second_begin_check_is_ignored() {
    let mut state = UpdaterState::default();
    assert!(state.begin_check());
    assert!(!state.begin_check());
  }

  #[test]
  fn timed_out_check_stays_idle_and_returns_toast() {
    let mut state = UpdaterState::default();
    let toast = state.apply_check(UpdateCheck::Failed("operation timed out".into()));
    assert_eq!(toast.as_deref(), Some("operation timed out"));
    assert!(state.button().is_none());
  }

  #[test]
  fn timed_out_download_restores_available() {
    let mut state = UpdaterState::default();
    state.apply_check(UpdateCheck::Available {
      version: "0.5.0".into(),
      update: None,
    });
    assert!(state.begin_install().is_some());
    state.set_percent(40);
    assert_eq!(state.button(), Some(("Updating 40%".into(), true)));
    let toast = state.finish_install(Err("operation timed out".into()));
    assert_eq!(toast.as_deref(), Some("operation timed out"));
    assert_eq!(state.button(), Some(("Update to v0.5.0".into(), false)));
  }

  #[test]
  fn manifest_parses_sample() {
    let json = r#"{
      "version": "v0.5.0",
      "notes": "Test version",
      "pub_date": "2020-06-22T19:25:57Z",
      "platforms": {
        "macos-aarch64": {
          "signature": "Content of app.tar.gz.sig",
          "url": "https://github.com/felipefdl/deathpush/releases/download/v0.5.0/DeathPush_0.5.0_aarch64.app.tar.gz",
          "format": "app"
        },
        "linux-x86_64": {
          "signature": "Content of app.AppImage.sig",
          "url": "https://github.com/felipefdl/deathpush/releases/download/v0.5.0/DeathPush_0.5.0_amd64.AppImage.tar.gz",
          "format": "appimage"
        },
        "windows-x86_64": {
          "signature": "Content of app-setup.exe.sig",
          "url": "https://github.com/felipefdl/deathpush/releases/download/v0.5.0/DeathPush_0.5.0_x64-setup.nsis.zip",
          "format": "nsis"
        }
      }
    }"#;
    let release: RemoteRelease = serde_json::from_str(json).expect("sample latest.json");
    assert_eq!(release.version.to_string(), "0.5.0");
    assert_eq!(release.notes.as_deref(), Some("Test version"));
    assert!(release.pub_date.is_some());
    match release.data {
      RemoteReleaseData::Static { platforms } => {
        assert!(platforms.contains_key("macos-aarch64"));
        assert!(platforms.contains_key("linux-x86_64"));
        assert!(platforms.contains_key("windows-x86_64"));
        assert_eq!(
          platforms["macos-aarch64"].url.as_str(),
          "https://github.com/felipefdl/deathpush/releases/download/v0.5.0/DeathPush_0.5.0_aarch64.app.tar.gz"
        );
      }
      RemoteReleaseData::Dynamic(_) => panic!("expected platforms map"),
    }
  }

  #[test]
  fn download_percent_uses_content_length() {
    assert_eq!(download_percent(0, Some(100)), 0);
    assert_eq!(download_percent(50, Some(200)), 25);
    assert_eq!(download_percent(200, Some(200)), 100);
    assert_eq!(download_percent(10, None), 0);
    assert_eq!(download_percent(10, Some(0)), 0);
  }
}
