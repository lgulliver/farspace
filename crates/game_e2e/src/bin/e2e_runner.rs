use game_core::GalaxySize;
use game_e2e::{run_e2e_scenario, scenario::E2eScenario, simulated_player::SimulatedPlayerPolicy};

fn main() -> anyhow::Result<()> {
    let mut scenario = E2eScenario::default();
    let mut report_override: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                if let Some(seed) = args.next() {
                    scenario.seed = seed.parse()?;
                }
            }
            "--turns" => {
                if let Some(turns) = args.next() {
                    scenario.max_turns = turns.parse()?;
                }
            }
            "--report" => {
                report_override = args.next();
            }
            "--galaxy" => {
                if let Some(size) = args.next() {
                    scenario.galaxy_size = match size.to_ascii_lowercase().as_str() {
                        "tiny" => GalaxySize::Tiny,
                        "small" => GalaxySize::Small,
                        "large" => GalaxySize::Large,
                        "huge" => GalaxySize::Huge,
                        "epic" => GalaxySize::Epic,
                        "custom" => GalaxySize::Custom,
                        _ => GalaxySize::Medium,
                    };
                }
            }
            "--ai" => {
                if let Some(ai) = args.next() {
                    scenario.ai_empire_count = ai.parse()?;
                }
            }
            _ => {}
        }
    }

    scenario.player_policy = SimulatedPlayerPolicy::BalancedExplorer;
    let report = run_e2e_scenario(scenario)?;

    if let Some(path) = report_override {
        let json = serde_json::to_vec_pretty(&report)?;
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
    }

    println!(
        "E2E complete: turns={}/{} failures={} warnings={}",
        report.turns_completed,
        report.turns_requested,
        report.failures.len(),
        report.warnings.len()
    );

    Ok(())
}
