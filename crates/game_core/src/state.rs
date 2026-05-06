//! Game state types and domain models

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for a star system
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StarId(pub u64);

/// Unique identifier for an empire
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmpireId(pub u64);

/// Unique identifier for a colony
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColonyId(pub u64);

/// Unique identifier for a fleet
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FleetId(pub u64);

/// Spectral classification of a star
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SpectralClass {
    O,
    B,
    A,
    F,
    G,
    K,
    M,
}

impl SpectralClass {
    /// Returns display character for the spectral class
    pub fn as_char(&self) -> char {
        match self {
            SpectralClass::O => 'O',
            SpectralClass::B => 'B',
            SpectralClass::A => 'A',
            SpectralClass::F => 'F',
            SpectralClass::G => 'G',
            SpectralClass::K => 'K',
            SpectralClass::M => 'M',
        }
    }

    /// Returns all spectral classes for random selection
    pub fn all() -> &'static [SpectralClass] {
        &[
            SpectralClass::O,
            SpectralClass::B,
            SpectralClass::A,
            SpectralClass::F,
            SpectralClass::G,
            SpectralClass::K,
            SpectralClass::M,
        ]
    }
}

/// Size category for a planet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetSize {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}

impl PlanetSize {
    /// Returns all planet sizes for random selection
    pub fn all() -> &'static [PlanetSize] {
        &[
            PlanetSize::Tiny,
            PlanetSize::Small,
            PlanetSize::Medium,
            PlanetSize::Large,
            PlanetSize::Huge,
        ]
    }

    /// Base population capacity for this planet size
    pub fn base_capacity(&self) -> u64 {
        match self {
            PlanetSize::Tiny => 2,
            PlanetSize::Small => 4,
            PlanetSize::Medium => 8,
            PlanetSize::Large => 12,
            PlanetSize::Huge => 16,
        }
    }
}

/// A planet within a star system
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Planet {
    pub name: String,
    pub size: PlanetSize,
    pub colony: Option<ColonyId>,
}

/// A star system in the galaxy
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Star {
    pub id: StarId,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub spectral_class: SpectralClass,
    pub planets: Vec<Planet>,
}

/// An empire (player or AI)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Empire {
    pub id: EmpireId,
    pub name: String,
    pub credits: i64,
    pub research_points: i64,
    pub home_star: StarId,
}

/// Items that can be built at a colony
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BuildItem {
    Scout,
    Colony,
    Outpost,
}

impl BuildItem {
    /// Production cost for this item
    pub fn cost(&self) -> u64 {
        match self {
            BuildItem::Scout => 50,
            BuildItem::Colony => 200,
            BuildItem::Outpost => 100,
        }
    }

    /// Display name for this item
    pub fn name(&self) -> &'static str {
        match self {
            BuildItem::Scout => "Scout",
            BuildItem::Colony => "Colony Ship",
            BuildItem::Outpost => "Outpost",
        }
    }
}

/// A colony on a planet
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Colony {
    pub id: ColonyId,
    pub star: StarId,
    pub planet_index: usize,
    pub owner: EmpireId,
    pub population: u64,
    pub production: u64,
    pub prod_pct: u8,
    pub research_pct: u8,
    pub build_queue: Vec<BuildItem>,
    pub accumulated_production: u64,
}

/// A fleet of ships
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Fleet {
    pub id: FleetId,
    pub owner: EmpireId,
    pub location: StarId,
    pub ships: u32,
}

/// Complete game state
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GameState {
    pub seed: u64,
    pub turn: u32,
    pub stars: BTreeMap<StarId, Star>,
    pub empires: BTreeMap<EmpireId, Empire>,
    pub colonies: BTreeMap<ColonyId, Colony>,
    pub fleets: BTreeMap<FleetId, Fleet>,
    pub player_empire: EmpireId,
    #[cfg_attr(feature = "serde", serde(with = "rng_serde"))]
    pub rng: ChaCha8Rng,
    pub event_log: Vec<String>,
    pub next_colony_id: u64,
    pub next_fleet_id: u64,
}

impl GameState {
    /// Generate a new colony ID
    pub fn next_colony_id(&mut self) -> ColonyId {
        let id = ColonyId(self.next_colony_id);
        self.next_colony_id += 1;
        id
    }

    /// Generate a new fleet ID
    pub fn next_fleet_id(&mut self) -> FleetId {
        let id = FleetId(self.next_fleet_id);
        self.next_fleet_id += 1;
        id
    }
}

impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        self.seed == other.seed
            && self.turn == other.turn
            && self.stars == other.stars
            && self.empires == other.empires
            && self.colonies == other.colonies
            && self.fleets == other.fleets
            && self.player_empire == other.player_empire
            && self.event_log == other.event_log
            && self.next_colony_id == other.next_colony_id
            && self.next_fleet_id == other.next_fleet_id
    }
}

/// Serde helper for ChaCha8Rng serialization
#[cfg(feature = "serde")]
mod rng_serde {
    use rand_chacha::ChaCha8Rng;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(rng: &ChaCha8Rng, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // ChaCha8Rng has serde support via the serde1 feature
        rng.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ChaCha8Rng, D::Error>
    where
        D: Deserializer<'de>,
    {
        ChaCha8Rng::deserialize(deserializer)
    }
}

impl Default for GameState {
    fn default() -> Self {
        use rand::SeedableRng;
        GameState {
            seed: 0,
            turn: 1,
            stars: BTreeMap::new(),
            empires: BTreeMap::new(),
            colonies: BTreeMap::new(),
            fleets: BTreeMap::new(),
            player_empire: EmpireId(0),
            rng: ChaCha8Rng::seed_from_u64(0),
            event_log: Vec::new(),
            next_colony_id: 1,
            next_fleet_id: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_id_ordering() {
        let id1 = StarId(1);
        let id2 = StarId(2);
        assert!(id1 < id2);
    }

    #[test]
    fn empire_id_ordering() {
        let id1 = EmpireId(5);
        let id2 = EmpireId(3);
        assert!(id1 > id2);
    }

    #[test]
    fn build_item_costs() {
        assert_eq!(BuildItem::Scout.cost(), 50);
        assert_eq!(BuildItem::Colony.cost(), 200);
        assert_eq!(BuildItem::Outpost.cost(), 100);
    }

    #[test]
    fn planet_size_capacities() {
        assert_eq!(PlanetSize::Tiny.base_capacity(), 2);
        assert_eq!(PlanetSize::Huge.base_capacity(), 16);
    }

    #[test]
    fn game_state_next_ids() {
        let mut state = GameState::default();
        let c1 = state.next_colony_id();
        let c2 = state.next_colony_id();
        assert_eq!(c1.0, 1);
        assert_eq!(c2.0, 2);

        let f1 = state.next_fleet_id();
        let f2 = state.next_fleet_id();
        assert_eq!(f1.0, 1);
        assert_eq!(f2.0, 2);
    }
}
