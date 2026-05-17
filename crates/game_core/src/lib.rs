//! FARSPACE game core - headless game logic
//!
//! This crate contains the core game mechanics, state management,
//! command processing, and event generation for FARSPACE.

pub mod ai;
pub mod commands;
pub mod deterministic;
pub mod dispatch;
pub mod engine;
pub mod events;
pub mod galaxy;
pub mod state;
pub mod victory;
pub mod yield_model;

pub use commands::Command;
pub use dispatch::{
    generate_dispatch, DispatchCategory, DispatchItem, DispatchSeverity, GalacticDispatch,
    DISPATCH_CADENCE, DISPATCH_MAX_HISTORY,
};
pub use engine::Engine;
pub use events::Event;
pub use state::{
    all_components, all_empire_definitions, all_hull_templates, all_ship_designs, all_techs,
    available_tech_ids, components_for_slot, empire_definition_by_id, is_component_unlocked,
    is_tech_available, planet_yield_effect, tech_by_id, tech_yield_bonus_per_colony, BuildItem,
    BuildingType, Colony, ColonyId, ColonyRole, ColonySupplyState, ComponentDef, ComponentId,
    ComponentTag, CustomDesignId, CustomShipDesign, DerivedShipStats, DifficultyLevel, Empire,
    EmpireDefinition, EmpireDefinitionId, EmpireId, EmpireTraitModifiers, Fleet, FleetId,
    FleetKind, FleetLocation, FleetMission, FleetOrder, GalaxySize, GameState, HullId,
    HullTemplate, HyperspaceLane, OrbitalStructureType, Planet, PlanetClass, PlanetSize,
    PlanetSpecial, PlaystyleTag, ProductionItem, RelationshipStatus, ResearchState, RoleModifiers,
    ScenarioSetup, ScoutMission, Sector, SectorId, ShipDesignId, ShipDesignRecord, SlotCategory,
    SpectralClass, Star, StarId, StrategicResource, SurveyMission, TechCapability, TechDomain,
    TechId, TechRecord, TechTier, TechUnlock, VictoryCondition, VictoryPath, VictoryProgress,
    VictoryProgressValue, VictorySettings, VictoryStatus, YieldEffect, YieldType,
};
pub use yield_model::{
    ColonyWorkforceSummary, ColonyYield, JobAssignment, JobType, YieldBreakdown, YieldContext,
};
