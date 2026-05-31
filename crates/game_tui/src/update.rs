//! Update state types shared between the TUI and the update system in the binary crate.

/// Which release channel the user wants to track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateChannel {
    #[default]
    Stable,
    /// Alpha/beta prereleases (tags matching `v*-alpha*` or `v*-beta*`).
    Preview,
    /// Nightly pre-releases (tags matching `nightly-*`).
    Nightly,
}

impl UpdateChannel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Stable => "Stable",
            Self::Preview => "Preview",
            Self::Nightly => "Nightly",
        }
    }

    pub const fn config_value(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Preview => "preview",
            Self::Nightly => "nightly",
        }
    }

    pub fn from_config_value(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "stable" => Some(Self::Stable),
            "preview" | "alpha" | "beta" => Some(Self::Preview),
            "nightly" => Some(Self::Nightly),
            _ => None,
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Stable => Self::Preview,
            Self::Preview => Self::Nightly,
            Self::Nightly => Self::Stable,
        }
    }
}

/// Information about an available release.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub channel: UpdateChannel,
    /// Direct download URL for the binary asset matching the current platform.
    pub download_url: String,
}

/// State machine for the update lifecycle shown on the menu screen.
#[derive(Debug, Clone, Default)]
pub enum UpdateState {
    /// Nothing to show.
    #[default]
    Idle,
    /// Check running in background.
    Checking,
    /// Update found and waiting for user action (or auto-download queued).
    Available(UpdateInfo),
    /// Binary being downloaded.
    Downloading,
    /// Binary staged on disk; restart needed.
    Staged { version: String },
    /// User dismissed the notification.
    Dismissed,
    /// Something went wrong.
    Error(String),
}

/// What the update confirmation dialog will do when the user presses Y.
#[derive(Debug, Clone)]
pub enum UpdateConfirmKind {
    /// Download and stage the update.
    Download(UpdateInfo),
    /// Apply the already-staged update and restart the process.
    ApplyAndRestart { version: String },
}

impl UpdateConfirmKind {
    pub fn title(&self) -> &str {
        match self {
            Self::Download(_) => "Download Update?",
            Self::ApplyAndRestart { .. } => "Apply Update & Restart?",
        }
    }

    pub fn body(&self) -> String {
        match self {
            Self::Download(info) => format!(
                "Version {} is available.\nDownload and stage for installation?",
                info.version
            ),
            Self::ApplyAndRestart { version } => format!(
                "Version {} is staged and ready.\nApply now and restart FARSPACE?",
                version
            ),
        }
    }
}

impl UpdateState {
    /// Returns true if a notification widget should be shown on the menu screen.
    pub fn is_notifiable(&self) -> bool {
        matches!(
            self,
            Self::Available(_) | Self::Downloading | Self::Staged { .. } | Self::Error(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_cycle_roundtrip() {
        assert_eq!(
            UpdateChannel::Stable.next().next().next(),
            UpdateChannel::Stable
        );
    }

    #[test]
    fn channel_config_roundtrip() {
        for ch in [
            UpdateChannel::Stable,
            UpdateChannel::Preview,
            UpdateChannel::Nightly,
        ] {
            assert_eq!(
                UpdateChannel::from_config_value(ch.config_value()),
                Some(ch)
            );
        }
    }

    #[test]
    fn notifiable_states() {
        assert!(!UpdateState::Idle.is_notifiable());
        assert!(!UpdateState::Checking.is_notifiable());
        assert!(!UpdateState::Dismissed.is_notifiable());
        assert!(UpdateState::Downloading.is_notifiable());
        assert!(
            UpdateState::Staged {
                version: "v1".into()
            }
            .is_notifiable()
        );
        assert!(UpdateState::Error("x".into()).is_notifiable());
    }
}
