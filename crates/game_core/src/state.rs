//! Game state types and domain models

use crate::balance;
use rand::rngs::ChaCha8Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::{Deref, DerefMut};

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
mod validate;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiscoveryRarity {
    Common,
    Uncommon,
    Rare,
    Legendary,
}

impl DiscoveryRarity {
    pub fn label(self) -> &'static str {
        match self {
            DiscoveryRarity::Common => "Common",
            DiscoveryRarity::Uncommon => "Uncommon",
            DiscoveryRarity::Rare => "Rare",
            DiscoveryRarity::Legendary => "Legendary",
        }
    }

    pub const fn valuation_weight(self) -> i32 {
        match self {
            DiscoveryRarity::Common => 1,
            DiscoveryRarity::Uncommon => 2,
            DiscoveryRarity::Rare => 4,
            DiscoveryRarity::Legendary => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetSpecialCategory {
    Resource,
    Scientific,
    Biological,
    Industrial,
    Environmental,
    Precursor,
    Hazard,
    Strategic,
    Cultural,
}

impl PlanetSpecialCategory {
    pub fn label(self) -> &'static str {
        match self {
            PlanetSpecialCategory::Resource => "Resource",
            PlanetSpecialCategory::Scientific => "Scientific",
            PlanetSpecialCategory::Biological => "Biological",
            PlanetSpecialCategory::Industrial => "Industrial",
            PlanetSpecialCategory::Environmental => "Environmental",
            PlanetSpecialCategory::Precursor => "Precursor",
            PlanetSpecialCategory::Hazard => "Hazard",
            PlanetSpecialCategory::Strategic => "Strategic",
            PlanetSpecialCategory::Cultural => "Cultural",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnomalyCategory {
    Stellar,
    Precursor,
    Biological,
    Temporal,
    Gravitational,
    Military,
    Archaeological,
    ExoticPhysics,
}

impl AnomalyCategory {
    pub fn label(self) -> &'static str {
        match self {
            AnomalyCategory::Stellar => "Stellar",
            AnomalyCategory::Precursor => "Precursor",
            AnomalyCategory::Biological => "Biological",
            AnomalyCategory::Temporal => "Temporal",
            AnomalyCategory::Gravitational => "Gravitational",
            AnomalyCategory::Military => "Military",
            AnomalyCategory::Archaeological => "Archaeological",
            AnomalyCategory::ExoticPhysics => "Exotic Physics",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnomalyRiskLevel {
    Low,
    Moderate,
    High,
    Severe,
}

impl AnomalyRiskLevel {
    pub fn label(self) -> &'static str {
        match self {
            AnomalyRiskLevel::Low => "Low",
            AnomalyRiskLevel::Moderate => "Moderate",
            AnomalyRiskLevel::High => "High",
            AnomalyRiskLevel::Severe => "Severe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveryRequirements {
    pub surveyed: bool,
    pub required_techs: &'static [TechId],
}

impl DiscoveryRequirements {
    pub const fn surveyed() -> Self {
        Self {
            surveyed: true,
            required_techs: &[],
        }
    }

    pub const fn surveyed_with(required_techs: &'static [TechId]) -> Self {
        Self {
            surveyed: true,
            required_techs,
        }
    }
}

/// Returns true when `requirements` are satisfied by the given survey + completed techs.
pub fn requirements_met(
    requirements: DiscoveryRequirements,
    surveyed: bool,
    completed_techs: &[TechId],
) -> bool {
    (!requirements.surveyed || surveyed)
        && requirements
            .required_techs
            .iter()
            .all(|tech| completed_techs.contains(tech))
}

/// Discoverable planet special that modifies colony yield or triggers one-time events.
///
/// Specials are generated deterministically from the galaxy seed and hidden until
/// survey is complete. A colonized planet automatically benefits from its revealed
/// specials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetSpecial {
    MineralRich,
    FertileBiosphere,
    AncientRuins,
    CrystalFormations,
    HostileWeather,
    LowGravity,
    CrystalForests,
    SubterraneanMegacaverns,
    HyperconductiveOceans,
    VolatileCoreInstability,
    PrecursorBeacon,
    BioluminescentJungles,
    AncientDefenseGrid,
    FrozenDataVault,
    NaniteScarfields,
    GravitationalFractureZone,
    OrbitalGraveyard,
}

/// Survey-revealed anomaly anchored to a planetary orbit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PlanetAnomaly {
    SilentRelayNetwork,
    TemporalEchoField,
    CollapsedJumpCorridor,
    PrecursorListeningPost,
    RogueNaniteSwarm,
    QuantumReflectionZone,
    FrozenColonyVessel,
    GraviticStormFront,
    DerelictBattleSphere,
    VoidSignalArray,
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
            PlanetSpecial::CrystalForests,
            PlanetSpecial::SubterraneanMegacaverns,
            PlanetSpecial::HyperconductiveOceans,
            PlanetSpecial::VolatileCoreInstability,
            PlanetSpecial::PrecursorBeacon,
            PlanetSpecial::BioluminescentJungles,
            PlanetSpecial::AncientDefenseGrid,
            PlanetSpecial::FrozenDataVault,
            PlanetSpecial::NaniteScarfields,
            PlanetSpecial::GravitationalFractureZone,
            PlanetSpecial::OrbitalGraveyard,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => "mineral_rich",
            PlanetSpecial::FertileBiosphere => "fertile_biosphere",
            PlanetSpecial::AncientRuins => "ancient_ruins",
            PlanetSpecial::CrystalFormations => "crystal_formations",
            PlanetSpecial::HostileWeather => "hostile_weather",
            PlanetSpecial::LowGravity => "low_gravity",
            PlanetSpecial::CrystalForests => "crystal_forests",
            PlanetSpecial::SubterraneanMegacaverns => "subterranean_megacaverns",
            PlanetSpecial::HyperconductiveOceans => "hyperconductive_oceans",
            PlanetSpecial::VolatileCoreInstability => "volatile_core_instability",
            PlanetSpecial::PrecursorBeacon => "precursor_beacon",
            PlanetSpecial::BioluminescentJungles => "bioluminescent_jungles",
            PlanetSpecial::AncientDefenseGrid => "ancient_defense_grid",
            PlanetSpecial::FrozenDataVault => "frozen_data_vault",
            PlanetSpecial::NaniteScarfields => "nanite_scarfields",
            PlanetSpecial::GravitationalFractureZone => "gravitational_fracture_zone",
            PlanetSpecial::OrbitalGraveyard => "orbital_graveyard",
        }
    }

    /// Short display name for this special.
    pub fn name(self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => "Mineral Rich",
            PlanetSpecial::FertileBiosphere => "Fertile Biosphere",
            PlanetSpecial::AncientRuins => "Ancient Ruins",
            PlanetSpecial::CrystalFormations => "Crystal Formations",
            PlanetSpecial::HostileWeather => "Hostile Weather",
            PlanetSpecial::LowGravity => "Low Gravity",
            PlanetSpecial::CrystalForests => "Crystal Forests",
            PlanetSpecial::SubterraneanMegacaverns => "Subterranean Megacaverns",
            PlanetSpecial::HyperconductiveOceans => "Hyperconductive Oceans",
            PlanetSpecial::VolatileCoreInstability => "Volatile Core Instability",
            PlanetSpecial::PrecursorBeacon => "Precursor Beacon",
            PlanetSpecial::BioluminescentJungles => "Bioluminescent Jungles",
            PlanetSpecial::AncientDefenseGrid => "Ancient Defense Grid",
            PlanetSpecial::FrozenDataVault => "Frozen Data Vault",
            PlanetSpecial::NaniteScarfields => "Nanite Scarfields",
            PlanetSpecial::GravitationalFractureZone => "Gravitational Fracture Zone",
            PlanetSpecial::OrbitalGraveyard => "Orbital Graveyard",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => {
                "Dense lithic seams support efficient industrial extraction."
            }
            PlanetSpecial::FertileBiosphere => {
                "Robust native ecologies enrich local harvest cycles."
            }
            PlanetSpecial::AncientRuins => {
                "Collapsed vault-cities preserve scientific fragments from a vanished age."
            }
            PlanetSpecial::CrystalFormations => {
                "Resonant crystal growths amplify commerce and laboratory output."
            }
            PlanetSpecial::HostileWeather => {
                "Unstable storms make long-term development expensive and slow."
            }
            PlanetSpecial::LowGravity => {
                "Reduced gravity eases heavy construction and orbital logistics."
            }
            PlanetSpecial::CrystalForests => {
                "Shardwood canopies refract starlight into a persistent bioelectric haze."
            }
            PlanetSpecial::SubterraneanMegacaverns => {
                "Planet-deep caverns create immense room for industry and storage."
            }
            PlanetSpecial::HyperconductiveOceans => {
                "Electroactive oceans turn storms into usable planetary energy."
            }
            PlanetSpecial::VolatileCoreInstability => {
                "The world burns hot beneath its crust, promising power and danger."
            }
            PlanetSpecial::PrecursorBeacon => {
                "A dormant beacon still sweeps the void with old coordinate pulses."
            }
            PlanetSpecial::BioluminescentJungles => {
                "Luminous jungles support rich habitats and unusual field research."
            }
            PlanetSpecial::AncientDefenseGrid => {
                "Automated gun-emplacements still ring the world in broken silence."
            }
            PlanetSpecial::FrozenDataVault => {
                "Cryo-sealed archives keep knowledge intact beneath ancient ice."
            }
            PlanetSpecial::NaniteScarfields => {
                "Dead nanite clouds still restructure the surface in slow metallic tides."
            }
            PlanetSpecial::GravitationalFractureZone => {
                "Localized gravitic shears make transport difficult but physics rich."
            }
            PlanetSpecial::OrbitalGraveyard => {
                "Decayed hulls and memorial debris encircle the world in silent layers."
            }
        }
    }

    pub fn effect_summary(self) -> &'static str {
        match self {
            PlanetSpecial::MineralRich => "+2 industry",
            PlanetSpecial::FertileBiosphere => "+2 food",
            PlanetSpecial::AncientRuins => "+2 science, one-time discovery event",
            PlanetSpecial::CrystalFormations => "+1 credits, +1 science",
            PlanetSpecial::HostileWeather => "-1 food, -1 industry",
            PlanetSpecial::LowGravity => "+2 industry",
            PlanetSpecial::CrystalForests => "+1 food, +1 science",
            PlanetSpecial::SubterraneanMegacaverns => "+2 industry, +1 credits",
            PlanetSpecial::HyperconductiveOceans => "+2 credits, +1 science",
            PlanetSpecial::VolatileCoreInstability => "+3 industry, -1 food",
            PlanetSpecial::PrecursorBeacon => "+3 science, +1 credits",
            PlanetSpecial::BioluminescentJungles => "+1 food, +2 science",
            PlanetSpecial::AncientDefenseGrid => "+1 industry, +2 science",
            PlanetSpecial::FrozenDataVault => "+3 science",
            PlanetSpecial::NaniteScarfields => "+2 industry, +1 credits",
            PlanetSpecial::GravitationalFractureZone => "+2 science, -1 food",
            PlanetSpecial::OrbitalGraveyard => "+1 credits, +1 science",
        }
    }

    pub fn rarity(self) -> DiscoveryRarity {
        match self {
            PlanetSpecial::MineralRich
            | PlanetSpecial::FertileBiosphere
            | PlanetSpecial::LowGravity => DiscoveryRarity::Common,
            PlanetSpecial::CrystalFormations
            | PlanetSpecial::HostileWeather
            | PlanetSpecial::CrystalForests
            | PlanetSpecial::SubterraneanMegacaverns
            | PlanetSpecial::BioluminescentJungles
            | PlanetSpecial::OrbitalGraveyard => DiscoveryRarity::Uncommon,
            PlanetSpecial::AncientRuins
            | PlanetSpecial::HyperconductiveOceans
            | PlanetSpecial::VolatileCoreInstability
            | PlanetSpecial::AncientDefenseGrid
            | PlanetSpecial::FrozenDataVault
            | PlanetSpecial::NaniteScarfields
            | PlanetSpecial::GravitationalFractureZone => DiscoveryRarity::Rare,
            PlanetSpecial::PrecursorBeacon => DiscoveryRarity::Legendary,
        }
    }

    pub fn category(self) -> PlanetSpecialCategory {
        match self {
            PlanetSpecial::MineralRich
            | PlanetSpecial::CrystalFormations
            | PlanetSpecial::HyperconductiveOceans => PlanetSpecialCategory::Resource,
            PlanetSpecial::AncientRuins | PlanetSpecial::FrozenDataVault => {
                PlanetSpecialCategory::Scientific
            }
            PlanetSpecial::FertileBiosphere
            | PlanetSpecial::CrystalForests
            | PlanetSpecial::BioluminescentJungles => PlanetSpecialCategory::Biological,
            PlanetSpecial::LowGravity
            | PlanetSpecial::SubterraneanMegacaverns
            | PlanetSpecial::NaniteScarfields => PlanetSpecialCategory::Industrial,
            PlanetSpecial::GravitationalFractureZone => PlanetSpecialCategory::Environmental,
            PlanetSpecial::PrecursorBeacon => PlanetSpecialCategory::Precursor,
            PlanetSpecial::HostileWeather | PlanetSpecial::VolatileCoreInstability => {
                PlanetSpecialCategory::Hazard
            }
            PlanetSpecial::AncientDefenseGrid => PlanetSpecialCategory::Strategic,
            PlanetSpecial::OrbitalGraveyard => PlanetSpecialCategory::Cultural,
        }
    }

    pub fn visibility_requirements(self) -> DiscoveryRequirements {
        DiscoveryRequirements::surveyed()
    }

    pub fn survey_requirements(self) -> DiscoveryRequirements {
        DiscoveryRequirements::surveyed_with(&[TechId::SURVEY_DRONES])
    }

    pub fn tags(self) -> &'static [&'static str] {
        match self {
            PlanetSpecial::MineralRich => &["mining", "industry"],
            PlanetSpecial::FertileBiosphere => &["food", "biosphere"],
            PlanetSpecial::AncientRuins => &["ruins", "archaeology"],
            PlanetSpecial::CrystalFormations => &["crystal", "trade"],
            PlanetSpecial::HostileWeather => &["hazard", "storm"],
            PlanetSpecial::LowGravity => &["orbital", "construction"],
            PlanetSpecial::CrystalForests => &["biological", "crystal"],
            PlanetSpecial::SubterraneanMegacaverns => &["infrastructure", "subsurface"],
            PlanetSpecial::HyperconductiveOceans => &["energy", "oceanic"],
            PlanetSpecial::VolatileCoreInstability => &["hazard", "core"],
            PlanetSpecial::PrecursorBeacon => &["precursor", "navigation"],
            PlanetSpecial::BioluminescentJungles => &["biology", "science"],
            PlanetSpecial::AncientDefenseGrid => &["defense", "precursor"],
            PlanetSpecial::FrozenDataVault => &["archive", "precursor"],
            PlanetSpecial::NaniteScarfields => &["nanites", "industry"],
            PlanetSpecial::GravitationalFractureZone => &["gravity", "physics"],
            PlanetSpecial::OrbitalGraveyard => &["history", "salvage"],
        }
    }

    pub fn future_hook(self) -> Option<&'static str> {
        match self {
            PlanetSpecial::AncientRuins => Some("archaeology_site"),
            PlanetSpecial::PrecursorBeacon => Some("precursor_signal_chain"),
            PlanetSpecial::AncientDefenseGrid => Some("reactivate_defenses"),
            PlanetSpecial::FrozenDataVault => Some("vault_decryption"),
            PlanetSpecial::VolatileCoreInstability => Some("catastrophe_containment"),
            PlanetSpecial::OrbitalGraveyard => Some("salvage_operation"),
            _ => None,
        }
    }

    /// Major discoveries emit dedicated discovery events on survey completion.
    ///
    /// Rare and Legendary specials always qualify. Precursor and Strategic specials
    /// also qualify regardless of rarity so that uniquely valuable worlds surface in
    /// reports and Dispatch.
    pub fn is_major_discovery(self) -> bool {
        self.rarity() >= DiscoveryRarity::Rare
            || matches!(
                self.category(),
                PlanetSpecialCategory::Precursor | PlanetSpecialCategory::Strategic
            )
    }

    /// Flat yield modifiers applied each turn to a colonized, surveyed planet.
    pub fn yield_effect(self) -> YieldEffect {
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
            PlanetSpecial::CrystalForests => YieldEffect {
                food: 1,
                science: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::SubterraneanMegacaverns => YieldEffect {
                industry: 2,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::HyperconductiveOceans => YieldEffect {
                credits: 2,
                science: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::VolatileCoreInstability => YieldEffect {
                industry: 3,
                food: -1,
                ..YieldEffect::default()
            },
            PlanetSpecial::PrecursorBeacon => YieldEffect {
                science: 3,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::BioluminescentJungles => YieldEffect {
                food: 1,
                science: 2,
                ..YieldEffect::default()
            },
            PlanetSpecial::AncientDefenseGrid => YieldEffect {
                industry: 1,
                science: 2,
                ..YieldEffect::default()
            },
            PlanetSpecial::FrozenDataVault => YieldEffect {
                science: 3,
                ..YieldEffect::default()
            },
            PlanetSpecial::NaniteScarfields => YieldEffect {
                industry: 2,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetSpecial::GravitationalFractureZone => YieldEffect {
                science: 2,
                food: -1,
                ..YieldEffect::default()
            },
            PlanetSpecial::OrbitalGraveyard => YieldEffect {
                credits: 1,
                science: 1,
                ..YieldEffect::default()
            },
        }
    }
}

impl PlanetAnomaly {
    pub fn all() -> &'static [PlanetAnomaly] {
        &[
            PlanetAnomaly::SilentRelayNetwork,
            PlanetAnomaly::TemporalEchoField,
            PlanetAnomaly::CollapsedJumpCorridor,
            PlanetAnomaly::PrecursorListeningPost,
            PlanetAnomaly::RogueNaniteSwarm,
            PlanetAnomaly::QuantumReflectionZone,
            PlanetAnomaly::FrozenColonyVessel,
            PlanetAnomaly::GraviticStormFront,
            PlanetAnomaly::DerelictBattleSphere,
            PlanetAnomaly::VoidSignalArray,
        ]
    }

    pub fn id(self) -> &'static str {
        match self {
            PlanetAnomaly::SilentRelayNetwork => "silent_relay_network",
            PlanetAnomaly::TemporalEchoField => "temporal_echo_field",
            PlanetAnomaly::CollapsedJumpCorridor => "collapsed_jump_corridor",
            PlanetAnomaly::PrecursorListeningPost => "precursor_listening_post",
            PlanetAnomaly::RogueNaniteSwarm => "rogue_nanite_swarm",
            PlanetAnomaly::QuantumReflectionZone => "quantum_reflection_zone",
            PlanetAnomaly::FrozenColonyVessel => "frozen_colony_vessel",
            PlanetAnomaly::GraviticStormFront => "gravitic_storm_front",
            PlanetAnomaly::DerelictBattleSphere => "derelict_battle_sphere",
            PlanetAnomaly::VoidSignalArray => "void_signal_array",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PlanetAnomaly::SilentRelayNetwork => "Silent Relay Network",
            PlanetAnomaly::TemporalEchoField => "Temporal Echo Field",
            PlanetAnomaly::CollapsedJumpCorridor => "Collapsed Jump Corridor",
            PlanetAnomaly::PrecursorListeningPost => "Precursor Listening Post",
            PlanetAnomaly::RogueNaniteSwarm => "Rogue Nanite Swarm",
            PlanetAnomaly::QuantumReflectionZone => "Quantum Reflection Zone",
            PlanetAnomaly::FrozenColonyVessel => "Frozen Colony Vessel",
            PlanetAnomaly::GraviticStormFront => "Gravitic Storm Front",
            PlanetAnomaly::DerelictBattleSphere => "Derelict Battle Sphere",
            PlanetAnomaly::VoidSignalArray => "Void Signal Array",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PlanetAnomaly::SilentRelayNetwork => {
                "Dormant relays still hand off ghost traffic across the local void."
            }
            PlanetAnomaly::TemporalEchoField => {
                "Fragments of prior motion remain measurable as delayed temporal returns."
            }
            PlanetAnomaly::CollapsedJumpCorridor => {
                "Residual lane stresses mark a once-stable transit corridor now bent shut."
            }
            PlanetAnomaly::PrecursorListeningPost => {
                "A hardened sensor keep watches the dark with forgotten protocols."
            }
            PlanetAnomaly::RogueNaniteSwarm => {
                "Autonomous repair clouds keep rebuilding whatever they can still reach."
            }
            PlanetAnomaly::QuantumReflectionZone => {
                "Matter and signal patterns rebound with impossible precision."
            }
            PlanetAnomaly::FrozenColonyVessel => {
                "An intact colony ark drifts in cold stasis above the world."
            }
            PlanetAnomaly::GraviticStormFront => {
                "Localized gravity shear rolls through orbit like weather."
            }
            PlanetAnomaly::DerelictBattleSphere => {
                "A shattered war-orbital remains armed in places and valuable in all."
            }
            PlanetAnomaly::VoidSignalArray => {
                "A deep-space lattice emits structured pulses with no active operator."
            }
        }
    }

    pub fn category(self) -> AnomalyCategory {
        match self {
            PlanetAnomaly::SilentRelayNetwork | PlanetAnomaly::VoidSignalArray => {
                AnomalyCategory::Precursor
            }
            PlanetAnomaly::TemporalEchoField => AnomalyCategory::Temporal,
            PlanetAnomaly::CollapsedJumpCorridor => AnomalyCategory::Stellar,
            PlanetAnomaly::PrecursorListeningPost | PlanetAnomaly::FrozenColonyVessel => {
                AnomalyCategory::Archaeological
            }
            PlanetAnomaly::RogueNaniteSwarm | PlanetAnomaly::DerelictBattleSphere => {
                AnomalyCategory::Military
            }
            PlanetAnomaly::QuantumReflectionZone => AnomalyCategory::ExoticPhysics,
            PlanetAnomaly::GraviticStormFront => AnomalyCategory::Gravitational,
        }
    }

    pub fn rarity(self) -> DiscoveryRarity {
        match self {
            PlanetAnomaly::CollapsedJumpCorridor | PlanetAnomaly::FrozenColonyVessel => {
                DiscoveryRarity::Uncommon
            }
            PlanetAnomaly::SilentRelayNetwork
            | PlanetAnomaly::TemporalEchoField
            | PlanetAnomaly::RogueNaniteSwarm
            | PlanetAnomaly::QuantumReflectionZone
            | PlanetAnomaly::GraviticStormFront
            | PlanetAnomaly::DerelictBattleSphere => DiscoveryRarity::Rare,
            PlanetAnomaly::PrecursorListeningPost | PlanetAnomaly::VoidSignalArray => {
                DiscoveryRarity::Legendary
            }
        }
    }

    pub fn detection_requirements(self) -> DiscoveryRequirements {
        match self {
            PlanetAnomaly::CollapsedJumpCorridor | PlanetAnomaly::FrozenColonyVessel => {
                DiscoveryRequirements::surveyed()
            }
            PlanetAnomaly::SilentRelayNetwork
            | PlanetAnomaly::TemporalEchoField
            | PlanetAnomaly::RogueNaniteSwarm
            | PlanetAnomaly::QuantumReflectionZone
            | PlanetAnomaly::GraviticStormFront => {
                DiscoveryRequirements::surveyed_with(&[TechId::ADVANCED_SURVEY])
            }
            PlanetAnomaly::DerelictBattleSphere | PlanetAnomaly::PrecursorListeningPost => {
                DiscoveryRequirements::surveyed_with(&[TechId::SECTOR_CARTOGRAPHY])
            }
            PlanetAnomaly::VoidSignalArray => {
                DiscoveryRequirements::surveyed_with(&[TechId::PAN_GALACTIC_SENSOR_NET])
            }
        }
    }

    pub fn resolution_requirements(self) -> DiscoveryRequirements {
        match self {
            PlanetAnomaly::VoidSignalArray => {
                DiscoveryRequirements::surveyed_with(&[TechId::PAN_GALACTIC_SENSOR_NET])
            }
            PlanetAnomaly::PrecursorListeningPost
            | PlanetAnomaly::DerelictBattleSphere
            | PlanetAnomaly::SilentRelayNetwork => {
                DiscoveryRequirements::surveyed_with(&[TechId::SECTOR_CARTOGRAPHY])
            }
            _ => DiscoveryRequirements::surveyed_with(&[TechId::ADVANCED_SURVEY]),
        }
    }

    pub fn risk_level(self) -> Option<AnomalyRiskLevel> {
        match self {
            PlanetAnomaly::FrozenColonyVessel | PlanetAnomaly::CollapsedJumpCorridor => {
                Some(AnomalyRiskLevel::Low)
            }
            PlanetAnomaly::SilentRelayNetwork
            | PlanetAnomaly::TemporalEchoField
            | PlanetAnomaly::QuantumReflectionZone => Some(AnomalyRiskLevel::Moderate),
            PlanetAnomaly::GraviticStormFront | PlanetAnomaly::DerelictBattleSphere => {
                Some(AnomalyRiskLevel::High)
            }
            PlanetAnomaly::RogueNaniteSwarm | PlanetAnomaly::VoidSignalArray => {
                Some(AnomalyRiskLevel::Severe)
            }
            PlanetAnomaly::PrecursorListeningPost => Some(AnomalyRiskLevel::High),
        }
    }

    pub fn tags(self) -> &'static [&'static str] {
        match self {
            PlanetAnomaly::SilentRelayNetwork => &["signal", "precursor"],
            PlanetAnomaly::TemporalEchoField => &["temporal", "science"],
            PlanetAnomaly::CollapsedJumpCorridor => &["lane", "frontier"],
            PlanetAnomaly::PrecursorListeningPost => &["precursor", "sensor"],
            PlanetAnomaly::RogueNaniteSwarm => &["nanites", "hazard"],
            PlanetAnomaly::QuantumReflectionZone => &["quantum", "research"],
            PlanetAnomaly::FrozenColonyVessel => &["salvage", "ark"],
            PlanetAnomaly::GraviticStormFront => &["gravity", "hazard"],
            PlanetAnomaly::DerelictBattleSphere => &["military", "salvage"],
            PlanetAnomaly::VoidSignalArray => &["precursor", "signal", "mystery"],
        }
    }

    pub fn future_hook(self) -> Option<&'static str> {
        match self {
            PlanetAnomaly::SilentRelayNetwork => Some("reactivate_relay_network"),
            PlanetAnomaly::PrecursorListeningPost => Some("precursor_archive_chain"),
            PlanetAnomaly::RogueNaniteSwarm => Some("contain_nanites"),
            PlanetAnomaly::FrozenColonyVessel => Some("stasis_revival"),
            PlanetAnomaly::VoidSignalArray => Some("void_signal_chain"),
            _ => None,
        }
    }

    pub fn yield_effect(self) -> YieldEffect {
        match self {
            PlanetAnomaly::SilentRelayNetwork => YieldEffect {
                science: 2,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::TemporalEchoField => YieldEffect {
                science: 2,
                ..YieldEffect::default()
            },
            PlanetAnomaly::CollapsedJumpCorridor => YieldEffect {
                credits: 1,
                science: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::PrecursorListeningPost => YieldEffect {
                science: 3,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::RogueNaniteSwarm => YieldEffect {
                industry: 2,
                food: -1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::QuantumReflectionZone => YieldEffect {
                science: 2,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::FrozenColonyVessel => YieldEffect {
                food: 1,
                credits: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::GraviticStormFront => YieldEffect {
                industry: 2,
                credits: -1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::DerelictBattleSphere => YieldEffect {
                industry: 2,
                science: 1,
                ..YieldEffect::default()
            },
            PlanetAnomaly::VoidSignalArray => YieldEffect {
                science: 3,
                credits: 1,
                ..YieldEffect::default()
            },
        }
    }

    pub fn formatted_risk(self) -> &'static str {
        match self.risk_level() {
            Some(AnomalyRiskLevel::Low) => "Low risk",
            Some(AnomalyRiskLevel::Moderate) => "Moderate risk",
            Some(AnomalyRiskLevel::High) => "High risk",
            Some(AnomalyRiskLevel::Severe) => "Severe risk",
            None => "No recorded risk",
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
                description: "Phase-stable crystal lattices that amplify defensive field harmonics.",
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
    planet
        .resources
        .iter()
        .copied()
        .filter(|resource| {
            let req = resource.record().discovery_requirements;
            (!req.surveyed || planet.surveyed)
                && is_resource_discoverable(*resource, completed_techs)
        })
        .collect()
}

/// Returns planet specials visible to an empire on this planet.
pub fn visible_specials_for_empire(
    planet: &Planet,
    completed_techs: &[TechId],
) -> Vec<PlanetSpecial> {
    planet
        .specials
        .iter()
        .copied()
        .filter(|special| {
            requirements_met(
                special.visibility_requirements(),
                planet.surveyed,
                completed_techs,
            )
        })
        .collect()
}

/// Returns anomalies visible to an empire on this planet.
pub fn visible_anomalies_for_empire(
    planet: &Planet,
    completed_techs: &[TechId],
) -> Vec<PlanetAnomaly> {
    planet
        .anomalies
        .iter()
        .copied()
        .filter(|anomaly| {
            requirements_met(
                anomaly.detection_requirements(),
                planet.surveyed,
                completed_techs,
            )
        })
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
    /// Persistent anomalies anchored to the world or its orbital envelope.
    #[cfg_attr(feature = "serde", serde(default))]
    pub anomalies: Vec<PlanetAnomaly>,
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
    for anomaly in &planet.anomalies {
        total = total.combine(anomaly.yield_effect());
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
    /// Logistics anchor extending fleet supply reach from this colony
    SupplyHub,
}

impl OrbitalStructureType {
    /// All orbital structure types available for construction
    pub fn all() -> &'static [OrbitalStructureType] {
        &[
            OrbitalStructureType::Shipyard,
            OrbitalStructureType::SupplyHub,
        ]
    }

    /// Display name for this orbital structure
    pub fn name(&self) -> &'static str {
        match self {
            OrbitalStructureType::Shipyard => "Shipyard",
            OrbitalStructureType::SupplyHub => "Supply Hub",
        }
    }

    /// Short description of what this orbital structure does
    pub fn description(&self) -> &'static str {
        match self {
            OrbitalStructureType::Shipyard => {
                "Orbital drydock — required to construct ships at this colony"
            }
            OrbitalStructureType::SupplyHub => {
                "Logistics anchor — extends supply projection for nearby fleets"
            }
        }
    }

    /// Production cost to construct this orbital structure
    pub fn cost(&self) -> u64 {
        match self {
            OrbitalStructureType::Shipyard => 200,
            OrbitalStructureType::SupplyHub => 160,
        }
    }

    /// Credit maintenance cost per turn for this orbital structure
    pub fn maintenance_cost(&self) -> i64 {
        match self {
            OrbitalStructureType::Shipyard => 2,
            OrbitalStructureType::SupplyHub => 1,
        }
    }

    /// Technology required to construct this orbital structure, if any
    pub fn required_tech(&self) -> Option<TechId> {
        match self {
            OrbitalStructureType::Shipyard => Some(TechId::ORBITAL_ENGINEERING),
            OrbitalStructureType::SupplyHub => Some(TechId::RAPID_TRANSIT),
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

/// Strategic focus assigned to a sector, biasing colony build automation.
///
/// Directives are **advisory**: they bias which build item an automated colony
/// queues when its queue is empty, but they never modify yields directly and
/// never override an explicit player queue.  Defaults to `Balanced`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SectorDirective {
    /// No bias — general-purpose infrastructure order.
    #[default]
    Balanced,
    /// Prefer industrial production buildings.
    Industrial,
    /// Prefer research buildings.
    Research,
    /// Prefer food production buildings.
    Agricultural,
    /// Prefer orbital shipyards, then production.
    Military,
    /// Prefer production, then logistics infrastructure.
    Economic,
    /// Prioritise food/housing relief to address unrest.
    Stabilization,
}

impl SectorDirective {
    /// All directives in display order.
    pub fn all() -> &'static [SectorDirective] {
        &[
            SectorDirective::Balanced,
            SectorDirective::Industrial,
            SectorDirective::Research,
            SectorDirective::Agricultural,
            SectorDirective::Military,
            SectorDirective::Economic,
            SectorDirective::Stabilization,
        ]
    }

    /// Short display name for this directive.
    pub fn name(&self) -> &'static str {
        match self {
            SectorDirective::Balanced => "Balanced",
            SectorDirective::Industrial => "Industrial",
            SectorDirective::Research => "Research",
            SectorDirective::Agricultural => "Agricultural",
            SectorDirective::Military => "Military",
            SectorDirective::Economic => "Economic",
            SectorDirective::Stabilization => "Stabilization",
        }
    }

    /// One-line description of the automation bias this directive applies.
    pub fn description(&self) -> &'static str {
        match self {
            SectorDirective::Balanced => "No bias; general-purpose infrastructure",
            SectorDirective::Industrial => "Prefers production buildings",
            SectorDirective::Research => "Prefers research buildings",
            SectorDirective::Agricultural => "Prefers food buildings",
            SectorDirective::Military => "Prefers orbital shipyards",
            SectorDirective::Economic => "Prefers production, then logistics",
            SectorDirective::Stabilization => "Addresses food, housing and unrest",
        }
    }

    /// Return the next directive in cycle order (wraps to the first).
    pub fn next(&self) -> SectorDirective {
        let all = SectorDirective::all();
        let idx = all.iter().position(|d| d == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

/// Whether a colony's production queue is player-controlled or sector-guided.
///
/// Defaults to `Manual`.  `SectorGuided` colonies have their empty build queue
/// filled deterministically by the engine, biased by their sector directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColonyAutomation {
    /// The player controls the build queue; the engine never modifies it.
    #[default]
    Manual,
    /// The engine queues builds when the queue is empty, biased by directive.
    SectorGuided,
}

impl ColonyAutomation {
    /// Short display name for this automation mode.
    pub fn name(&self) -> &'static str {
        match self {
            ColonyAutomation::Manual => "Manual",
            ColonyAutomation::SectorGuided => "Sector-guided",
        }
    }

    /// Return the toggled automation mode.
    pub fn toggled(&self) -> ColonyAutomation {
        match self {
            ColonyAutomation::Manual => ColonyAutomation::SectorGuided,
            ColonyAutomation::SectorGuided => ColonyAutomation::Manual,
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

    /// Returns `true` if this colony has a Supply Hub in its orbital installations
    pub fn has_supply_hub(&self) -> bool {
        self.orbital_installations
            .contains(&OrbitalStructureType::SupplyHub)
    }

    /// Returns true when colony stability is low enough to be considered unrest.
    pub fn is_unrest(&self) -> bool {
        self.stability < 60
    }

    /// Human-readable stability state.
    pub fn unrest_label(&self) -> &'static str {
        if self.is_unrest() { "Unrest" } else { "Stable" }
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

/// Reason a trade route was disrupted this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TradeDisruptionReason {
    Blockade,
    WarZone,
    HostileFleetPresence,
    OutOfSupply,
}

impl TradeDisruptionReason {
    pub fn label(self) -> &'static str {
        match self {
            TradeDisruptionReason::Blockade => "Blockade",
            TradeDisruptionReason::WarZone => "War zone",
            TradeDisruptionReason::HostileFleetPresence => "Hostile fleet",
            TradeDisruptionReason::OutOfSupply => "Out of supply",
        }
    }
}

/// A deterministic derived trade route between two colony systems.
///
/// Routes are computed each turn from the state graph and require no
/// manual player management. Stored sorted by `(from, to)`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TradeRoute {
    pub from: StarId,
    pub to: StarId,
    /// Gross trade value in credits per turn before disruption.
    pub base_value: i64,
    /// Net trade value after all disruption multipliers are applied.
    pub net_value: i64,
    /// True when any disruption factor reduced the route value.
    pub disrupted: bool,
    /// The first applicable disruption reason, if any.
    pub disruption_reason: Option<TradeDisruptionReason>,
}

/// Coarse internal order state for a colony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ColonyUnrestState {
    #[default]
    Calm,
    Strained,
    Unrest,
    RevoltRisk,
}

impl ColonyUnrestState {
    pub fn from_stability(stability: u8) -> Self {
        if stability >= 85 {
            ColonyUnrestState::Calm
        } else if stability >= 70 {
            ColonyUnrestState::Strained
        } else if stability >= 50 {
            ColonyUnrestState::Unrest
        } else {
            ColonyUnrestState::RevoltRisk
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ColonyUnrestState::Calm => "Calm",
            ColonyUnrestState::Strained => "Strained",
            ColonyUnrestState::Unrest => "Unrest",
            ColonyUnrestState::RevoltRisk => "Revolt Risk",
        }
    }

    pub fn is_unrest(self) -> bool {
        matches!(
            self,
            ColonyUnrestState::Unrest | ColonyUnrestState::RevoltRisk
        )
    }

    pub fn production_percent(self) -> i64 {
        match self {
            ColonyUnrestState::Calm => 100,
            ColonyUnrestState::Strained => 90,
            ColonyUnrestState::Unrest => 75,
            ColonyUnrestState::RevoltRisk => 60,
        }
    }

    pub fn economy_percent(self) -> i64 {
        match self {
            ColonyUnrestState::Calm => 100,
            ColonyUnrestState::Strained => 92,
            ColonyUnrestState::Unrest => 78,
            ColonyUnrestState::RevoltRisk => 62,
        }
    }

    pub fn maintenance_percent(self) -> i64 {
        match self {
            ColonyUnrestState::Calm => 100,
            ColonyUnrestState::Strained => 110,
            ColonyUnrestState::Unrest => 125,
            ColonyUnrestState::RevoltRisk => 145,
        }
    }

    pub fn base_rebellion_risk_bp(self) -> u16 {
        match self {
            ColonyUnrestState::Calm => 0,
            ColonyUnrestState::Strained => 50,
            ColonyUnrestState::Unrest => 250,
            ColonyUnrestState::RevoltRisk => 800,
        }
    }
}

/// Deterministic unrest contributors for explainability and UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum UnrestCause {
    FoodShortage,
    HousingShortage,
    LowStability,
    RecentConquest,
    Blockade,
    Isolated,
    WarExhaustion,
    Overextension,
}

impl UnrestCause {
    pub fn label(self) -> &'static str {
        match self {
            UnrestCause::FoodShortage => "Food shortage",
            UnrestCause::HousingShortage => "Housing shortage",
            UnrestCause::LowStability => "Low stability",
            UnrestCause::RecentConquest => "Recent conquest",
            UnrestCause::Blockade => "Blockade",
            UnrestCause::Isolated => "Out of supply / isolated",
            UnrestCause::WarExhaustion => "War exhaustion",
            UnrestCause::Overextension => "Overextension",
        }
    }
}

/// Current logistics status for a fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FleetSupplyState {
    #[default]
    Supplied,
    Extended,
    OutOfSupply,
}

impl FleetSupplyState {
    pub fn label(self) -> &'static str {
        match self {
            FleetSupplyState::Supplied => "Supplied",
            FleetSupplyState::Extended => "Extended",
            FleetSupplyState::OutOfSupply => "Out of Supply",
        }
    }

    pub fn movement_penalty_pct(self) -> u32 {
        match self {
            FleetSupplyState::Supplied => 100,
            FleetSupplyState::Extended => 125,
            FleetSupplyState::OutOfSupply => 160,
        }
    }

    pub fn combat_attack_pct(self) -> u32 {
        match self {
            FleetSupplyState::Supplied => 100,
            FleetSupplyState::Extended => 90,
            FleetSupplyState::OutOfSupply => 75,
        }
    }

    pub fn combat_defense_pct(self) -> u32 {
        match self {
            FleetSupplyState::Supplied => 100,
            FleetSupplyState::Extended => 95,
            FleetSupplyState::OutOfSupply => 80,
        }
    }

    pub fn invasion_strength_pct(self) -> u32 {
        match self {
            FleetSupplyState::Supplied => 100,
            FleetSupplyState::Extended => 75,
            FleetSupplyState::OutOfSupply => 0,
        }
    }

    pub fn penalty_summary(self) -> &'static str {
        match self {
            FleetSupplyState::Supplied => "No logistics penalty",
            FleetSupplyState::Extended => "-10% attack, -5% defense, +25% travel, -25% invasion",
            FleetSupplyState::OutOfSupply => {
                "-25% attack, -20% defense, +60% travel, cannot invade"
            }
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub supply_a: FleetSupplyState,
    #[cfg_attr(feature = "serde", serde(default))]
    pub supply_b: FleetSupplyState,
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

#[cfg(feature = "serde")]
fn default_next_battle_session_id() -> u64 {
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

/// Discrete intelligence level the player has on another empire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntelLevel {
    #[default]
    Unknown,
    Contacted,
    Basic,
    Informed,
    Deep,
}

impl IntelLevel {
    pub fn label(self) -> &'static str {
        match self {
            IntelLevel::Unknown => "Unknown",
            IntelLevel::Contacted => "Contacted",
            IntelLevel::Basic => "Basic",
            IntelLevel::Informed => "Informed",
            IntelLevel::Deep => "Deep",
        }
    }

    pub fn reveals_colony_count(self) -> bool {
        self >= IntelLevel::Basic
    }

    pub fn reveals_fleet_strength(self) -> bool {
        self >= IntelLevel::Basic
    }

    pub fn reveals_tech_level(self) -> bool {
        self >= IntelLevel::Informed
    }

    pub fn reveals_economy_summary(self) -> bool {
        self >= IntelLevel::Informed
    }

    pub fn reveals_diplomatic_stance(self) -> bool {
        self >= IntelLevel::Contacted
    }

    pub fn reveals_strategic_resources(self) -> bool {
        self >= IntelLevel::Deep
    }

    /// Whether espionage on an empire reveals its relations with third
    /// parties — including empires the player has never met.
    pub fn reveals_foreign_relations(self) -> bool {
        self >= IntelLevel::Informed
    }
}

/// Deterministic source that improved empire intelligence this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum IntelSource {
    Contact,
    NearbyFleets,
    SensorTech,
    Treaty,
    GatherIntelligence,
}

impl IntelSource {
    pub fn label(self) -> &'static str {
        match self {
            IntelSource::Contact => "contact",
            IntelSource::NearbyFleets => "nearby fleets",
            IntelSource::SensorTech => "sensor tech",
            IntelSource::Treaty => "treaty",
            IntelSource::GatherIntelligence => "gather intel",
        }
    }
}

/// Lightweight espionage actions surfaced in diplomacy UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EspionageMission {
    GatherIntelligence,
    SabotageProduction,
    StealResearch,
}

impl EspionageMission {
    pub fn label(self) -> &'static str {
        match self {
            EspionageMission::GatherIntelligence => "Gather Intelligence",
            EspionageMission::SabotageProduction => "Sabotage Production",
            EspionageMission::StealResearch => "Steal Research",
        }
    }
}

/// Persisted intelligence state for one foreign empire.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmpireIntel {
    #[cfg_attr(feature = "serde", serde(default))]
    pub level: IntelLevel,
    #[cfg_attr(feature = "serde", serde(default))]
    pub points: u16,
    #[cfg_attr(feature = "serde", serde(default))]
    pub last_gather_turn: Option<u32>,
}

impl EmpireIntel {
    pub fn new_contacted() -> Self {
        Self {
            level: IntelLevel::Contacted,
            points: 0,
            last_gather_turn: None,
        }
    }
}

/// Aggregated deterministic economy snapshot for empire-intelligence UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmpireEconomySummary {
    pub food_balance: i64,
    pub industry: i64,
    pub science: i64,
    pub credits: i64,
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
///
/// `Custom` has no fixed count; the caller provides `star_count_override`
/// and/or `sector_count_override` in `ScenarioSetup`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GalaxySize {
    /// Compact galaxy — 40 stars, 3 sectors
    Tiny,
    /// Small galaxy — 80 stars, 4 sectors
    Small,
    /// Standard galaxy — 150 stars, 6 sectors
    #[default]
    Medium,
    /// Large galaxy — 250 stars, 8 sectors
    Large,
    /// Huge galaxy — 400 stars, 12 sectors
    Huge,
    /// Epic galaxy — 700 stars, 16 sectors
    Epic,
    /// Custom size — use `star_count_override` and `sector_count_override`.
    Custom,
}

impl GalaxySize {
    /// All available galaxy sizes in display order.
    pub fn all() -> &'static [GalaxySize] {
        &[
            GalaxySize::Tiny,
            GalaxySize::Small,
            GalaxySize::Medium,
            GalaxySize::Large,
            GalaxySize::Huge,
            GalaxySize::Epic,
            GalaxySize::Custom,
        ]
    }

    /// Short display label.
    pub fn label(&self) -> &'static str {
        match self {
            GalaxySize::Tiny => "Tiny",
            GalaxySize::Small => "Small",
            GalaxySize::Medium => "Medium",
            GalaxySize::Large => "Large",
            GalaxySize::Huge => "Huge",
            GalaxySize::Epic => "Epic",
            GalaxySize::Custom => "Custom",
        }
    }

    /// Default number of star systems for this size.
    /// Returns 0 for `Custom` — callers must use `effective_star_count`
    /// on `ScenarioSetup` instead.
    pub fn default_star_count(&self) -> usize {
        match self {
            GalaxySize::Tiny => 40,
            GalaxySize::Small => 80,
            GalaxySize::Medium => 150,
            GalaxySize::Large => 250,
            GalaxySize::Huge => 400,
            GalaxySize::Epic => 700,
            GalaxySize::Custom => 0,
        }
    }

    /// Default number of sectors for this size.
    /// Returns 0 for `Custom` — callers must use `effective_sector_count`
    /// on `ScenarioSetup` instead.
    pub fn default_sector_count(&self) -> usize {
        match self {
            GalaxySize::Tiny => 3,
            GalaxySize::Small => 4,
            GalaxySize::Medium => 6,
            GalaxySize::Large => 8,
            GalaxySize::Huge => 12,
            GalaxySize::Epic => 16,
            GalaxySize::Custom => 0,
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
    /// Override for the number of star systems.  When `None` the count
    /// is derived from `galaxy_size`.  Required when `galaxy_size` is
    /// `Custom`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub star_count_override: Option<usize>,
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
    /// Uses `Small` (80 stars, 4 sectors) as the default so that tests
    /// and quick-start remain responsive while still providing a
    /// meaningful map.  Real campaigns use `Medium` (150 stars) or
    /// larger through the setup screen.
    pub fn default_for_seed(seed: u64) -> Self {
        ScenarioSetup {
            seed,
            galaxy_size: GalaxySize::Small,
            ai_empire_count: 1,
            sector_count_override: None,
            star_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: None,
            victory_settings: VictorySettings::default_v1(),
        }
    }

    /// Effective number of star systems for this setup.
    pub fn effective_star_count(&self) -> usize {
        self.star_count_override
            .unwrap_or_else(|| self.galaxy_size.default_star_count())
    }

    /// Effective number of sectors for this setup.
    pub fn effective_sector_count(&self) -> usize {
        match self.sector_count_override {
            Some(n) => n,
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
        if let Some(n) = self.sector_count_override
            && !(2..=16).contains(&n)
        {
            return Err(format!("Sector count must be 2–16, got {}", n));
        }
        if self.galaxy_size == GalaxySize::Custom && self.star_count_override.is_none() {
            return Err("Custom galaxy size requires star_count_override to be set".to_string());
        }
        if let Some(n) = self.star_count_override
            && !(10..=2000).contains(&n)
        {
            return Err(format!("Star count must be 10–2000, got {}", n));
        }
        if let Some(def_id) = self.player_empire_def
            && empire_definition_by_id(def_id).is_none()
        {
            return Err(format!("Unknown player empire definition id {}", def_id.0));
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
                VictoryCondition::Ascendancy {
                    control_percent, ..
                } if *control_percent == 0 || *control_percent > 100 => {
                    return Err(format!(
                        "Ascendancy control threshold must be 1–100, got {}",
                        control_percent
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

/// Difficulty level affecting AI decision quality and (on Brutal) modest
/// economic bonuses.  Difficulty primarily shapes planning quality,
/// aggression threshold, and expansion confidence rather than raw
/// resource yields; only the highest tier grants a small credit bonus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DifficultyLevel {
    /// Relaxed AI — slower expansion, lower aggression, no bonuses.
    Easy,
    /// Balanced AI — the default, intended for a fair challenge.
    #[default]
    Standard,
    /// Sharper AI — more aggressive, faster decision pacing, small
    /// research and expansion bonuses.
    Hard,
    /// Ruthless AI — the most aggressive with a modest +15% credit
    /// income bonus to keep pressure on experienced players.
    Brutal,
}

impl DifficultyLevel {
    pub fn all() -> &'static [DifficultyLevel] {
        &[
            DifficultyLevel::Easy,
            DifficultyLevel::Standard,
            DifficultyLevel::Hard,
            DifficultyLevel::Brutal,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            DifficultyLevel::Easy => "Easy",
            DifficultyLevel::Standard => "Standard",
            DifficultyLevel::Hard => "Hard",
            DifficultyLevel::Brutal => "Brutal",
        }
    }

    /// Bonus multiplier applied to AI research scores (100 = baseline).
    pub fn research_bonus_pct(self) -> i32 {
        match self {
            DifficultyLevel::Easy => 90,
            DifficultyLevel::Standard => 100,
            DifficultyLevel::Hard => 110,
            DifficultyLevel::Brutal => 120,
        }
    }

    /// Aggression threshold offset.  Lower values = more aggressive.
    /// The engine reduces the threshold for going to war / demanding
    /// tribute when this value is smaller.
    pub fn aggression_offset(self) -> i32 {
        match self {
            DifficultyLevel::Easy => 15,
            DifficultyLevel::Standard => 0,
            DifficultyLevel::Hard => -5,
            DifficultyLevel::Brutal => -10,
        }
    }

    /// Extra starting credits for the AI on Brutal.
    pub fn ai_credit_bonus(self) -> i64 {
        match self {
            DifficultyLevel::Brutal => 50,
            _ => 0,
        }
    }
}

/// Seeded ChaCha8 RNG wrapper that implements Clone via serialization.
///
/// ChaCha8Rng in rand 0.10 does not implement Clone. This wrapper uses
/// `serialize_state`/`deserialize_state` to clone the RNG state, which
/// is a zero-cost read on the source and produces an identical RNG.
#[derive(Debug)]
pub struct SeededRng(ChaCha8Rng);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        use rand::SeedableRng;
        Self(ChaCha8Rng::seed_from_u64(seed))
    }

    pub fn inner(&self) -> &ChaCha8Rng {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut ChaCha8Rng {
        &mut self.0
    }
}

impl Clone for SeededRng {
    fn clone(&self) -> Self {
        Self(ChaCha8Rng::deserialize_state(&self.0.serialize_state()))
    }
}

impl PartialEq for SeededRng {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Default for SeededRng {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Deref for SeededRng {
    type Target = ChaCha8Rng;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SeededRng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
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
    pub rng: SeededRng,
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
    /// Legacy shared AI exploration set. Superseded by
    /// [`GameState::empire_explored_stars`]; retained so pre-v40 saves
    /// deserialize, and migrated into the per-empire map on load.
    #[cfg_attr(feature = "serde", serde(default))]
    pub ai_explored_stars: BTreeSet<StarId>,
    /// Stars explored by each AI empire (the player's set is
    /// [`GameState::explored_stars`]). Each AI has its own fog of war.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_explored_stars: BTreeMap<EmpireId, BTreeSet<StarId>>,
    /// Relationship status between pairs of AI empires, stored under the
    /// lower empire id mapping to the higher id. Pairs not present are
    /// implicitly `Unknown`. Player relationships live in
    /// [`GameState::diplomacy`]/[`GameState::diplomacy_relationships`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub ai_relations: BTreeMap<EmpireId, BTreeMap<EmpireId, RelationshipStatus>>,
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
    /// Last computed fleet supply states, keyed by fleet ID.
    ///
    /// This map is deterministic and derived from fleet location, empire
    /// technology, active colonies, orbital infrastructure, and blockade state.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fleet_supply: BTreeMap<FleetId, FleetSupplyState>,
    /// Active blockades this turn: maps blockaded `ColonyId` to the `EmpireId` of the
    /// primary blockading empire.
    ///
    /// Derived each turn from idle hostile/war-status fleet positions.
    /// Persisted to detect start/end transitions for event emission on the next turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_blockade: BTreeMap<ColonyId, EmpireId>,
    /// Deterministic colony unrest state cache for UI/reporting and penalties.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_unrest: BTreeMap<ColonyId, ColonyUnrestState>,
    /// Deterministic unrest causes captured for each colony this turn.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_unrest_causes: BTreeMap<ColonyId, Vec<UnrestCause>>,
    /// Deterministic rebellion-risk basis points per colony (future rebellion hook).
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_rebellion_risk_bp: BTreeMap<ColonyId, u16>,
    /// Turn number when colony was last conquered; used for recent-conquest unrest decay.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_recent_conquest_turn: BTreeMap<ColonyId, u32>,
    /// Current strategic-resource access counts per empire.
    ///
    /// Counts are deterministic and derived from colony control, survey/discovery,
    /// extraction requirements, supply connectivity, and blockade status.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_resource_access: BTreeMap<EmpireId, BTreeMap<StrategicResource, u32>>,
    /// Deterministic victory-condition progress and winner state.
    #[cfg_attr(feature = "serde", serde(default))]
    pub victory_status: VictoryStatus,
    /// Per-star constellation cluster assignment.  Set during galaxy
    /// generation; used for AI strategic positioning and terrain-aware
    /// behaviour.  Empty for pre-v migration saves.
    #[cfg_attr(feature = "serde", serde(default))]
    pub star_constellations: BTreeMap<StarId, u32>,
    /// Per-star nebula membership.  Stars inside nebula bands get
    /// modified planetary discovery weights.  Empty for pre-v saves.
    #[cfg_attr(feature = "serde", serde(default))]
    pub star_nebulae: BTreeMap<StarId, u32>,
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
    /// Persisted intelligence state per known foreign empire.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_intel: BTreeMap<EmpireId, EmpireIntel>,
    /// Strategic directive assigned to each sector.  Absent entries are `Balanced`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sector_directives: BTreeMap<SectorId, SectorDirective>,
    /// Build-queue automation mode per colony.  Absent entries are `Manual`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub colony_automation: BTreeMap<ColonyId, ColonyAutomation>,
    /// Actual per-colony yield produced on the most recent processed turn, after
    /// all engine modifiers (tech/trait/resource bonuses, research percentage,
    /// unrest, isolation/blockade penalties). Empty until the first turn is
    /// processed. Read by the UI so reported output matches real income.
    #[cfg_attr(feature = "serde", serde(default))]
    pub last_colony_yields: BTreeMap<ColonyId, ColonyYieldSnapshot>,
    /// Derived trade routes per empire for the current turn.
    ///
    /// Re-computed each turn from colony connectivity, population, buildings,
    /// tech, strategic resources, diplomacy state, and disruption factors.
    /// Persisted for UI rendering and transition detection.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_trade_routes: BTreeMap<EmpireId, Vec<TradeRoute>>,
    /// Total trade credits per empire for the current turn.
    ///
    /// Derived from `empire_trade_routes` — sum of `route.net_value` per empire.
    #[cfg_attr(feature = "serde", serde(default))]
    pub empire_trade_income: BTreeMap<EmpireId, i64>,
    /// Monotonic identifier for Combat v3 battle sessions.
    ///
    /// Defaults to `1` so v40→v41 migrations can stay empty-handed.
    #[cfg_attr(feature = "serde", serde(default = "default_next_battle_session_id"))]
    pub next_battle_session_id: u64,
    /// Active Combat v3 battle awaiting player input.  `Some` while a
    /// player-involved engagement is paused for card plays.  `None` when
    /// no battle is pending (most turns).
    #[cfg_attr(feature = "serde", serde(default))]
    pub pending_battle_session: Option<crate::combat_v3::BattleSession>,
    /// Recent Combat v3 battle reports.  Bounded to the same history
    /// limit as `battle_reports` (legacy v2 reports).  Oldest at front.
    #[cfg_attr(feature = "serde", serde(default))]
    pub battle_reports_v3: VecDeque<crate::combat_v3::BattleReportV3>,
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
    const FLEET_SUPPLY_RANGE_SAME_SECTOR: i64 = 500;
    const FLEET_SUPPLY_RANGE_CROSS_SECTOR: i64 = 280;
    const FLEET_EXTENDED_RANGE_SAME_SECTOR: i64 = 800;
    const FLEET_EXTENDED_RANGE_CROSS_SECTOR: i64 = 450;
    const FLEET_SUPPLY_TECH_BONUS: i64 = 100;
    const FLEET_SUPPLY_SHIPYARD_BONUS: i64 = 120;
    const FLEET_SUPPLY_HUB_BONUS: i64 = 220;
    const FLEET_SUPPLY_LANE_BONUS: i64 = 200;
    const FLEET_EXTENDED_LANE_BONUS: i64 = 260;

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

    pub fn colony_unrest_state(&self, colony_id: ColonyId) -> ColonyUnrestState {
        self.colony_unrest
            .get(&colony_id)
            .copied()
            .unwrap_or_else(|| {
                self.colonies
                    .get(&colony_id)
                    .map(|colony| ColonyUnrestState::from_stability(colony.stability))
                    .unwrap_or_default()
            })
    }

    pub fn colony_unrest_label(&self, colony_id: ColonyId) -> &'static str {
        self.colony_unrest_state(colony_id).label()
    }

    pub fn colony_unrest_causes(&self, colony_id: ColonyId) -> &[UnrestCause] {
        self.colony_unrest_causes
            .get(&colony_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn colony_rebellion_risk_bp(&self, colony_id: ColonyId) -> u16 {
        self.colony_rebellion_risk_bp
            .get(&colony_id)
            .copied()
            .unwrap_or_else(|| self.colony_unrest_state(colony_id).base_rebellion_risk_bp())
    }

    /// The sector a colony belongs to, derived from the star it orbits.
    pub fn colony_sector(&self, colony_id: ColonyId) -> Option<SectorId> {
        let star_id = self.colonies.get(&colony_id)?.star;
        self.stars.get(&star_id).map(|star| star.sector)
    }

    /// All colony IDs whose star belongs to `sector_id`, in sorted order.
    pub fn colonies_in_sector(&self, sector_id: SectorId) -> Vec<ColonyId> {
        crate::deterministic::sorted_colony_ids(&self.colonies)
            .into_iter()
            .filter(|&id| self.colony_sector(id) == Some(sector_id))
            .collect()
    }

    /// The directive assigned to a sector (defaults to `Balanced`).
    pub fn sector_directive(&self, sector_id: SectorId) -> SectorDirective {
        self.sector_directives
            .get(&sector_id)
            .copied()
            .unwrap_or_default()
    }

    /// The automation mode for a colony (defaults to `Manual`).
    pub fn colony_automation_mode(&self, colony_id: ColonyId) -> ColonyAutomation {
        self.colony_automation
            .get(&colony_id)
            .copied()
            .unwrap_or_default()
    }

    pub fn fleet_supply_state(&self, fleet_id: FleetId) -> FleetSupplyState {
        self.fleet_supply
            .get(&fleet_id)
            .copied()
            .unwrap_or_else(|| self.derived_fleet_supply_state(fleet_id))
    }

    pub fn derived_fleet_supply_state(&self, fleet_id: FleetId) -> FleetSupplyState {
        let Some(location) = self.fleet_location(fleet_id) else {
            return FleetSupplyState::OutOfSupply;
        };
        let Some(empire_id) = self.fleets.get(&fleet_id).map(|fleet| fleet.owner) else {
            return FleetSupplyState::OutOfSupply;
        };
        match location {
            FleetLocation::AtStar(star_id) => self.projected_fleet_supply(empire_id, star_id),
            FleetLocation::Travelling { destination, .. } => {
                self.projected_fleet_supply(empire_id, destination)
            }
        }
    }

    pub fn projected_fleet_supply(&self, empire_id: EmpireId, star_id: StarId) -> FleetSupplyState {
        if !self
            .colonies
            .values()
            .any(|colony| colony.owner == empire_id)
        {
            return FleetSupplyState::OutOfSupply;
        }
        let mut best = FleetSupplyState::OutOfSupply;
        for colony in self
            .colonies
            .values()
            .filter(|colony| colony.owner == empire_id)
        {
            if self.colony_blockade_state(colony.id).is_some() {
                continue;
            }

            if colony.star == star_id {
                return FleetSupplyState::Supplied;
            }

            let connected = self.colony_supply_state(colony.id) == ColonySupplyState::Connected;
            let has_shipyard = colony.has_shipyard();
            let has_supply_hub = colony.has_supply_hub();
            if !connected && !has_shipyard && !has_supply_hub {
                continue;
            }

            let Some(from) = self.stars.get(&colony.star) else {
                continue;
            };
            let Some(to) = self.stars.get(&star_id) else {
                continue;
            };

            let mut full_range = if from.sector == to.sector {
                Self::FLEET_SUPPLY_RANGE_SAME_SECTOR
            } else {
                Self::FLEET_SUPPLY_RANGE_CROSS_SECTOR
            };
            let mut extended_range = if from.sector == to.sector {
                Self::FLEET_EXTENDED_RANGE_SAME_SECTOR
            } else {
                Self::FLEET_EXTENDED_RANGE_CROSS_SECTOR
            };

            if self.empire_has_hyperspace_trade(empire_id) {
                full_range += Self::FLEET_SUPPLY_TECH_BONUS;
                extended_range += Self::FLEET_SUPPLY_TECH_BONUS;
            }
            if has_shipyard {
                full_range += Self::FLEET_SUPPLY_SHIPYARD_BONUS;
                extended_range += Self::FLEET_SUPPLY_SHIPYARD_BONUS;
            }
            if has_supply_hub {
                full_range += Self::FLEET_SUPPLY_HUB_BONUS;
                extended_range += Self::FLEET_SUPPLY_HUB_BONUS;
            }

            if let Some(lane) = HyperspaceLane::new(colony.star, star_id)
                && self.empire_can_use_trade_lane(empire_id, lane)
            {
                full_range += Self::FLEET_SUPPLY_LANE_BONUS;
                extended_range += Self::FLEET_EXTENDED_LANE_BONUS;
            }

            if !connected {
                full_range = 0;
                extended_range /= 2;
            }

            let dx = (from.x - to.x) as i64;
            let dy = (from.y - to.y) as i64;
            let sq_dist = dx * dx + dy * dy;
            if sq_dist <= full_range * full_range {
                return FleetSupplyState::Supplied;
            }
            if sq_dist <= extended_range * extended_range {
                best = FleetSupplyState::Extended;
            }
        }
        best
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
        if let Some(required) = extraction.required_surface_building
            && !colony.buildings.contains(&required)
        {
            return false;
        }
        if let Some(required) = extraction.required_orbital_structure
            && !colony.orbital_installations.contains(&required)
        {
            return false;
        }
        if let Some(required_tech) = extraction.required_tech
            && !completed_techs.contains(&required_tech)
        {
            return false;
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

        if let Some(design_id) = self.fleet_custom_designs.get(&fleet_id)
            && let Some(design) = self.custom_designs.get(design_id)
        {
            let stats = design.derived_stats();
            offensive = offensive.saturating_add(stats.attack);
            defensive = defensive.saturating_add(stats.defense.saturating_add(stats.hp / 5));
            invasion_capability = invasion_capability.saturating_add(stats.invasion_strength);
            survey_capability = survey_capability.saturating_add(stats.survey_effectiveness);
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

    /// Derive the relationship between two empires.
    ///
    /// Player↔AI pairs come from the player diplomacy maps; AI↔AI pairs from
    /// [`GameState::ai_relations`]. Untracked pairs are `Unknown`.
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
            return self.ai_relation(empire_a, empire_b);
        };
        if let Some(relationship) = self.diplomacy_relationships.get(&other) {
            return relationship.state;
        }
        self.diplomacy
            .get(&other)
            .copied()
            .unwrap_or(RelationshipStatus::Unknown)
    }

    /// Relationship between two AI empires (`Unknown` for untracked pairs,
    /// identical empires, or when either side is the player).
    pub fn ai_relation(&self, empire_a: EmpireId, empire_b: EmpireId) -> RelationshipStatus {
        if empire_a == empire_b || empire_a == self.player_empire || empire_b == self.player_empire
        {
            return RelationshipStatus::Unknown;
        }
        let (lo, hi) = Self::ai_relation_key(empire_a, empire_b);
        self.ai_relations
            .get(&lo)
            .and_then(|inner| inner.get(&hi))
            .copied()
            .unwrap_or(RelationshipStatus::Unknown)
    }

    /// Set the relationship between two AI empires. Ignored when either side
    /// is the player (player relationships live in the diplomacy maps) or the
    /// empires are identical.
    pub fn set_ai_relation(
        &mut self,
        empire_a: EmpireId,
        empire_b: EmpireId,
        status: RelationshipStatus,
    ) {
        if empire_a == empire_b || empire_a == self.player_empire || empire_b == self.player_empire
        {
            return;
        }
        let (lo, hi) = Self::ai_relation_key(empire_a, empire_b);
        self.ai_relations.entry(lo).or_default().insert(hi, status);
    }

    /// AI↔AI relations the player is allowed to see. A pair is visible when
    /// the empires have met each other and either the player has made contact
    /// with both, or espionage intel on one side is deep enough to reveal
    /// that empire's foreign relations
    /// ([`IntelLevel::reveals_foreign_relations`]) — the other side may then
    /// be an empire the player has never met. Callers must still mask the
    /// identity of unmet empires when presenting these pairs. Returned in
    /// deterministic (lower id, higher id) order.
    pub fn known_ai_relations(&self) -> Vec<(EmpireId, EmpireId, RelationshipStatus)> {
        let mut visible = Vec::new();
        for (&a, inner) in &self.ai_relations {
            for (&b, &status) in inner {
                if status == RelationshipStatus::Unknown {
                    continue;
                }
                let met_both = self.player_knows_empire(a) && self.player_knows_empire(b);
                let spied = self.intel_level_for_empire(a).reveals_foreign_relations()
                    || self.intel_level_for_empire(b).reveals_foreign_relations();
                if met_both || spied {
                    visible.push((a, b, status));
                }
            }
        }
        visible
    }

    fn ai_relation_key(empire_a: EmpireId, empire_b: EmpireId) -> (EmpireId, EmpireId) {
        if empire_a <= empire_b {
            (empire_a, empire_b)
        } else {
            (empire_b, empire_a)
        }
    }

    /// The set of stars `empire` has explored. The player's fog lives in
    /// [`GameState::explored_stars`]; each AI empire has its own set.
    pub fn explored_stars_for(&self, empire: EmpireId) -> &BTreeSet<StarId> {
        static EMPTY: BTreeSet<StarId> = BTreeSet::new();
        if empire == self.player_empire {
            &self.explored_stars
        } else {
            self.empire_explored_stars.get(&empire).unwrap_or(&EMPTY)
        }
    }

    /// Record that `empire` has explored `star`.
    pub fn mark_star_explored(&mut self, empire: EmpireId, star: StarId) {
        if empire == self.player_empire {
            self.explored_stars.insert(star);
        } else {
            self.empire_explored_stars
                .entry(empire)
                .or_default()
                .insert(star);
        }
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

    pub fn player_knows_empire(&self, empire_id: EmpireId) -> bool {
        empire_id == self.player_empire
            || self.intel_level_for_empire(empire_id) >= IntelLevel::Contacted
    }

    pub fn intel_level_for_empire(&self, empire_id: EmpireId) -> IntelLevel {
        if empire_id == self.player_empire {
            return IntelLevel::Deep;
        }
        if let Some(intel) = self.empire_intel.get(&empire_id) {
            return intel.level;
        }
        match self.relationship_status(self.player_empire, empire_id) {
            RelationshipStatus::Unknown => IntelLevel::Unknown,
            _ => IntelLevel::Contacted,
        }
    }

    pub fn empire_colony_count(&self, empire_id: EmpireId) -> usize {
        self.colonies
            .values()
            .filter(|colony| colony.owner == empire_id)
            .count()
    }

    pub fn empire_total_fleet_strength(&self, empire_id: EmpireId) -> u32 {
        self.fleets
            .values()
            .filter(|fleet| fleet.owner == empire_id)
            .map(|fleet| fleet.strength.saturating_mul(fleet.ships.max(1)))
            .sum()
    }

    pub fn empire_fleet_strength_band(&self, empire_id: EmpireId) -> &'static str {
        match self.empire_total_fleet_strength(empire_id) {
            0 => "No signal",
            1..=12 => "Light",
            13..=32 => "Moderate",
            33..=64 => "Strong",
            _ => "Overwhelming",
        }
    }

    pub fn empire_highest_tech_tier(&self, empire_id: EmpireId) -> Option<TechTier> {
        let empire = self.empires.get(&empire_id)?;
        empire
            .research
            .completed
            .iter()
            .filter_map(|tech_id| tech_by_id(*tech_id))
            .map(|record| record.tier)
            .max()
    }

    pub fn empire_economy_summary(&self, empire_id: EmpireId) -> Option<EmpireEconomySummary> {
        if !self.empires.contains_key(&empire_id) {
            return None;
        }
        let mut summary = EmpireEconomySummary::default();
        let mut found = false;
        for colony in self
            .colonies
            .values()
            .filter(|colony| colony.owner == empire_id)
        {
            let planet = self
                .stars
                .get(&colony.star)
                .and_then(|star| star.planets.get(colony.planet_index));
            let yield_snapshot = crate::yield_model::calculate_yield(colony, planet);
            summary.food_balance += yield_snapshot.food - yield_snapshot.food_consumed;
            summary.industry += yield_snapshot.industry;
            summary.science += yield_snapshot.science;
            summary.credits += yield_snapshot.credits;
            found = true;
        }
        found.then_some(summary)
    }

    pub fn visible_empire_resources_for_player(
        &self,
        empire_id: EmpireId,
    ) -> Vec<(StrategicResource, u32)> {
        let mut resources = self
            .empire_resource_access
            .get(&empire_id)
            .map(|by_resource| {
                by_resource
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(resource, count)| (*resource, *count))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        resources.sort_by(|a, b| a.0.name().cmp(b.0.name()));
        resources
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

    pub fn recompute_fleet_supply(&self) -> BTreeMap<FleetId, FleetSupplyState> {
        self.fleets
            .keys()
            .copied()
            .map(|fleet_id| (fleet_id, self.derived_fleet_supply_state(fleet_id)))
            .collect()
    }

    fn empire_trade_hub_star(
        &self,
        empire_id: EmpireId,
        empire_colonies: &[(ColonyId, StarId)],
    ) -> Option<StarId> {
        let home_star = self.empires.get(&empire_id).map(|e| e.home_star);
        if let Some(home_star) = home_star
            && empire_colonies.iter().any(|(_, star)| *star == home_star)
        {
            return Some(home_star);
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

        if let Some(lane) = HyperspaceLane::new(from, to)
            && self.empire_can_use_trade_lane(empire_id, lane)
        {
            return true;
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
            && self.empire_explored_stars == other.empire_explored_stars
            && self.ai_relations == other.ai_relations
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
            && self.fleet_supply == other.fleet_supply
            && self.colony_blockade == other.colony_blockade
            && self.colony_unrest == other.colony_unrest
            && self.colony_unrest_causes == other.colony_unrest_causes
            && self.colony_rebellion_risk_bp == other.colony_rebellion_risk_bp
            && self.colony_recent_conquest_turn == other.colony_recent_conquest_turn
            && self.empire_resource_access == other.empire_resource_access
            && self.victory_status == other.victory_status
            && self.custom_designs == other.custom_designs
            && self.next_custom_design_id == other.next_custom_design_id
            && self.fleet_custom_designs == other.fleet_custom_designs
            && self.galactic_dispatches == other.galactic_dispatches
            && self.next_battle_report_id == other.next_battle_report_id
            && self.battle_reports == other.battle_reports
            && self.empire_intel == other.empire_intel
            && self.sector_directives == other.sector_directives
            && self.colony_automation == other.colony_automation
            && self.last_colony_yields == other.last_colony_yields
            && self.empire_trade_routes == other.empire_trade_routes
            && self.empire_trade_income == other.empire_trade_income
            && self.next_battle_session_id == other.next_battle_session_id
            && self.pending_battle_session == other.pending_battle_session
            && self.battle_reports_v3 == other.battle_reports_v3
    }
}

/// Serde helper for SeededRng serialization
#[cfg(feature = "serde")]
mod rng_serde {
    use super::SeededRng;
    use rand::rngs::ChaCha8Rng;
    use serde::{Deserializer, Serialize, Serializer};

    pub fn serialize<S>(rng: &SeededRng, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = rng.serialize_state();
        state.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SeededRng, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RngVisitor;
        impl<'de> serde::de::Visitor<'de> for RngVisitor {
            type Value = SeededRng;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a byte array of length 49 or an RNG map (legacy format)")
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<SeededRng, A::Error> {
                let mut arr = [0u8; 49];
                for (i, elem) in arr.iter_mut().enumerate() {
                    *elem = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(SeededRng(ChaCha8Rng::deserialize_state(&arr)))
            }

            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<SeededRng, M::Error> {
                // Legacy format (rand_chacha 0.3): consume map, return default RNG
                while map
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {}
                Ok(SeededRng::default())
            }
        }

        deserializer.deserialize_any(RngVisitor)
    }
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            seed: 0,
            turn: 1,
            sectors: BTreeMap::new(),
            stars: BTreeMap::new(),
            empires: BTreeMap::new(),
            colonies: BTreeMap::new(),
            fleets: BTreeMap::new(),
            player_empire: EmpireId(0),
            rng: SeededRng::default(),
            event_log: Vec::new(),
            next_colony_id: 1,
            next_fleet_id: 1,
            explored_stars: BTreeSet::new(),
            scout_missions: BTreeMap::new(),
            survey_missions: BTreeMap::new(),
            fleet_missions: BTreeMap::new(),
            ai_empire: None,
            ai_explored_stars: BTreeSet::new(),
            empire_explored_stars: BTreeMap::new(),
            ai_relations: BTreeMap::new(),
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
            fleet_supply: BTreeMap::new(),
            colony_blockade: BTreeMap::new(),
            colony_unrest: BTreeMap::new(),
            colony_unrest_causes: BTreeMap::new(),
            colony_rebellion_risk_bp: BTreeMap::new(),
            colony_recent_conquest_turn: BTreeMap::new(),
            star_constellations: BTreeMap::new(),
            star_nebulae: BTreeMap::new(),
            empire_resource_access: BTreeMap::new(),
            victory_status: VictoryStatus::default(),
            galactic_dispatches: VecDeque::new(),
            custom_designs: BTreeMap::new(),
            next_custom_design_id: 0,
            fleet_custom_designs: BTreeMap::new(),
            next_battle_report_id: 1,
            battle_reports: VecDeque::new(),
            empire_intel: BTreeMap::new(),
            sector_directives: BTreeMap::new(),
            colony_automation: BTreeMap::new(),
            last_colony_yields: BTreeMap::new(),
            empire_trade_routes: BTreeMap::new(),
            empire_trade_income: BTreeMap::new(),
            next_battle_session_id: 1,
            pending_battle_session: None,
            battle_reports_v3: VecDeque::new(),
        }
    }
}

/// Actual per-colony yield produced on the most recently processed turn, after
/// every engine modifier (bonuses, research percentage, unrest, isolation and
/// blockade penalties). This is the figure that fed empire income that turn, so
/// the UI can display real numbers instead of re-deriving an estimate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ColonyYieldSnapshot {
    pub industry: i64,
    pub credits: i64,
    pub science: i64,
    /// Food produced after penalties (zeroed when isolated/blockaded).
    pub food: i64,
    /// Gross food consumed by population (not reduced by penalties).
    pub food_consumed: i64,
    pub maintenance: i64,
}

/// Flat per-colony bonuses granted by an empire's controlled strategic
/// resources, plus the compounding research percentage some resources provide.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StrategicResourceBonuses {
    pub industry_per_colony: i64,
    pub credits_per_colony: i64,
    pub science_per_colony: i64,
    pub food_per_colony: i64,
    pub research_percent_bonus: i64,
}

/// Flat per-colony yield bonuses an empire applies to every owned colony each
/// turn, summed from completed technologies, faction identity traits, and
/// controlled strategic resources.
///
/// This is the additive part the engine layers on top of the base pop/jobs
/// yield. It deliberately excludes percentage modifiers (see
/// [`StrategicResourceBonuses::research_percent_bonus`]), isolation/blockade
/// penalties, and unrest effects, all of which depend on per-colony state and
/// cannot be expressed as a single empire-wide flat figure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PerColonyYieldBonuses {
    pub industry: i64,
    pub credits: i64,
    pub science: i64,
    pub food: i64,
}

impl GameState {
    /// True when any hostile/war-relation empire fleet is present at `star_id`.
    pub fn star_has_hostile_fleet(&self, empire_id: EmpireId, star_id: StarId) -> bool {
        self.fleets.values().any(|fleet| {
            if fleet.location != star_id || fleet.owner == empire_id {
                return false;
            }
            self.relationship_status(empire_id, fleet.owner)
                .is_hostile_or_war()
        })
    }

    /// True when the empire is at war with any other empire in the game.
    pub fn empire_is_at_war(&self, _empire_id: EmpireId) -> bool {
        self.diplomacy
            .values()
            .any(|s| *s == RelationshipStatus::War)
            || self
                .diplomacy_relationships
                .values()
                .any(|r| r.state == RelationshipStatus::War)
    }

    /// Compute the trade-route bonus permille contributed by strategic resources.
    fn trade_resource_bonus_permille(&self, empire_id: EmpireId) -> i64 {
        self.empire_resource_access
            .get(&empire_id)
            .map(|resources| {
                let count = resources
                    .iter()
                    .filter(|(res, count)| **count > 0 && res.record().trade_value > 0)
                    .count() as i64;
                count * balance::TRADE_RESOURCE_BONUS_PERMILLE
            })
            .unwrap_or(0)
    }

    /// Recompute deterministic trade routes and income for all empires.
    ///
    /// Returns `(routes_by_empire, income_by_empire)`. Routes are derived from
    /// colony connectivity, population, buildings, tech, strategic resources,
    /// and disruption factors (blockade, isolation, war, hostile fleets).
    pub fn recompute_empire_trade_routes(
        &self,
    ) -> (BTreeMap<EmpireId, Vec<TradeRoute>>, BTreeMap<EmpireId, i64>) {
        let mut all_routes: BTreeMap<EmpireId, Vec<TradeRoute>> = BTreeMap::new();
        let mut all_income: BTreeMap<EmpireId, i64> = BTreeMap::new();

        for empire_id in self.empires.keys().copied() {
            let mut empire_routes = Vec::new();
            let mut empire_total: i64 = 0;

            // Collect empire colony stars sorted for determinism
            let colony_stars: BTreeSet<StarId> = self
                .colonies
                .values()
                .filter(|c| c.owner == empire_id)
                .map(|c| c.star)
                .collect();

            if colony_stars.len() < 2 {
                all_routes.insert(empire_id, empire_routes);
                all_income.insert(empire_id, empire_total);
                continue;
            }

            // Check war status once per empire
            let war_disrupted = self.empire_is_at_war(empire_id);

            // Check tech bonuses
            let has_trade_tech = self
                .empires
                .get(&empire_id)
                .is_some_and(|e| e.research.completed.contains(&TechId(28)));

            // Check strategic resource bonus
            let resource_permille = self.trade_resource_bonus_permille(empire_id);

            // Index colony-by-star for fast lookup
            let star_colony: BTreeMap<StarId, ColonyId> = self
                .colonies
                .values()
                .filter(|c| c.owner == empire_id)
                .map(|c| (c.star, c.id))
                .collect();

            let star_vec: Vec<StarId> = colony_stars.into_iter().collect();
            for i in 0..star_vec.len() {
                for j in (i + 1)..star_vec.len() {
                    let from = star_vec[i];
                    let to = star_vec[j];

                    // Must have a trade link (lane or distance)
                    if !self.stars_have_trade_link(empire_id, from, to) {
                        continue;
                    }

                    // Look up colony data for both endpoints
                    let colony_a = match star_colony
                        .get(&from)
                        .and_then(|cid| self.colonies.get(cid))
                    {
                        Some(c) => c,
                        None => continue,
                    };
                    let colony_b = match star_colony.get(&to).and_then(|cid| self.colonies.get(cid))
                    {
                        Some(c) => c,
                        None => continue,
                    };

                    let pop_a = colony_a.population as i64;
                    let pop_b = colony_b.population as i64;
                    if pop_a == 0 || pop_b == 0 {
                        continue;
                    }

                    // Base value = sqrt(pop_a * pop_b) * TRADE_BASE_VALUE_PER_POP
                    let product = pop_a.saturating_mul(pop_b);
                    let base_geo_mean = (product as f64).sqrt() as i64;
                    let mut base_value =
                        base_geo_mean.saturating_mul(balance::TRADE_BASE_VALUE_PER_POP);

                    // Development bonus per Fabrication Yard
                    for colony in &[colony_a, colony_b] {
                        let yard_count = colony
                            .buildings
                            .iter()
                            .filter(|b| **b == BuildingType::FabricationYard)
                            .count() as i64;
                        if yard_count > 0 {
                            base_value = base_value.saturating_add(
                                base_value * yard_count * balance::TRADE_YARD_BONUS_PERMILLE / 1000,
                            );
                        }

                        // Hub bonus (shipyard or supply hub)
                        if colony.has_shipyard() || colony.has_supply_hub() {
                            base_value = base_value.saturating_add(
                                base_value * balance::TRADE_HUB_BONUS_PERMILLE / 1000,
                            );
                        }
                    }

                    // Tech bonus
                    if has_trade_tech {
                        base_value = base_value
                            .saturating_add(base_value * balance::TRADE_TECH_BONUS_PERMILLE / 1000);
                    }

                    // Strategic resource bonus
                    if resource_permille > 0 {
                        base_value =
                            base_value.saturating_add(base_value * resource_permille / 1000);
                    }

                    // Disruption evaluation
                    let mut disrupted = false;
                    let mut disruption_reason: Option<TradeDisruptionReason> = None;

                    // 1. Blockade at either endpoint
                    if !disrupted {
                        for colony in &[colony_a, colony_b] {
                            if self.colony_blockade_state(colony.id).is_some() {
                                disrupted = true;
                                disruption_reason = Some(TradeDisruptionReason::Blockade);
                                break;
                            }
                        }
                    }

                    // 2. Out of supply at either endpoint
                    if !disrupted {
                        for colony in &[colony_a, colony_b] {
                            if self.colony_supply_state(colony.id) != ColonySupplyState::Connected {
                                disrupted = true;
                                disruption_reason = Some(TradeDisruptionReason::OutOfSupply);
                                break;
                            }
                        }
                    }

                    // 3. Hostile fleet at either endpoint
                    if !disrupted {
                        for star in &[from, to] {
                            if self.star_has_hostile_fleet(empire_id, *star) {
                                disrupted = true;
                                disruption_reason =
                                    Some(TradeDisruptionReason::HostileFleetPresence);
                                break;
                            }
                        }
                    }

                    // 4. War zone
                    if !disrupted && war_disrupted {
                        disrupted = true;
                        disruption_reason = Some(TradeDisruptionReason::WarZone);
                    }

                    // Net value after disruption
                    let net_value = match disruption_reason {
                        Some(TradeDisruptionReason::Blockade) => {
                            (base_value * balance::TRADE_BLOCKADE_PENALTY_PERMILLE).max(0) / 1000
                        }
                        Some(TradeDisruptionReason::OutOfSupply) => {
                            (base_value * balance::TRADE_ISOLATION_PENALTY_PERMILLE).max(0) / 1000
                        }
                        Some(TradeDisruptionReason::HostileFleetPresence) => {
                            (base_value * balance::TRADE_HOSTILE_FLEET_PENALTY_PERMILLE).max(0)
                                / 1000
                        }
                        Some(TradeDisruptionReason::WarZone) => {
                            (base_value * balance::TRADE_WAR_ZONE_PENALTY_PERMILLE).max(0) / 1000
                        }
                        None => base_value,
                    };

                    empire_routes.push(TradeRoute {
                        from,
                        to,
                        base_value,
                        net_value,
                        disrupted,
                        disruption_reason,
                    });
                    empire_total = empire_total.saturating_add(net_value);
                }
            }

            // Deterministic sort by (from, to)
            empire_routes.sort_by_key(|a| (a.from, a.to));
            all_routes.insert(empire_id, empire_routes);
            all_income.insert(empire_id, empire_total);
        }

        (all_routes, all_income)
    }

    /// Per-colony bonuses from the strategic resources this empire controls.
    pub fn strategic_resource_bonuses_for_empire(
        &self,
        empire_id: EmpireId,
    ) -> StrategicResourceBonuses {
        let count = |resource| self.empire_resource_count(empire_id, resource);
        let has = |resource| count(resource) > 0;
        let scaled = |resource| i64::from(count(resource).min(2));

        let mut bonuses = StrategicResourceBonuses::default();
        if has(StrategicResource::ReactiveIsotopes) {
            bonuses.industry_per_colony += scaled(StrategicResource::ReactiveIsotopes);
        }
        if has(StrategicResource::QuantumCrystals) {
            bonuses.science_per_colony += scaled(StrategicResource::QuantumCrystals);
        }
        if has(StrategicResource::HyperfiberOrganics) {
            bonuses.food_per_colony += scaled(StrategicResource::HyperfiberOrganics);
        }
        if has(StrategicResource::AntimatterResidue) {
            bonuses.industry_per_colony += 1;
            bonuses.science_per_colony += 1;
        }
        if has(StrategicResource::DarkMatter) {
            bonuses.credits_per_colony += 1;
        }
        if has(StrategicResource::PsionicSpores) {
            bonuses.science_per_colony += 1;
        }
        if has(StrategicResource::NeutroniumDeposits) {
            bonuses.industry_per_colony += 1;
        }
        if has(StrategicResource::LivingAlloy) {
            bonuses.credits_per_colony += 1;
        }
        if has(StrategicResource::PrecursorDatacores) {
            bonuses.research_percent_bonus += 15;
        }
        bonuses
    }

    /// Total flat per-colony yield bonuses (tech + faction trait + strategic
    /// resource) the engine adds to every colony this empire owns each turn.
    ///
    /// Both the simulation and the TUI derive per-turn output from this so the
    /// two never disagree on the additive bonuses.
    pub fn per_colony_yield_bonuses(&self, empire_id: EmpireId) -> PerColonyYieldBonuses {
        let empire = self.empires.get(&empire_id);
        let completed = empire
            .map(|e| e.research.completed.as_slice())
            .unwrap_or(&[]);
        let traits = empire
            .and_then(|e| e.empire_def)
            .and_then(empire_definition_by_id)
            .map(|d| d.trait_modifiers)
            .unwrap_or_default();
        let resource = self.strategic_resource_bonuses_for_empire(empire_id);

        PerColonyYieldBonuses {
            industry: traits.industry_per_colony + resource.industry_per_colony,
            credits: tech_yield_bonus_per_colony(completed, YieldType::Credits)
                + traits.credits_per_colony
                + resource.credits_per_colony,
            science: tech_yield_bonus_per_colony(completed, YieldType::Science)
                + traits.science_per_colony
                + resource.science_per_colony,
            food: tech_yield_bonus_per_colony(completed, YieldType::Food)
                + traits.food_per_colony
                + resource.food_per_colony,
        }
    }
}

#[cfg(test)]
mod tests;
