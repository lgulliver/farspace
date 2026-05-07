//! FARSPACE game content - static game data
//!
//! This crate contains static game content such as planet traits,
//! ship templates, and tech trees.

use game_core::state::{BuildItem, PlanetSize};

/// A trait that can be assigned to a planet
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetTrait {
    pub name: String,
    pub description: String,
    pub production_modifier: i32,
    pub research_modifier: i32,
    pub habitability_modifier: i32,
}

impl PlanetTrait {
    /// Get the default set of planet traits
    pub fn defaults() -> Vec<PlanetTrait> {
        vec![
            PlanetTrait {
                name: "Mineral Rich".to_string(),
                description: "Abundant mineral deposits increase production.".to_string(),
                production_modifier: 25,
                research_modifier: 0,
                habitability_modifier: 0,
            },
            PlanetTrait {
                name: "Ancient Ruins".to_string(),
                description: "Ruins of an ancient civilization boost research.".to_string(),
                production_modifier: 0,
                research_modifier: 25,
                habitability_modifier: 0,
            },
            PlanetTrait {
                name: "Hostile Environment".to_string(),
                description: "Harsh conditions reduce habitability.".to_string(),
                production_modifier: 0,
                research_modifier: 0,
                habitability_modifier: -25,
            },
            PlanetTrait {
                name: "Garden World".to_string(),
                description: "Ideal conditions for colonization.".to_string(),
                production_modifier: 10,
                research_modifier: 10,
                habitability_modifier: 25,
            },
        ]
    }
}

/// A ship design template
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipTemplate {
    pub name: String,
    pub build_item: BuildItem,
    pub cost: u64,
    pub attack: u32,
    pub defense: u32,
    pub speed: u32,
    pub range: u32,
}

impl ShipTemplate {
    /// Get the default ship templates
    pub fn defaults() -> Vec<ShipTemplate> {
        vec![
            ShipTemplate {
                name: "Scout".to_string(),
                build_item: BuildItem::Ship(game_core::ShipDesignId::SCOUT),
                cost: BuildItem::Ship(game_core::ShipDesignId::SCOUT).cost(),
                attack: 1,
                defense: 1,
                speed: 3,
                range: 5,
            },
            ShipTemplate {
                name: "Colony Ship".to_string(),
                build_item: BuildItem::Ship(game_core::ShipDesignId::COLONY),
                cost: BuildItem::Ship(game_core::ShipDesignId::COLONY).cost(),
                attack: 0,
                defense: 1,
                speed: 2,
                range: 3,
            },
            ShipTemplate {
                name: "Science Ship".to_string(),
                build_item: BuildItem::Ship(game_core::ShipDesignId::SCIENCE),
                cost: BuildItem::Ship(game_core::ShipDesignId::SCIENCE).cost(),
                attack: 0,
                defense: 1,
                speed: 2,
                range: 4,
            },
        ]
    }
}

/// A technology that can be researched
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Technology {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub cost: i64,
    pub prerequisites: Vec<u32>,
}

impl Technology {
    /// Get the starting technologies
    pub fn starter_techs() -> Vec<Technology> {
        vec![
            Technology {
                id: 1,
                name: "Basic Propulsion".to_string(),
                description: "Standard sublight engines for interplanetary travel.".to_string(),
                cost: 50,
                prerequisites: vec![],
            },
            Technology {
                id: 2,
                name: "Colony Infrastructure".to_string(),
                description: "Basic infrastructure for establishing new colonies.".to_string(),
                cost: 100,
                prerequisites: vec![],
            },
            Technology {
                id: 3,
                name: "Advanced Sensors".to_string(),
                description: "Improved sensors for deep space exploration.".to_string(),
                cost: 75,
                prerequisites: vec![1],
            },
        ]
    }
}

/// Get the base production value for a planet size
pub fn base_production(size: PlanetSize) -> u64 {
    match size {
        PlanetSize::Tiny => 5,
        PlanetSize::Small => 8,
        PlanetSize::Medium => 12,
        PlanetSize::Large => 16,
        PlanetSize::Massive => 20,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_traits_defaults() {
        let traits = PlanetTrait::defaults();
        assert!(!traits.is_empty());
        assert!(traits.iter().any(|t| t.name == "Mineral Rich"));
    }

    #[test]
    fn ship_templates_defaults() {
        let templates = ShipTemplate::defaults();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.name == "Scout"));
        assert!(templates.iter().any(|t| t.name == "Science Ship"));
    }

    #[test]
    fn starter_technologies() {
        let techs = Technology::starter_techs();
        assert_eq!(techs.len(), 3);
        assert!(techs.iter().any(|t| t.prerequisites.is_empty()));
    }

    #[test]
    fn base_production_values() {
        assert!(base_production(PlanetSize::Massive) > base_production(PlanetSize::Tiny));
    }
}
