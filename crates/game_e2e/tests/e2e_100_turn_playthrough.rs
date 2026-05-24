use game_e2e::{default_scenario, run_e2e_scenario};

#[test]
fn e2e_100_turn_playthrough() {
    let report = run_e2e_scenario(default_scenario()).expect("E2E scenario should run");

    assert_eq!(
        report.turns_completed, report.turns_requested,
        "turn progression stopped early (see target/e2e/playthrough-report.md)"
    );

    assert!(
        !report.has_blocking_failures(),
        "blocking E2E failures detected (see target/e2e/playthrough-report.md)"
    );
}

#[test]
fn e2e_100_turn_playthrough_is_deterministic() {
    let a = run_e2e_scenario(default_scenario()).expect("first run should succeed");
    let b = run_e2e_scenario(default_scenario()).expect("second run should succeed");

    assert_eq!(a.state_hash_per_turn, b.state_hash_per_turn);
}
