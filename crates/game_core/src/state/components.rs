use super::*;

/// Unique identifier for a hull template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HullId(pub u32);

/// Unique identifier for a ship component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComponentId(pub u32);

/// Unique identifier for a custom ship design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CustomDesignId(pub u32);

/// Ship component slot category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum SlotCategory {
    Weapon,
    Defense,
    Engine,
    MissionModule,
    Utility,
}

/// Special functional tags for components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ComponentTag {
    Colony,
    Invasion,
    LongRange,
    Sensors,
    Survey,
}

/// Static definition of a ship component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentDef {
    pub component_id: ComponentId,
    pub category: SlotCategory,
    pub name: &'static str,
    pub attack_modifier: i32,
    pub defense_modifier: i32,
    pub hp_modifier: i32,
    pub cost_modifier: i64,
    pub maintenance_modifier: i32,
    pub movement_modifier: i32,
    pub special_tags: &'static [ComponentTag],
    pub required_tech: Option<TechId>,
}

/// Static definition of a ship hull template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HullTemplate {
    pub hull_id: HullId,
    pub name: &'static str,
    pub fleet_kind: FleetKind,
    pub base_cost: u64,
    pub base_maintenance: u32,
    pub base_attack: u32,
    pub base_defense: u32,
    pub base_hp: u32,
    pub slots: &'static [SlotCategory],
    pub required_tech: Option<TechId>,
    pub role: &'static str,
}

// ---------------------------------------------------------------------------
// Static data
// ---------------------------------------------------------------------------

static HULL_TEMPLATES: [HullTemplate; 11] = [
    HullTemplate {
        hull_id: HullId(1),
        name: "Scout",
        fleet_kind: FleetKind::Scout,
        base_cost: 50,
        base_maintenance: 1,
        base_attack: 1,
        base_defense: 1,
        base_hp: 10,
        slots: &[SlotCategory::Engine, SlotCategory::Utility],
        required_tech: None,
        role: "exploration",
    },
    HullTemplate {
        hull_id: HullId(2),
        name: "Colony Ship",
        fleet_kind: FleetKind::Colonizer,
        base_cost: 200,
        base_maintenance: 1,
        base_attack: 1,
        base_defense: 1,
        base_hp: 10,
        slots: &[SlotCategory::Engine, SlotCategory::MissionModule],
        required_tech: Some(TechId(2)),
        role: "colonization",
    },
    HullTemplate {
        hull_id: HullId(3),
        name: "Science Vessel",
        fleet_kind: FleetKind::Science,
        base_cost: 100,
        base_maintenance: 1,
        base_attack: 1,
        base_defense: 1,
        base_hp: 10,
        slots: &[SlotCategory::Engine, SlotCategory::Utility],
        required_tech: Some(TechId(12)),
        role: "science",
    },
    HullTemplate {
        hull_id: HullId(4),
        name: "Troop Transport",
        fleet_kind: FleetKind::TroopTransport,
        base_cost: 150,
        base_maintenance: 2,
        base_attack: 2,
        base_defense: 3,
        base_hp: 15,
        slots: &[SlotCategory::Engine, SlotCategory::Defense, SlotCategory::MissionModule],
        required_tech: Some(TechId(11)),
        role: "invasion",
    },
    HullTemplate {
        hull_id: HullId(5),
        name: "Fast Scout",
        fleet_kind: FleetKind::FastScout,
        base_cost: 75,
        base_maintenance: 1,
        base_attack: 1,
        base_defense: 1,
        base_hp: 10,
        slots: &[SlotCategory::Engine, SlotCategory::Utility],
        required_tech: Some(TechId(13)),
        role: "fast-exploration",
    },
    HullTemplate {
        hull_id: HullId(6),
        name: "Survey Cutter",
        fleet_kind: FleetKind::SurveyCutter,
        base_cost: 150,
        base_maintenance: 2,
        base_attack: 1,
        base_defense: 1,
        base_hp: 10,
        slots: &[SlotCategory::Engine, SlotCategory::Utility, SlotCategory::Utility],
        required_tech: Some(TechId(12)),
        role: "survey",
    },
    HullTemplate {
        hull_id: HullId(7),
        name: "Colony Ark",
        fleet_kind: FleetKind::ColonyArk,
        base_cost: 350,
        base_maintenance: 2,
        base_attack: 2,
        base_defense: 2,
        base_hp: 15,
        slots: &[SlotCategory::Engine, SlotCategory::MissionModule, SlotCategory::Utility],
        required_tech: Some(TechId(15)),
        role: "mass-colonization",
    },
    HullTemplate {
        hull_id: HullId(8),
        name: "Escort Frigate",
        fleet_kind: FleetKind::EscortFrigate,
        base_cost: 120,
        base_maintenance: 2,
        base_attack: 3,
        base_defense: 5,
        base_hp: 20,
        slots: &[SlotCategory::Weapon, SlotCategory::Defense, SlotCategory::Engine, SlotCategory::Utility],
        required_tech: Some(TechId(16)),
        role: "defense",
    },
    HullTemplate {
        hull_id: HullId(9),
        name: "Missile Frigate",
        fleet_kind: FleetKind::MissileFrigate,
        base_cost: 200,
        base_maintenance: 3,
        base_attack: 6,
        base_defense: 3,
        base_hp: 20,
        slots: &[SlotCategory::Weapon, SlotCategory::Weapon, SlotCategory::Defense, SlotCategory::Engine],
        required_tech: Some(TechId(17)),
        role: "offense",
    },
    HullTemplate {
        hull_id: HullId(10),
        name: "Destroyer",
        fleet_kind: FleetKind::Destroyer,
        base_cost: 300,
        base_maintenance: 4,
        base_attack: 8,
        base_defense: 5,
        base_hp: 30,
        slots: &[SlotCategory::Weapon, SlotCategory::Weapon, SlotCategory::Defense, SlotCategory::Engine, SlotCategory::Utility],
        required_tech: Some(TechId(18)),
        role: "capital",
    },
    HullTemplate {
        hull_id: HullId(11),
        name: "Patrol Corvette",
        fleet_kind: FleetKind::PatrolCorvette,
        base_cost: 80,
        base_maintenance: 1,
        base_attack: 2,
        base_defense: 3,
        base_hp: 15,
        slots: &[SlotCategory::Weapon, SlotCategory::Defense, SlotCategory::Engine],
        required_tech: Some(TechId(16)),
        role: "patrol",
    },
];

static COMPONENT_DEFS: [ComponentDef; 13] = [
    // Weapon
    ComponentDef {
        component_id: ComponentId(1),
        category: SlotCategory::Weapon,
        name: "Kinetic Battery",
        attack_modifier: 2,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 20,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: Some(TechId(4)),
    },
    ComponentDef {
        component_id: ComponentId(2),
        category: SlotCategory::Weapon,
        name: "Missile Rack",
        attack_modifier: 4,
        defense_modifier: -1,
        hp_modifier: 0,
        cost_modifier: 40,
        maintenance_modifier: 1,
        movement_modifier: -1,
        special_tags: &[],
        required_tech: Some(TechId(17)),
    },
    // Defense
    ComponentDef {
        component_id: ComponentId(10),
        category: SlotCategory::Defense,
        name: "Reinforced Plating",
        attack_modifier: 0,
        defense_modifier: 3,
        hp_modifier: 5,
        cost_modifier: 15,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: Some(TechId(4)),
    },
    ComponentDef {
        component_id: ComponentId(11),
        category: SlotCategory::Defense,
        name: "Shield Matrix",
        attack_modifier: 0,
        defense_modifier: 4,
        hp_modifier: 3,
        cost_modifier: 35,
        maintenance_modifier: 1,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: Some(TechId(16)),
    },
    ComponentDef {
        component_id: ComponentId(12),
        category: SlotCategory::Defense,
        name: "Point Defense Grid",
        attack_modifier: -1,
        defense_modifier: 2,
        hp_modifier: 0,
        cost_modifier: 25,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: Some(TechId(16)),
    },
    // Engine
    ComponentDef {
        component_id: ComponentId(20),
        category: SlotCategory::Engine,
        name: "Chemical Thrusters",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 0,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: None,
    },
    ComponentDef {
        component_id: ComponentId(21),
        category: SlotCategory::Engine,
        name: "Ion Drive",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 20,
        maintenance_modifier: 0,
        movement_modifier: 1,
        special_tags: &[],
        required_tech: Some(TechId(13)),
    },
    // Utility
    ComponentDef {
        component_id: ComponentId(30),
        category: SlotCategory::Utility,
        name: "Targeting Suite",
        attack_modifier: 1,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 15,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: None,
    },
    ComponentDef {
        component_id: ComponentId(31),
        category: SlotCategory::Utility,
        name: "Long-Range Sensors",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 10,
        maintenance_modifier: 0,
        movement_modifier: 1,
        special_tags: &[ComponentTag::Sensors, ComponentTag::LongRange],
        required_tech: Some(TechId(3)),
    },
    ComponentDef {
        component_id: ComponentId(32),
        category: SlotCategory::Utility,
        name: "Cargo Pods",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 2,
        cost_modifier: 5,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[],
        required_tech: None,
    },
    // MissionModule
    ComponentDef {
        component_id: ComponentId(40),
        category: SlotCategory::MissionModule,
        name: "Colony Core",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 50,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[ComponentTag::Colony],
        required_tech: Some(TechId(2)),
    },
    ComponentDef {
        component_id: ComponentId(41),
        category: SlotCategory::MissionModule,
        name: "Survey Array",
        attack_modifier: 0,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 30,
        maintenance_modifier: 0,
        movement_modifier: 0,
        special_tags: &[ComponentTag::Survey],
        required_tech: Some(TechId(12)),
    },
    ComponentDef {
        component_id: ComponentId(42),
        category: SlotCategory::MissionModule,
        name: "Troop Bays",
        attack_modifier: 1,
        defense_modifier: 0,
        hp_modifier: 0,
        cost_modifier: 40,
        maintenance_modifier: 1,
        movement_modifier: 0,
        special_tags: &[ComponentTag::Invasion],
        required_tech: Some(TechId(11)),
    },
];

// ---------------------------------------------------------------------------
// Public accessors
// ---------------------------------------------------------------------------

/// All hull templates in deterministic order.
pub fn all_hull_templates() -> &'static [HullTemplate] {
    &HULL_TEMPLATES
}

/// All component definitions in deterministic order.
pub fn all_components() -> &'static [ComponentDef] {
    &COMPONENT_DEFS
}

/// All components that fit a given slot category.
pub fn components_for_slot(category: SlotCategory) -> Vec<&'static ComponentDef> {
    all_components()
        .iter()
        .filter(|c| c.category == category)
        .collect()
}

/// Returns `true` if the given component's tech requirement is satisfied.
pub fn is_component_unlocked(component_id: ComponentId, completed_techs: &[TechId]) -> bool {
    match component_id.def() {
        Some(def) => match def.required_tech {
            None => true,
            Some(tech) => completed_techs.contains(&tech),
        },
        None => false,
    }
}

// ---------------------------------------------------------------------------
// impl HullId
// ---------------------------------------------------------------------------

impl HullId {
    pub const SCOUT: HullId = HullId(1);
    pub const COLONY: HullId = HullId(2);
    pub const SCIENCE: HullId = HullId(3);
    pub const TROOP_TRANSPORT: HullId = HullId(4);
    pub const FAST_SCOUT: HullId = HullId(5);
    pub const SURVEY_CUTTER: HullId = HullId(6);
    pub const COLONY_ARK: HullId = HullId(7);
    pub const ESCORT_FRIGATE: HullId = HullId(8);
    pub const MISSILE_FRIGATE: HullId = HullId(9);
    pub const DESTROYER: HullId = HullId(10);
    pub const PATROL_CORVETTE: HullId = HullId(11);

    /// Look up the static hull template for this ID.
    pub fn template(self) -> Option<&'static HullTemplate> {
        all_hull_templates().iter().find(|h| h.hull_id == self)
    }

    /// Map a `ShipDesignId` to a `HullId` when the IDs overlap (1–11).
    pub fn from_ship_design_id(id: ShipDesignId) -> Option<HullId> {
        if id.0 >= 1 && id.0 <= 11 {
            Some(HullId(id.0))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// impl ComponentId
// ---------------------------------------------------------------------------

impl ComponentId {
    pub const KINETIC_BATTERY: ComponentId = ComponentId(1);
    pub const MISSILE_RACK: ComponentId = ComponentId(2);
    pub const REINFORCED_PLATING: ComponentId = ComponentId(10);
    pub const SHIELD_MATRIX: ComponentId = ComponentId(11);
    pub const POINT_DEFENSE_GRID: ComponentId = ComponentId(12);
    pub const CHEMICAL_THRUSTERS: ComponentId = ComponentId(20);
    pub const ION_DRIVE: ComponentId = ComponentId(21);
    pub const TARGETING_SUITE: ComponentId = ComponentId(30);
    pub const LONG_RANGE_SENSORS: ComponentId = ComponentId(31);
    pub const CARGO_PODS: ComponentId = ComponentId(32);
    pub const COLONY_CORE: ComponentId = ComponentId(40);
    pub const SURVEY_ARRAY: ComponentId = ComponentId(41);
    pub const TROOP_BAYS: ComponentId = ComponentId(42);

    /// Look up the static component definition for this ID.
    pub fn def(self) -> Option<&'static ComponentDef> {
        all_components().iter().find(|c| c.component_id == self)
    }
}
