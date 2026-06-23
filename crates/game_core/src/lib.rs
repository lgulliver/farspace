//! FARSPACE game core - headless game logic
//!
//! This crate contains the core game mechanics, state management,
//! command processing, and event generation for FARSPACE.

pub mod advisor;
pub mod ai;
pub mod balance;
pub mod combat_v3;
pub mod commands;
pub mod deterministic;
pub mod dispatch;
pub mod engine;
pub mod events;
pub mod galaxy;
pub mod state;
pub mod victory;
pub mod yield_model;

pub use combat_v3::{
    BattleOutcome, BattleReportV3, BattleRoundSummary, BattleSession, BattleSessionState,
    BattleSetupSummary, BattleSide, CardId, CardVerb, HAND_SIZE, MAX_ROUNDS, apply_retreat,
    apply_round, build_hand, build_setup_summary, card_by_id, play_player_card,
};
pub use commands::Command;
pub use dispatch::{
    DISPATCH_CADENCE, DISPATCH_MAX_HISTORY, DispatchCategory, DispatchItem, DispatchSeverity,
    GalacticDispatch, generate_dispatch,
};
pub use engine::{Engine, fleet_maintenance_for_empire};
pub use events::Event;
pub use state::{
    AnomalyCategory, AnomalyRiskLevel, BattleReport, BuildItem, BuildingType, Colony,
    ColonyAutomation, ColonyId, ColonyRole, ColonySupplyState, ColonyUnrestState,
    ColonyYieldSnapshot, CombatPhase, CombatPhaseSummary, ComponentDef, ComponentId, ComponentTag,
    CustomDesignId, CustomShipDesign, DerivedShipStats, DifficultyLevel, DiplomaticCommunication,
    DiplomaticCommunicationType, DiplomaticRelationship, DiplomaticResponse, DiplomaticTone,
    DiplomaticTreaty, DiscoveryRarity, DiscoveryRequirements, Empire, EmpireDefinition,
    EmpireDefinitionId, EmpireEconomySummary, EmpireId, EmpireIntel, EmpireTraitModifiers,
    EspionageMission, Fleet, FleetEvaluation, FleetFormation, FleetId, FleetKind, FleetLocation,
    FleetMission, FleetOrder, FleetRole, FleetSupplyState, GalaxySize, GameState, HullId,
    HullTemplate, HyperspaceLane, IntelLevel, IntelSource, OrbitalStructureType,
    PerColonyYieldBonuses, Planet, PlanetAnomaly, PlanetClass, PlanetSize, PlanetSpecial,
    PlanetSpecialCategory, PlaystyleTag, ProductionItem, RelationshipStatus, ResearchState,
    RoleModifiers, ScenarioSetup, ScoutMission, Sector, SectorDirective, SectorId, SeededRng,
    ShipDesignId, ShipDesignRecord, SlotCategory, SpectralClass, Star, StarId, StrategicResource,
    StrategicResourceBonuses, StrategicResourceCategory, StrategicResourceRarity,
    StrategicResourceRecord, SurveyMission, TechCapability, TechDomain, TechId, TechRecord,
    TechTier, TechUnlock, TradeDisruptionReason, TradeRoute, TreatyType, UnrestCause,
    VictoryCondition, VictoryPath, VictoryProgress, VictoryProgressValue, VictorySettings,
    VictoryStatus, YieldEffect, YieldType, all_components, all_empire_definitions,
    all_hull_templates, all_ship_designs, all_techs, available_tech_ids, components_for_slot,
    empire_definition_by_id, is_component_available, is_component_unlocked,
    is_resource_discoverable, is_tech_available, planet_yield_effect, tech_by_id,
    tech_yield_bonus_per_colony, visible_anomalies_for_empire, visible_resources_for_empire,
    visible_specials_for_empire,
};
pub use yield_model::{
    ColonyWorkforceSummary, ColonyYield, JobAssignment, JobType, YieldBreakdown, YieldContext,
};
