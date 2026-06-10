pub mod assertions;
pub mod render;
pub mod report;
pub mod scenario;
pub mod simulated_player;

use anyhow::Result;
use assertions::{
    assert_no_diplomacy_before_contact, stable_state_hash, validate_command_result,
    validate_events_and_dispatch, validate_game_state, validate_save_load_roundtrip,
    validate_visibility,
};
use game_core::{Command, Engine, Event};
use rand::SeedableRng;
use rand::rngs::ChaCha8Rng;
use report::{E2eCommandTrace, E2eFailureCategory, E2eRunReport, E2eSeverity};
use scenario::{E2eScenario, build_scenario_setup};
use serde_json::json;
use simulated_player::{
    BalancedExplorerPlayer, PlayerObservation, SimulatedPlayer, SimulatedPlayerPolicy,
};

pub fn default_scenario() -> E2eScenario {
    E2eScenario::default()
}

pub fn run_e2e_scenario(scenario: E2eScenario) -> Result<E2eRunReport> {
    let mut report = E2eRunReport::new(&scenario);
    let setup = build_scenario_setup(&scenario)?;
    let mut engine = Engine::new_from_setup(setup);

    let mut simulated_player = build_simulated_player(&scenario.player_policy);
    let mut rng = ChaCha8Rng::seed_from_u64(scenario.seed ^ 0x0E2E_5EED);

    for expected_turn in 1..=scenario.max_turns {
        if validate_game_state(&engine.state, expected_turn, &mut report).is_err() {
            break;
        }

        let rendered_texts =
            render::render_and_validate_major_screens(&engine, expected_turn, &mut report)?;
        let observation = PlayerObservation::from_state(&engine.state);
        let commands = simulated_player.choose_actions(&observation, &mut rng);

        for command in commands {
            let events = apply_command_and_record(&mut engine, expected_turn, command, &mut report);
            if validate_game_state(&engine.state, expected_turn, &mut report).is_err() {
                break;
            }
            for event in &events {
                report.record_event_sample(expected_turn, "command", event.to_log_message());
            }
        }

        let turn_before = engine.state.turn;
        let end_turn_events =
            apply_command_and_record(&mut engine, expected_turn, Command::EndTurn, &mut report);

        if engine.state.turn == turn_before {
            report.push_failure(
                expected_turn,
                E2eSeverity::Fatal,
                E2eFailureCategory::TurnProgressionBlocked,
                None,
                "turn did not advance after EndTurn",
                json!({"turn_before": turn_before, "turn_after": engine.state.turn}),
            );
            break;
        }

        validate_events_and_dispatch(&engine.state, expected_turn, &end_turn_events, &mut report);
        validate_visibility(&engine.state, expected_turn, &rendered_texts, &mut report);
        assert_no_diplomacy_before_contact(
            &engine.state,
            expected_turn,
            &rendered_texts,
            &mut report,
        );

        if expected_turn % 10 == 0 {
            validate_save_load_roundtrip(&engine, expected_turn, &mut report)?;
        }

        report.record_state_hash(expected_turn, stable_state_hash(&engine)?);
        report.turns_completed = expected_turn;

        if report
            .failures
            .iter()
            .any(|failure| matches!(failure.severity, E2eSeverity::Fatal))
        {
            break;
        }
    }

    report.write_outputs()?;
    Ok(report)
}

fn build_simulated_player(policy: &SimulatedPlayerPolicy) -> Box<dyn SimulatedPlayer> {
    match policy {
        SimulatedPlayerPolicy::BalancedExplorer => Box::new(BalancedExplorerPlayer::new()),
    }
}

fn apply_command_and_record(
    engine: &mut Engine,
    turn: u32,
    command: Command,
    report: &mut E2eRunReport,
) -> Vec<Event> {
    let events = engine.apply_turn(vec![command.clone()]);
    validate_command_result(turn, &command, &events, report);

    report.commands_issued.push(E2eCommandTrace {
        turn,
        command: format!("{command:?}"),
        event_count: events.len(),
        event_log: events.iter().map(Event::to_log_message).collect(),
        had_error: events.iter().any(Event::is_error),
    });

    events
}
