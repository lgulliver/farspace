use crate::report::{E2eFailureCategory, E2eRunReport, E2eSeverity};
use anyhow::{anyhow, Context, Result};
use game_core::{
    Command, DiplomaticCommunicationType, Engine, Event, GameState, RelationshipStatus,
};
use serde_json::json;
use std::collections::{hash_map::DefaultHasher, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

const FORBIDDEN_UI_STRINGS: &[&str] = &[
    "TODO",
    "FIXME",
    "unimplemented",
    "panic",
    "panicked",
    "unwrap failed",
    "<missing>",
    "<unknown>",
    "NaN",
    "inf",
];

pub fn validate_game_state(state: &GameState, turn: u32, report: &mut E2eRunReport) -> Result<()> {
    if state.turn == 0 {
        report.push_failure(
            turn,
            E2eSeverity::Fatal,
            E2eFailureCategory::InvalidGameState,
            None,
            "game turn is zero",
            json!({"state_turn": state.turn}),
        );
        return Err(anyhow!("invalid state turn"));
    }

    if !state.empires.contains_key(&state.player_empire) {
        report.push_failure(
            turn,
            E2eSeverity::Fatal,
            E2eFailureCategory::InvalidGameState,
            None,
            "player empire missing",
            json!({"player_empire": state.player_empire.0}),
        );
        return Err(anyhow!("player empire missing"));
    }

    for colony in state.colonies.values() {
        if !state.empires.contains_key(&colony.owner) {
            return fail_state_link(
                turn,
                report,
                "colony owner missing",
                json!({"colony": colony.id.0, "owner": colony.owner.0}),
            );
        }
        let Some(star) = state.stars.get(&colony.star) else {
            return fail_state_link(
                turn,
                report,
                "colony star missing",
                json!({"colony": colony.id.0, "star": colony.star.0}),
            );
        };
        if colony.planet_index >= star.planets.len() {
            return fail_state_link(
                turn,
                report,
                "colony planet index out of bounds",
                json!({
                    "colony": colony.id.0,
                    "star": colony.star.0,
                    "planet_index": colony.planet_index,
                    "planet_count": star.planets.len()
                }),
            );
        }
    }

    for fleet in state.fleets.values() {
        if !state.empires.contains_key(&fleet.owner) {
            return fail_state_link(
                turn,
                report,
                "fleet owner missing",
                json!({"fleet": fleet.id.0, "owner": fleet.owner.0}),
            );
        }
        if !state.stars.contains_key(&fleet.location) {
            return fail_state_link(
                turn,
                report,
                "fleet location missing",
                json!({"fleet": fleet.id.0, "location": fleet.location.0}),
            );
        }
    }

    Ok(())
}

pub fn validate_command_result(
    turn: u32,
    command: &Command,
    events: &[Event],
    report: &mut E2eRunReport,
) {
    let had_error = events.iter().any(Event::is_error);
    if had_error {
        report.push_failure(
            turn,
            E2eSeverity::Error,
            E2eFailureCategory::CommandRejected,
            None,
            format!("command produced error event: {command:?}"),
            json!({
                "command": format!("{command:?}"),
                "events": events.iter().map(|event| event.to_log_message()).collect::<Vec<_>>()
            }),
        );
    }
}

pub fn validate_events_and_dispatch(
    state: &GameState,
    turn: u32,
    events: &[Event],
    report: &mut E2eRunReport,
) {
    for event in events {
        let message = event.to_log_message();
        if message.to_ascii_lowercase().contains("debug") {
            report.push_failure(
                turn,
                E2eSeverity::Error,
                E2eFailureCategory::EventLogError,
                None,
                "event contains debug/internal text",
                json!({"event": format!("{event:?}"), "message": message}),
            );
        }

        if event.is_error() {
            report.push_failure(
                turn,
                E2eSeverity::Error,
                E2eFailureCategory::EventLogError,
                None,
                "error event emitted",
                json!({"event": format!("{event:?}"), "message": message}),
            );
        }
    }

    if state
        .event_log
        .iter()
        .rev()
        .take(20)
        .any(|entry| entry.to_ascii_lowercase().starts_with("error:"))
    {
        report.push_failure(
            turn,
            E2eSeverity::Error,
            E2eFailureCategory::EventLogError,
            None,
            "player event log contains error entries",
            json!({"recent_entries": state.event_log.iter().rev().take(20).cloned().collect::<Vec<_>>() }),
        );
    }

    for dispatch in state.galactic_dispatches.iter().rev().take(3) {
        let mut seen = HashSet::new();
        for item in &dispatch.items {
            if !seen.insert(item.headline.clone()) {
                report.push_failure(
                    turn,
                    E2eSeverity::Warning,
                    E2eFailureCategory::DispatchError,
                    None,
                    "duplicate dispatch headline",
                    json!({"headline": item.headline, "turn": dispatch.turn}),
                );
            }
        }
    }
}

pub fn validate_render_text(
    turn: u32,
    target: &str,
    text: &str,
    width: u16,
    height: u16,
    report: &mut E2eRunReport,
) {
    if text.trim().is_empty() {
        report.push_failure(
            turn,
            E2eSeverity::Error,
            E2eFailureCategory::RenderFailure,
            Some(target.to_string()),
            "rendered output is empty",
            json!({"width": width, "height": height}),
        );
    }

    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() > height as usize {
        report.push_failure(
            turn,
            E2eSeverity::Error,
            E2eFailureCategory::RenderFailure,
            Some(target.to_string()),
            "rendered output exceeded expected height",
            json!({"line_count": lines.len(), "height": height}),
        );
    }

    for (index, line) in lines.iter().enumerate() {
        if line.chars().count() > width as usize {
            report.push_failure(
                turn,
                E2eSeverity::Error,
                E2eFailureCategory::RenderFailure,
                Some(target.to_string()),
                "rendered output exceeded expected width",
                json!({"line": index, "line_len": line.chars().count(), "width": width}),
            );
        }
    }

    let lowered = text.to_ascii_lowercase();
    for forbidden in FORBIDDEN_UI_STRINGS {
        if lowered.contains(&forbidden.to_ascii_lowercase()) {
            report.push_failure(
                turn,
                E2eSeverity::Error,
                E2eFailureCategory::RenderFailure,
                Some(target.to_string()),
                "forbidden UI string rendered",
                json!({"forbidden": forbidden}),
            );
        }
    }
}

pub fn assert_no_diplomacy_before_contact(
    state: &GameState,
    turn: u32,
    render_texts: &[String],
    report: &mut E2eRunReport,
) {
    let player = state.player_empire;
    let mut unknown_ai = BTreeSet::new();

    for (empire_id, empire) in &state.empires {
        if *empire_id == player {
            continue;
        }
        if state.relationship_status(player, *empire_id) == RelationshipStatus::Unknown {
            unknown_ai.insert((*empire_id, empire.name.clone()));
        }
    }

    for communication in &state.diplomacy_pending_communications {
        if communication.receiving_empire != player {
            continue;
        }
        let relation = state.relationship_status(player, communication.sending_empire);
        if relation == RelationshipStatus::Unknown
            && communication.communication_type != DiplomaticCommunicationType::FirstContact
        {
            report.push_failure(
                turn,
                E2eSeverity::Error,
                E2eFailureCategory::DiplomacyBeforeContact,
                Some("Diplomacy".to_string()),
                "non-first-contact communication before contact",
                json!({
                    "communication_id": communication.communication_id,
                    "type": format!("{:?}", communication.communication_type),
                    "from_empire": communication.sending_empire.0,
                    "to_empire": communication.receiving_empire.0
                }),
            );
        }
    }

    for (_, empire_name) in unknown_ai {
        for text in render_texts {
            if text.contains(&empire_name) {
                report.push_failure(
                    turn,
                    E2eSeverity::Error,
                    E2eFailureCategory::DiplomacyBeforeContact,
                    None,
                    "unknown empire name shown in rendered UI",
                    json!({"empire_name": empire_name, "visible_text": trim_for_context(text)}),
                );
            }
        }
    }
}

pub fn validate_visibility(
    state: &GameState,
    turn: u32,
    render_texts: &[String],
    report: &mut E2eRunReport,
) {
    let player = state.player_empire;
    let unknown_empire_names = state
        .empires
        .iter()
        .filter(|(id, _)| **id != player)
        .filter(|(id, _)| state.relationship_status(player, **id) == RelationshipStatus::Unknown)
        .map(|(_, empire)| empire.name.clone())
        .collect::<Vec<_>>();

    for unknown_name in unknown_empire_names {
        let lower_name = unknown_name.to_ascii_lowercase();
        for text in render_texts {
            let lower_text = text.to_ascii_lowercase();
            if lower_text.contains(&lower_name)
                && !lower_text.contains("first contact")
                && !lower_text.contains("unknown contact")
                && !lower_text.contains("rumor")
                && !lower_text.contains("sensor contact")
            {
                report.push_failure(
                    turn,
                    E2eSeverity::Error,
                    E2eFailureCategory::VisibilityLeak,
                    None,
                    "unknown AI empire name leaked to player-visible UI",
                    json!({"empire_name": unknown_name, "visible_text": trim_for_context(text)}),
                );
            }
        }

        for log_entry in state.event_log.iter().rev().take(30) {
            let lower_log = log_entry.to_ascii_lowercase();
            if lower_log.contains(&lower_name)
                && !lower_log.contains("first contact")
                && !lower_log.contains("unknown contact")
            {
                report.push_failure(
                    turn,
                    E2eSeverity::Error,
                    E2eFailureCategory::VisibilityLeak,
                    None,
                    "unknown AI empire name leaked to event log",
                    json!({"empire_name": unknown_name, "event_log_entry": log_entry}),
                );
            }
        }
    }
}

pub fn validate_save_load_roundtrip(
    engine: &Engine,
    turn: u32,
    report: &mut E2eRunReport,
) -> Result<()> {
    let before = stable_state_hash_state(&engine.state)?;
    let saved = game_save::save_to_string(&engine.state).context("failed to save state")?;
    let loaded = game_save::load_from_string(&saved).context("failed to load saved state")?;
    let after = stable_state_hash_state(&loaded)?;

    if before != after {
        report.push_failure(
            turn,
            E2eSeverity::Error,
            E2eFailureCategory::SaveLoadFailure,
            None,
            "save/load roundtrip hash drift",
            json!({"before": before, "after": after}),
        );
    }

    Ok(())
}

pub fn stable_state_hash(engine: &Engine) -> Result<u64> {
    stable_state_hash_state(&engine.state)
}

pub fn stable_state_hash_state(state: &GameState) -> Result<u64> {
    let saved = game_save::save_to_string(state)?;
    let mut value: serde_json::Value = serde_json::from_str(&saved).context("state to json")?;
    if let Some(object) = value.as_object_mut() {
        object.remove("metadata");
    }
    let canonical = serde_json::to_string(&value)?;
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    Ok(hasher.finish())
}

fn fail_state_link(
    turn: u32,
    report: &mut E2eRunReport,
    message: &str,
    context: serde_json::Value,
) -> Result<()> {
    report.push_failure(
        turn,
        E2eSeverity::Fatal,
        E2eFailureCategory::InvalidGameState,
        None,
        message,
        context,
    );
    Err(anyhow!(message.to_string()))
}

fn trim_for_context(text: &str) -> String {
    let mut chars = text.chars();
    let sample = chars.by_ref().take(240).collect::<String>();
    if chars.next().is_some() {
        format!("{sample}...")
    } else {
        sample
    }
}
