//! Automatic updates (Tauri updater). QC Buddy checks the update feed at
//! `qcbuddy-latest/latest-{{target}}.json` (latest-windows.json here) 10 s
//! after starting and every 6 hours. A newer version is offered in the
//! flyout; nothing is installed until the user clicks. The download is
//! verified against the public key in tauri.conf.json, installed passively,
//! and QC Buddy restarts. The feed only changes when a release is published
//! (windows-publish-update.yml), so drafts are never offered.
//!
//! Dev builds don't check unless QCBUDDY_UPDATE_CHECK is set;
//! QCBUDDY_UPDATE_URL points the check at another feed (for testing).

use std::sync::Mutex;
use std::time::Duration;

use qcbuddy_core::log::{default_path, Log};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const FIRST_CHECK: Duration = Duration::from_secs(10);
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// What the flyout shows about updates.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateStatus {
    #[default]
    None,
    Available { version: String },
    Downloading { percent: Option<u8> },
    Installing,
    Failed { message: String },
}

#[derive(Default)]
pub struct Updates {
    status: Mutex<UpdateStatus>,
    /// The update found by the last check, kept until the user installs it.
    pending: Mutex<Option<Update>>,
}

impl Updates {
    pub fn status(&self) -> UpdateStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

/// Percent downloaded, when the size is known.
pub fn percent(downloaded: u64, total: Option<u64>) -> Option<u8> {
    total.filter(|t| *t > 0).map(|t| ((downloaded.min(t) * 100) / t) as u8)
}

/// Whether this build checks for updates.
pub fn checks_enabled() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("QCBUDDY_UPDATE_CHECK").is_some()
}

fn set_status(app: &AppHandle, status: UpdateStatus) {
    if let Some(updates) = app.try_state::<Updates>() {
        if let Ok(mut slot) = updates.status.lock() {
            *slot = status.clone();
        }
    }
    let _ = app.emit("update", status);
}

/// Check now and then every CHECK_EVERY, on a thread of its own.
pub fn start(app: AppHandle) {
    if !checks_enabled() {
        return;
    }
    let _ = std::thread::Builder::new().name("qcbuddy-updates".into()).spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            tauri::async_runtime::block_on(check(&app));
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

async fn check(app: &AppHandle) {
    let log = Log::file(default_path());
    // Don't interrupt an update that's downloading or installing.
    if matches!(app.state::<Updates>().status(), UpdateStatus::Downloading { .. } | UpdateStatus::Installing) {
        return;
    }
    let mut builder = app.updater_builder();
    if let Some(url) = std::env::var("QCBUDDY_UPDATE_URL").ok().and_then(|u| Url::parse(&u).ok()) {
        builder = match builder.endpoints(vec![url]) {
            Ok(b) => b,
            Err(e) => return log.write(format!("update check failed: {e}")),
        };
    }
    let result = match builder.build() {
        Ok(updater) => updater.check().await,
        Err(e) => Err(e),
    };
    match result {
        Ok(Some(update)) => {
            log.write(format!("update available: {} (running {})", update.version, update.current_version));
            let version = update.version.clone();
            if let Ok(mut pending) = app.state::<Updates>().pending.lock() {
                *pending = Some(update);
            }
            set_status(app, UpdateStatus::Available { version });
        }
        Ok(None) => {}
        // No network, GitHub down, feed missing: keep running this version quietly.
        Err(e) => log.write(format!("update check failed: {e}")),
    }
}

/// Download, verify and install the pending update, then restart. Called
/// when the user clicks "Install and restart".
pub async fn install(app: AppHandle) -> Result<(), String> {
    let log = Log::file(default_path());
    let update = app
        .state::<Updates>()
        .pending
        .lock()
        .ok()
        .and_then(|mut p| p.take())
        .ok_or_else(|| "There's no update to install.".to_string())?;
    log.write(format!("installing update {}", update.version));
    set_status(&app, UpdateStatus::Downloading { percent: Some(0) });
    let mut downloaded: u64 = 0;
    let mut shown: Option<u8> = Some(0);
    let progress_app = app.clone();
    let finished_app = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let now = percent(downloaded, total);
                if now != shown {
                    shown = now;
                    set_status(&progress_app, UpdateStatus::Downloading { percent: now });
                }
            },
            move || set_status(&finished_app, UpdateStatus::Installing),
        )
        .await;
    match result {
        Ok(()) => {
            log.write(format!("update {} installed; restarting", update.version));
            app.restart();
        }
        Err(e) => {
            // A bad signature lands here too: nothing unverified is installed.
            let message = format!("Couldn't install the update: {e}");
            log.write(&message);
            if let Ok(mut pending) = app.state::<Updates>().pending.lock() {
                *pending = Some(update); // so "Try again" can retry it
            }
            set_status(&app, UpdateStatus::Failed { message: message.clone() });
            Err(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_progress() {
        assert_eq!(percent(0, Some(200)), Some(0));
        assert_eq!(percent(50, Some(200)), Some(25));
        assert_eq!(percent(250, Some(200)), Some(100), "never past 100");
        assert_eq!(percent(50, None), None, "size unknown");
        assert_eq!(percent(50, Some(0)), None);
    }

    #[test]
    fn status_serializes_for_the_flyout() {
        let json = |s: UpdateStatus| serde_json::to_value(s).unwrap();
        assert_eq!(json(UpdateStatus::None), serde_json::json!({"kind": "none"}));
        assert_eq!(
            json(UpdateStatus::Available { version: "0.1.3".into() }),
            serde_json::json!({"kind": "available", "version": "0.1.3"})
        );
        assert_eq!(json(UpdateStatus::Downloading { percent: Some(42) }), serde_json::json!({"kind": "downloading", "percent": 42}));
        assert_eq!(json(UpdateStatus::Installing), serde_json::json!({"kind": "installing"}));
    }

    /// The feed our release workflow writes (scripts/update-feed.mjs, whose
    /// own test pins this shape) must parse with the plugin's own type, and
    /// the plugin must find the Windows download in it.
    #[test]
    fn our_feed_format_is_what_the_updater_reads() {
        let feed = serde_json::json!({
            "version": "0.1.3",
            "notes": "QC Buddy 0.1.3. Release notes: https://github.com/hu13/BoseBuddy/releases/tag/qcbuddy-v0.1.3",
            "pub_date": "2026-11-25T23:00:00.000Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==",
                    "url": "https://github.com/hu13/BoseBuddy/releases/download/qcbuddy-v0.1.3/QCBuddy_0.1.3_x64-setup.exe"
                }
            }
        });
        let release: tauri_plugin_updater::RemoteRelease = serde_json::from_value(feed).unwrap();
        assert_eq!(release.version.to_string(), "0.1.3");
        assert_eq!(
            release.download_url("windows-x86_64").unwrap().as_str(),
            "https://github.com/hu13/BoseBuddy/releases/download/qcbuddy-v0.1.3/QCBuddy_0.1.3_x64-setup.exe"
        );
        assert_eq!(release.signature("windows-x86_64").unwrap(), "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==");
        assert!(release.download_url("darwin-aarch64").is_err(), "a Windows feed has no Mac entry");
    }

    /// The feed URL and key are baked into every installed copy: guard them.
    #[test]
    fn update_config_is_per_platform_and_has_a_key() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let updater = &conf["plugins"]["updater"];
        assert_eq!(
            updater["endpoints"],
            serde_json::json!(["https://github.com/hu13/BoseBuddy/releases/download/qcbuddy-latest/latest-{{target}}.json"]),
            "one feed per platform (latest-windows.json), so platforms can release separately"
        );
        assert!(updater["pubkey"].as_str().unwrap().len() > 100, "public key present");
        assert_eq!(updater["windows"]["installMode"], "passive");
        // Updater artifacts (signed .sig files) only in release builds, which have the key.
        assert!(conf["bundle"]["createUpdaterArtifacts"].is_null());
        let release: serde_json::Value = serde_json::from_str(include_str!("../tauri.release.conf.json")).unwrap();
        assert_eq!(release["bundle"]["createUpdaterArtifacts"], true);
    }
}
