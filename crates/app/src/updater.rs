use std::sync::atomic::{AtomicU64, Ordering};

use cargo_packager_updater::{Config, Update, UpdaterBuilder};

pub const UPDATER_ENDPOINT: &str = "https://github.com/felipefdl/deathpush/releases/latest/download/latest.json";
pub const UPDATER_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDJGNjkyMjhEQkE4NUJEMDAKUldRQXZZVzZqU0pwTHh2VGlkTzg4UHNxdGFyb09BOEJDRzl5UnNrYmRzVWJXMGVJbGZXZnRHMk8K";

pub enum UpdateCheck {
  UpToDate,
  Available(Box<Update>),
  Failed(String),
}

pub fn should_check_on_launch(debug: bool) -> bool {
  !debug
}

pub fn update_button_label(version: &str) -> String {
  format!("Update to v{version}")
}

pub fn updating_button_label(percent: u8) -> String {
  format!("Updating {percent}%")
}

pub fn download_percent(received: u64, total: Option<u64>) -> u8 {
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
  match UpdaterBuilder::new(version, config).build() {
    Ok(updater) => match updater.check() {
      Ok(Some(update)) => UpdateCheck::Available(Box::new(update)),
      Ok(None) => UpdateCheck::UpToDate,
      Err(err) => UpdateCheck::Failed(err.to_string()),
    },
    Err(err) => UpdateCheck::Failed(err.to_string()),
  }
}

pub async fn check(current: &str) -> UpdateCheck {
  let current = current.to_string();
  match tokio::task::spawn_blocking(move || check_sync(&current)).await {
    Ok(result) => result,
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

pub async fn install(update: Update, progress: tokio::sync::mpsc::UnboundedSender<u8>) -> Result<(), String> {
  tokio::task::spawn_blocking(move || {
    let received = AtomicU64::new(0);
    update
      .download_and_install_extended(
        |chunk, total| {
          let previous = received.fetch_add(chunk as u64, Ordering::Relaxed);
          let _ = progress.send(download_percent(previous.saturating_add(chunk as u64), total));
        },
        || {},
      )
      .map_err(|err| err.to_string())?;
    relaunch(&update)
  })
  .await
  .map_err(|err| err.to_string())?
}

#[cfg(test)]
mod tests {
  use super::*;
  use cargo_packager_updater::{RemoteRelease, RemoteReleaseData};

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
