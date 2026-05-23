//! Game state types and domain models

use rand_chacha::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    pub const TROOP_TRANSPORTS: TechId = TechId(11);
    // Root-tier named constants for use in prerequisites
    pub const VOID_PROPULSION: TechId = TechId(1);
    pub const KINETIC_BARRIERS: TechId = TechId(4);
    pub const COLONIAL_LOGISTICS: TechId = TechId(10);
    pub const BATTLE_DOCTRINE: TechId = TechId(11);
    // Advanced ship archetype techs
    pub const RAPID_TRANSIT: TechId = TechId(13);
    pub const ADVANCED_SURVEY: TechId = TechId(14);
    pub const COLONIAL_VANGUARD: TechId = TechId(15);
    pub const PERIMETER_DEFENSE: TechId = TechId(16);
    pub const STRIKE_DOCTRINE: TechId = TechId(17);
    pub const FLEET_COORDINATION: TechId = TechId(18);
    pub const SECTOR_CARTOGRAPHY: TechId = TechId(19);
    pub const LANE_STABILIZATION: TechId = TechId(20);
    pub const PAN_GALACTIC_SENSOR_NET: TechId = TechId(21);
}

/// Unique identifier for a ship design template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ShipDesignId(pub u32);

/// Unique identifier for an empire definition (static faction template).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmpireDefinitionId(pub u8);

mod components;
mod factions;
mod ships;
mod technology;
mod victory;
pub use components::*;
pub use factions::*;
pub use ships::*;
pub use technology::*;
pub use victory::*;

/// Per-empire research progress tracking
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResearchState {
    /// The technology currently being researched, if any
    pub current_tech: Option<TechId>,
    /// Research points accumulated toward `current_tech`
    pub progress: i64,
    /// Ordered player-planned technologies to start automatically after completion
    #[cfg_attr(feature = "serde", serde(default))]
    pub queue: Vec<TechId>,
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

/// Strategic resource category for AI valuation and UI grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StrategicResourceCategory {
    Industrial,
    Energy,
    Military,
    Exotic,
    Biological,
    Precursor,
}

impl StrategicResourceCategory {
    pub fn label(self) -> &'static str {
        match self {
            StrategicResourceCategory::Industrial => "Industrial",
            StrategicResourceCategory::Energy => "Energy",
            StrategicResourceCategory::Military => "Military",
            StrategicResourceCategory::Exotic => "Exotic",
            StrategicResourceCategory::Biological => "Biological",
            StrategicResourceCategory::Precursor => "Precursor",
        }
    }
}

/// Strategic-resource rarity tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StrategicResourceRarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl StrategicResourceRarity {
    pub fn label(self) -> &'static str {
        match self {
            StrategicResourceRarity::Common => "Common",
            StrategicResourceRarity::Uncommon => "Uncommon",
            StrategicResourceRarity::Rare => "Rare",
            StrategicResourceRarity::Legendary => "Legendary",
        }
    }
}

/// Discovery gate for revealing a resource after survey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDiscoveryRequirements {
    pub surveyed: bool,
    pub required_tech: Option<TechId>,
}

/// Extraction gate for empire-wide access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceExtractionRequirements {
    pub requires_colony_control: bool,
    pub requires_supply: bool,
    pub blocked_by_blockade: bool,
    pub required_surface_building: Option<BuildingType>,
    pub required_orbital_structure: Option<OrbitalStructureType>,
    pub required_tech: Option<TechId>,
}

/// Static record defining one strategic resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategicResourceRecord {
    pub resource_id: u16,
    pub name: &'static str,
    pub description: &'static str,
    pub rarity: StrategicResourceRarity,
    pub category: StrategicResourceCategory,
    pub discovery_requirements: ResourceDiscoveryRequirements,
    pub extraction_requirements: ResourceExtractionRequirements,
    pub tech_requirements: Option<TechId>,
    pub strategic_effects: &'static [&'static str],
    pub trade_value: u16,
    pub future_hook_megaproject: bool,
}

/// Strategic resource presence on a planet — a capability modifier, not an inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum StrategicResource {
    QuantumCrystals,
    ReactiveIsotopes,
    DarkMatter,
    LivingAlloy,
    HyperfiberOrganics,
    Helium3,
    PsionicSpores,
    NeutroniumDeposits,
    AntimatterResidue,
    PrecursorDatacores,
}

impl StrategicResource {
    /// All strategic resources in deterministic `resource_id` order.
    ///
    /// Keep this aligned with `record().resource_id` for stable generation and save diffs.
    pub fn all() -> &'static [StrategicResource] {
        &[
            StrategicResource::QuantumCrystals,
            StrategicResource::ReactiveIsotopes,
            StrategicResource::DarkMatter,
            StrategicResource::LivingAlloy,
            StrategicResource::HyperfiberOrganics,
            StrategicResource::Helium3,
            StrategicResource::PsionicSpores,
            StrategicResource::NeutroniumDeposits,
            StrategicResource::AntimatterResidue,
            StrategicResource::PrecursorDatacores,
        ]
    }

    pub fn record(self) -> StrategicResourceRecord {
        match self {
            StrategicResource::QuantumCrystals => StrategicResourceRecord {
                resource_id: 1,
                name: "Quantum Crystals",
                description:
                    "Phase-stable crystal lattices that amplify defensive field harmonics.",
                rarity: StrategicResourceRarity::Rare,
                category: StrategicResourceCategory::Exotic,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId::ADVANCED_SURVEY),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::ScienceNexus),
                    required_orbital_structure: None,
                    required_tech: Some(TechId(14)),
                },
                tech_requirements: Some(TechId(14)),
                strategic_effects: &[
                    "Unlocks shield-matrix refinement",
                    "Adds empire-wide research calibration",
                ],
                trade_value: 110,
                future_hook_megaproject: true,
            },
            StrategicResource::ReactiveIsotopes => StrategicResourceRecord {
                resource_id: 2,
                name: "Reactive Isotopes",
                description: "Volatile isotope bundles ideal for advanced ordnance and boosters.",
                rarity: StrategicResourceRarity::Uncommon,
                category: StrategicResourceCategory::Military,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: None,
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::FabricationYard),
                    required_orbital_structure: None,
                    required_tech: None,
                },
                tech_requirements: None,
                strategic_effects: &[
                    "Unlocks missile rack stabilization",
                    "Improves wartime production tempo",
                ],
                trade_value: 70,
                future_hook_megaproject: false,
            },
            StrategicResource::DarkMatter => StrategicResourceRecord {
                resource_id: 3,
                name: "Dark Matter",
                description: "Containment-grade dark-mass traces suitable for extreme propulsion.",
                rarity: StrategicResourceRarity::Legendary,
                category: StrategicResourceCategory::Energy,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId::PAN_GALACTIC_SENSOR_NET),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: None,
                    required_orbital_structure: Some(OrbitalStructureType::Shipyard),
                    required_tech: Some(TechId::HYPERSPACE_CARTOGRAPHY),
                },
                tech_requirements: Some(TechId::PAN_GALACTIC_SENSOR_NET),
                strategic_effects: &[
                    "Unlocks elite drive architecture",
                    "Increases strategic fleet mobility",
                ],
                trade_value: 160,
                future_hook_megaproject: true,
            },
            StrategicResource::LivingAlloy => StrategicResourceRecord {
                resource_id: 4,
                name: "Living Alloy",
                description: "Self-healing metamaterial colonies with adaptive structural memory.",
                rarity: StrategicResourceRarity::Rare,
                category: StrategicResourceCategory::Exotic,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId(15)),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::FabricationYard),
                    required_orbital_structure: Some(OrbitalStructureType::Shipyard),
                    required_tech: Some(TechId(23)),
                },
                tech_requirements: Some(TechId(23)),
                strategic_effects: &[
                    "Unlocks resilient hull plating",
                    "Future hook for colossal construction",
                ],
                trade_value: 130,
                future_hook_megaproject: true,
            },
            StrategicResource::HyperfiberOrganics => StrategicResourceRecord {
                resource_id: 5,
                name: "Hyperfiber Organics",
                description: "Engineered growth filaments that optimize food and bio-mesh yields.",
                rarity: StrategicResourceRarity::Uncommon,
                category: StrategicResourceCategory::Biological,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId(9)),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::AquacultureBay),
                    required_orbital_structure: None,
                    required_tech: None,
                },
                tech_requirements: Some(TechId(9)),
                strategic_effects: &[
                    "Boosts biological throughput",
                    "Stabilizes frontier growth curves",
                ],
                trade_value: 65,
                future_hook_megaproject: false,
            },
            StrategicResource::Helium3 => StrategicResourceRecord {
                resource_id: 6,
                name: "Helium-3",
                description: "High-purity fusion feedstock that sustains long logistics chains.",
                rarity: StrategicResourceRarity::Common,
                category: StrategicResourceCategory::Energy,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: None,
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::FabricationYard),
                    required_orbital_structure: None,
                    required_tech: None,
                },
                tech_requirements: None,
                strategic_effects: &[
                    "Reduces per-fleet maintenance burden",
                    "Strengthens empire logistical endurance",
                ],
                trade_value: 45,
                future_hook_megaproject: false,
            },
            StrategicResource::PsionicSpores => StrategicResourceRecord {
                resource_id: 7,
                name: "Psionic Spores",
                description: "Neural-active spores enabling high-fidelity cognition interfaces.",
                rarity: StrategicResourceRarity::Rare,
                category: StrategicResourceCategory::Biological,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId(9)),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::ScienceNexus),
                    required_orbital_structure: None,
                    required_tech: Some(TechId(46)),
                },
                tech_requirements: Some(TechId(46)),
                strategic_effects: &[
                    "Accelerates advanced biology research",
                    "Improves diplomatic signal interpretation",
                ],
                trade_value: 120,
                future_hook_megaproject: false,
            },
            StrategicResource::NeutroniumDeposits => StrategicResourceRecord {
                resource_id: 8,
                name: "Neutronium Deposits",
                description: "Ultra-dense metal seams used for compact armor and bastions.",
                rarity: StrategicResourceRarity::Rare,
                category: StrategicResourceCategory::Industrial,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId(14)),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::FabricationYard),
                    required_orbital_structure: Some(OrbitalStructureType::Shipyard),
                    required_tech: Some(TechId(28)),
                },
                tech_requirements: Some(TechId(28)),
                strategic_effects: &[
                    "Reinforces fleet durability planning",
                    "Improves fortress-world survivability",
                ],
                trade_value: 140,
                future_hook_megaproject: true,
            },
            StrategicResource::AntimatterResidue => StrategicResourceRecord {
                resource_id: 9,
                name: "Antimatter Residue",
                description: "Residual annihilation condensate recovered from hazardous systems.",
                rarity: StrategicResourceRarity::Legendary,
                category: StrategicResourceCategory::Military,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId::PAN_GALACTIC_SENSOR_NET),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: None,
                    required_orbital_structure: Some(OrbitalStructureType::Shipyard),
                    required_tech: Some(TechId(34)),
                },
                tech_requirements: Some(TechId(34)),
                strategic_effects: &[
                    "Enables high-energy weapons programs",
                    "Elevates war-readiness pressure on rivals",
                ],
                trade_value: 180,
                future_hook_megaproject: true,
            },
            StrategicResource::PrecursorDatacores => StrategicResourceRecord {
                resource_id: 10,
                name: "Precursor Datacores",
                description: "Encrypted knowledge vaults left by vanished interstellar architects.",
                rarity: StrategicResourceRarity::Legendary,
                category: StrategicResourceCategory::Precursor,
                discovery_requirements: ResourceDiscoveryRequirements {
                    surveyed: true,
                    required_tech: Some(TechId::PAN_GALACTIC_SENSOR_NET),
                },
                extraction_requirements: ResourceExtractionRequirements {
                    requires_colony_control: true,
                    requires_supply: true,
                    blocked_by_blockade: true,
                    required_surface_building: Some(BuildingType::ScienceNexus),
                    required_orbital_structure: None,
                    required_tech: Some(TechId(63)),
                },
                tech_requirements: Some(TechId(63)),
                strategic_effects: &[
                    "Accelerates high-tier research lanes",
                    "Future hook for precursor megaproject chains",
                ],
                trade_value: 210,
                future_hook_megaproject: true,
            },
        }
    }

    pub fn name(self) -> &'static str {
        self.record().name
    }

    pub fn description(self) -> &'static str {
        self.record().description
    }

    pub fn rarity(self) -> StrategicResourceRarity {
        self.record().rarity
    }

    pub fn category(self) -> StrategicResourceCategory {
        self.record().category
    }

    pub fn trade_value(self) -> u16 {
        self.record().trade_value
    }

    pub fn strategic_effects(self) -> &'static [&'static str] {
        self.record().strategic_effects
    }

    /// Flat yield modifiers when the resource is discovered and actively extracted.
    pub fn yield_effect(self) -> YieldEffect {
        YieldEffect::default()
    }
}

/// Returns true when `resource` is revealable for an empire with `completed_techs`.
pub fn is_resource_discoverable(resource: StrategicResource, completed_techs: &[TechId]) -> bool {
    let req = resource.record().discovery_requirements;
    match req.required_tech {
        Some(tech) => completed_techs.contains(&tech),
        None => true,
    }
}

/// Returns the strategic resources visible to an empire on this planet.
///
/// Visibility requires survey completion and any resource-specific discovery tech.
pub fn visible_resources_for_empire(
    planet: &Planet,
    completed_techs: &[TechId],
) -> Vec<StrategicResource> {
    if !planet.surveyed {
        return Vec::new();
    }
    planet
        .resources
        .iter()
        .copied()
        .filter(|resource| is_resource_discoverable(*resource, completed_techs))
        .collect()
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
    /// The static empire definition (faction identity) chosen at game start.
    /// `None` for saves created before empire identities were introduced (pre-v22).
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_def: Option<EmpireDefinitionId>,
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
    /// A ship built from a player or AI custom design.
    CustomShip(CustomDesignId),
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
            ProductionItem::Ship(_)
            | ProductionItem::CustomShip(_)
            | ProductionItem::Scout
            | ProductionItem::Colony => "Ship",
            ProductionItem::Outpost => "Surface",
        }
    }

    /// Returns true if this production item is a ship.
    pub fn is_ship(&self) -> bool {
        matches!(
            self,
            ProductionItem::Ship(_)
                | ProductionItem::CustomShip(_)
                | ProductionItem::Scout
                | ProductionItem::Colony
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
    /// Returns `u64::MAX` for `CustomShip(_)` — callers that need the real cost should
    /// use `Engine::effective_build_cost` which can look up the design in state.
    pub fn cost(&self) -> u64 {
        match self {
            ProductionItem::Ship(design_id) => {
                design_id.record().map(|d| d.cost).unwrap_or(u64::MAX)
            }
            ProductionItem::CustomShip(_) => u64::MAX,
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
            ProductionItem::CustomShip(_) => "Custom Ship",
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
            ProductionItem::CustomShip(_) => None, // tech checks handled by engine
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

    /// Returns true when colony stability is low enough to be considered unrest.
    pub fn is_unrest(&self) -> bool {
        self.stability < 60
    }

    /// Human-readable stability state.
    pub fn unrest_label(&self) -> &'static str {
        if self.is_unrest() {
            "Unrest"
        } else {
            "Stable"
        }
    }
}

/// Whether a colony is connected to its empire trade/supply network this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColonySupplyState {
    Connected,
    Isolated,
}

impl ColonySupplyState {
    pub fn label(self) -> &'static str {
        match self {
            ColonySupplyState::Connected => "Connected",
            ColonySupplyState::Isolated => "Isolated",
        }
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

/// Strategic mission role assigned to a fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetRole {
    #[default]
    PatrolFleet,
    ExplorationFleet,
    SurveyGroup,
    ColonyEscort,
    StrikeFleet,
    DefenseFleet,
    InvasionFleet,
    BlockadeFleet,
    RapidResponseFleet,
    TradeProtectionFleet,
}

impl FleetRole {
    pub fn all() -> &'static [FleetRole] {
        &[
            FleetRole::ExplorationFleet,
            FleetRole::SurveyGroup,
            FleetRole::ColonyEscort,
            FleetRole::PatrolFleet,
            FleetRole::StrikeFleet,
            FleetRole::DefenseFleet,
            FleetRole::InvasionFleet,
            FleetRole::BlockadeFleet,
            FleetRole::RapidResponseFleet,
            FleetRole::TradeProtectionFleet,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            FleetRole::ExplorationFleet => "Exploration Fleet",
            FleetRole::SurveyGroup => "Survey Group",
            FleetRole::ColonyEscort => "Colony Escort",
            FleetRole::PatrolFleet => "Patrol Fleet",
            FleetRole::StrikeFleet => "Strike Fleet",
            FleetRole::DefenseFleet => "Defense Fleet",
            FleetRole::InvasionFleet => "Invasion Fleet",
            FleetRole::BlockadeFleet => "Blockade Fleet",
            FleetRole::RapidResponseFleet => "Rapid Response Fleet",
            FleetRole::TradeProtectionFleet => "Trade Protection Fleet",
        }
    }

    pub fn default_for_kind(kind: FleetKind) -> FleetRole {
        match kind {
            FleetKind::Scout | FleetKind::FastScout => FleetRole::ExplorationFleet,
            FleetKind::Science | FleetKind::SurveyCutter => FleetRole::SurveyGroup,
            FleetKind::Colonizer | FleetKind::ColonyArk => FleetRole::ColonyEscort,
            FleetKind::TroopTransport => FleetRole::InvasionFleet,
            FleetKind::EscortFrigate | FleetKind::PatrolCorvette => FleetRole::DefenseFleet,
            FleetKind::MissileFrigate | FleetKind::Destroyer => FleetRole::StrikeFleet,
        }
    }
}

/// Abstract fleet formation posture used by strategic auto-resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetFormation {
    #[default]
    Balanced,
    Aggressive,
    Defensive,
    FastAttack,
    Artillery,
    EscortScreen,
}

impl FleetFormation {
    pub fn all() -> &'static [FleetFormation] {
        &[
            FleetFormation::Balanced,
            FleetFormation::Aggressive,
            FleetFormation::Defensive,
            FleetFormation::FastAttack,
            FleetFormation::Artillery,
            FleetFormation::EscortScreen,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            FleetFormation::Balanced => "Balanced",
            FleetFormation::Aggressive => "Aggressive",
            FleetFormation::Defensive => "Defensive",
            FleetFormation::FastAttack => "Fast Attack",
            FleetFormation::Artillery => "Artillery",
            FleetFormation::EscortScreen => "Escort Screen",
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
    /// Troop transport ship — used for strategic planetary invasions
    TroopTransport,
    /// Fast Scout — reduced travel time, preferred by explorer factions
    FastScout,
    /// Survey Cutter — improved deep-survey capability
    SurveyCutter,
    /// Colony Ark — larger colony ship with better starting conditions
    ColonyArk,
    /// Escort Frigate — defensive light combat ship
    EscortFrigate,
    /// Missile Frigate — high-attack stand-off combatant
    MissileFrigate,
    /// Destroyer — strong mid-tier combat ship
    Destroyer,
    /// Patrol Corvette — cheap local security ship
    PatrolCorvette,
}

impl FleetKind {
    pub fn label(self) -> &'static str {
        match self {
            FleetKind::Scout => "Scout",
            FleetKind::Science => "Science Ship",
            FleetKind::Colonizer => "Colony Ship",
            FleetKind::TroopTransport => "Troop Transport",
            FleetKind::FastScout => "Fast Scout",
            FleetKind::SurveyCutter => "Survey Cutter",
            FleetKind::ColonyArk => "Colony Ark",
            FleetKind::EscortFrigate => "Escort Frigate",
            FleetKind::MissileFrigate => "Missile Frigate",
            FleetKind::Destroyer => "Destroyer",
            FleetKind::PatrolCorvette => "Patrol Corvette",
        }
    }

    /// Credits-per-turn maintenance cost for one fleet of this kind.
    pub fn maintenance_cost(self) -> u32 {
        match self {
            FleetKind::Scout
            | FleetKind::FastScout
            | FleetKind::Science
            | FleetKind::Colonizer
            | FleetKind::PatrolCorvette => 1,
            FleetKind::SurveyCutter
            | FleetKind::ColonyArk
            | FleetKind::TroopTransport
            | FleetKind::EscortFrigate => 2,
            FleetKind::MissileFrigate => 3,
            FleetKind::Destroyer => 4,
        }
    }

    /// Returns true if this fleet kind is a dedicated combat archetype.
    ///
    /// Note: `TroopTransport` is intentionally excluded — it is an invasion
    /// fleet handled separately in the engine, with its own cost modifier and
    /// AI preference flag (`prefers_troop_transports`).
    pub fn is_combat(self) -> bool {
        matches!(
            self,
            FleetKind::EscortFrigate
                | FleetKind::MissileFrigate
                | FleetKind::Destroyer
                | FleetKind::PatrolCorvette
        )
    }

    /// Returns true if this fleet kind is a colonization archetype.
    pub fn is_colonizer(self) -> bool {
        matches!(self, FleetKind::Colonizer | FleetKind::ColonyArk)
    }

    /// Returns true if this fleet kind is an exploration/scout archetype.
    pub fn is_scout(self) -> bool {
        matches!(self, FleetKind::Scout | FleetKind::FastScout)
    }

    /// Returns true if this fleet kind is a survey/science archetype.
    pub fn is_survey(self) -> bool {
        matches!(self, FleetKind::Science | FleetKind::SurveyCutter)
    }
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

/// Deterministic derived fleet summary used by AI planning and strategic UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FleetEvaluation {
    pub offensive: u32,
    pub defensive: u32,
    pub invasion_capability: u32,
    pub survey_capability: u32,
    pub mobility: u32,
    pub blockade_strength: u32,
    pub escort_quality: u32,
}

/// Deterministic auto-resolve phase identifiers used by Combat v2 reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CombatPhase {
    Detection,
    Positioning,
    OpeningVolley,
    MainEngagement,
    Attrition,
    RetreatOrCollapse,
    Resolution,
}

impl CombatPhase {
    pub fn label(self) -> &'static str {
        match self {
            CombatPhase::Detection => "Detection",
            CombatPhase::Positioning => "Positioning",
            CombatPhase::OpeningVolley => "Opening Volley",
            CombatPhase::MainEngagement => "Main Engagement",
            CombatPhase::Attrition => "Attrition",
            CombatPhase::RetreatOrCollapse => "Retreat/Collapse",
            CombatPhase::Resolution => "Resolution",
        }
    }
}

/// Per-phase deterministic summary values captured in a battle report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CombatPhaseSummary {
    pub phase: CombatPhase,
    pub pressure_a: u32,
    pub pressure_b: u32,
    pub note: String,
}

/// Structured tactical auto-resolve report for one fleet-vs-fleet engagement.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BattleReport {
    pub report_id: u64,
    pub turn: u32,
    pub star: StarId,
    pub fleet_a: FleetId,
    pub fleet_b: FleetId,
    pub empire_a: EmpireId,
    pub empire_b: EmpireId,
    pub role_a: FleetRole,
    pub role_b: FleetRole,
    pub formation_a: FleetFormation,
    pub formation_b: FleetFormation,
    pub doctrine_a: String,
    pub doctrine_b: String,
    pub kind_a: FleetKind,
    pub kind_b: FleetKind,
    pub ships_a: u32,
    pub ships_b: u32,
    pub integrity_a_start: u32,
    pub integrity_b_start: u32,
    pub integrity_a_end: u32,
    pub integrity_b_end: u32,
    pub fleet_a_destroyed: bool,
    pub fleet_b_destroyed: bool,
    pub fleet_a_retreated: bool,
    pub fleet_b_retreated: bool,
    pub phases: Vec<CombatPhaseSummary>,
    pub system_outcome: String,
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

#[cfg(feature = "serde")]
fn default_next_battle_report_id() -> u64 {
    1
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
    /// The empires have established first contact but no treaty yet
    Contacted,
    /// Relations are stable and peaceful
    Neutral,
    /// Relations are actively cooperative
    Cooperative,
    /// Relations are strained; no open hostility yet
    Tense,
    /// Empires are in open conflict; combat and blockades apply
    Hostile,
    /// Empires are in a formal state of war; combat and blockades apply
    War,
}

impl RelationshipStatus {
    /// Returns `true` when the status represents open warfare or active hostility.
    ///
    /// Fleets from a `Hostile` or `War` empire can blockade colonies.
    pub fn is_hostile_or_war(self) -> bool {
        matches!(self, RelationshipStatus::Hostile | RelationshipStatus::War)
    }

    /// Returns `true` when fleets of the two empires can engage in combat.
    ///
    /// `Contacted` is kept combat-eligible for backward compatibility with
    /// v1 saves and tests.  `Hostile` and `War` are also combat-eligible.
    pub fn is_combat_eligible(self) -> bool {
        matches!(
            self,
            RelationshipStatus::Contacted
                | RelationshipStatus::Tense
                | RelationshipStatus::Hostile
                | RelationshipStatus::War
        )
    }

    /// Short display label for this status.
    pub fn label(self) -> &'static str {
        match self {
            RelationshipStatus::Unknown => "Unknown",
            RelationshipStatus::Contacted => "Contacted",
            RelationshipStatus::Neutral => "Neutral",
            RelationshipStatus::Cooperative => "Cooperative",
            RelationshipStatus::Tense => "Tense",
            RelationshipStatus::Hostile => "Hostile",
            RelationshipStatus::War => "At War",
        }
    }
}

/// Supported treaty types in diplomacy v3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TreatyType {
    NonAggressionPact,
    Truce,
}

impl TreatyType {
    pub fn label(self) -> &'static str {
        match self {
            TreatyType::NonAggressionPact => "Non-Aggression Pact",
            TreatyType::Truce => "Truce",
        }
    }
}

/// Tone used when generating diplomatic communications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiplomaticTone {
    Cooperative,
    Formal,
    Suspicious,
    Threatening,
    Hostile,
    Desperate,
    Triumphant,
}

impl DiplomaticTone {
    pub fn label(self) -> &'static str {
        match self {
            DiplomaticTone::Cooperative => "Cooperative",
            DiplomaticTone::Formal => "Formal",
            DiplomaticTone::Suspicious => "Suspicious",
            DiplomaticTone::Threatening => "Threatening",
            DiplomaticTone::Hostile => "Hostile",
            DiplomaticTone::Desperate => "Desperate",
            DiplomaticTone::Triumphant => "Triumphant",
        }
    }
}

/// Structured diplomacy communication categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiplomaticCommunicationType {
    FirstContact,
    TreatyProposal,
    TreatyAccepted,
    TreatyRejected,
    Warning,
    TributeDemand,
    PeaceOffer,
    WarDeclaration,
}

/// Player/AI response options for communications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiplomaticResponse {
    Acknowledge,
    Accept,
    Reject,
    Comply,
    Refuse,
}

impl DiplomaticResponse {
    pub fn label(self) -> &'static str {
        match self {
            DiplomaticResponse::Acknowledge => "Acknowledge",
            DiplomaticResponse::Accept => "Accept",
            DiplomaticResponse::Reject => "Reject",
            DiplomaticResponse::Comply => "Comply",
            DiplomaticResponse::Refuse => "Refuse",
        }
    }
}

/// Active or historical treaty record for an empire pair.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiplomaticTreaty {
    pub treaty_type: TreatyType,
    pub with_empire: EmpireId,
    pub start_turn: u32,
    pub duration_turns: u32,
}

impl DiplomaticTreaty {
    pub fn expires_turn(&self) -> u32 {
        self.start_turn.saturating_add(self.duration_turns)
    }

    pub fn is_active(&self, current_turn: u32) -> bool {
        current_turn < self.expires_turn()
    }
}

/// Rich relationship state for diplomacy v3.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiplomaticRelationship {
    pub state: RelationshipStatus,
    pub relationship_score: i32,
    pub tension_score: i32,
    pub trust_score: Option<i32>,
    pub last_major_diplomatic_event_turn: u32,
    pub active_treaties: Vec<DiplomaticTreaty>,
    pub recent_grievances: Vec<String>,
    pub known_doctrine: Option<String>,
    pub first_contact_turn: Option<u32>,
}

impl DiplomaticRelationship {
    pub fn from_status(state: RelationshipStatus) -> Self {
        let (relationship_score, tension_score, trust_score) = match state {
            RelationshipStatus::Unknown => (-40, 0, None),
            RelationshipStatus::Contacted => (0, 5, Some(50)),
            RelationshipStatus::Neutral => (15, 10, Some(55)),
            RelationshipStatus::Cooperative => (35, 5, Some(70)),
            RelationshipStatus::Tense => (-10, 35, Some(40)),
            RelationshipStatus::Hostile => (-30, 65, Some(25)),
            RelationshipStatus::War => (-60, 100, Some(10)),
        };
        Self {
            state,
            relationship_score,
            tension_score,
            trust_score,
            last_major_diplomatic_event_turn: 0,
            active_treaties: Vec::new(),
            recent_grievances: Vec::new(),
            known_doctrine: None,
            first_contact_turn: None,
        }
    }
}

/// A single pending diplomatic communication.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DiplomaticCommunication {
    pub communication_id: u64,
    pub sending_empire: EmpireId,
    pub receiving_empire: EmpireId,
    pub turn: u32,
    pub communication_type: DiplomaticCommunicationType,
    pub tone: DiplomaticTone,
    pub title: String,
    pub body: String,
    pub available_responses: Vec<DiplomaticResponse>,
    pub expires_turn: Option<u32>,
    pub treaty_type: Option<TreatyType>,
}

/// Coarse galaxy size preset used in scenario setup.
///
/// Each variant defines default star and sector counts.  Counts are kept in a
/// range rather than a fixed value so that future per-size variation (e.g.
/// slightly randomised counts) remains backward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GalaxySize {
    /// Compact galaxy — 10 stars, 2 sectors
    Small,
    /// Standard galaxy — 20 stars, 4 sectors
    #[default]
    Medium,
    /// Large sprawling galaxy — 40 stars, 6 sectors
    Large,
}

impl GalaxySize {
    /// All available galaxy sizes in display order.
    pub fn all() -> &'static [GalaxySize] {
        &[GalaxySize::Small, GalaxySize::Medium, GalaxySize::Large]
    }

    /// Short display label.
    pub fn label(&self) -> &'static str {
        match self {
            GalaxySize::Small => "Small",
            GalaxySize::Medium => "Medium",
            GalaxySize::Large => "Large",
        }
    }

    /// Default number of star systems for this size.
    pub fn default_star_count(&self) -> usize {
        match self {
            GalaxySize::Small => 10,
            GalaxySize::Medium => 20,
            GalaxySize::Large => 40,
        }
    }

    /// Default number of sectors for this size.
    pub fn default_sector_count(&self) -> usize {
        match self {
            GalaxySize::Small => 2,
            GalaxySize::Medium => 4,
            GalaxySize::Large => 6,
        }
    }
}

/// Scenario setup options captured before a game starts.
///
/// These drive deterministic galaxy generation and empire placement.
/// The struct is stored in `GameState` so that save/load round-trips
/// preserve the original setup for display and future scenario tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScenarioSetup {
    /// Galaxy RNG seed.  Must be fixed before the game begins.
    pub seed: u64,
    /// Coarse size preset that drives star and sector counts.
    pub galaxy_size: GalaxySize,
    /// Number of AI-controlled empires (1 – 4).
    pub ai_empire_count: u8,
    /// Override for the number of sectors.  When `None` the count is
    /// derived from `galaxy_size`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sector_count_override: Option<usize>,
    /// Placeholder difficulty level label (v1 — no mechanical effect yet).
    #[cfg_attr(feature = "serde", serde(default))]
    pub difficulty: DifficultyLevel,
    /// The empire definition chosen by the player.  When `None` the engine
    /// assigns the first available definition deterministically.
    #[cfg_attr(feature = "serde", serde(default))]
    pub player_empire_def: Option<EmpireDefinitionId>,
    /// Configurable victory-path enablement and thresholds.
    #[cfg_attr(feature = "serde", serde(default))]
    pub victory_settings: VictorySettings,
}

impl ScenarioSetup {
    /// Construct a setup with sensible defaults.
    pub fn default_for_seed(seed: u64) -> Self {
        ScenarioSetup {
            seed,
            galaxy_size: GalaxySize::Medium,
            ai_empire_count: 1,
            sector_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: None,
            victory_settings: VictorySettings::default_v1(),
        }
    }

    /// Effective number of star systems for this setup.
    pub fn effective_star_count(&self) -> usize {
        self.galaxy_size.default_star_count()
    }

    /// Effective number of sectors for this setup.
    pub fn effective_sector_count(&self) -> usize {
        match self.sector_count_override {
            Some(n) => n.clamp(2, 8),
            None => self.galaxy_size.default_sector_count(),
        }
    }

    /// Validate setup options.  Returns an error string if any value is out of range.
    pub fn validate(&self) -> Result<(), String> {
        if self.ai_empire_count == 0 || self.ai_empire_count > 4 {
            return Err(format!(
                "AI empire count must be 1–4, got {}",
                self.ai_empire_count
            ));
        }
        if let Some(n) = self.sector_count_override {
            if !(2..=8).contains(&n) {
                return Err(format!("Sector count must be 2–8, got {}", n));
            }
        }
        if let Some(def_id) = self.player_empire_def {
            if empire_definition_by_id(def_id).is_none() {
                return Err(format!("Unknown player empire definition id {}", def_id.0));
            }
        }
        let mut seen_paths = BTreeSet::new();
        for condition in &self.victory_settings.conditions {
            if !seen_paths.insert(condition.path()) {
                return Err(format!(
                    "Duplicate victory condition configured for {}",
                    condition.path().label()
                ));
            }
            match condition {
                VictoryCondition::Dominion {
                    control_percent_required,
                    ..
                } if *control_percent_required == 0 || *control_percent_required > 100 => {
                    return Err(format!(
                        "Dominion control threshold must be 1–100, got {}",
                        control_percent_required
                    ));
                }
                VictoryCondition::Prosperity {
                    avg_stability_required,
                    ..
                } if *avg_stability_required > 200 => {
                    return Err(format!(
                        "Prosperity stability threshold must be 0–200, got {}",
                        avg_stability_required
                    ));
                }
                VictoryCondition::Discovery {
                    systems_explored_percent_required,
                    planets_surveyed_percent_required,
                    ..
                } if *systems_explored_percent_required > 100
                    || *planets_surveyed_percent_required > 100 =>
                {
                    return Err(format!(
                        "Discovery thresholds must be <=100, got systems={} planets={}",
                        systems_explored_percent_required, planets_surveyed_percent_required
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Default for ScenarioSetup {
    fn default() -> Self {
        ScenarioSetup::default_for_seed(42)
    }
}

/// Placeholder difficulty level (v1 — no mechanical effect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DifficultyLevel {
    #[default]
    Standard,
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
    /// Rich diplomacy relationship data keyed by foreign empire ID.
    #[cfg_attr(feature = "serde", serde(default))]
    pub diplomacy_relationships: BTreeMap<EmpireId, DiplomaticRelationship>,
    /// Pending diplomatic communications (processed in queue order).
    #[cfg_attr(feature = "serde", serde(default))]
    pub diplomacy_pending_communications: VecDeque<DiplomaticCommunication>,
    /// Monotonic communication id counter.
    #[cfg_attr(feature = "serde", serde(default))]
    pub diplomacy_next_communication_id: u64,
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
    /// Authoritative strategic role assignment per fleet.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_roles: BTreeMap<FleetId, FleetRole>,
    /// Authoritative formation stance per fleet.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_formations: BTreeMap<FleetId, FleetFormation>,
    /// Optional user/AI-assigned fleet names.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_names: BTreeMap<FleetId, String>,
    /// Scenario setup used to create this game.  Preserved through save/load
    /// for display and future scenario tooling.  `None` for games started
    /// before this field was introduced (pre-v20 saves).
    #[cfg_attr(feature = "serde", serde(default))]
    pub scenario: Option<ScenarioSetup>,
    /// All AI-controlled empire IDs in the game.
    ///
    /// Supersedes the legacy `ai_empire` field which always points to the
    /// first entry of this list (when non-empty) for backward compatibility.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ai_empires: Vec<EmpireId>,
    /// Last computed colony supply states, keyed by colony ID.
    ///
    /// This map is deterministic and derivable from galaxy + empire + colony state.
    /// It is persisted to simplify UI rendering and turn-over-turn transition reporting.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_supply: BTreeMap<ColonyId, ColonySupplyState>,
    /// Active blockades this turn: maps blockaded `ColonyId` to the `EmpireId` of the
    /// primary blockading empire.
    ///
    /// Derived each turn from idle hostile/war-status fleet positions.
    /// Persisted to detect start/end transitions for event emission on the next turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_blockade: BTreeMap<ColonyId, EmpireId>,
    /// Current strategic-resource access counts per empire.
    ///
    /// Counts are deterministic and derived from colony control, survey/discovery,
    /// extraction requirements, supply connectivity, and blockade status.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_resource_access: BTreeMap<EmpireId, BTreeMap<StrategicResource, u32>>,
    /// Deterministic victory-condition progress and winner state.
    #[cfg_attr(feature = "serde", serde(default))]
    pub victory_status: VictoryStatus,
    /// Recent Galactic Dispatch bulletins.  Bounded to
    /// `crate::dispatch::DISPATCH_MAX_HISTORY` entries.  Newest dispatch is at
    /// the back.
    #[cfg_attr(feature = "serde", serde(default))]
    pub galactic_dispatches: VecDeque<crate::dispatch::GalacticDispatch>,
    /// Custom ship designs created by players or generated for AI empires.
    #[cfg_attr(feature = "serde", serde(default))]
    pub custom_designs: BTreeMap<CustomDesignId, CustomShipDesign>,
    /// Counter for the next custom design ID to allocate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub next_custom_design_id: u32,
    /// Maps fleets built from custom designs to their originating design ID.
    /// Used to apply derived stats (maintenance, defense) for custom-built fleets.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_custom_designs: BTreeMap<FleetId, CustomDesignId>,
    /// Monotonic identifier for generated battle reports.
    #[cfg_attr(feature = "serde", serde(default = "default_next_battle_report_id"))]
    pub next_battle_report_id: u64,
    /// Recent deterministic combat battle reports (oldest at front).
    #[cfg_attr(feature = "serde", serde(default))]
    pub battle_reports: VecDeque<BattleReport>,
}

impl GameState {
    // Trade-link distance thresholds in galaxy coordinate units.
    //
    // Stars are generated in roughly ±500 space on each axis.  We allow longer
    // same-sector links (550 units) to keep nearby local colonies connected, and
    // a tighter cross-sector threshold (325 units) so sector boundaries retain
    // strategic weight unless bridged by hyperspace lanes.
    const TRADE_LINK_RANGE_SQ_SAME_SECTOR: i64 = 550 * 550;
    const TRADE_LINK_RANGE_SQ_CROSS_SECTOR: i64 = 325 * 325;

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

    pub fn colony_supply_state(&self, colony_id: ColonyId) -> ColonySupplyState {
        self.colony_supply
            .get(&colony_id)
            .copied()
            .unwrap_or(ColonySupplyState::Connected)
    }

    /// Returns the empire currently blockading `colony_id`, or `None` if unblockaded.
    pub fn colony_blockade_state(&self, colony_id: ColonyId) -> Option<EmpireId> {
        self.colony_blockade.get(&colony_id).copied()
    }

    /// Returns true when a colony currently satisfies extraction requirements for `resource`.
    pub fn colony_can_extract_resource(
        &self,
        colony_id: ColonyId,
        resource: StrategicResource,
    ) -> bool {
        let Some(colony) = self.colonies.get(&colony_id) else {
            return false;
        };
        let Some(planet) = self
            .stars
            .get(&colony.star)
            .and_then(|s| s.planets.get(colony.planet_index))
        else {
            return false;
        };
        if !planet.surveyed {
            return false;
        }
        if !planet.resources.contains(&resource) {
            return false;
        }
        let Some(empire) = self.empires.get(&colony.owner) else {
            return false;
        };
        let completed_techs = &empire.research.completed;
        if !is_resource_discoverable(resource, completed_techs) {
            return false;
        }

        let extraction = resource.record().extraction_requirements;
        if extraction.requires_colony_control && colony.owner != empire.id {
            return false;
        }
        if extraction.requires_supply
            && self.colony_supply_state(colony_id) != ColonySupplyState::Connected
        {
            return false;
        }
        if extraction.blocked_by_blockade && self.colony_blockade_state(colony_id).is_some() {
            return false;
        }
        if let Some(required) = extraction.required_surface_building {
            if !colony.buildings.contains(&required) {
                return false;
            }
        }
        if let Some(required) = extraction.required_orbital_structure {
            if !colony.orbital_installations.contains(&required) {
                return false;
            }
        }
        if let Some(required_tech) = extraction.required_tech {
            if !completed_techs.contains(&required_tech) {
                return false;
            }
        }
        true
    }

    /// Recompute deterministic strategic-resource access counts for all empires.
    pub fn recompute_empire_resource_access(
        &self,
    ) -> BTreeMap<EmpireId, BTreeMap<StrategicResource, u32>> {
        let mut access: BTreeMap<EmpireId, BTreeMap<StrategicResource, u32>> = BTreeMap::new();
        for (colony_id, colony) in &self.colonies {
            let Some(planet) = self
                .stars
                .get(&colony.star)
                .and_then(|s| s.planets.get(colony.planet_index))
            else {
                continue;
            };
            let Some(empire) = self.empires.get(&colony.owner) else {
                continue;
            };
            let visible = visible_resources_for_empire(planet, &empire.research.completed);
            for resource in visible {
                // Count each extractable colony-source for this empire/resource pair.
                if self.colony_can_extract_resource(*colony_id, resource) {
                    *access
                        .entry(colony.owner)
                        .or_default()
                        .entry(resource)
                        .or_insert(0) += 1;
                }
            }
        }
        access
    }

    /// Convenience accessor: extraction count for one empire/resource.
    pub fn empire_resource_count(&self, empire_id: EmpireId, resource: StrategicResource) -> u32 {
        self.empire_resource_access
            .get(&empire_id)
            .and_then(|by_resource| by_resource.get(&resource))
            .copied()
            .unwrap_or(0)
    }

    /// Return fleet role, defaulting deterministically from fleet kind.
    pub fn fleet_role_for(&self, fleet_id: FleetId) -> FleetRole {
        self.fleet_roles
            .get(&fleet_id)
            .copied()
            .or_else(|| {
                self.fleets
                    .get(&fleet_id)
                    .map(|fleet| FleetRole::default_for_kind(fleet.kind))
            })
            .unwrap_or(FleetRole::PatrolFleet)
    }

    /// Return fleet formation, defaulting to Balanced.
    pub fn fleet_formation_for(&self, fleet_id: FleetId) -> FleetFormation {
        self.fleet_formations
            .get(&fleet_id)
            .copied()
            .unwrap_or(FleetFormation::Balanced)
    }

    /// Return fleet display name, falling back to deterministic generated name.
    pub fn fleet_name_for(&self, fleet_id: FleetId) -> String {
        if let Some(name) = self.fleet_names.get(&fleet_id) {
            return name.clone();
        }
        self.fleets
            .get(&fleet_id)
            .map(|fleet| format!("{} {}", fleet.kind.label(), fleet_id.0))
            .unwrap_or_else(|| format!("Unknown Fleet {}", fleet_id.0))
    }

    /// Compute deterministic strategic summary for one fleet.
    pub fn fleet_evaluation(&self, fleet_id: FleetId) -> Option<FleetEvaluation> {
        let fleet = self.fleets.get(&fleet_id)?;
        let role = self.fleet_role_for(fleet_id);
        let formation = self.fleet_formation_for(fleet_id);

        let mut offensive = fleet.strength.saturating_mul(fleet.ships.max(1));
        let mut defensive = (fleet.integrity / 10)
            .saturating_add(fleet.strength)
            .saturating_mul(fleet.ships.max(1));
        let mut invasion_capability = if fleet.kind == FleetKind::TroopTransport {
            fleet.ships.saturating_mul(12)
        } else {
            0
        };
        let mut survey_capability: u32 = if fleet.kind.is_survey() { 100 } else { 0 };
        let mut mobility: i32 = 100;
        let mut blockade_strength =
            if fleet.kind.is_combat() || fleet.kind == FleetKind::TroopTransport {
                fleet.strength
            } else {
                0
            };
        let mut escort_quality = if matches!(
            fleet.kind,
            FleetKind::EscortFrigate | FleetKind::PatrolCorvette | FleetKind::Destroyer
        ) {
            fleet.strength
        } else {
            0
        };

        if let Some(design_id) = self.fleet_custom_designs.get(&fleet_id) {
            if let Some(design) = self.custom_designs.get(design_id) {
                let stats = design.derived_stats();
                offensive = offensive.saturating_add(stats.attack);
                defensive = defensive.saturating_add(stats.defense.saturating_add(stats.hp / 5));
                invasion_capability = invasion_capability.saturating_add(stats.invasion_strength);
                survey_capability = survey_capability.saturating_add(stats.survey_effectiveness);
            }
        }

        match role {
            FleetRole::ExplorationFleet => {
                offensive = offensive.saturating_mul(85) / 100;
                defensive = defensive.saturating_mul(90) / 100;
                mobility += 20;
            }
            FleetRole::SurveyGroup => {
                survey_capability = survey_capability.saturating_add(35);
                mobility += 10;
            }
            FleetRole::ColonyEscort => {
                escort_quality = escort_quality.saturating_add(12);
                defensive = defensive.saturating_mul(110) / 100;
            }
            FleetRole::PatrolFleet => {
                blockade_strength = blockade_strength.saturating_add(8);
            }
            FleetRole::StrikeFleet => {
                offensive = offensive.saturating_mul(120) / 100;
            }
            FleetRole::DefenseFleet => {
                defensive = defensive.saturating_mul(125) / 100;
                escort_quality = escort_quality.saturating_add(18);
            }
            FleetRole::InvasionFleet => {
                invasion_capability = invasion_capability.saturating_add(20);
                escort_quality = escort_quality.saturating_add(6);
            }
            FleetRole::BlockadeFleet => {
                blockade_strength = blockade_strength.saturating_mul(140) / 100;
                offensive = offensive.saturating_mul(110) / 100;
            }
            FleetRole::RapidResponseFleet => {
                mobility += 30;
                offensive = offensive.saturating_mul(105) / 100;
            }
            FleetRole::TradeProtectionFleet => {
                defensive = defensive.saturating_mul(115) / 100;
                escort_quality = escort_quality.saturating_add(14);
            }
        }

        match formation {
            FleetFormation::Balanced => {}
            FleetFormation::Aggressive => {
                offensive = offensive.saturating_mul(125) / 100;
                defensive = defensive.saturating_mul(90) / 100;
            }
            FleetFormation::Defensive => {
                offensive = offensive.saturating_mul(90) / 100;
                defensive = defensive.saturating_mul(125) / 100;
                mobility -= 10;
            }
            FleetFormation::FastAttack => {
                mobility += 20;
                offensive = offensive.saturating_mul(110) / 100;
                defensive = defensive.saturating_mul(85) / 100;
            }
            FleetFormation::Artillery => {
                offensive = offensive.saturating_mul(120) / 100;
                defensive = defensive.saturating_mul(90) / 100;
                blockade_strength = blockade_strength.saturating_add(10);
            }
            FleetFormation::EscortScreen => {
                offensive = offensive.saturating_mul(90) / 100;
                defensive = defensive.saturating_mul(120) / 100;
                escort_quality = escort_quality.saturating_add(25);
            }
        }

        if let Some(def) = self
            .empires
            .get(&fleet.owner)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
        {
            let aggression = def.doctrine_weight(AiDoctrine::Militarist) as u32
                + def.doctrine_weight(AiDoctrine::Imperial) as u32;
            let caution = def.doctrine_weight(AiDoctrine::Isolationist) as u32
                + def.doctrine_weight(AiDoctrine::Merchant) as u32;
            let mobility_bias = def.doctrine_weight(AiDoctrine::Explorer) as i32
                + def.doctrine_weight(AiDoctrine::Expansionist) as i32;

            offensive = offensive.saturating_mul(100 + aggression.min(25)) / 100;
            defensive = defensive.saturating_mul(100 + caution.min(25)) / 100;
            mobility += (mobility_bias / 2).clamp(-20, 20);
        }

        Some(FleetEvaluation {
            offensive: offensive.max(1),
            defensive: defensive.max(1),
            invasion_capability,
            survey_capability,
            mobility: mobility.clamp(50, 180) as u32,
            blockade_strength,
            escort_quality,
        })
    }

    /// Derive the relationship between two empires from the player's perspective.
    ///
    /// If neither empire is the player, returns `Unknown` (AI–AI not tracked).
    pub fn relationship_status(
        &self,
        empire_a: EmpireId,
        empire_b: EmpireId,
    ) -> RelationshipStatus {
        let player = self.player_empire;
        let other = if empire_a == player {
            empire_b
        } else if empire_b == player {
            empire_a
        } else {
            return RelationshipStatus::Unknown;
        };
        if let Some(relationship) = self.diplomacy_relationships.get(&other) {
            return relationship.state;
        }
        self.diplomacy
            .get(&other)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown)
    }

    /// Returns relationship data for a foreign empire if present.
    pub fn relationship_data(&self, other: EmpireId) -> Option<&DiplomaticRelationship> {
        self.diplomacy_relationships.get(&other)
    }

    /// Returns true when an active treaty of `treaty_type` exists with `other`.
    pub fn has_active_treaty(&self, other: EmpireId, treaty_type: TreatyType) -> bool {
        self.diplomacy_relationships
            .get(&other)
            .is_some_and(|relationship| {
                relationship
                    .active_treaties
                    .iter()
                    .any(|treaty| treaty.treaty_type == treaty_type && treaty.is_active(self.turn))
            })
    }

    /// Recompute which colonies are blockaded based on current idle fleet positions
    /// and diplomacy.
    ///
    /// A colony is blockaded when:
    /// 1. At least one idle enemy fleet with `Hostile` or `War` relationship is present
    ///    at the colony's star system.
    /// 2. No idle friendly fleet belonging to the colony owner is present at that star.
    ///
    /// Returns a map from blockaded `ColonyId` to the primary blockading `EmpireId`
    /// (lowest `FleetId` among the hostile fleets, for determinism).
    pub fn recompute_colony_blockade(&self) -> BTreeMap<ColonyId, EmpireId> {
        // Index idle fleet owners by star. BTreeMap iteration is deterministic.
        let mut star_idle_fleets: BTreeMap<StarId, Vec<(FleetId, EmpireId)>> = BTreeMap::new();
        for (fleet_id, fleet) in &self.fleets {
            if !self.fleet_missions.contains_key(fleet_id)
                && !self.scout_missions.contains_key(fleet_id)
                && !self.survey_missions.contains_key(fleet_id)
            {
                star_idle_fleets
                    .entry(fleet.location)
                    .or_default()
                    .push((*fleet_id, fleet.owner));
            }
        }

        let mut blockaded = BTreeMap::new();

        for (colony_id, colony) in &self.colonies {
            let colony_star = colony.star;
            let colony_owner = colony.owner;

            let idle = match star_idle_fleets.get(&colony_star) {
                Some(v) => v.as_slice(),
                None => continue,
            };

            // Is there any hostile/war fleet at this star?
            let blockading_fleet = idle
                .iter()
                .filter(|(_, owner)| {
                    *owner != colony_owner
                        && self
                            .relationship_status(colony_owner, *owner)
                            .is_hostile_or_war()
                })
                .min_by_key(|(fid, _)| {
                    let strength = self
                        .fleet_evaluation(*fid)
                        .map(|eval| eval.blockade_strength)
                        .unwrap_or(0);
                    (u32::MAX.saturating_sub(strength), *fid)
                });

            if let Some((_, blockading_empire)) = blockading_fleet {
                // Only blocked if no friendly idle fleet is also present.
                let has_defender = idle.iter().any(|(_, owner)| *owner == colony_owner);
                if !has_defender {
                    blockaded.insert(*colony_id, *blockading_empire);
                }
            }
        }

        blockaded
    }

    pub fn recompute_colony_supply(&self) -> BTreeMap<ColonyId, ColonySupplyState> {
        let mut supply = BTreeMap::new();
        for empire_id in self.empires.keys().copied() {
            let empire_colonies: Vec<(ColonyId, StarId)> = self
                .colonies
                .iter()
                .filter(|(_, c)| c.owner == empire_id)
                .map(|(cid, c)| (*cid, c.star))
                .collect();
            if empire_colonies.is_empty() {
                continue;
            }

            let mut colony_stars: BTreeSet<StarId> = BTreeSet::new();
            for (_, star_id) in &empire_colonies {
                colony_stars.insert(*star_id);
            }

            let Some(hub_star) = self.empire_trade_hub_star(empire_id, &empire_colonies) else {
                continue;
            };

            let mut reachable = BTreeSet::new();
            let mut queue = VecDeque::new();
            reachable.insert(hub_star);
            queue.push_back(hub_star);

            while let Some(from_star) = queue.pop_front() {
                for to_star in colony_stars.iter().copied() {
                    if reachable.contains(&to_star) {
                        continue;
                    }
                    if self.stars_have_trade_link(empire_id, from_star, to_star) {
                        reachable.insert(to_star);
                        queue.push_back(to_star);
                    }
                }
            }

            for (colony_id, star_id) in empire_colonies {
                let state = if reachable.contains(&star_id) {
                    ColonySupplyState::Connected
                } else {
                    ColonySupplyState::Isolated
                };
                supply.insert(colony_id, state);
            }
        }
        supply
    }

    fn empire_trade_hub_star(
        &self,
        empire_id: EmpireId,
        empire_colonies: &[(ColonyId, StarId)],
    ) -> Option<StarId> {
        let home_star = self.empires.get(&empire_id).map(|e| e.home_star);
        if let Some(home_star) = home_star {
            if empire_colonies.iter().any(|(_, star)| *star == home_star) {
                return Some(home_star);
            }
        }
        empire_colonies.iter().map(|(_, star)| *star).next()
    }

    fn empire_has_hyperspace_trade(&self, empire_id: EmpireId) -> bool {
        self.empires.get(&empire_id).is_some_and(|e| {
            e.research
                .completed
                .contains(&TechId::HYPERSPACE_CARTOGRAPHY)
        })
    }

    fn empire_can_use_trade_lane(&self, empire_id: EmpireId, lane: HyperspaceLane) -> bool {
        if !self.hyperspace_lanes.contains(&lane) || !self.empire_has_hyperspace_trade(empire_id) {
            return false;
        }
        if empire_id == self.player_empire {
            self.known_hyperspace_lanes.contains(&lane)
        } else {
            true
        }
    }

    fn stars_have_trade_link(&self, empire_id: EmpireId, from: StarId, to: StarId) -> bool {
        if from == to {
            return true;
        }

        if let Some(lane) = HyperspaceLane::new(from, to) {
            if self.empire_can_use_trade_lane(empire_id, lane) {
                return true;
            }
        }

        let (Some(a), Some(b)) = (self.stars.get(&from), self.stars.get(&to)) else {
            return false;
        };
        let dx = (a.x - b.x) as i64;
        let dy = (a.y - b.y) as i64;
        let sq_dist = dx * dx + dy * dy;
        let max_sq_dist = if a.sector == b.sector {
            Self::TRADE_LINK_RANGE_SQ_SAME_SECTOR
        } else {
            Self::TRADE_LINK_RANGE_SQ_CROSS_SECTOR
        };
        sq_dist <= max_sq_dist
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
            && self.diplomacy_relationships == other.diplomacy_relationships
            && self.diplomacy_pending_communications == other.diplomacy_pending_communications
            && self.diplomacy_next_communication_id == other.diplomacy_next_communication_id
            && self.hyperspace_lanes == other.hyperspace_lanes
            && self.known_hyperspace_lanes == other.known_hyperspace_lanes
            && self.fleet_orders == other.fleet_orders
            && self.fleet_roles == other.fleet_roles
            && self.fleet_formations == other.fleet_formations
            && self.fleet_names == other.fleet_names
            && self.scenario == other.scenario
            && self.ai_empires == other.ai_empires
            && self.colony_supply == other.colony_supply
            && self.colony_blockade == other.colony_blockade
            && self.empire_resource_access == other.empire_resource_access
            && self.victory_status == other.victory_status
            && self.custom_designs == other.custom_designs
            && self.next_custom_design_id == other.next_custom_design_id
            && self.fleet_custom_designs == other.fleet_custom_designs
            && self.galactic_dispatches == other.galactic_dispatches
            && self.next_battle_report_id == other.next_battle_report_id
            && self.battle_reports == other.battle_reports
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
            diplomacy_relationships: BTreeMap::new(),
            diplomacy_pending_communications: VecDeque::new(),
            diplomacy_next_communication_id: 1,
            hyperspace_lanes: BTreeSet::new(),
            known_hyperspace_lanes: BTreeSet::new(),
            fleet_orders: BTreeMap::new(),
            fleet_roles: BTreeMap::new(),
            fleet_formations: BTreeMap::new(),
            fleet_names: BTreeMap::new(),
            scenario: None,
            ai_empires: Vec::new(),
            colony_supply: BTreeMap::new(),
            colony_blockade: BTreeMap::new(),
            empire_resource_access: BTreeMap::new(),
            victory_status: VictoryStatus::default(),
            galactic_dispatches: VecDeque::new(),
            custom_designs: BTreeMap::new(),
            next_custom_design_id: 0,
            fleet_custom_designs: BTreeMap::new(),
            next_battle_report_id: 1,
            battle_reports: VecDeque::new(),
        }
    }
}

#[cfg(test)]
mod tests;
