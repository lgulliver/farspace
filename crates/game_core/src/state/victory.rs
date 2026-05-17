use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Deterministic victory-path ordering. Earlier variants win ties.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryPath {
    Dominion,
    Ascendancy,
    Prosperity,
    Discovery,
    Unity,
}

impl VictoryPath {
    pub fn label(self) -> &'static str {
        match self {
            VictoryPath::Dominion => "Dominion",
            VictoryPath::Ascendancy => "Ascendancy",
            VictoryPath::Prosperity => "Prosperity",
            VictoryPath::Discovery => "Discovery",
            VictoryPath::Unity => "Unity",
        }
    }

    pub fn tie_break_order() -> &'static [VictoryPath] {
        &[
            VictoryPath::Dominion,
            VictoryPath::Ascendancy,
            VictoryPath::Prosperity,
            VictoryPath::Discovery,
            VictoryPath::Unity,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryCondition {
    Dominion {
        control_percent_required: u8,
        allow_elimination: bool,
    },
    Ascendancy {
        required_victory_techs: u32,
        victory_tech_ids: Vec<TechId>,
    },
    Prosperity {
        population_required: u64,
        credits_required: i64,
        connected_colonies_required: u32,
        avg_stability_required: u8,
        food_surplus_required: Option<i64>,
    },
    Discovery {
        systems_explored_percent_required: u8,
        planets_surveyed_percent_required: u8,
        required_tech_ids: Vec<TechId>,
    },
    Unity {
        contacted_empires_required: u32,
        non_war_relations_required: u32,
        connected_colonies_required: u32,
    },
}

impl VictoryCondition {
    pub fn path(&self) -> VictoryPath {
        match self {
            VictoryCondition::Dominion { .. } => VictoryPath::Dominion,
            VictoryCondition::Ascendancy { .. } => VictoryPath::Ascendancy,
            VictoryCondition::Prosperity { .. } => VictoryPath::Prosperity,
            VictoryCondition::Discovery { .. } => VictoryPath::Discovery,
            VictoryCondition::Unity { .. } => VictoryPath::Unity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictorySettings {
    pub enabled_paths: BTreeSet<VictoryPath>,
    pub conditions: Vec<VictoryCondition>,
}

impl VictorySettings {
    pub fn default_v1() -> Self {
        let enabled_paths = [
            VictoryPath::Dominion,
            VictoryPath::Ascendancy,
            VictoryPath::Prosperity,
            VictoryPath::Discovery,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let conditions = vec![
            VictoryCondition::Dominion {
                control_percent_required: 60,
                allow_elimination: true,
            },
            VictoryCondition::Ascendancy {
                required_victory_techs: 4,
                victory_tech_ids: vec![
                    TechId(34),
                    TechId(49),
                    TechId(50),
                    TechId(51),
                    TechId(59),
                    TechId(60),
                ],
            },
            VictoryCondition::Prosperity {
                population_required: 40,
                credits_required: 300,
                connected_colonies_required: 4,
                avg_stability_required: 95,
                food_surplus_required: Some(0),
            },
            VictoryCondition::Discovery {
                systems_explored_percent_required: 80,
                planets_surveyed_percent_required: 70,
                required_tech_ids: vec![TechId::HYPERSPACE_CARTOGRAPHY, TechId::SECTOR_CARTOGRAPHY],
            },
            VictoryCondition::Unity {
                contacted_empires_required: 2,
                non_war_relations_required: 2,
                connected_colonies_required: 3,
            },
        ];
        Self {
            enabled_paths,
            conditions,
        }
    }

    pub fn condition_for(&self, path: VictoryPath) -> Option<&VictoryCondition> {
        self.conditions
            .iter()
            .find(|condition| condition.path() == path)
    }

    pub fn is_enabled(&self, path: VictoryPath) -> bool {
        self.enabled_paths.contains(&path)
    }
}

impl Default for VictorySettings {
    fn default() -> Self {
        Self::default_v1()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VictoryProgressValue {
    Dominion {
        controlled_systems: u32,
        total_colonized_systems: u32,
        control_percent: u8,
        active_major_empires: u32,
    },
    Ascendancy {
        completed_victory_techs: u32,
        required_victory_techs: u32,
    },
    Prosperity {
        population: u64,
        population_required: u64,
        credits: i64,
        credits_required: i64,
        connected_colonies: u32,
        connected_colonies_required: u32,
        avg_stability: u8,
        avg_stability_required: u8,
        food_surplus: i64,
        food_surplus_required: Option<i64>,
    },
    Discovery {
        explored_systems_percent: u8,
        required_explored_systems_percent: u8,
        surveyed_planets_percent: u8,
        required_surveyed_planets_percent: u8,
        required_techs_total: u32,
        required_techs_completed: u32,
    },
    Unity {
        contacted_empires: u32,
        contacted_empires_required: u32,
        non_war_relations: u32,
        non_war_relations_required: u32,
        connected_colonies: u32,
        connected_colonies_required: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictoryProgress {
    pub path: VictoryPath,
    pub enabled: bool,
    pub condition: VictoryCondition,
    pub value: VictoryProgressValue,
    pub progress_percent: u8,
    pub achieved: bool,
    pub leading_empire: Option<EmpireId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VictoryStatus {
    #[cfg_attr(feature = "serde", serde(default))]
    pub progress: Vec<VictoryProgress>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub winner: Option<EmpireId>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub winning_path: Option<VictoryPath>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub turn_achieved: Option<u32>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub milestone_levels: BTreeMap<VictoryPath, u8>,
}
