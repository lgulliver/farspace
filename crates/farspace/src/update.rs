//! Update checking, downloading, and staging for FARSPACE.
//!
//! This module is the only place in the workspace that performs network I/O.
//! All HTTP work runs on detached threads so the TUI is never blocked.

use game_tui::{UpdateChannel, UpdateInfo};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

type UpdateCheckRx = mpsc::Receiver<Option<UpdateInfo>>;
type DownloadTx = mpsc::SyncSender<UpdateInfo>;
type DownloadResultRx = mpsc::Receiver<Result<String, String>>;

const GITHUB_API: &str = "https://api.github.com/repos/lgulliver/farspace/releases";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// GitHub API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

// ---------------------------------------------------------------------------
// Platform asset name
// ---------------------------------------------------------------------------

fn platform_asset_name() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "farspace-linux-x86_64";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "farspace-linux-aarch64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "farspace-macos-x86_64";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "farspace-macos-aarch64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "farspace-windows-x86_64.exe";
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return "";
}

// ---------------------------------------------------------------------------
// Version comparison
// ---------------------------------------------------------------------------

/// Returns true if `candidate` is newer than the currently running binary.
///
/// For stable/preview: compares semver strings (strips leading 'v').
/// For nightly: compares the date+time suffix lexicographically.
fn is_newer(channel: UpdateChannel, candidate: &str) -> bool {
    let candidate = candidate.trim_start_matches('v');
    match channel {
        UpdateChannel::Nightly => {
            // Tags: nightly-YYYYMMDD-HHMM  →  compare as strings (ISO order)
            let current_tag = option_env!("FARSPACE_BUILD_TAG").unwrap_or("");
            if current_tag.is_empty() {
                return false; // stable binary, ignore nightly updates
            }
            candidate > current_tag.trim_start_matches('v')
        }
        UpdateChannel::Stable | UpdateChannel::Preview => {
            // Simple lexicographic semver — good enough for MAJOR.MINOR.PATCH
            let current = CURRENT_VERSION.trim_start_matches('v');
            semver_gt(candidate, current)
        }
    }
}

/// Naive semver greater-than: compares each numeric component left to right.
fn semver_gt(a: &str, b: &str) -> bool {
    let parse = |s: &str| -> [u64; 3] {
        let mut parts = s.split('-').next().unwrap_or("").split('.');
        [
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        ]
    };
    parse(a) > parse(b)
}

fn release_matches_channel(release: &GhRelease, channel: UpdateChannel) -> bool {
    if release.draft {
        return false;
    }
    match channel {
        UpdateChannel::Stable => !release.prerelease,
        UpdateChannel::Preview => {
            release.prerelease
                && (release.tag_name.contains("alpha") || release.tag_name.contains("beta"))
        }
        UpdateChannel::Nightly => release.prerelease && release.tag_name.starts_with("nightly-"),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Set up the three channels for update checking and downloading.
///
/// Returns `(check_rx, download_tx, download_rx)` to pass to
/// `App::set_update_channels()`.
pub fn setup_update_system(
    channel: UpdateChannel,
) -> (UpdateCheckRx, DownloadTx, DownloadResultRx) {
    let (check_tx, check_rx) = mpsc::channel::<Option<UpdateInfo>>();
    let (request_tx, request_rx) = mpsc::sync_channel::<UpdateInfo>(1);
    let (result_tx, result_rx) = mpsc::channel::<Result<String, String>>();

    // Background update check
    thread::spawn(move || {
        let result = check_latest(channel);
        let _ = check_tx.send(result);
    });

    // Background download worker
    thread::spawn(move || {
        while let Ok(info) = request_rx.recv() {
            let outcome = download_and_stage(&info)
                .map(|_| info.version.clone())
                .map_err(|e| e.to_string());
            if result_tx.send(outcome).is_err() {
                break;
            }
        }
    });

    (check_rx, request_tx, result_rx)
}

/// Check GitHub releases for a newer version on the given channel.
/// Returns `Some(UpdateInfo)` if an update is available, `None` otherwise.
fn check_latest(channel: UpdateChannel) -> Option<UpdateInfo> {
    let asset_name = platform_asset_name();
    if asset_name.is_empty() {
        return None; // unsupported platform
    }

    let response: Vec<GhRelease> = ureq::get(GITHUB_API)
        .set("User-Agent", &format!("farspace/{CURRENT_VERSION}"))
        .set("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .into_json()
        .ok()?;

    let release = response
        .into_iter()
        .find(|r| release_matches_channel(r, channel))?;

    if !is_newer(channel, &release.tag_name) {
        return None;
    }

    let asset = release.assets.iter().find(|a| a.name == asset_name)?;

    Some(UpdateInfo {
        version: release.tag_name.clone(),
        channel,
        download_url: asset.browser_download_url.clone(),
    })
}

/// Download the update binary to a staging path next to the current executable.
/// The staged file is `<exe>.update` and is applied on the next launch.
pub fn download_and_stage(info: &UpdateInfo) -> anyhow::Result<PathBuf> {
    let staging = staged_path()?;

    let response = ureq::get(&info.download_url)
        .set("User-Agent", &format!("farspace/{CURRENT_VERSION}"))
        .call()?;

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&staging)?;
    std::io::copy(&mut reader, &mut file)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&staging)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&staging, perms)?;
    }

    Ok(staging)
}

/// Path where the staged update binary is written.
pub fn staged_path() -> anyhow::Result<PathBuf> {
    let mut path = std::env::current_exe()?;
    path.set_extension("update");
    Ok(path)
}

/// Check for a staged update and apply it before the TUI starts.
///
/// On Unix: atomically renames the staged file over the current binary.
/// On Windows: copies the staged file over the current binary (best-effort).
///
/// Returns `true` if an update was applied (the caller should note that the
/// binary on disk has changed, though the running process is the old version).
pub fn check_and_apply_staged() -> bool {
    let Ok(staged) = staged_path() else {
        return false;
    };
    if !staged.exists() {
        return false;
    }

    let Ok(current) = std::env::current_exe() else {
        let _ = std::fs::remove_file(&staged);
        return false;
    };

    let applied = apply_staged(&current, &staged);
    if applied {
        let _ = std::fs::remove_file(&staged);
    }
    applied
}

#[cfg(unix)]
fn apply_staged(current: &std::path::Path, staged: &std::path::Path) -> bool {
    std::fs::rename(staged, current).is_ok()
}

#[cfg(windows)]
fn apply_staged(current: &std::path::Path, staged: &std::path::Path) -> bool {
    std::fs::copy(staged, current).is_ok()
}

#[cfg(not(any(unix, windows)))]
fn apply_staged(_current: &std::path::Path, _staged: &std::path::Path) -> bool {
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_gt_detects_newer() {
        assert!(semver_gt("0.2.0", "0.1.0"));
        assert!(semver_gt("1.0.0", "0.9.9"));
        assert!(!semver_gt("0.1.0", "0.1.0"));
        assert!(!semver_gt("0.0.9", "0.1.0"));
    }

    #[test]
    fn release_channel_filtering() {
        let stable = GhRelease {
            tag_name: "v0.2.0".into(),
            prerelease: false,
            draft: false,
            assets: vec![],
        };
        let nightly = GhRelease {
            tag_name: "nightly-20260520-1200".into(),
            prerelease: true,
            draft: false,
            assets: vec![],
        };
        let preview = GhRelease {
            tag_name: "v0.2.0-alpha.1".into(),
            prerelease: true,
            draft: false,
            assets: vec![],
        };

        assert!(release_matches_channel(&stable, UpdateChannel::Stable));
        assert!(!release_matches_channel(&stable, UpdateChannel::Nightly));
        assert!(!release_matches_channel(&stable, UpdateChannel::Preview));

        assert!(release_matches_channel(&nightly, UpdateChannel::Nightly));
        assert!(!release_matches_channel(&nightly, UpdateChannel::Stable));

        assert!(release_matches_channel(&preview, UpdateChannel::Preview));
        assert!(!release_matches_channel(&preview, UpdateChannel::Stable));
    }

    #[test]
    fn draft_releases_never_match() {
        let draft = GhRelease {
            tag_name: "v1.0.0".into(),
            prerelease: false,
            draft: true,
            assets: vec![],
        };
        assert!(!release_matches_channel(&draft, UpdateChannel::Stable));
        assert!(!release_matches_channel(&draft, UpdateChannel::Preview));
        assert!(!release_matches_channel(&draft, UpdateChannel::Nightly));
    }

    #[test]
    fn is_newer_stable_detects_greater_version() {
        // CURRENT_VERSION = "0.1.0" at test time
        assert!(is_newer(UpdateChannel::Stable, "v0.2.0"));
        assert!(is_newer(UpdateChannel::Stable, "0.2.0"));
        assert!(!is_newer(UpdateChannel::Stable, "v0.1.0"));
        assert!(!is_newer(UpdateChannel::Stable, "v0.0.9"));
    }

    #[test]
    fn is_newer_preview_uses_semver_comparison() {
        assert!(is_newer(UpdateChannel::Preview, "v0.2.0-alpha.1"));
        assert!(!is_newer(UpdateChannel::Preview, "v0.1.0-alpha.1"));
    }

    #[test]
    fn is_newer_nightly_returns_false_without_build_tag() {
        // FARSPACE_BUILD_TAG not set in test env → stable binary ignores nightlies
        assert!(!is_newer(UpdateChannel::Nightly, "nightly-20260520-1200"));
    }

    #[test]
    fn staged_path_has_update_extension() {
        let path = staged_path().expect("staged_path should succeed");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("update"));
    }

    #[test]
    fn check_and_apply_staged_returns_false_when_no_file() {
        let staged = staged_path().unwrap();
        if staged.exists() {
            return; // skip if a real staged file is present
        }
        assert!(!check_and_apply_staged());
    }

    #[test]
    #[cfg(unix)]
    fn apply_staged_moves_file_on_unix() {
        let dir = std::env::temp_dir();
        let counter = std::sync::atomic::AtomicUsize::new(0);
        let id = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let staged = dir.join(format!("farspace_test_staged_{}.update", id));
        let target = dir.join(format!("farspace_test_target_{}", id));

        std::fs::write(&staged, b"new binary").unwrap();
        std::fs::write(&target, b"old binary").unwrap();

        let result = apply_staged(&target, &staged);
        assert!(result, "apply_staged should succeed");

        let content = std::fs::read(&target).unwrap_or_default();
        assert_eq!(content, b"new binary");

        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&staged);
    }

    #[test]
    fn preview_tag_without_alpha_beta_does_not_match_preview_channel() {
        let strange = GhRelease {
            tag_name: "v0.2.0-rc.1".into(),
            prerelease: true,
            draft: false,
            assets: vec![],
        };
        assert!(!release_matches_channel(&strange, UpdateChannel::Preview));
        assert!(!release_matches_channel(&strange, UpdateChannel::Nightly));
        assert!(!release_matches_channel(&strange, UpdateChannel::Stable));
    }

    #[test]
    #[cfg(unix)]
    fn check_and_apply_staged_applies_and_removes_staged_file() {
        use std::env;

        // Write a dummy staged file next to a fake "current exe" path.
        // We can't replace the real exe in tests, so we test apply_staged directly
        // with temp paths instead.
        let dir = env::temp_dir();
        let staged = dir.join("farspace_staged_apply_test.update");
        let target = dir.join("farspace_staged_apply_target");

        std::fs::write(&staged, b"updated binary").unwrap();
        std::fs::write(&target, b"original binary").unwrap();

        let applied = apply_staged(&target, &staged);
        assert!(applied);

        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "updated binary");

        // staged should have been moved, not copied
        assert!(!staged.exists());

        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn platform_asset_name_is_nonempty_on_known_platforms() {
        let name = platform_asset_name();
        // On CI / developer machines this runs on a supported platform
        assert!(
            !name.is_empty()
                || cfg!(not(any(
                    target_os = "linux",
                    target_os = "macos",
                    target_os = "windows"
                ))),
            "expected non-empty asset name on supported platform"
        );
    }
}
