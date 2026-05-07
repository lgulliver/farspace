//! Game state types and domain models

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

/// Unique identifier for a technology
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TechId(pub u32);

/// Static record describing a researchable technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechRecord {
    pub id: TechId,
    pub name: &'static str,
    pub description: &'static str,
    pub cost: i64,
}

/// All researchable technologies available in the game.
///
/// Returns a static slice — no heap allocation on every call.
pub fn all_techs() -> &'static [TechRecord] {
    &[
        TechRecord {
            id: TechId(1),
            name: "Void Propulsion",
            description: "Efficient sublight drives harnessing vacuum energy gradients.",
            cost: 50,
        },
        TechRecord {
            id: TechId(2),
            name: "Habitat Seeding",
            description: "Rapid colony establishment protocols for marginal worlds.",
            cost: 80,
        },
        TechRecord {
            id: TechId(3),
            name: "Neutrino Sensors",
            description:
                "Deep-penetrating sensor arrays that detect matter through interference patterns.",
            cost: 60,
        },
        TechRecord {
            id: TechId(4),
            name: "Kinetic Barriers",
            description: "Directed kinetic deflection fields for hull protection.",
            cost: 100,
        },
        TechRecord {
            id: TechId(5),
            name: "Lattice Processing",
            description: "Crystalline processor arrays with massively parallel throughput.",
            cost: 120,
        },
        TechRecord {
            id: TechId(6),
            name: "Drift Mapping",
            description: "Predictive navigation charts derived from gravitational drift analysis.",
            cost: 90,
        },
        TechRecord {
            id: TechId(7),
            name: "Orbital Engineering",
            description:
                "Advanced construction techniques for assembling large structures in orbit.",
            cost: 150,
        },
    ]
}

/// Per-empire research progress tracking
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResearchState {
    /// The technology currently being researched, if any
    pub current_tech: Option<TechId>,
    /// Research points accumulated toward `current_tech`
    pub progress: i64,
    /// Technologies that have already been completed
    pub completed: Vec<TechId>,
}

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

/// Geological class of a planet, affecting resource and habitability profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetClass {
    Terran,
    Desert,
    Oceanic,
    Volcanic,
    Frozen,
    Barren,
}

impl PlanetClass {
    /// Returns all planet classes for random selection
    pub fn all() -> &'static [PlanetClass] {
        &[
            PlanetClass::Terran,
            PlanetClass::Desert,
            PlanetClass::Oceanic,
            PlanetClass::Volcanic,
            PlanetClass::Frozen,
            PlanetClass::Barren,
        ]
    }

    /// Human-readable name for this planet class
    pub fn name(&self) -> &'static str {
        match self {
            PlanetClass::Terran => "Terran",
            PlanetClass::Desert => "Desert",
            PlanetClass::Oceanic => "Oceanic",
            PlanetClass::Volcanic => "Volcanic",
            PlanetClass::Frozen => "Frozen",
            PlanetClass::Barren => "Barren",
        }
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
    Massive,
}

impl PlanetSize {
    /// Returns all planet sizes for random selection
    pub fn all() -> &'static [PlanetSize] {
        &[
            PlanetSize::Tiny,
            PlanetSize::Small,
            PlanetSize::Medium,
            PlanetSize::Large,
            PlanetSize::Massive,
        ]
    }

    /// Base population capacity for this planet size
    pub fn base_capacity(&self) -> u64 {
        match self {
            PlanetSize::Tiny => 2,
            PlanetSize::Small => 4,
            PlanetSize::Medium => 8,
            PlanetSize::Large => 12,
            PlanetSize::Massive => 16,
        }
    }

    /// Number of surface infrastructure slots available on this planet
    pub fn surface_slots(&self) -> usize {
        match self {
            PlanetSize::Tiny => 3,
            PlanetSize::Small => 5,
            PlanetSize::Medium => 7,
            PlanetSize::Large => 10,
            PlanetSize::Massive => 14,
        }
    }

    /// Number of orbital infrastructure slots available around this planet
    pub fn orbital_slots(&self) -> usize {
        match self {
            PlanetSize::Tiny => 1,
            PlanetSize::Small => 1,
            PlanetSize::Medium => 2,
            PlanetSize::Large => 3,
            PlanetSize::Massive => 4,
        }
    }
}

/// A planet within a star system
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Planet {
    pub name: String,
    pub size: PlanetSize,
    /// Geological classification of this planet
    #[cfg_attr(feature = "serde", serde(default = "default_planet_class"))]
    pub class: PlanetClass,
    pub colony: Option<ColonyId>,
    /// Whether this planet can support a colony
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub habitable: bool,
}

#[cfg(feature = "serde")]
fn default_planet_class() -> PlanetClass {
    PlanetClass::Terran
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
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
    /// Research progress for this empire
    #[cfg_attr(feature = "serde", serde(default))]
    pub research: ResearchState,
    /// Empire-wide food stockpile (net of production minus consumption each turn)
    #[cfg_attr(feature = "serde", serde(default))]
    pub food: i64,
}

/// Permanent buildings that can be constructed at a colony
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BuildingType {
    /// Increases food production and supports a larger population
    AquacultureBay,
    /// Boosts industrial output and production speed
    FabricationYard,
    /// Accelerates research and technological advancement
    ScienceNexus,
}

impl BuildingType {
    /// All available building types
    pub fn all() -> &'static [BuildingType] {
        &[
            BuildingType::AquacultureBay,
            BuildingType::FabricationYard,
            BuildingType::ScienceNexus,
        ]
    }

    /// Display name for this building
    pub fn name(&self) -> &'static str {
        match self {
            BuildingType::AquacultureBay => "Aquaculture Bay",
            BuildingType::FabricationYard => "Fabrication Yard",
            BuildingType::ScienceNexus => "Science Nexus",
        }
    }

    /// Short description of what this building does
    pub fn description(&self) -> &'static str {
        match self {
            BuildingType::AquacultureBay => "Increases food and population capacity",
            BuildingType::FabricationYard => "Increases industrial output",
            BuildingType::ScienceNexus => "Increases research output",
        }
    }

    /// Production cost to construct this building
    pub fn cost(&self) -> u64 {
        match self {
            BuildingType::AquacultureBay => 60,
            BuildingType::FabricationYard => 80,
            BuildingType::ScienceNexus => 100,
        }
    }

    /// Credit maintenance cost per turn for this building
    pub fn maintenance_cost(&self) -> i64 {
        match self {
            BuildingType::AquacultureBay => 0,
            BuildingType::FabricationYard => 1,
            BuildingType::ScienceNexus => 1,
        }
    }

    /// Extra food produced per turn by this building, given the colony population.
    ///
    /// `AquacultureBay` produces additional food equal to the colony population,
    /// effectively doubling the base food yield.  Other buildings produce no food.
    pub fn food_bonus(&self, population: u64) -> i64 {
        match self {
            BuildingType::AquacultureBay => population as i64,
            BuildingType::FabricationYard => 0,
            BuildingType::ScienceNexus => 0,
        }
    }
}

/// Orbital infrastructure types that occupy orbital slots around a colony's planet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum OrbitalStructureType {
    /// A large orbital drydock capable of constructing and refitting warships
    Shipyard,
}

impl OrbitalStructureType {
    /// All orbital structure types available for construction
    pub fn all() -> &'static [OrbitalStructureType] {
        &[OrbitalStructureType::Shipyard]
    }

    /// Display name for this orbital structure
    pub fn name(&self) -> &'static str {
        match self {
            OrbitalStructureType::Shipyard => "Shipyard",
        }
    }

    /// Short description of what this orbital structure does
    pub fn description(&self) -> &'static str {
        match self {
            OrbitalStructureType::Shipyard => {
                "Orbital drydock — required to construct ships at this colony"
            }
        }
    }

    /// Production cost to construct this orbital structure
    pub fn cost(&self) -> u64 {
        match self {
            OrbitalStructureType::Shipyard => 200,
        }
    }

    /// Credit maintenance cost per turn for this orbital structure
    pub fn maintenance_cost(&self) -> i64 {
        match self {
            OrbitalStructureType::Shipyard => 2,
        }
    }

    /// Technology required to construct this orbital structure, if any
    pub fn required_tech(&self) -> Option<TechId> {
        match self {
            OrbitalStructureType::Shipyard => Some(TechId(7)),
        }
    }
}

/// Items that can be built at a colony
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BuildItem {
    Scout,
    Colony,
    Outpost,
    /// A permanent surface structure to be built on the colony
    Structure(BuildingType),
    /// An orbital structure to be assembled in orbit around the colony's planet
    OrbitalStructure(OrbitalStructureType),
}

impl BuildItem {
    /// Production cost for this item
    pub fn cost(&self) -> u64 {
        match self {
            BuildItem::Scout => 50,
            BuildItem::Colony => 200,
            BuildItem::Outpost => 100,
            BuildItem::Structure(bt) => bt.cost(),
            BuildItem::OrbitalStructure(ot) => ot.cost(),
        }
    }

    /// Display name for this item
    pub fn name(&self) -> &'static str {
        match self {
            BuildItem::Scout => "Scout",
            BuildItem::Colony => "Colony Ship",
            BuildItem::Outpost => "Outpost",
            BuildItem::Structure(bt) => bt.name(),
            BuildItem::OrbitalStructure(ot) => ot.name(),
        }
    }

    /// Technology required before this item can be queued, if any
    pub fn required_tech(&self) -> Option<TechId> {
        match self {
            BuildItem::Scout | BuildItem::Colony | BuildItem::Outpost | BuildItem::Structure(_) => {
                None
            }
            BuildItem::OrbitalStructure(ot) => ot.required_tech(),
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
    /// Completed permanent buildings at this colony (used for effect calculations)
    #[cfg_attr(feature = "serde", serde(default))]
    pub buildings: Vec<BuildingType>,
    /// Surface building installations tracked for slot capacity
    #[cfg_attr(feature = "serde", serde(default))]
    pub surface_installations: Vec<BuildingType>,
    /// Orbital installations tracked for slot capacity
    #[cfg_attr(feature = "serde", serde(default))]
    pub orbital_installations: Vec<OrbitalStructureType>,
}

impl Colony {
    /// Check if a building can be placed on this colony's surface
    pub fn can_place_surface_building(&self, planet_size: PlanetSize) -> bool {
        self.surface_installations.len() < planet_size.surface_slots()
    }

    /// Check if a building can be placed in orbit around this colony's planet
    pub fn can_place_orbital_installation(&self, planet_size: PlanetSize) -> bool {
        self.orbital_installations.len() < planet_size.orbital_slots()
    }

    /// Get the number of available surface slots
    pub fn available_surface_slots(&self, planet_size: PlanetSize) -> usize {
        planet_size
            .surface_slots()
            .saturating_sub(self.surface_installations.len())
    }

    /// Get the number of available orbital slots
    pub fn available_orbital_slots(&self, planet_size: PlanetSize) -> usize {
        planet_size
            .orbital_slots()
            .saturating_sub(self.orbital_installations.len())
    }

    /// Returns `true` if this colony has a Shipyard in its orbital installations
    pub fn has_shipyard(&self) -> bool {
        self.orbital_installations
            .contains(&OrbitalStructureType::Shipyard)
    }
}

/// The role of a fleet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetKind {
    /// General-purpose scout/exploration fleet
    #[default]
    Scout,
    /// Colony ship — consumed when founding a new colony
    Colonizer,
}

/// A fleet of ships
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Fleet {
    pub id: FleetId,
    pub owner: EmpireId,
    pub location: StarId,
    pub ships: u32,
    /// Role of this fleet
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: FleetKind,
    /// Military combat strength of this fleet.
    ///
    /// **Invariant: must be ≥ 1.**  The combat resolution formula divides by this
    /// value; the engine defensively clamps via `.max(1)`, but callers should
    /// always initialise this field to at least 1.
    #[cfg_attr(feature = "serde", serde(default = "default_fleet_strength"))]
    pub strength: u32,
    /// Structural integrity of this fleet (starts at 100; 0 = destroyed)
    #[cfg_attr(feature = "serde", serde(default = "default_fleet_integrity"))]
    pub integrity: u32,
}

#[cfg(feature = "serde")]
fn default_fleet_strength() -> u32 {
    1
}

#[cfg(feature = "serde")]
fn default_fleet_integrity() -> u32 {
    100
}

/// An in-flight scout mission heading toward an unexplored system
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScoutMission {
    /// The fleet executing this mission
    pub fleet: FleetId,
    /// Target star system to explore
    pub destination: StarId,
    /// Turns remaining until the scout arrives
    pub turns_remaining: u32,
}

/// A general fleet movement mission heading toward an already-explored system
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FleetMission {
    /// The fleet executing this mission
    pub fleet: FleetId,
    /// Target star system (must already be explored)
    pub destination: StarId,
    /// Turns remaining until the fleet arrives
    pub turns_remaining: u32,
}

/// Where a fleet currently is or is headed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetLocation {
    /// Fleet is present at this star system
    AtStar(StarId),
    /// Fleet is en route to a destination
    Travelling {
        destination: StarId,
        turns_remaining: u32,
    },
}

/// Diplomatic relationship status between the player empire and another empire
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RelationshipStatus {
    /// The empires have never made contact
    Unknown,
    /// The empires have established first contact
    Contacted,
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
    /// Stars that have been explored by the player
    #[cfg_attr(feature = "serde", serde(default))]
    pub explored_stars: BTreeSet<StarId>,
    /// Active scout missions keyed by fleet ID (for exploring unexplored stars)
    #[cfg_attr(feature = "serde", serde(default))]
    pub scout_missions: BTreeMap<FleetId, ScoutMission>,
    /// Active fleet movement missions keyed by fleet ID (for moving to explored stars)
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_missions: BTreeMap<FleetId, FleetMission>,
    /// The AI-controlled empire, if one exists
    #[cfg_attr(feature = "serde", serde(default))]
    pub ai_empire: Option<EmpireId>,
    /// Stars that the AI empire has explored
    #[cfg_attr(feature = "serde", serde(default))]
    pub ai_explored_stars: BTreeSet<StarId>,
    /// Diplomatic relationship status between the player empire and each other empire.
    /// Empires not present in this map are implicitly `Unknown`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub diplomacy: BTreeMap<EmpireId, RelationshipStatus>,
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

    /// Compute the effective location of a fleet, accounting for active missions.
    ///
    /// Returns `None` if the fleet does not exist.
    pub fn fleet_location(&self, fleet_id: FleetId) -> Option<FleetLocation> {
        // Fleet mission takes priority (to explored stars)
        if let Some(mission) = self.fleet_missions.get(&fleet_id) {
            return Some(FleetLocation::Travelling {
                destination: mission.destination,
                turns_remaining: mission.turns_remaining,
            });
        }
        // Scout mission (to unexplored stars)
        if let Some(mission) = self.scout_missions.get(&fleet_id) {
            return Some(FleetLocation::Travelling {
                destination: mission.destination,
                turns_remaining: mission.turns_remaining,
            });
        }
        // At rest
        self.fleets
            .get(&fleet_id)
            .map(|f| FleetLocation::AtStar(f.location))
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
            && self.explored_stars == other.explored_stars
            && self.scout_missions == other.scout_missions
            && self.fleet_missions == other.fleet_missions
            && self.ai_empire == other.ai_empire
            && self.ai_explored_stars == other.ai_explored_stars
            && self.diplomacy == other.diplomacy
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
            explored_stars: BTreeSet::new(),
            scout_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: None,
            ai_explored_stars: BTreeSet::new(),
            diplomacy: BTreeMap::new(),
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
        assert_eq!(
            BuildItem::Structure(BuildingType::AquacultureBay).cost(),
            60
        );
        assert_eq!(
            BuildItem::Structure(BuildingType::FabricationYard).cost(),
            80
        );
        assert_eq!(BuildItem::Structure(BuildingType::ScienceNexus).cost(), 100);
    }

    #[test]
    fn planet_size_capacities() {
        assert_eq!(PlanetSize::Tiny.base_capacity(), 2);
        assert_eq!(PlanetSize::Massive.base_capacity(), 16);
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

    #[test]
    fn spectral_class_as_char() {
        assert_eq!(SpectralClass::O.as_char(), 'O');
        assert_eq!(SpectralClass::B.as_char(), 'B');
        assert_eq!(SpectralClass::A.as_char(), 'A');
        assert_eq!(SpectralClass::F.as_char(), 'F');
        assert_eq!(SpectralClass::G.as_char(), 'G');
        assert_eq!(SpectralClass::K.as_char(), 'K');
        assert_eq!(SpectralClass::M.as_char(), 'M');
    }

    #[test]
    fn spectral_class_all_contains_all_variants() {
        let all = SpectralClass::all();
        assert_eq!(all.len(), 7);
        assert!(all.contains(&SpectralClass::O));
        assert!(all.contains(&SpectralClass::M));
    }

    #[test]
    fn planet_size_all_contains_all_variants() {
        let all = PlanetSize::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&PlanetSize::Tiny));
        assert!(all.contains(&PlanetSize::Massive));
    }

    #[test]
    fn planet_size_all_base_capacities() {
        assert_eq!(PlanetSize::Small.base_capacity(), 4);
        assert_eq!(PlanetSize::Medium.base_capacity(), 8);
        assert_eq!(PlanetSize::Large.base_capacity(), 12);
    }

    #[test]
    fn build_item_names() {
        assert_eq!(BuildItem::Scout.name(), "Scout");
        assert_eq!(BuildItem::Colony.name(), "Colony Ship");
        assert_eq!(BuildItem::Outpost.name(), "Outpost");
        assert_eq!(
            BuildItem::Structure(BuildingType::AquacultureBay).name(),
            "Aquaculture Bay"
        );
        assert_eq!(
            BuildItem::Structure(BuildingType::FabricationYard).name(),
            "Fabrication Yard"
        );
        assert_eq!(
            BuildItem::Structure(BuildingType::ScienceNexus).name(),
            "Science Nexus"
        );
    }

    #[test]
    fn building_type_all_contains_three_variants() {
        let all = BuildingType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&BuildingType::AquacultureBay));
        assert!(all.contains(&BuildingType::FabricationYard));
        assert!(all.contains(&BuildingType::ScienceNexus));
    }

    #[test]
    fn building_type_names_and_descriptions_are_non_empty() {
        for bt in BuildingType::all() {
            assert!(!bt.name().is_empty());
            assert!(!bt.description().is_empty());
        }
    }

    #[test]
    fn building_type_costs_are_positive() {
        for bt in BuildingType::all() {
            assert!(bt.cost() > 0);
        }
    }

    #[test]
    fn game_state_partial_eq() {
        let state_a = GameState::default();
        let state_b = GameState::default();
        assert_eq!(state_a, state_b);

        let state_c = GameState {
            turn: 5,
            ..GameState::default()
        };
        assert_ne!(state_a, state_c);
    }

    #[test]
    fn tech_id_ordering() {
        let t1 = TechId(1);
        let t2 = TechId(2);
        assert!(t1 < t2);
    }

    #[test]
    fn all_techs_returns_seven_entries() {
        let techs = all_techs();
        assert_eq!(techs.len(), 7);
        assert!(
            techs.iter().any(|t| t.name == "Orbital Engineering"),
            "Orbital Engineering tech must be present"
        );
    }

    #[test]
    fn all_techs_have_unique_ids() {
        let techs = all_techs();
        let mut ids: Vec<TechId> = techs.iter().map(|t| t.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), techs.len(), "Tech IDs must be unique");
    }

    #[test]
    fn all_techs_have_positive_costs() {
        for tech in all_techs() {
            assert!(tech.cost > 0, "Tech {} must have positive cost", tech.name);
        }
    }

    #[test]
    fn all_techs_have_non_empty_names_and_descriptions() {
        for tech in all_techs() {
            assert!(!tech.name.is_empty());
            assert!(!tech.description.is_empty());
        }
    }

    #[test]
    fn research_state_default_is_empty() {
        let rs = ResearchState::default();
        assert!(rs.current_tech.is_none());
        assert_eq!(rs.progress, 0);
        assert!(rs.completed.is_empty());
    }

    #[test]
    fn empire_research_defaults_to_empty() {
        let state = GameState::default();
        // Default state has no empires, but we can construct one directly
        let empire = Empire {
            id: EmpireId(1),
            name: "Test".to_string(),
            credits: 0,
            research_points: 0,
            home_star: StarId(1),
            research: ResearchState::default(),
            food: 0,
        };
        assert!(empire.research.current_tech.is_none());
        assert!(empire.research.completed.is_empty());
        let _ = state;
    }

    #[test]
    fn scout_mission_fields() {
        let mission = ScoutMission {
            fleet: FleetId(1),
            destination: StarId(5),
            turns_remaining: 3,
        };
        assert_eq!(mission.fleet, FleetId(1));
        assert_eq!(mission.destination, StarId(5));
        assert_eq!(mission.turns_remaining, 3);
    }

    #[test]
    fn game_state_default_has_empty_exploration() {
        let state = GameState::default();
        assert!(state.explored_stars.is_empty());
        assert!(state.scout_missions.is_empty());
    }

    #[test]
    fn game_state_partial_eq_considers_explored_stars() {
        let mut state_a = GameState::default();
        let state_b = GameState::default();
        assert_eq!(state_a, state_b);

        state_a.explored_stars.insert(StarId(1));
        assert_ne!(state_a, state_b);
    }

    #[test]
    fn game_state_partial_eq_considers_scout_missions() {
        let mut state_a = GameState::default();
        let state_b = GameState::default();
        assert_eq!(state_a, state_b);

        state_a.scout_missions.insert(
            FleetId(1),
            ScoutMission {
                fleet: FleetId(1),
                destination: StarId(2),
                turns_remaining: 2,
            },
        );
        assert_ne!(state_a, state_b);
    }

    #[test]
    fn game_state_partial_eq_considers_fleet_missions() {
        let mut state_a = GameState::default();
        let state_b = GameState::default();
        assert_eq!(state_a, state_b);

        state_a.fleet_missions.insert(
            FleetId(1),
            FleetMission {
                fleet: FleetId(1),
                destination: StarId(2),
                turns_remaining: 2,
            },
        );
        assert_ne!(state_a, state_b);
    }

    #[test]
    fn fleet_mission_fields() {
        let mission = FleetMission {
            fleet: FleetId(3),
            destination: StarId(7),
            turns_remaining: 2,
        };
        assert_eq!(mission.fleet, FleetId(3));
        assert_eq!(mission.destination, StarId(7));
        assert_eq!(mission.turns_remaining, 2);
    }

    #[test]
    fn fleet_location_at_star() {
        let mut state = GameState::default();
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: EmpireId(1),
                location: StarId(5),
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        match state.fleet_location(FleetId(1)) {
            Some(FleetLocation::AtStar(id)) => assert_eq!(id, StarId(5)),
            other => panic!("Expected AtStar, got {:?}", other),
        }
    }

    #[test]
    fn fleet_location_travelling_via_fleet_mission() {
        let mut state = GameState::default();
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: EmpireId(1),
                location: StarId(5),
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        state.fleet_missions.insert(
            FleetId(1),
            FleetMission {
                fleet: FleetId(1),
                destination: StarId(9),
                turns_remaining: 2,
            },
        );
        match state.fleet_location(FleetId(1)) {
            Some(FleetLocation::Travelling {
                destination,
                turns_remaining,
            }) => {
                assert_eq!(destination, StarId(9));
                assert_eq!(turns_remaining, 2);
            }
            other => panic!("Expected Travelling, got {:?}", other),
        }
    }

    #[test]
    fn fleet_location_travelling_via_scout_mission() {
        let mut state = GameState::default();
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: EmpireId(1),
                location: StarId(5),
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        state.scout_missions.insert(
            FleetId(1),
            ScoutMission {
                fleet: FleetId(1),
                destination: StarId(11),
                turns_remaining: 3,
            },
        );
        match state.fleet_location(FleetId(1)) {
            Some(FleetLocation::Travelling {
                destination,
                turns_remaining,
            }) => {
                assert_eq!(destination, StarId(11));
                assert_eq!(turns_remaining, 3);
            }
            other => panic!("Expected Travelling, got {:?}", other),
        }
    }

    #[test]
    fn fleet_location_none_for_missing_fleet() {
        let state = GameState::default();
        assert!(state.fleet_location(FleetId(999)).is_none());
    }

    #[test]
    fn game_state_default_has_empty_fleet_missions() {
        let state = GameState::default();
        assert!(state.fleet_missions.is_empty());
    }

    #[test]
    fn planet_class_all_contains_all_variants() {
        let all = PlanetClass::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&PlanetClass::Terran));
        assert!(all.contains(&PlanetClass::Desert));
        assert!(all.contains(&PlanetClass::Oceanic));
        assert!(all.contains(&PlanetClass::Volcanic));
        assert!(all.contains(&PlanetClass::Frozen));
        assert!(all.contains(&PlanetClass::Barren));
    }

    #[test]
    fn planet_class_names_are_non_empty() {
        for class in PlanetClass::all() {
            assert!(!class.name().is_empty());
        }
    }

    #[test]
    fn planet_size_infrastructure_capacities() {
        assert_eq!(PlanetSize::Tiny.surface_slots(), 3);
        assert_eq!(PlanetSize::Tiny.orbital_slots(), 1);
        assert_eq!(PlanetSize::Small.surface_slots(), 5);
        assert_eq!(PlanetSize::Small.orbital_slots(), 1);
        assert_eq!(PlanetSize::Medium.surface_slots(), 7);
        assert_eq!(PlanetSize::Medium.orbital_slots(), 2);
        assert_eq!(PlanetSize::Large.surface_slots(), 10);
        assert_eq!(PlanetSize::Large.orbital_slots(), 3);
        assert_eq!(PlanetSize::Massive.surface_slots(), 14);
        assert_eq!(PlanetSize::Massive.orbital_slots(), 4);
    }

    #[test]
    fn colony_surface_slot_availability_starts_empty() {
        let colony = Colony {
            id: ColonyId(1),
            star: StarId(1),
            planet_index: 0,
            owner: EmpireId(1),
            population: 10,
            production: 10,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
        };

        assert!(colony.can_place_surface_building(PlanetSize::Medium));
        assert!(colony.can_place_orbital_installation(PlanetSize::Medium));
        assert_eq!(colony.available_surface_slots(PlanetSize::Medium), 7);
        assert_eq!(colony.available_orbital_slots(PlanetSize::Medium), 2);
    }

    #[test]
    fn colony_surface_slots_fill_and_reject_overflow() {
        let mut colony = Colony {
            id: ColonyId(1),
            star: StarId(1),
            planet_index: 0,
            owner: EmpireId(1),
            population: 10,
            production: 10,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: vec![BuildingType::FabricationYard],
            orbital_installations: Vec::new(),
        };

        // With 1 surface building on Tiny (capacity 3), we have 2 left
        assert!(colony.can_place_surface_building(PlanetSize::Tiny));
        assert_eq!(colony.available_surface_slots(PlanetSize::Tiny), 2);

        // Fill to capacity
        colony
            .surface_installations
            .push(BuildingType::ScienceNexus);
        colony
            .surface_installations
            .push(BuildingType::FabricationYard);
        assert!(!colony.can_place_surface_building(PlanetSize::Tiny));
        assert_eq!(colony.available_surface_slots(PlanetSize::Tiny), 0);
    }
}
