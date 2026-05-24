use anyhow::{anyhow, Result};
use game_core::{GalaxySize, ScenarioSetup};
use serde::{Deserialize, Serialize};

use crate::simulated_player::SimulatedPlayerPolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2eScenario {
    pub seed: u64,
    pub max_turns: u32,
    pub galaxy_size: GalaxySize,
    pub ai_empire_count: u8,
    pub player_policy: SimulatedPlayerPolicy,
}

impl Default for E2eScenario {
    fn default() -> Self {
        Self {
            seed: 12_345,
            max_turns: 100,
            galaxy_size: GalaxySize::Medium,
            ai_empire_count: 4,
            player_policy: SimulatedPlayerPolicy::BalancedExplorer,
        }
    }
}

pub fn build_scenario_setup(scenario: &E2eScenario) -> Result<ScenarioSetup> {
    if scenario.max_turns == 0 {
        return Err(anyhow!("max_turns must be > 0"));
    }

    let mut setup = ScenarioSetup::default_for_seed(scenario.seed);
    setup.galaxy_size = scenario.galaxy_size;
    setup.ai_empire_count = scenario.ai_empire_count;
    setup
        .validate()
        .map_err(|error| anyhow!("invalid E2E setup: {error}"))?;
    Ok(setup)
}
