//! Commands that can be issued to the game engine

use crate::state::{
    BuildItem, ColonyId, ColonyRole, EmpireId, FleetId, FleetOrder, StarId, TechId,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Commands that can be issued by the player
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Command {
    /// End the current turn
    EndTurn,
    /// Set production/research focus for a colony
    SetColonyFocus {
        colony: ColonyId,
        prod_pct: u8,
        research_pct: u8,
    },
    /// Move a fleet to a new star system
    MoveFleet { fleet: FleetId, destination: StarId },
    /// Add an item to a colony's build queue
    QueueBuild { colony: ColonyId, item: BuildItem },
    /// Cancel an item from a colony's build queue
    CancelBuild { colony: ColonyId, index: usize },
    /// Select a technology to research
    SelectResearch { tech: TechId },
    /// Queue a technology for future research
    QueueResearch { tech: TechId },
    /// Remove a technology from the research queue
    RemoveQueuedResearch { tech: TechId },
    /// Move a queued technology one position earlier
    MoveQueuedResearchUp { tech: TechId },
    /// Move a queued technology one position later
    MoveQueuedResearchDown { tech: TechId },
    /// Clear all queued technologies
    ClearResearchQueue,
    /// Dispatch a scout fleet to explore an unexplored star system
    SendScout { fleet: FleetId, destination: StarId },
    /// Start surveying a planet with a science fleet
    SurveyPlanet {
        fleet: FleetId,
        star: StarId,
        planet_index: usize,
    },
    /// Colonize a habitable, unowned planet with an idle colonizer fleet
    Colonize {
        fleet: FleetId,
        star: StarId,
        planet_index: usize,
    },
    /// Invade an enemy colony on a specific planet with a troop transport fleet
    Invade {
        fleet: FleetId,
        star: StarId,
        planet_index: usize,
    },
    /// Assign a specialisation role to a player-owned colony
    SetColonyRole { colony: ColonyId, role: ColonyRole },
    /// Set the rally point for a colony — newly produced ships will auto-route here
    SetRallyPoint { colony: ColonyId, star: StarId },
    /// Clear the rally point for a colony — newly produced ships will remain at their build star
    ClearRallyPoint { colony: ColonyId },
    /// Set a standing order on a fleet
    SetFleetOrder { fleet: FleetId, order: FleetOrder },
    /// Declare war on a known empire, setting the relationship to `War`
    DeclareWar { target: EmpireId },
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "serde")]
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn command_serialization() {
        let cmd = Command::EndTurn;
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn set_colony_focus_serialization() {
        let cmd = Command::SetColonyFocus {
            colony: ColonyId(1),
            prod_pct: 60,
            research_pct: 40,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn queue_build_serialization() {
        let cmd = Command::QueueBuild {
            colony: ColonyId(1),
            item: BuildItem::Scout,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn select_research_serialization() {
        use crate::state::TechId;
        let cmd = Command::SelectResearch { tech: TechId(3) };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn queue_research_serialization() {
        use crate::state::TechId;
        let cmd = Command::QueueResearch { tech: TechId(3) };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn send_scout_serialization() {
        use crate::state::{FleetId, StarId};
        let cmd = Command::SendScout {
            fleet: FleetId(1),
            destination: StarId(5),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn survey_planet_serialization() {
        let cmd = Command::SurveyPlanet {
            fleet: FleetId(2),
            star: StarId(5),
            planet_index: 1,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, parsed);
    }
}
