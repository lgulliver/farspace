//! Game state types and domain models

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Unique identifier for a star system
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StarId(pub u64);

/// Unique identifier for a sector
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SectorId(pub u64);

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

impl TechId {
    pub const HABITAT_SEEDING: TechId = TechId(2);
    pub const ORBITAL_ENGINEERING: TechId = TechId(7);
    pub const HYPERSPACE_CARTOGRAPHY: TechId = TechId(8);
    pub const SURVEY_DRONES: TechId = TechId(12);
}

/// Unique identifier for a ship design template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShipDesignId(pub u32);

/// Static record describing a researchable technology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TechRecord {
    pub id: TechId,
    pub name: &'static str,
    pub description: &'static str,
    pub domain: TechDomain,
    pub tier: TechTier,
    pub prerequisites: &'static [TechId],
    pub unlocks: &'static [TechUnlock],
    pub cost: i64,
}

/// High-level technology research domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TechDomain {
    Exploration,
    Engineering,
    Military,
    Economy,
    Biology,
}

impl TechDomain {
    pub fn name(&self) -> &'static str {
        match self {
            TechDomain::Exploration => "Exploration",
            TechDomain::Engineering => "Engineering",
            TechDomain::Military => "Military",
            TechDomain::Economy => "Economy",
            TechDomain::Biology => "Biology",
        }
    }
}

/// Coarse progression tier for a technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TechTier {
    I,
    II,
    III,
}

impl TechTier {
    pub fn label(&self) -> &'static str {
        match self {
            TechTier::I => "Tier I",
            TechTier::II => "Tier II",
            TechTier::III => "Tier III",
        }
    }
}

/// Non-item capabilities unlocked by technology completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechCapability {
    PlanetarySurvey,
    HyperspaceLaneTravel,
}

impl TechCapability {
    pub fn name(&self) -> &'static str {
        match self {
            TechCapability::PlanetarySurvey => "Planetary Survey",
            TechCapability::HyperspaceLaneTravel => "Hyperspace Lane Travel",
        }
    }
}

/// Resource type affected by a yield-improvement tech unlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldType {
    Credits,
    Science,
    Food,
}

impl YieldType {
    pub fn name(&self) -> &'static str {
        match self {
            YieldType::Credits => "Credits",
            YieldType::Science => "Science",
            YieldType::Food => "Food",
        }
    }
}

/// Things a technology can unlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechUnlock {
    Structure(BuildingType),
    OrbitalStructure(OrbitalStructureType),
    ShipDesign(ShipDesignId),
    Capability(TechCapability),
    YieldImprovement {
        yield_type: YieldType,
        amount_per_colony: i64,
    },
}

impl TechUnlock {
    pub fn description(&self) -> String {
        match self {
            TechUnlock::Structure(bt) => format!("Structure: {}", bt.name()),
            TechUnlock::OrbitalStructure(ot) => format!("Orbital Structure: {}", ot.name()),
            TechUnlock::ShipDesign(design) => {
                let name = design
                    .record()
                    .map(|r| r.name)
                    .unwrap_or("Unknown Ship Design");
                format!("Ship Design: {name}")
            }
            TechUnlock::Capability(cap) => format!("Capability: {}", cap.name()),
            TechUnlock::YieldImprovement {
                yield_type,
                amount_per_colony,
            } => format!(
                "Yield: +{} {} per colony",
                amount_per_colony,
                yield_type.name()
            ),
        }
    }
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
            domain: TechDomain::Exploration,
            tier: TechTier::I,
            prerequisites: &[],
            unlocks: &[TechUnlock::ShipDesign(ShipDesignId::SCOUT)],
            cost: 50,
        },
        TechRecord {
            id: TechId::HABITAT_SEEDING,
            name: "Habitat Seeding",
            description: "Rapid colony establishment protocols for marginal worlds.",
            domain: TechDomain::Biology,
            tier: TechTier::I,
            prerequisites: &[],
            unlocks: &[TechUnlock::ShipDesign(ShipDesignId::COLONY)],
            cost: 80,
        },
        TechRecord {
            id: TechId(3),
            name: "Neutrino Sensors",
            description:
                "Deep-penetrating sensor arrays that detect matter through interference patterns.",
            domain: TechDomain::Exploration,
            tier: TechTier::I,
            prerequisites: &[],
            unlocks: &[],
            cost: 60,
        },
        TechRecord {
            id: TechId(4),
            name: "Kinetic Barriers",
            description: "Directed kinetic deflection fields for hull protection.",
            domain: TechDomain::Military,
            tier: TechTier::I,
            prerequisites: &[],
            unlocks: &[],
            cost: 100,
        },
        TechRecord {
            id: TechId(5),
            name: "Lattice Processing",
            description: "Crystalline processor arrays with massively parallel throughput.",
            domain: TechDomain::Engineering,
            tier: TechTier::I,
            prerequisites: &[],
            unlocks: &[TechUnlock::YieldImprovement {
                yield_type: YieldType::Science,
                amount_per_colony: 1,
            }],
            cost: 120,
        },
        TechRecord {
            id: TechId(6),
            name: "Drift Mapping",
            description: "Predictive navigation charts derived from gravitational drift analysis.",
            domain: TechDomain::Exploration,
            tier: TechTier::II,
            prerequisites: &[TechId(3)],
            unlocks: &[],
            cost: 90,
        },
        TechRecord {
            id: TechId::ORBITAL_ENGINEERING,
            name: "Orbital Engineering",
            description:
                "Advanced construction techniques for assembling large structures in orbit.",
            domain: TechDomain::Engineering,
            tier: TechTier::II,
            prerequisites: &[TechId(5)],
            unlocks: &[TechUnlock::OrbitalStructure(OrbitalStructureType::Shipyard)],
            cost: 150,
        },
        TechRecord {
            id: TechId::HYPERSPACE_CARTOGRAPHY,
            name: "Hyperspace Cartography",
            description: "Stabilized hyperspace charts reveal and enable rapid lane transit.",
            domain: TechDomain::Exploration,
            tier: TechTier::III,
            prerequisites: &[TechId(6)],
            unlocks: &[TechUnlock::Capability(TechCapability::HyperspaceLaneTravel)],
            cost: 140,
        },
        TechRecord {
            id: TechId(9),
            name: "Xenobiotic Adaptation",
            description: "Adaptive bioengineering tuned for multi-biome colony resilience.",
            domain: TechDomain::Biology,
            tier: TechTier::II,
            prerequisites: &[TechId::HABITAT_SEEDING],
            unlocks: &[TechUnlock::YieldImprovement {
                yield_type: YieldType::Food,
                amount_per_colony: 1,
            }],
            cost: 100,
        },
        TechRecord {
            id: TechId(10),
            name: "Colonial Logistics",
            description: "Interstellar freight coordination improves local economic efficiency.",
            domain: TechDomain::Economy,
            tier: TechTier::II,
            prerequisites: &[TechId::HABITAT_SEEDING, TechId(5)],
            unlocks: &[TechUnlock::YieldImprovement {
                yield_type: YieldType::Credits,
                amount_per_colony: 1,
            }],
            cost: 120,
        },
        TechRecord {
            id: TechId(11),
            name: "Battle Doctrine",
            description: "Codified fleet engagement protocols for coordinated strike planning.",
            domain: TechDomain::Military,
            tier: TechTier::II,
            prerequisites: &[TechId(4)],
            unlocks: &[],
            cost: 130,
        },
        TechRecord {
            id: TechId::SURVEY_DRONES,
            name: "Survey Drones",
            description: "Autonomous orbital probes accelerate planetary reconnaissance workflows.",
            domain: TechDomain::Exploration,
            tier: TechTier::II,
            prerequisites: &[TechId(3)],
            unlocks: &[
                TechUnlock::ShipDesign(ShipDesignId::SCIENCE),
                TechUnlock::Capability(TechCapability::PlanetarySurvey),
            ],
            cost: 95,
        },
    ]
}

/// Resolve a technology ID to a known tech record.
pub fn tech_by_id(tech_id: TechId) -> Option<&'static TechRecord> {
    all_techs().iter().find(|t| t.id == tech_id)
}

/// Returns true if a technology is currently researchable for the given completed set.
pub fn is_tech_available(completed: &[TechId], tech_id: TechId) -> bool {
    let Some(tech) = tech_by_id(tech_id) else {
        return false;
    };
    if completed.contains(&tech_id) {
        return false;
    }
    tech.prerequisites.iter().all(|req| completed.contains(req))
}

/// Deterministic list of currently available technologies.
pub fn available_tech_ids(completed: &[TechId]) -> Vec<TechId> {
    all_techs()
        .iter()
        .filter(|tech| is_tech_available(completed, tech.id))
        .map(|tech| tech.id)
        .collect()
}

/// Sum all per-colony yield bonuses unlocked by completed technologies.
pub fn tech_yield_bonus_per_colony(completed: &[TechId], yield_type: YieldType) -> i64 {
    all_techs()
        .iter()
        .filter(|tech| completed.contains(&tech.id))
        .flat_map(|tech| tech.unlocks.iter())
        .filter_map(|unlock| match unlock {
            TechUnlock::YieldImprovement {
                yield_type: unlocked,
                amount_per_colony,
            } if *unlocked == yield_type => Some(*amount_per_colony),
            _ => None,
        })
        .sum()
}

/// Static record describing a constructible ship design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipDesignRecord {
    pub id: ShipDesignId,
    pub name: &'static str,
    pub cost: u64,
    pub fleet_kind: FleetKind,
    pub ships: u32,
    pub strength: u32,
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
            required_tech: None,
        },
        ShipDesignRecord {
            id: ShipDesignId::COLONY,
            name: "Colony Ship",
            cost: 200,
            fleet_kind: FleetKind::Colonizer,
            ships: 1,
            strength: 1,
            required_tech: Some(TechId::HABITAT_SEEDING),
        },
        ShipDesignRecord {
            id: ShipDesignId::SCIENCE,
            name: "Science Ship",
            cost: 100,
            fleet_kind: FleetKind::Science,
            ships: 1,
            strength: 1,
            required_tech: Some(TechId::SURVEY_DRONES),
        },
    ]
}

impl ShipDesignId {
    pub const SCOUT: ShipDesignId = ShipDesignId(1);
    pub const COLONY: ShipDesignId = ShipDesignId(2);
    pub const SCIENCE: ShipDesignId = ShipDesignId(3);

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

    /// Flat science bonus per turn for colonies on this planet class.
    ///
    /// Frozen worlds favour computation; all other classes provide no bonus.
    pub fn science_bonus(&self) -> i64 {
        match self {
            PlanetClass::Frozen => 1,
            _ => 0,
        }
    }

    /// Flat food bonus per turn for colonies on this planet class.
    ///
    /// Positive values indicate abundant resources; negative values reflect
    /// harsh or arid conditions that make food production harder.
    pub fn food_bonus(&self) -> i64 {
        match self {
            PlanetClass::Terran => 0,
            PlanetClass::Oceanic => 2,
            PlanetClass::Desert => -1,
            PlanetClass::Volcanic => -1,
            PlanetClass::Frozen => -1,
            PlanetClass::Barren => -2,
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

/// Flat yield effect contributed by a planet special or strategic resource.
///
/// All fields default to zero.  Applied additively on top of the base yield
/// calculated from population, buildings, planet class, and colony role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YieldEffect {
    /// Flat industry bonus/penalty (applied before credit/science scaling).
    pub industry: i64,
    /// Flat science bonus/penalty per turn.
    pub science: i64,
    /// Flat credits bonus/penalty per turn.
    pub credits: i64,
    /// Flat food bonus/penalty per turn.
    pub food: i64,
    /// Flat maintenance delta per turn (negative = reduction).
    pub maintenance: i64,
}

impl YieldEffect {
    /// Combine two effects additively.
    pub fn combine(self, other: YieldEffect) -> YieldEffect {
        YieldEffect {
            industry: self.industry + other.industry,
            science: self.science + other.science,
            credits: self.credits + other.credits,
            food: self.food + other.food,
            maintenance: self.maintenance + other.maintenance,
        }
    }
}

/// Discoverable planet special that modifies colony yield or triggers one-time events.
///
/// Specials are generated deterministically from the galaxy seed and hidden until
/// survey is complete.  A colonized planet automatically benefits from its revealed
/// specials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetSpecial {
    /// Rich mineral deposits boost industrial output.
    MineralRich,
    /// Abundant native life accelerates food production and population growth.
    FertileBiosphere,
    /// Remnants of a vanished civilisation — yields a science bonus and a one-time discovery event.
    AncientRuins,
    /// Resonant crystal lattices generate energy, boosting credits and science.
    CrystalFormations,
    /// Perpetual storm systems hamper agriculture and destabilise industry.
    HostileWeather,
    /// Reduced gravitational load makes orbital construction easier — industry bonus.
    LowGravity,
}

impl PlanetSpecial {
    /// All planet specials in deterministic order (used for random selection).
    pub fn all() -> &'static [PlanetSpecial] {
        &[
            PlanetSpecial::MineralRich,
            PlanetSpecial::FertileBiosphere,
            PlanetSpecial::AncientRuins,
            PlanetSpecial::CrystalFormations,
            PlanetSpecial::HostileWeather,
            PlanetSpecial::LowGravity,
        ]
    }

    /// Short display name for this special.
    pub fn name(&self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => "Mineral Rich",
            PlanetSpecial::FertileBiosphere => "Fertile Biosphere",
            PlanetSpecial::AncientRuins => "Ancient Ruins",
            PlanetSpecial::CrystalFormations => "Crystal Formations",
            PlanetSpecial::HostileWeather => "Hostile Weather",
            PlanetSpecial::LowGravity => "Low Gravity",
        }
    }

    /// One-line description of this special's effect.
    pub fn description(&self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => "+2 industry",
            PlanetSpecial::FertileBiosphere => "+2 food",
            PlanetSpecial::AncientRuins => "+2 science, one-time discovery event",
            PlanetSpecial::CrystalFormations => "+1 credits, +1 science",
            PlanetSpecial::HostileWeather => "-1 food, -1 industry",
            PlanetSpecial::LowGravity => "+2 industry",
        }
    }

    /// Flat yield modifiers applied each turn to a colonized, surveyed planet.
    pub fn yield_effect(&self) -> YieldEffect {
        match self {
            PlanetSpecial::MineralRich => YieldEffect {
                industry: 2,
                ..YieldEffect::default()
            },
            PlanetSpecial::FertileBiosphere => YieldEffect {
                food: 2,
                ..YieldEffect::default()
            },
            PlanetSpecial::AncientRuins => YieldEffect {
                science: 2,
                ..YieldEffect::default()
            },
            PlanetSpecial::CrystalFormations => YieldEffect {
                credits: 1,
                science: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::HostileWeather => YieldEffect {
                food: -1,
                industry: -1,
                ..YieldEffect::default()
            },
            PlanetSpecial::LowGravity => YieldEffect {
                industry: 2,
                ..YieldEffect::default()
            },
        }
    }
}

/// Strategic resource presence on a planet — a capability modifier, not an inventory.
///
/// Resources are generated deterministically from the galaxy seed and hidden until
/// survey is complete.  A colonized planet automatically benefits from its revealed
/// resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StrategicResource {
    /// Lightweight fusion fuel reduces fleet maintenance costs.
    Helium3,
    /// Exotic alloys enhance ship and industrial production.
    RareMetals,
    /// Dense nutrient cultures support rapid population growth.
    BioCultures,
    /// Hyper-ordered crystals amplify computational and hyperspace research.
    QuantumCrystals,
}

impl StrategicResource {
    /// All strategic resources in deterministic order (used for random selection).
    pub fn all() -> &'static [StrategicResource] {
        &[
            StrategicResource::Helium3,
            StrategicResource::RareMetals,
            StrategicResource::BioCultures,
            StrategicResource::QuantumCrystals,
        ]
    }

    /// Short display name for this resource.
    pub fn name(&self) -> &'static str {
        match self {
            StrategicResource::Helium3 => "Helium-3",
            StrategicResource::RareMetals => "Rare Metals",
            StrategicResource::BioCultures => "Bio-Cultures",
            StrategicResource::QuantumCrystals => "Quantum Crystals",
        }
    }

    /// One-line description of this resource's effect.
    pub fn description(&self) -> &'static str {
        match self {
            StrategicResource::Helium3 => "-1 maintenance",
            StrategicResource::RareMetals => "+1 industry",
            StrategicResource::BioCultures => "+2 food",
            StrategicResource::QuantumCrystals => "+2 science",
        }
    }

    /// Flat yield modifiers applied each turn to a colonized, surveyed planet.
    pub fn yield_effect(&self) -> YieldEffect {
        match self {
            StrategicResource::Helium3 => YieldEffect {
                maintenance: -1,
                ..YieldEffect::default()
            },
            StrategicResource::RareMetals => YieldEffect {
                industry: 1,
                ..YieldEffect::default()
            },
            StrategicResource::BioCultures => YieldEffect {
                food: 2,
                ..YieldEffect::default()
            },
            StrategicResource::QuantumCrystals => YieldEffect {
                science: 2,
                ..YieldEffect::default()
            },
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
    /// Whether this planet has been surveyed by the player.
    #[cfg_attr(feature = "serde", serde(default))]
    pub surveyed: bool,
    /// Planet specials — hidden until surveyed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub specials: Vec<PlanetSpecial>,
    /// Strategic resources present on this planet — hidden until surveyed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub resources: Vec<StrategicResource>,
    /// Whether the one-time Ancient Ruins discovery event has already been emitted.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ancient_ruins_collected: bool,
}

/// Compute the total `YieldEffect` from all revealed specials and resources on a planet.
///
/// Returns zero-effect when the planet is not yet surveyed (specials remain hidden).
pub fn planet_yield_effect(planet: &Planet) -> YieldEffect {
    if !planet.surveyed {
        return YieldEffect::default();
    }
    let mut total = YieldEffect::default();
    for special in &planet.specials {
        total = total.combine(special.yield_effect());
    }
    for resource in &planet.resources {
        total = total.combine(resource.yield_effect());
    }
    total
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub sector: SectorId,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub spectral_class: SpectralClass,
    pub planets: Vec<Planet>,
}

/// A sector (region) containing multiple star systems
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Sector {
    pub id: SectorId,
    pub name: String,
    pub x: i32,
    pub y: i32,
}

/// A direct hyperspace lane between two star systems.
///
/// Lane endpoints are normalized to ascending `StarId` order for deterministic
/// equality, ordering, and serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HyperspaceLane {
    a: StarId,
    b: StarId,
}

impl HyperspaceLane {
    /// Create a normalized lane. Returns `None` for self-links.
    pub fn new(a: StarId, b: StarId) -> Option<Self> {
        if a == b {
            return None;
        }
        if a < b {
            Some(Self { a, b })
        } else {
            Some(Self { a: b, b: a })
        }
    }

    /// Return true when this lane links the two provided stars (order agnostic).
    pub fn connects(&self, from: StarId, to: StarId) -> bool {
        Self::new(from, to) == Some(*self)
    }

    pub fn a(&self) -> StarId {
        self.a
    }

    pub fn b(&self) -> StarId {
        self.b
    }

    pub fn endpoints(&self) -> (StarId, StarId) {
        (self.a, self.b)
    }
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
            OrbitalStructureType::Shipyard => Some(TechId::ORBITAL_ENGINEERING),
        }
    }
}

/// Items that can be produced at a colony.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ProductionItem {
    /// A permanent surface structure to be built on the colony.
    SurfaceStructure(BuildingType),
    /// An orbital structure to be assembled in orbit around the colony's planet
    OrbitalStructure(OrbitalStructureType),
    /// A ship built from a specific design template.
    Ship(ShipDesignId),
    /// Legacy save compatibility variant for old ship queue entries.
    ///
    /// Kept to deserialize pre-v2 saves that encoded ships directly as `Scout`.
    #[cfg_attr(feature = "serde", serde(rename = "Scout"))]
    Scout,
    /// Legacy save compatibility variant for old ship queue entries.
    ///
    /// Kept to deserialize pre-v2 saves that encoded ships directly as `Colony`.
    #[cfg_attr(feature = "serde", serde(rename = "Colony"))]
    Colony,
    /// Legacy save compatibility variant for old queue entries.
    ///
    /// Kept to deserialize pre-v2 saves that encoded outpost entries as `Outpost`.
    #[cfg_attr(feature = "serde", serde(rename = "Outpost"))]
    Outpost,
    /// Legacy save compatibility variant for old queue entries.
    ///
    /// Kept to deserialize pre-v2 saves that used `Structure` before
    /// `SurfaceStructure` naming was introduced.
    #[cfg_attr(feature = "serde", serde(rename = "Structure"))]
    Structure(BuildingType),
}

impl ProductionItem {
    /// Human-readable category name for this item type.
    pub fn category_name(&self) -> &'static str {
        match self {
            ProductionItem::SurfaceStructure(_) | ProductionItem::Structure(_) => "Surface",
            ProductionItem::OrbitalStructure(_) => "Orbital",
            ProductionItem::Ship(_) | ProductionItem::Scout | ProductionItem::Colony => "Ship",
            ProductionItem::Outpost => "Surface",
        }
    }

    /// Returns true if this production item is a ship.
    pub fn is_ship(&self) -> bool {
        matches!(
            self,
            ProductionItem::Ship(_) | ProductionItem::Scout | ProductionItem::Colony
        )
    }

    /// Resolve this production item to a ship design when it is ship production.
    pub fn ship_design_id(&self) -> Option<ShipDesignId> {
        match self {
            ProductionItem::Ship(id) => Some(*id),
            ProductionItem::Scout => Some(ShipDesignId::SCOUT),
            ProductionItem::Colony => Some(ShipDesignId::COLONY),
            _ => None,
        }
    }

    /// Production cost for this item.
    ///
    /// Returns `u64::MAX` for `Ship(_)` with an invalid (unknown) design ID so the item
    /// can never be silently completed due to a zero-cost guard in queue processing.
    pub fn cost(&self) -> u64 {
        match self {
            ProductionItem::Ship(design_id) => {
                design_id.record().map(|d| d.cost).unwrap_or(u64::MAX)
            }
            ProductionItem::Scout => ShipDesignId::SCOUT.record().map(|d| d.cost).unwrap_or(0),
            ProductionItem::Colony => ShipDesignId::COLONY.record().map(|d| d.cost).unwrap_or(0),
            ProductionItem::Outpost => 100,
            ProductionItem::SurfaceStructure(bt) | ProductionItem::Structure(bt) => bt.cost(),
            ProductionItem::OrbitalStructure(ot) => ot.cost(),
        }
    }

    /// Display name for this item
    pub fn name(&self) -> &'static str {
        match self {
            ProductionItem::Ship(design_id) => design_id
                .record()
                .map(|d| d.name)
                .unwrap_or("Unknown Ship Design"),
            ProductionItem::Scout => "Scout",
            ProductionItem::Colony => "Colony Ship",
            ProductionItem::Outpost => "Outpost",
            ProductionItem::SurfaceStructure(bt) | ProductionItem::Structure(bt) => bt.name(),
            ProductionItem::OrbitalStructure(ot) => ot.name(),
        }
    }

    /// Technology required before this item can be queued, if any
    pub fn required_tech(&self) -> Option<TechId> {
        match self {
            ProductionItem::Ship(design_id) => design_id.record().and_then(|d| d.required_tech),
            ProductionItem::Scout => ShipDesignId::SCOUT.record().and_then(|d| d.required_tech),
            ProductionItem::Colony => ShipDesignId::COLONY.record().and_then(|d| d.required_tech),
            ProductionItem::Outpost
            | ProductionItem::SurfaceStructure(_)
            | ProductionItem::Structure(_) => None,
            ProductionItem::OrbitalStructure(ot) => ot.required_tech(),
        }
    }
}

/// Backward-compatible alias retained for existing code paths and save semantics.
pub type BuildItem = ProductionItem;

/// Specialisation role assigned to a colony, influencing its yield profile.
///
/// Roles apply small, deterministic flat modifiers on top of the base yield
/// calculation.  They complement — but never override — planet class identity
/// or installed infrastructure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColonyRole {
    /// No yield modifier — the default starting role.
    #[default]
    Balanced,
    /// +2 food, −1 industry.
    Agricultural,
    /// +2 industry, −1 science (flat).
    Industrial,
    /// +2 science (flat), −1 credits (flat).
    Scientific,
    /// +2 credits (flat), −1 industry.
    Financial,
    /// +1 maintenance, bonus ship production efficiency.
    Military,
}

impl ColonyRole {
    /// All available colony roles in display order.
    pub fn all() -> &'static [ColonyRole] {
        &[
            ColonyRole::Balanced,
            ColonyRole::Agricultural,
            ColonyRole::Industrial,
            ColonyRole::Scientific,
            ColonyRole::Financial,
            ColonyRole::Military,
        ]
    }

    /// Short display name for this role.
    pub fn name(&self) -> &'static str {
        match self {
            ColonyRole::Balanced => "Balanced",
            ColonyRole::Agricultural => "Agricultural",
            ColonyRole::Industrial => "Industrial",
            ColonyRole::Scientific => "Scientific",
            ColonyRole::Financial => "Financial",
            ColonyRole::Military => "Military",
        }
    }

    /// One-line description of the role's effects.
    pub fn description(&self) -> &'static str {
        match self {
            ColonyRole::Balanced => "No modifiers",
            ColonyRole::Agricultural => "+2 food, −1 industry",
            ColonyRole::Industrial => "+2 industry, −1 science",
            ColonyRole::Scientific => "+2 science, −1 credits",
            ColonyRole::Financial => "+2 credits, −1 industry",
            ColonyRole::Military => "+1 maintenance, +ship efficiency",
        }
    }

    /// Flat yield modifiers applied on top of the base colony yield each turn.
    pub fn modifiers(&self) -> RoleModifiers {
        match self {
            ColonyRole::Balanced => RoleModifiers::default(),
            ColonyRole::Agricultural => RoleModifiers {
                food: 2,
                industry: -1,
                ..RoleModifiers::default()
            },
            ColonyRole::Industrial => RoleModifiers {
                industry: 2,
                science: -1,
                ..RoleModifiers::default()
            },
            ColonyRole::Scientific => RoleModifiers {
                science: 2,
                credits: -1,
                ..RoleModifiers::default()
            },
            ColonyRole::Financial => RoleModifiers {
                credits: 2,
                industry: -1,
                ..RoleModifiers::default()
            },
            ColonyRole::Military => RoleModifiers {
                maintenance: 1,
                ..RoleModifiers::default()
            },
        }
    }

    /// Extra production units applied each turn when building ships at a Military colony.
    ///
    /// Returns 0 for all non-Military roles.
    pub fn ship_production_bonus(&self) -> u64 {
        match self {
            ColonyRole::Military => 2,
            _ => 0,
        }
    }
}

/// Flat yield modifiers contributed by a colony's assigned role.
///
/// All fields default to zero (Balanced).  Applied additively on top of the
/// base yield calculated from population, buildings, and planet class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoleModifiers {
    /// Flat food bonus/penalty per turn.
    pub food: i64,
    /// Flat industry bonus/penalty per turn (applied before credit/science scaling).
    pub industry: i64,
    /// Flat science bonus/penalty per turn.
    pub science: i64,
    /// Flat credits bonus/penalty per turn.
    pub credits: i64,
    /// Flat maintenance surcharge per turn.
    pub maintenance: i64,
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
    pub build_queue: Vec<ProductionItem>,
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
    /// Colony stability (0–200, neutral = 100).  Values above 100 boost industry;
    /// values below reduce it.  Defaults to 100 for all existing colonies.
    #[cfg_attr(feature = "serde", serde(default = "default_stability"))]
    pub stability: u8,
    /// Specialisation role for this colony.  Defaults to `Balanced` (no modifiers).
    #[cfg_attr(feature = "serde", serde(default))]
    pub role: ColonyRole,
    /// Rally point for newly produced ships at this colony.
    ///
    /// When set, every ship that completes production here automatically receives
    /// a `MoveToSystem` order toward this star.  Ships are never auto-routed to
    /// the colony's own star (such an entry is silently ignored).
    #[cfg_attr(feature = "serde", serde(default))]
    pub rally_point: Option<StarId>,
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

/// A persistent standing order assigned to a fleet (v1 semantics).
///
/// Orders are stored in `GameState.fleet_orders` and are used for display
/// and tracking purposes.  The current v1 engine behaviour is:
///
/// * `Hold` – recorded as a standing order; displayed in the fleet list.
///   The engine does **not** yet consult `Hold` orders when processing rally-point
///   routing; that suppression is a planned v2 feature.
/// * `MoveToSystem` – when set on an idle fleet via `Command::SetFleetOrder`, a
///   `FleetMission` is started immediately toward the target star.  The order is
///   cleared automatically when that specific mission resolves (fleet arrives).
///   When set by `maybe_route_to_rally_point` for a newly produced ship, the same
///   mission-starts-immediately behaviour applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetOrder {
    /// Hold position.  Displayed in the fleet list; Hold suppression of rally routing
    /// is planned for a future release and is not yet active.
    Hold,
    /// Move (or continue moving) toward a specific star system.
    MoveToSystem(StarId),
}

impl FleetOrder {
    /// Short display label used in the fleet list.
    pub fn label(&self) -> &'static str {
        match self {
            FleetOrder::Hold => "Hold",
            FleetOrder::MoveToSystem(_) => "Moving",
        }
    }
}

/// The role of a fleet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetKind {
    /// General-purpose scout/exploration fleet
    #[default]
    Scout,
    /// Science ship — performs planet surveys
    Science,
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

#[cfg(feature = "serde")]
fn default_stability() -> u8 {
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
    /// Origin star system (for progress tracking and animation)
    #[cfg_attr(feature = "serde", serde(default))]
    pub origin: StarId,
    /// Total travel duration in turns (for progress calculation)
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_duration: u32,
}

/// A planet survey mission executed by a science fleet
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SurveyMission {
    /// The fleet executing this mission
    pub fleet: FleetId,
    /// Target star system
    pub star: StarId,
    /// Target planet within the star
    pub planet_index: usize,
    /// Turns remaining until survey completes
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
    /// Origin star system (for progress tracking and animation)
    #[cfg_attr(feature = "serde", serde(default))]
    pub origin: StarId,
    /// Total travel duration in turns (for progress calculation)
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_duration: u32,
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub sectors: BTreeMap<SectorId, Sector>,
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
    /// Active survey missions keyed by fleet ID (for science ships)
    #[cfg_attr(feature = "serde", serde(default))]
    pub survey_missions: BTreeMap<FleetId, SurveyMission>,
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
    /// All generated hyperspace lanes in this galaxy.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hyperspace_lanes: BTreeSet<HyperspaceLane>,
    /// Hyperspace lanes known to the player empire (discovery state).
    #[cfg_attr(feature = "serde", serde(default))]
    pub known_hyperspace_lanes: BTreeSet<HyperspaceLane>,
    /// Persistent standing orders keyed by fleet ID.
    ///
    /// A fleet with no entry here has no explicit order and is considered idle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_orders: BTreeMap<FleetId, FleetOrder>,
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
        if let Some(mission) = self.survey_missions.get(&fleet_id) {
            return Some(FleetLocation::AtStar(mission.star));
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
            && self.sectors == other.sectors
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
            && self.survey_missions == other.survey_missions
            && self.fleet_missions == other.fleet_missions
            && self.ai_empire == other.ai_empire
            && self.ai_explored_stars == other.ai_explored_stars
            && self.diplomacy == other.diplomacy
            && self.hyperspace_lanes == other.hyperspace_lanes
            && self.known_hyperspace_lanes == other.known_hyperspace_lanes
            && self.fleet_orders == other.fleet_orders
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
            sectors: BTreeMap::new(),
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
            survey_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: None,
            ai_explored_stars: BTreeSet::new(),
            diplomacy: BTreeMap::new(),
            hyperspace_lanes: BTreeSet::new(),
            known_hyperspace_lanes: BTreeSet::new(),
            fleet_orders: BTreeMap::new(),
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
    fn sector_id_ordering() {
        let id1 = SectorId(1);
        let id2 = SectorId(2);
        assert!(id1 < id2);
    }

    #[test]
    fn sector_id_equality() {
        let id1 = SectorId(42);
        let id2 = SectorId(42);
        let id3 = SectorId(43);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn hyperspace_lane_normalizes_endpoint_order() {
        let lane = HyperspaceLane::new(StarId(9), StarId(2)).expect("distinct stars");
        assert_eq!(lane.a(), StarId(2));
        assert_eq!(lane.b(), StarId(9));
        assert!(lane.connects(StarId(9), StarId(2)));
        assert!(lane.connects(StarId(2), StarId(9)));
        assert!(HyperspaceLane::new(StarId(7), StarId(7)).is_none());
    }

    #[test]
    fn empire_id_ordering() {
        let id1 = EmpireId(5);
        let id2 = EmpireId(3);
        assert!(id1 > id2);
    }

    #[test]
    fn build_item_costs() {
        assert_eq!(BuildItem::Ship(ShipDesignId::SCOUT).cost(), 50);
        assert_eq!(BuildItem::Ship(ShipDesignId::SCIENCE).cost(), 100);
        assert_eq!(BuildItem::Ship(ShipDesignId::COLONY).cost(), 200);
        assert_eq!(BuildItem::Scout.cost(), 50);
        assert_eq!(
            BuildItem::Ship(ShipDesignId::SCIENCE).name(),
            "Science Ship"
        );
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
        assert_eq!(BuildItem::Ship(ShipDesignId::SCOUT).name(), "Scout");
        assert_eq!(BuildItem::Ship(ShipDesignId::COLONY).name(), "Colony Ship");
        assert_eq!(
            BuildItem::Ship(ShipDesignId::SCIENCE).name(),
            "Science Ship"
        );
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
    fn ship_design_records_are_resolvable() {
        for id in ShipDesignId::all() {
            assert!(id.record().is_some(), "known design ID must resolve");
        }
        assert!(
            ShipDesignId(999).record().is_none(),
            "unknown design ID must be invalid"
        );
    }

    #[test]
    fn all_ship_designs_contains_science_ship() {
        let all = all_ship_designs();
        assert_eq!(all.len(), 3);
        assert!(all.iter().any(|d| d.name == "Science Ship"));
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
    fn all_techs_returns_twelve_entries() {
        let techs = all_techs();
        assert_eq!(techs.len(), 12);
        assert!(
            techs.iter().any(|t| t.name == "Orbital Engineering"),
            "Orbital Engineering tech must be present"
        );
        assert!(
            techs.iter().any(|t| t.name == "Hyperspace Cartography"),
            "Hyperspace Cartography tech must be present"
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
    fn tech_with_no_prerequisites_is_available() {
        assert!(
            is_tech_available(&[], TechId(1)),
            "tier-1 root tech should be available with no completed prerequisites"
        );
    }

    #[test]
    fn tech_with_unmet_prerequisites_is_locked() {
        assert!(
            !is_tech_available(&[], TechId(6)),
            "Drift Mapping should be locked until Neutrino Sensors is completed"
        );
    }

    #[test]
    fn completed_prerequisite_unlocks_dependent_tech() {
        assert!(
            is_tech_available(&[TechId(3)], TechId(6)),
            "completing Neutrino Sensors should unlock Drift Mapping"
        );
    }

    #[test]
    fn available_tech_ids_order_is_deterministic() {
        let completed_unsorted = vec![TechId(5), TechId(2), TechId(3)];
        let first = available_tech_ids(&completed_unsorted);
        let second = available_tech_ids(&completed_unsorted);
        assert_eq!(
            first, second,
            "available tech ordering must be deterministic"
        );
        assert_eq!(
            first,
            vec![
                TechId(1),
                TechId(4),
                TechId(6),
                TechId(7),
                TechId(9),
                TechId(10),
                TechId(12),
            ],
            "available tech order should follow static deterministic tech definition order"
        );
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
            origin: StarId(0),
            total_duration: 3,
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
        assert!(state.survey_missions.is_empty());
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
                origin: StarId(0),
                total_duration: 2,
            },
        );
        assert_ne!(state_a, state_b);
    }

    #[test]
    fn game_state_partial_eq_considers_survey_missions() {
        let mut state_a = GameState::default();
        let state_b = GameState::default();
        assert_eq!(state_a, state_b);

        state_a.survey_missions.insert(
            FleetId(1),
            SurveyMission {
                fleet: FleetId(1),
                star: StarId(2),
                planet_index: 0,
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
                origin: StarId(0),
                total_duration: 2,
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
            origin: StarId(1),
            total_duration: 2,
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
                origin: StarId(5),
                total_duration: 2,
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
                origin: StarId(5),
                total_duration: 3,
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
    fn fleet_location_at_star_via_survey_mission() {
        let mut state = GameState::default();
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: EmpireId(1),
                location: StarId(5),
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );
        state.survey_missions.insert(
            FleetId(1),
            SurveyMission {
                fleet: FleetId(1),
                star: StarId(5),
                planet_index: 1,
                turns_remaining: 2,
            },
        );
        match state.fleet_location(FleetId(1)) {
            Some(FleetLocation::AtStar(id)) => assert_eq!(id, StarId(5)),
            other => panic!("Expected AtStar, got {:?}", other),
        }
    }

    #[test]
    fn survey_mission_fields() {
        let mission = SurveyMission {
            fleet: FleetId(3),
            star: StarId(7),
            planet_index: 2,
            turns_remaining: 2,
        };
        assert_eq!(mission.fleet, FleetId(3));
        assert_eq!(mission.star, StarId(7));
        assert_eq!(mission.planet_index, 2);
        assert_eq!(mission.turns_remaining, 2);
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
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
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
            stability: 100,
            role: ColonyRole::Balanced,
            rally_point: None,
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
