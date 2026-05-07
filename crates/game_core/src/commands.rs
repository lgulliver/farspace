//! Commands that can be issued to the game engine

use crate::state::{BuildItem, ColonyId, ColonyRole, FleetId, StarId, TechId};
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
    /// Dispatch a scout fleet to explore an unexplored star system
    SendScout { fleet: FleetId, destination: StarId },
    /// Colonize a habitable, unowned planet with an idle colonizer fleet
    Colonize {
        fleet: FleetId,
        star: StarId,
        planet_index: usize,
    },
    /// Assign a specialisation role to a player-owned colony
    SetColonyRole { colony: ColonyId, role: ColonyRole },
}

#[cfg(test)]
mod tests {
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
}
