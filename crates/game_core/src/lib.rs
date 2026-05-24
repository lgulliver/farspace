//! FARSPACE game core - headless game logic
//!
//! This crate contains the core game mechanics, state management,
//! command processing, and event generation for FARSPACE.

pub mod advisor;
pub mod ai;
pub mod balance;
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
pub use engine::{fleet_maintenance_for_empire, Engine};
pub use events::Event;
pub use state::{
    all_components, all_empire_definitions, all_hull_templates, all_ship_designs, all_techs,
    available_tech_ids, components_for_slot, empire_definition_by_id, is_component_available,
    is_component_unlocked, is_resource_discoverable, is_tech_available, planet_yield_effect,
    tech_by_id, tech_yield_bonus_per_colony, visible_anomalies_for_empire,
    visible_resources_for_empire, visible_specials_for_empire, AnomalyCategory, AnomalyRiskLevel,
    BattleReport, BuildItem, BuildingType, Colony, ColonyId, ColonyRole, ColonySupplyState,
    ColonyUnrestState, CombatPhase, CombatPhaseSummary, ComponentDef, ComponentId, ComponentTag,
    CustomDesignId, CustomShipDesign, DerivedShipStats, DifficultyLevel, DiplomaticCommunication,
    DiplomaticCommunicationType, DiplomaticRelationship, DiplomaticResponse, DiplomaticTone,
    DiplomaticTreaty, DiscoveryRarity, DiscoveryRequirements, Empire, EmpireDefinition,
    EmpireDefinitionId, EmpireId, EmpireTraitModifiers, Fleet, FleetEvaluation, FleetFormation,
    FleetId, FleetKind, FleetLocation, FleetMission, FleetOrder, FleetRole, FleetSupplyState,
    GalaxySize, GameState, HullId, HullTemplate, HyperspaceLane, OrbitalStructureType, Planet,
    PlanetAnomaly, PlanetClass, PlanetSize, PlanetSpecial, PlanetSpecialCategory, PlaystyleTag,
    ProductionItem, RelationshipStatus, ResearchState, RoleModifiers, ScenarioSetup, ScoutMission,
    Sector, SectorId, ShipDesignId, ShipDesignRecord, SlotCategory, SpectralClass, Star, StarId,
    StrategicResource, StrategicResourceCategory, StrategicResourceRarity, StrategicResourceRecord,
    SurveyMission, TechCapability, TechDomain, TechId, TechRecord, TechTier, TechUnlock,
    TreatyType, UnrestCause, VictoryCondition, VictoryPath, VictoryProgress, VictoryProgressValue,
    VictorySettings, VictoryStatus, YieldEffect, YieldType,
};
pub use yield_model::{
    ColonyWorkforceSummary, ColonyYield, JobAssignment, JobType, YieldBreakdown, YieldContext,
};
