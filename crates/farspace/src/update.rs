//! Update checking, downloading, and staging for FARSPACE.
//!
//! This module is the only place in the workspace that performs network I/O.
//! All HTTP work runs on detached threads so the TUI is never blocked.
//!
//! Security model: updates are only fetched from this repository's GitHub
//! release download URLs (enforced by [`is_trusted_release_url`]), responses
//! are size-capped, and a binary is only staged after its SHA-256 digest
//! matches the checksum asset published with the release. Releases without a
//! checksum asset are not offered as updates.

use game_tui::{UpdateChannel, UpdateInfo};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

type UpdateCheckRx = mpsc::Receiver<Result<Option<UpdateInfo>, String>>;
type DownloadTx = mpsc::SyncSender<UpdateInfo>;
type DownloadResultRx = mpsc::Receiver<Result<String, String>>;

const GITHUB_API: &str = "https://api.github.com/repos/lgulliver/farspace/releases";
/// Only assets served from this repository's release downloads are trusted.
/// The prefix pins scheme, host, and repository path, so a crafted
/// `browser_download_url` in the API response cannot point elsewhere.
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/lgulliver/farspace/releases/download/";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Size caps for network responses, to bound disk/memory use if the endpoint
/// misbehaves.
const MAX_API_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_CHECKSUM_BYTES: u64 = 64 * 1024;
const MAX_BINARY_BYTES: u64 = 256 * 1024 * 1024;

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
    is_newer_against(
        channel,
        candidate,
        CURRENT_VERSION,
        option_env!("FARSPACE_BUILD_TAG").unwrap_or(""),
    )
}

/// Testable inner implementation of [`is_newer`] with explicit `current_version`
/// and `build_tag` parameters.
fn is_newer_against(
    channel: UpdateChannel,
    candidate: &str,
    current_version: &str,
    build_tag: &str,
) -> bool {
    let candidate = candidate.trim_start_matches('v');
    match channel {
        UpdateChannel::Nightly => {
            // Tags: nightly-YYYYMMDD-HHMM  →  compare as strings (ISO order)
            if build_tag.is_empty() {
                return false; // stable binary, ignore nightly updates
            }
            candidate > build_tag.trim_start_matches('v')
        }
        UpdateChannel::Stable | UpdateChannel::Preview => {
            let current = current_version.trim_start_matches('v');
            semver_gt(candidate, current)
        }
    }
}

/// Semver greater-than including prerelease precedence (SemVer 2.0 §11).
///
/// Numeric components (MAJOR.MINOR.PATCH) are compared first. When equal, a
/// version without a prerelease suffix outranks one with a suffix, and
/// suffixes are compared identifier-by-identifier: numeric identifiers
/// numerically (so `alpha.10 > alpha.9`), alphanumeric ones lexically, with
/// numeric identifiers ranking below alphanumeric ones.
fn semver_gt(a: &str, b: &str) -> bool {
    semver_cmp(a, b) == std::cmp::Ordering::Greater
}

fn semver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse_nums(s: &str) -> [u64; 3] {
        let ver = s.split_once('-').map_or(s, |(v, _)| v);
        let mut parts = ver.split('.');
        [
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            parts.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        ]
    }
    fn parse_pre(s: &str) -> &str {
        s.split_once('-').map_or("", |(_, p)| p)
    }

    let nums = parse_nums(a).cmp(&parse_nums(b));
    if nums != std::cmp::Ordering::Equal {
        return nums;
    }
    prerelease_cmp(parse_pre(a), parse_pre(b))
}

fn prerelease_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a.is_empty(), b.is_empty()) {
        (true, true) => Ordering::Equal,
        // A release outranks any prerelease of the same numeric version.
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let mut ai = a.split('.');
            let mut bi = b.split('.');
            loop {
                match (ai.next(), bi.next()) {
                    (None, None) => return Ordering::Equal,
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(x), Some(y)) => {
                        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                            (Ok(xn), Ok(yn)) => xn.cmp(&yn),
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => x.cmp(y),
                        };
                        if ord != Ordering::Equal {
                            return ord;
                        }
                    }
                }
            }
        }
    }
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
// Download trust helpers
// ---------------------------------------------------------------------------

/// True when `url` points at this repository's GitHub release downloads.
fn is_trusted_release_url(url: &str) -> bool {
    url.starts_with(RELEASE_DOWNLOAD_PREFIX)
}

/// Find the checksum asset for `asset_name` in a release.
///
/// Supports a per-asset `<asset>.sha256` file or an aggregate `SHA256SUMS`.
fn find_checksum_asset<'a>(assets: &'a [GhAsset], asset_name: &str) -> Option<&'a GhAsset> {
    let per_asset = format!("{asset_name}.sha256");
    assets
        .iter()
        .find(|a| a.name == per_asset)
        .or_else(|| assets.iter().find(|a| a.name == "SHA256SUMS"))
}

/// Extract the expected SHA-256 hex digest for `asset_name` from checksum
/// file contents. Accepts `sha256sum` output ("HEX  name", with an optional
/// `*` binary marker) and bare single-digest files.
fn parse_expected_checksum(contents: &str, asset_name: &str) -> Option<String> {
    fn is_sha256_hex(s: &str) -> bool {
        s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
    }

    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(digest) = fields.next() else {
            continue;
        };
        if !is_sha256_hex(digest) {
            continue;
        }
        match fields.next() {
            // Bare digest line: only valid when it's the whole file's digest.
            None => return Some(digest.to_ascii_lowercase()),
            Some(name) => {
                if name.trim_start_matches('*') == asset_name {
                    return Some(digest.to_ascii_lowercase());
                }
            }
        }
    }
    None
}

/// Stream `reader` to `dest`, enforcing `max_bytes` and verifying the SHA-256
/// digest against `expected_hex` before the file is allowed to persist. On any
/// failure the partial file is removed.
fn write_verified(
    mut reader: impl Read,
    dest: &Path,
    expected_hex: &str,
    max_bytes: u64,
) -> anyhow::Result<()> {
    let result = (|| {
        let mut file = std::fs::File::create(dest)?;
        let mut hasher = Sha256::new();
        let mut total: u64 = 0;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            if total > max_bytes {
                anyhow::bail!("download exceeded the {max_bytes}-byte size limit");
            }
            hasher.update(&buf[..n]);
            file.write_all(&buf[..n])?;
        }
        file.flush()?;

        let digest = hasher.finalize();
        let mut digest_hex = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(digest_hex, "{byte:02x}");
        }
        if !digest_hex.eq_ignore_ascii_case(expected_hex) {
            anyhow::bail!("SHA-256 mismatch: expected {expected_hex}, got {digest_hex}");
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
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
    let (check_tx, check_rx) = mpsc::channel::<Result<Option<UpdateInfo>, String>>();
    let (request_tx, request_rx) = mpsc::sync_channel::<UpdateInfo>(1);
    let (result_tx, result_rx) = mpsc::channel::<Result<String, String>>();

    // Background update check
    thread::spawn(move || {
        let result = check_latest(channel).map_err(|e| e.to_string());
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

fn http_agent(global_timeout_secs: u64) -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .timeout_connect(Some(std::time::Duration::from_secs(10)))
            .timeout_global(Some(std::time::Duration::from_secs(global_timeout_secs)))
            .build(),
    )
}

/// Check GitHub releases for a newer version on the given channel.
/// Returns `Ok(Some(UpdateInfo))` if an update is available, `Ok(None)` if up-to-date,
/// or `Err` with a human-readable message if the check could not be completed.
fn check_latest(channel: UpdateChannel) -> anyhow::Result<Option<UpdateInfo>> {
    let asset_name = platform_asset_name();
    if asset_name.is_empty() {
        return Ok(None); // unsupported platform
    }

    let agent = http_agent(30);

    let response: Vec<GhRelease> = agent
        .get(GITHUB_API)
        .header("User-Agent", &format!("farspace/{CURRENT_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| anyhow::anyhow!("update check request failed: {e}"))?
        .into_body()
        .with_config()
        .limit(MAX_API_RESPONSE_BYTES)
        .read_json()
        .map_err(|e| anyhow::anyhow!("update check response parse failed: {e}"))?;

    let release = match response
        .into_iter()
        .find(|r| release_matches_channel(r, channel))
    {
        Some(r) => r,
        None => return Ok(None),
    };

    if !is_newer(channel, &release.tag_name) {
        return Ok(None);
    }

    let asset = match release.assets.iter().find(|a| a.name == asset_name) {
        Some(a) => a,
        None => return Ok(None),
    };

    // Updates are only offered when the release also publishes a checksum we
    // can verify the download against, and both URLs come from this repo.
    let checksum_asset = match find_checksum_asset(&release.assets, asset_name) {
        Some(a) => a,
        None => return Ok(None),
    };
    if !is_trusted_release_url(&asset.browser_download_url)
        || !is_trusted_release_url(&checksum_asset.browser_download_url)
    {
        return Ok(None);
    }

    Ok(Some(UpdateInfo {
        version: release.tag_name.clone(),
        channel,
        download_url: asset.browser_download_url.clone(),
        checksum_url: Some(checksum_asset.browser_download_url.clone()),
    }))
}

/// Download the update binary to a staging path next to the current executable.
/// The staged file has the current executable's extension replaced with `.update`
/// (e.g. `farspace` → `farspace.update`, `farspace.exe` → `farspace.update`).
/// It is applied on the next launch.
///
/// The download is written to a `.part` file first and only renamed to the
/// staged path after its SHA-256 digest matches the release checksum, so an
/// unverified binary never sits at the path the launcher applies.
pub fn download_and_stage(info: &UpdateInfo) -> anyhow::Result<PathBuf> {
    if !is_trusted_release_url(&info.download_url) {
        anyhow::bail!(
            "refusing update download from untrusted URL: {}",
            info.download_url
        );
    }
    let checksum_url = info.checksum_url.as_deref().ok_or_else(|| {
        anyhow::anyhow!("release has no SHA-256 checksum asset; refusing to stage")
    })?;
    if !is_trusted_release_url(checksum_url) {
        anyhow::bail!("refusing checksum download from untrusted URL: {checksum_url}");
    }

    let staging = staged_path()?;
    let agent = http_agent(300);

    let checksum_text = agent
        .get(checksum_url)
        .header("User-Agent", &format!("farspace/{CURRENT_VERSION}"))
        .call()?
        .into_body()
        .with_config()
        .limit(MAX_CHECKSUM_BYTES)
        .read_to_string()?;
    let expected = parse_expected_checksum(&checksum_text, platform_asset_name())
        .ok_or_else(|| anyhow::anyhow!("release checksum file has no entry for this platform"))?;

    let response = agent
        .get(&info.download_url)
        .header("User-Agent", &format!("farspace/{CURRENT_VERSION}"))
        .call()?;

    let partial = staging.with_extension("part");
    let reader = response.into_body().into_reader();
    write_verified(reader, &partial, &expected, MAX_BINARY_BYTES)?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&partial)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&partial, perms)?;
    }

    std::fs::rename(&partial, &staging)?;
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
/// On Windows: renames the running binary aside, then moves the staged file
/// into place (a running image cannot be overwritten in place).
///
/// Returns `true` if an update was applied (the caller should note that the
/// binary on disk has changed, though the running process is the old version).
pub fn check_and_apply_staged() -> bool {
    let Ok(staged) = staged_path() else {
        return false;
    };

    // Clean up the previous binary left aside by a Windows update.
    #[cfg(windows)]
    if let Ok(current) = std::env::current_exe() {
        let _ = std::fs::remove_file(current.with_extension("old"));
    }

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
    // Windows locks a running executable image, so copying over `current`
    // always fails. Renaming it is allowed: move it aside, then move the
    // staged binary into place; roll back if that second step fails.
    let old = current.with_extension("old");
    let _ = std::fs::remove_file(&old);
    if std::fs::rename(current, &old).is_err() {
        return false;
    }
    if std::fs::rename(staged, current).is_ok() {
        true
    } else {
        let _ = std::fs::rename(&old, current);
        false
    }
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

    static TEST_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[test]
    fn semver_gt_detects_newer() {
        assert!(semver_gt("0.2.0", "0.1.0"));
        assert!(semver_gt("1.0.0", "0.9.9"));
        assert!(!semver_gt("0.1.0", "0.1.0"));
        assert!(!semver_gt("0.0.9", "0.1.0"));
        // prerelease comparisons
        assert!(semver_gt("0.2.0-alpha.2", "0.2.0-alpha.1"));
        assert!(!semver_gt("0.2.0-alpha.1", "0.2.0-alpha.2"));
        assert!(!semver_gt("0.2.0-alpha.1", "0.2.0-alpha.1"));
    }

    #[test]
    fn semver_gt_orders_numeric_prerelease_identifiers_numerically() {
        assert!(semver_gt("0.2.0-alpha.10", "0.2.0-alpha.9"));
        assert!(!semver_gt("0.2.0-alpha.9", "0.2.0-alpha.10"));
    }

    #[test]
    fn semver_gt_release_outranks_prerelease_of_same_version() {
        assert!(semver_gt("0.2.0", "0.2.0-alpha.1"));
        assert!(!semver_gt("0.2.0-alpha.1", "0.2.0"));
    }

    #[test]
    fn semver_gt_prerelease_identifier_rules() {
        // alphanumeric identifiers compare lexically
        assert!(semver_gt("0.2.0-beta.1", "0.2.0-alpha.9"));
        // numeric identifiers rank below alphanumeric ones
        assert!(semver_gt("0.2.0-alpha.rc", "0.2.0-alpha.1"));
        // longer identifier list outranks an equal prefix
        assert!(semver_gt("0.2.0-alpha.1.1", "0.2.0-alpha.1"));
    }

    #[test]
    fn trusted_release_url_allowlist() {
        assert!(is_trusted_release_url(
            "https://github.com/lgulliver/farspace/releases/download/v0.2.0/farspace-linux-x86_64"
        ));
        // wrong scheme
        assert!(!is_trusted_release_url(
            "http://github.com/lgulliver/farspace/releases/download/v0.2.0/x"
        ));
        // wrong host
        assert!(!is_trusted_release_url(
            "https://evil.example.com/lgulliver/farspace/releases/download/v0.2.0/x"
        ));
        // host suffix trick
        assert!(!is_trusted_release_url(
            "https://github.com.evil.example/lgulliver/farspace/releases/download/v0.2.0/x"
        ));
        // wrong repository
        assert!(!is_trusted_release_url(
            "https://github.com/attacker/farspace/releases/download/v0.2.0/x"
        ));
    }

    #[test]
    fn checksum_parsing_supports_common_formats() {
        let digest = "a".repeat(64);
        // sha256sum two-column output, including '*' binary marker
        let sums = format!(
            "{digest}  farspace-linux-x86_64\n{}  other-asset\n",
            "b".repeat(64)
        );
        assert_eq!(
            parse_expected_checksum(&sums, "farspace-linux-x86_64").as_deref(),
            Some(digest.as_str())
        );
        let starred = format!("{digest} *farspace-linux-x86_64\n");
        assert_eq!(
            parse_expected_checksum(&starred, "farspace-linux-x86_64").as_deref(),
            Some(digest.as_str())
        );
        // bare single-digest file
        assert_eq!(
            parse_expected_checksum(&format!("{digest}\n"), "anything").as_deref(),
            Some(digest.as_str())
        );
    }

    #[test]
    fn checksum_parsing_rejects_missing_or_malformed_entries() {
        let digest = "c".repeat(64);
        let sums = format!("{digest}  some-other-asset\n");
        assert_eq!(
            parse_expected_checksum(&sums, "farspace-linux-x86_64"),
            None
        );
        // not 64 hex chars
        assert_eq!(
            parse_expected_checksum("deadbeef  farspace-linux-x86_64", "farspace-linux-x86_64"),
            None
        );
        assert_eq!(parse_expected_checksum("", "farspace-linux-x86_64"), None);
    }

    #[test]
    fn write_verified_accepts_matching_digest_and_rejects_mismatch() {
        let dir = std::env::temp_dir();
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let dest = dir.join(format!("farspace_verify_test_{}_{}", pid, id));

        let payload = b"farspace update payload";
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let expected: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        write_verified(&payload[..], &dest, &expected, 1024).expect("matching digest accepted");
        assert_eq!(std::fs::read(&dest).unwrap(), payload);
        let _ = std::fs::remove_file(&dest);

        let wrong = "0".repeat(64);
        let err = write_verified(&payload[..], &dest, &wrong, 1024).unwrap_err();
        assert!(err.to_string().contains("SHA-256 mismatch"));
        assert!(!dest.exists(), "partial file must be removed on mismatch");
    }

    #[test]
    fn write_verified_enforces_size_cap() {
        let dir = std::env::temp_dir();
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let dest = dir.join(format!("farspace_size_cap_test_{}_{}", pid, id));

        let payload = vec![0u8; 2048];
        let err = write_verified(&payload[..], &dest, &"0".repeat(64), 1024).unwrap_err();
        assert!(err.to_string().contains("size limit"));
        assert!(!dest.exists());
    }

    #[test]
    fn download_and_stage_refuses_untrusted_or_unverifiable_updates() {
        // Untrusted download URL — rejected before any network I/O.
        let info = UpdateInfo {
            version: "v9.9.9".into(),
            channel: UpdateChannel::Stable,
            download_url: "https://evil.example.com/farspace".into(),
            checksum_url: Some(format!("{RELEASE_DOWNLOAD_PREFIX}v9.9.9/SHA256SUMS")),
        };
        let err = download_and_stage(&info).unwrap_err();
        assert!(err.to_string().contains("untrusted URL"));

        // Missing checksum — rejected.
        let info = UpdateInfo {
            version: "v9.9.9".into(),
            channel: UpdateChannel::Stable,
            download_url: format!("{RELEASE_DOWNLOAD_PREFIX}v9.9.9/farspace-linux-x86_64"),
            checksum_url: None,
        };
        let err = download_and_stage(&info).unwrap_err();
        assert!(err.to_string().contains("no SHA-256 checksum"));

        // Untrusted checksum URL — rejected.
        let info = UpdateInfo {
            version: "v9.9.9".into(),
            channel: UpdateChannel::Stable,
            download_url: format!("{RELEASE_DOWNLOAD_PREFIX}v9.9.9/farspace-linux-x86_64"),
            checksum_url: Some("https://evil.example.com/SHA256SUMS".into()),
        };
        let err = download_and_stage(&info).unwrap_err();
        assert!(err.to_string().contains("untrusted URL"));
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
    fn checksum_asset_lookup_prefers_per_asset_file() {
        let assets = vec![
            GhAsset {
                name: "SHA256SUMS".into(),
                browser_download_url: format!("{RELEASE_DOWNLOAD_PREFIX}v1/SHA256SUMS"),
            },
            GhAsset {
                name: "farspace-linux-x86_64.sha256".into(),
                browser_download_url: format!(
                    "{RELEASE_DOWNLOAD_PREFIX}v1/farspace-linux-x86_64.sha256"
                ),
            },
        ];
        let found = find_checksum_asset(&assets, "farspace-linux-x86_64").unwrap();
        assert_eq!(found.name, "farspace-linux-x86_64.sha256");

        let only_sums = &assets[..1];
        let found = find_checksum_asset(only_sums, "farspace-linux-x86_64").unwrap();
        assert_eq!(found.name, "SHA256SUMS");

        assert!(find_checksum_asset(&[], "farspace-linux-x86_64").is_none());
    }

    #[test]
    fn is_newer_stable_detects_greater_version() {
        // Use is_newer_against with explicit versions to avoid hardcoding CURRENT_VERSION
        assert!(is_newer_against(
            UpdateChannel::Stable,
            "v0.2.0",
            "0.1.0",
            ""
        ));
        assert!(is_newer_against(
            UpdateChannel::Stable,
            "0.2.0",
            "0.1.0",
            ""
        ));
        assert!(!is_newer_against(
            UpdateChannel::Stable,
            "v0.1.0",
            "0.1.0",
            ""
        ));
        assert!(!is_newer_against(
            UpdateChannel::Stable,
            "v0.0.9",
            "0.1.0",
            ""
        ));
    }

    #[test]
    fn is_newer_preview_uses_semver_comparison() {
        assert!(is_newer_against(
            UpdateChannel::Preview,
            "v0.2.0-alpha.1",
            "0.1.0",
            ""
        ));
        assert!(!is_newer_against(
            UpdateChannel::Preview,
            "v0.1.0-alpha.1",
            "0.2.0",
            ""
        ));
        // Same base version, later prerelease
        assert!(is_newer_against(
            UpdateChannel::Preview,
            "v0.2.0-alpha.2",
            "0.2.0-alpha.1",
            ""
        ));
        assert!(!is_newer_against(
            UpdateChannel::Preview,
            "v0.2.0-alpha.1",
            "0.2.0-alpha.2",
            ""
        ));
        // A stable binary on the preview channel ignores prereleases of its
        // own version (release outranks prerelease).
        assert!(!is_newer_against(
            UpdateChannel::Preview,
            "v0.2.0-alpha.1",
            "0.2.0",
            ""
        ));
    }

    #[test]
    fn is_newer_nightly_returns_false_without_build_tag() {
        // No build tag → stable binary ignores nightly updates
        assert!(!is_newer_against(
            UpdateChannel::Nightly,
            "nightly-20260520-1200",
            "0.1.0",
            ""
        ));
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
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let staged = dir.join(format!("farspace_test_staged_{}_{}.update", pid, id));
        let target = dir.join(format!("farspace_test_target_{}_{}", pid, id));

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
    #[cfg(windows)]
    fn apply_staged_renames_running_binary_aside_on_windows() {
        let dir = std::env::temp_dir();
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let staged = dir.join(format!("farspace_test_staged_{}_{}.update", pid, id));
        let target = dir.join(format!("farspace_test_target_{}_{}.exe", pid, id));

        std::fs::write(&staged, b"new binary").unwrap();
        std::fs::write(&target, b"old binary").unwrap();

        assert!(apply_staged(&target, &staged));
        assert_eq!(std::fs::read(&target).unwrap(), b"new binary");
        // Old binary parked alongside for cleanup on next launch.
        let old = target.with_extension("old");
        assert_eq!(std::fs::read(&old).unwrap(), b"old binary");
        assert!(!staged.exists());

        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&old);
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
        let id = TEST_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        let staged = dir.join(format!("farspace_staged_apply_test_{}_{}.update", pid, id));
        let target = dir.join(format!("farspace_staged_apply_target_{}_{}", pid, id));

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
