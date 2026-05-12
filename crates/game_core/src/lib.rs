//! FARSPACE game core - headless game logic
//!
//! This crate contains the core game mechanics, state management,
//! command processing, and event generation for FARSPACE.

pub mod ai;
pub mod commands;
pub mod deterministic;
pub mod engine;
pub mod events;
pub mod galaxy;
pub mod state;
pub mod yield_model;

pub use commands::Command;
pub use engine::Engine;
pub use events::Event;
pub use state::{
    all_ship_designs, all_techs, available_tech_ids, is_tech_available, planet_yield_effect,
    tech_by_id, tech_yield_bonus_per_colony, BuildItem, BuildingType, Colony, ColonyId, ColonyRole,
    Empire, EmpireId, Fleet, FleetId, FleetKind, FleetLocation, FleetMission, GameState,
    HyperspaceLane, OrbitalStructureType, Planet, PlanetClass, PlanetSize, PlanetSpecial,
    ProductionItem, RelationshipStatus, ResearchState, RoleModifiers, ScoutMission, Sector,
    SectorId, ShipDesignId, ShipDesignRecord, SpectralClass, Star, StarId, StrategicResource,
    SurveyMission, TechCapability, TechDomain, TechId, TechRecord, TechTier, TechUnlock,
    YieldEffect, YieldType,
};
pub use yield_model::ColonyYield;
