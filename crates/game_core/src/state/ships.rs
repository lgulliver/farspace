use super::*;

/// Static record describing a constructible ship design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipDesignRecord {
    pub id: ShipDesignId,
    pub name: &'static str,
    pub cost: u64,
    pub fleet_kind: FleetKind,
    pub ships: u32,
    pub strength: u32,
    /// Credits-per-turn upkeep cost for one fleet of this design.
    pub maintenance: u32,
    /// Short role description shown in the production UI.
    pub role: &'static str,
    pub required_tech: Option<TechId>,
}

/// All constructible ship designs in deterministic display order.
pub fn all_ship_designs() -> &'static [ShipDesignRecord] {
    &[
        ShipDesignRecord {
            id: ShipDesignId::SCOUT,
            name: "Scout",
            cost: 50,
            fleet_kind: FleetKind::Scout,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Exploration",
            required_tech: None,
        },
        ShipDesignRecord {
            id: ShipDesignId::COLONY,
            name: "Colony Ship",
            cost: 200,
            fleet_kind: FleetKind::Colonizer,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Colonization",
            required_tech: Some(TechId::HABITAT_SEEDING),
        },
        ShipDesignRecord {
            id: ShipDesignId::SCIENCE,
            name: "Science Ship",
            cost: 100,
            fleet_kind: FleetKind::Science,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Survey",
            required_tech: Some(TechId::SURVEY_DRONES),
        },
        ShipDesignRecord {
            id: ShipDesignId::TROOP_TRANSPORT,
            name: "Troop Transport",
            cost: 150,
            fleet_kind: FleetKind::TroopTransport,
            ships: 1,
            strength: 1,
            maintenance: 2,
            role: "Invasion",
            required_tech: Some(TechId::TROOP_TRANSPORTS),
        },
        ShipDesignRecord {
            id: ShipDesignId::FAST_SCOUT,
            name: "Fast Scout",
            cost: 75,
            fleet_kind: FleetKind::FastScout,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Rapid Exploration",
            required_tech: Some(TechId::RAPID_TRANSIT),
        },
        ShipDesignRecord {
            id: ShipDesignId::SURVEY_CUTTER,
            name: "Survey Cutter",
            cost: 150,
            fleet_kind: FleetKind::SurveyCutter,
            ships: 1,
            strength: 1,
            maintenance: 2,
            role: "Deep Survey",
            required_tech: Some(TechId::ADVANCED_SURVEY),
        },
        ShipDesignRecord {
            id: ShipDesignId::COLONY_ARK,
            name: "Colony Ark",
            cost: 350,
            fleet_kind: FleetKind::ColonyArk,
            ships: 1,
            strength: 2,
            maintenance: 2,
            role: "Mass Colonization",
            required_tech: Some(TechId::COLONIAL_VANGUARD),
        },
        ShipDesignRecord {
            id: ShipDesignId::ESCORT_FRIGATE,
            name: "Escort Frigate",
            cost: 120,
            fleet_kind: FleetKind::EscortFrigate,
            ships: 2,
            strength: 3,
            maintenance: 2,
            role: "Defensive Combat",
            required_tech: Some(TechId::PERIMETER_DEFENSE),
        },
        ShipDesignRecord {
            id: ShipDesignId::MISSILE_FRIGATE,
            name: "Missile Frigate",
            cost: 200,
            fleet_kind: FleetKind::MissileFrigate,
            ships: 2,
            strength: 5,
            maintenance: 3,
            role: "Strike Combat",
            required_tech: Some(TechId::STRIKE_DOCTRINE),
        },
        ShipDesignRecord {
            id: ShipDesignId::DESTROYER,
            name: "Destroyer",
            cost: 300,
            fleet_kind: FleetKind::Destroyer,
            ships: 3,
            strength: 8,
            maintenance: 4,
            role: "Heavy Combat",
            required_tech: Some(TechId::FLEET_COORDINATION),
        },
        ShipDesignRecord {
            id: ShipDesignId::PATROL_CORVETTE,
            name: "Patrol Corvette",
            cost: 80,
            fleet_kind: FleetKind::PatrolCorvette,
            ships: 1,
            strength: 2,
            maintenance: 1,
            role: "Local Security",
            required_tech: Some(TechId::PERIMETER_DEFENSE),
        },
    ]
}

impl ShipDesignId {
    pub const SCOUT: ShipDesignId = ShipDesignId(1);
    pub const COLONY: ShipDesignId = ShipDesignId(2);
    pub const SCIENCE: ShipDesignId = ShipDesignId(3);
    pub const TROOP_TRANSPORT: ShipDesignId = ShipDesignId(4);
    pub const FAST_SCOUT: ShipDesignId = ShipDesignId(5);
    pub const SURVEY_CUTTER: ShipDesignId = ShipDesignId(6);
    pub const COLONY_ARK: ShipDesignId = ShipDesignId(7);
    pub const ESCORT_FRIGATE: ShipDesignId = ShipDesignId(8);
    pub const MISSILE_FRIGATE: ShipDesignId = ShipDesignId(9);
    pub const DESTROYER: ShipDesignId = ShipDesignId(10);
    pub const PATROL_CORVETTE: ShipDesignId = ShipDesignId(11);

    /// All design IDs in deterministic display order, derived from `all_ship_designs()`
    /// to ensure both stay in sync automatically.
    pub fn all() -> impl Iterator<Item = ShipDesignId> {
        all_ship_designs().iter().map(|d| d.id)
    }

    /// Resolve this ID to a known design record.
    pub fn record(&self) -> Option<&'static ShipDesignRecord> {
        all_ship_designs().iter().find(|d| d.id == *self)
    }
}
