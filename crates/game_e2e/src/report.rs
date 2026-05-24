use crate::scenario::E2eScenario;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eCommandTrace {
    pub turn: u32,
    pub command: String,
    pub event_count: usize,
    pub event_log: Vec<String>,
    pub had_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateHashEntry {
    pub turn: u32,
    pub hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eEventSample {
    pub turn: u32,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub turn: u32,
    pub target: String,
    pub width: u16,
    pub height: u16,
    pub preview: String,
    pub snapshot_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eFailure {
    pub turn: u32,
    pub severity: E2eSeverity,
    pub category: E2eFailureCategory,
    pub screen: Option<String>,
    pub message: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum E2eSeverity {
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum E2eFailureCategory {
    Panic,
    InvalidGameState,
    CommandRejected,
    EventLogError,
    DispatchError,
    RenderFailure,
    NavigationFailure,
    VisibilityLeak,
    DiplomacyBeforeContact,
    TurnProgressionBlocked,
    SaveLoadFailure,
    NonDeterminism,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eRunReport {
    pub seed: u64,
    pub turns_requested: u32,
    pub turns_completed: u32,
    pub failures: Vec<E2eFailure>,
    pub warnings: Vec<E2eFailure>,
    pub visited_screens: Vec<String>,
    pub commands_issued: Vec<E2eCommandTrace>,
    pub state_hash_per_turn: Vec<StateHashEntry>,
    pub event_samples: Vec<E2eEventSample>,
    pub render_snapshots: Vec<RenderSnapshot>,
}

impl E2eRunReport {
    pub fn new(scenario: &E2eScenario) -> Self {
        Self {
            seed: scenario.seed,
            turns_requested: scenario.max_turns,
            turns_completed: 0,
            failures: Vec::new(),
            warnings: Vec::new(),
            visited_screens: Vec::new(),
            commands_issued: Vec::new(),
            state_hash_per_turn: Vec::new(),
            event_samples: Vec::new(),
            render_snapshots: Vec::new(),
        }
    }

    pub fn has_blocking_failures(&self) -> bool {
        self.failures
            .iter()
            .any(|failure| !matches!(failure.severity, E2eSeverity::Warning))
    }

    pub fn push_failure(
        &mut self,
        turn: u32,
        severity: E2eSeverity,
        category: E2eFailureCategory,
        screen: Option<String>,
        message: impl Into<String>,
        context: serde_json::Value,
    ) {
        let failure = E2eFailure {
            turn,
            severity,
            category,
            screen,
            message: message.into(),
            context,
        };
        if matches!(failure.severity, E2eSeverity::Warning) {
            self.warnings.push(failure);
        } else {
            self.failures.push(failure);
        }
    }

    pub fn record_state_hash(&mut self, turn: u32, hash: u64) {
        self.state_hash_per_turn.push(StateHashEntry { turn, hash });
    }

    pub fn record_screen_visit(&mut self, screen: impl Into<String>) {
        let screen = screen.into();
        if !self.visited_screens.contains(&screen) {
            self.visited_screens.push(screen);
        }
    }

    pub fn record_event_sample(&mut self, turn: u32, source: &str, message: String) {
        if self.event_samples.len() >= 300 {
            return;
        }
        self.event_samples.push(E2eEventSample {
            turn,
            source: source.to_string(),
            message,
        });
    }

    pub fn write_outputs(&self) -> anyhow::Result<()> {
        let base_dir = Self::workspace_target_e2e_dir();
        fs::create_dir_all(&base_dir).context("failed to create target/e2e")?;
        fs::create_dir_all(base_dir.join("snapshots"))
            .context("failed to create target/e2e/snapshots")?;

        let json_path = base_dir.join("playthrough-report.json");
        let md_path = base_dir.join("playthrough-report.md");

        let json_bytes = serde_json::to_vec_pretty(self)?;
        fs::write(&json_path, json_bytes).context("failed to write JSON report")?;
        fs::write(&md_path, self.as_markdown()).context("failed to write markdown report")?;

        Ok(())
    }

    pub fn snapshot_file_path(&self, turn: u32, target: &str, width: u16, height: u16) -> PathBuf {
        let safe_target = target
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>();
        Self::workspace_target_e2e_dir()
            .join("snapshots")
            .join(format!("turn-{turn:03}-{safe_target}-{width}x{height}.txt"))
    }

    pub fn as_markdown(&self) -> String {
        let first_failure = self.failures.first();
        let grouped = self.failures.iter().fold(
            BTreeMap::<E2eFailureCategory, Vec<&E2eFailure>>::new(),
            |mut acc, failure| {
                acc.entry(failure.category.clone())
                    .or_default()
                    .push(failure);
                acc
            },
        );

        let mut lines = vec![
            "# FARSPACE E2E 100-Turn Playthrough Report".to_string(),
            String::new(),
            format!("- Seed: `{}`", self.seed),
            format!("- Turns requested: `{}`", self.turns_requested),
            format!("- Turns completed: `{}`", self.turns_completed),
            format!("- Failure count: `{}`", self.failures.len()),
            format!("- Warning count: `{}`", self.warnings.len()),
            format!("- Last successful turn: `{}`", self.turns_completed),
            format!(
                "- First failure: `{}`",
                first_failure
                    .map(|failure| format!(
                        "turn {} {:?}: {}",
                        failure.turn, failure.category, failure.message
                    ))
                    .unwrap_or_else(|| "none".to_string())
            ),
            String::new(),
            "## Screens visited".to_string(),
            self.visited_screens
                .iter()
                .map(|screen| format!("- {screen}"))
                .collect::<Vec<_>>()
                .join("\n"),
            String::new(),
            "## Failures by category".to_string(),
        ];

        if grouped.is_empty() {
            lines.push("- none".to_string());
        } else {
            for (category, failures) in grouped {
                lines.push(format!("### {:?}", category));
                for failure in failures {
                    lines.push(format!(
                        "- Turn {} ({:?}) {}",
                        failure.turn, failure.severity, failure.message
                    ));
                    lines.push(format!("  - Context: `{}`", failure.context));
                }

                lines.push(String::new());
            }
        }

        lines.push("## Command trace (last 20)".to_string());
        for trace in self.commands_issued.iter().rev().take(20).rev() {
            lines.push(format!(
                "- Turn {} `{}` events={} error={} ",
                trace.turn, trace.command, trace.event_count, trace.had_error
            ));
        }

        lines.push(String::new());
        lines.push("## Event/dispatch samples (last 20)".to_string());
        for sample in self.event_samples.iter().rev().take(20).rev() {
            lines.push(format!(
                "- Turn {} [{}] {}",
                sample.turn, sample.source, sample.message
            ));
        }

        lines.push(String::new());
        lines.push("## Snapshot paths".to_string());
        for snapshot in &self.render_snapshots {
            if let Some(path) = &snapshot.snapshot_path {
                lines.push(format!("- {}", path));
            }
        }

        lines.join("\n")
    }

    pub fn brief_context(turn: u32, detail: impl Into<String>) -> serde_json::Value {
        json!({"turn": turn, "detail": detail.into()})
    }

    fn workspace_target_e2e_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("e2e")
    }
}
