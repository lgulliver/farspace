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
    all_techs, BuildItem, BuildingType, Colony, ColonyId, Empire, EmpireId, Fleet, FleetId,
    FleetKind, FleetLocation, FleetMission, GameState, OrbitalStructureType, Planet, PlanetClass,
    PlanetSize, RelationshipStatus, ResearchState, ScoutMission, SpectralClass, Star, StarId,
    TechId, TechRecord,
};
pub use yield_model::ColonyYield;
