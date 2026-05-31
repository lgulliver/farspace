use super::*;

/// High-level playstyle orientation tag for an empire.
///
/// Tags influence AI build/research priorities and serve as display metadata
/// for the player.  Multiple tags may apply to a single empire definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaystyleTag {
    /// Prefers production buildings and infrastructure.
    Industrial,
    /// Prefers research structures and technology advancement.
    Scientific,
    /// Prefers scouts, science ships, and rapid colonization.
    Expansionist,
    /// Prefers military fleets and defense.
    Militarist,
    /// Prefers food/growth stability and population.
    Agrarian,
    /// Diplomatic bonus placeholder — no full diplomacy effect yet.
    Diplomatic,
}

impl PlaystyleTag {
    /// Short display label for this tag.
    pub fn label(&self) -> &'static str {
        match self {
            PlaystyleTag::Industrial => "Industrial",
            PlaystyleTag::Scientific => "Scientific",
            PlaystyleTag::Expansionist => "Expansionist",
            PlaystyleTag::Militarist => "Militarist",
            PlaystyleTag::Agrarian => "Agrarian",
            PlaystyleTag::Diplomatic => "Diplomatic",
        }
    }
}

/// High-level AI doctrine axis used for deterministic weighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AiDoctrine {
    Explorer,
    Technologist,
    Merchant,
    Imperial,
    Militarist,
    Industrialist,
    Expansionist,
    Isolationist,
    Biologist,
}

impl AiDoctrine {
    pub fn label(&self) -> &'static str {
        match self {
            AiDoctrine::Explorer => "Explorer",
            AiDoctrine::Technologist => "Technologist",
            AiDoctrine::Merchant => "Merchant",
            AiDoctrine::Imperial => "Imperial",
            AiDoctrine::Militarist => "Militarist",
            AiDoctrine::Industrialist => "Industrialist",
            AiDoctrine::Expansionist => "Expansionist",
            AiDoctrine::Isolationist => "Isolationist",
            AiDoctrine::Biologist => "Biologist",
        }
    }

    pub fn short_code(&self) -> &'static str {
        match self {
            AiDoctrine::Explorer => "EXP",
            AiDoctrine::Technologist => "TEC",
            AiDoctrine::Merchant => "MCH",
            AiDoctrine::Imperial => "IMP",
            AiDoctrine::Militarist => "MIL",
            AiDoctrine::Industrialist => "IND",
            AiDoctrine::Expansionist => "XPN",
            AiDoctrine::Isolationist => "ISO",
            AiDoctrine::Biologist => "BIO",
        }
    }

    pub fn short_summary(&self) -> &'static str {
        match self {
            AiDoctrine::Explorer => "Scouting, intel, and map reach first.",
            AiDoctrine::Technologist => "Science velocity and advanced capability timing.",
            AiDoctrine::Merchant => "Economic throughput and logistics efficiency.",
            AiDoctrine::Imperial => "Command posture, coercive leverage, and control.",
            AiDoctrine::Militarist => "Fleet strength, pressure, and battle-readiness.",
            AiDoctrine::Industrialist => "Production capacity and infrastructure depth.",
            AiDoctrine::Expansionist => "Rapid footprint growth and colony tempo.",
            AiDoctrine::Isolationist => "Secure borders and low-risk internal stability.",
            AiDoctrine::Biologist => "Population growth, food, housing, and adaptation.",
        }
    }
}

/// Doctrine weight row for an empire definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireDoctrineWeight {
    pub doctrine: AiDoctrine,
    pub weight: u8,
}

/// Per-colony flat yield modifiers granted by an empire's identity.
///
/// Applied every turn to each colony owned by the empire, on top of the
/// standard yield model.  Values may be negative (e.g. a trade-off design).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmpireTraitModifiers {
    /// Extra industry per colony per turn.
    pub industry_per_colony: i64,
    /// Extra science per colony per turn.
    pub science_per_colony: i64,
    /// Extra credits per colony per turn.
    pub credits_per_colony: i64,
    /// Extra food per colony per turn.
    pub food_per_colony: i64,
}

/// Deterministic diplomacy posture granted by an empire's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireDiplomacyProfile {
    /// Relationship state established on first contact.
    pub first_contact_status: RelationshipStatus,
    /// Relationship state this empire drifts toward when borders are quiet.
    pub resting_status: RelationshipStatus,
    /// Relationship state this empire drifts toward when border pressure is present.
    pub border_tension_status: RelationshipStatus,
    /// Relationship state this empire drifts toward under severe border pressure.
    pub severe_border_tension_status: RelationshipStatus,
}

impl Default for EmpireDiplomacyProfile {
    fn default() -> Self {
        Self::standard()
    }
}

impl EmpireDiplomacyProfile {
    pub const fn standard() -> Self {
        Self {
            first_contact_status: RelationshipStatus::Contacted,
            resting_status: RelationshipStatus::Neutral,
            border_tension_status: RelationshipStatus::Tense,
            severe_border_tension_status: RelationshipStatus::Hostile,
        }
    }
}

/// Deterministic production and upkeep modifiers granted by an empire's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmpireMilitaryModifiers {
    /// Percentage adjustment applied to scout ship production cost.
    pub scout_cost_modifier_pct: i8,
    /// Percentage adjustment applied to science ship production cost.
    pub science_ship_cost_modifier_pct: i8,
    /// Percentage adjustment applied to troop transport production cost.
    pub troop_transport_cost_modifier_pct: i8,
    /// Percentage adjustment applied to shipyard production cost.
    pub shipyard_cost_modifier_pct: i8,
    /// Per-fleet maintenance delta applied after the baseline cost.
    pub fleet_maintenance_modifier_per_fleet: i64,
    /// Flat invasion strength bonus per troop transport ship.
    pub invasion_strength_bonus_per_transport: u32,
    /// Percentage adjustment applied to combat ship production cost
    /// (Escort Frigate, Missile Frigate, Destroyer, Patrol Corvette).
    pub combat_ship_cost_modifier_pct: i8,
}

impl EmpireMilitaryModifiers {
    pub const fn none() -> Self {
        Self {
            scout_cost_modifier_pct: 0,
            science_ship_cost_modifier_pct: 0,
            troop_transport_cost_modifier_pct: 0,
            shipyard_cost_modifier_pct: 0,
            fleet_maintenance_modifier_per_fleet: 0,
            invasion_strength_bonus_per_transport: 0,
            combat_ship_cost_modifier_pct: 0,
        }
    }
}

/// High-level deterministic AI preferences granted by an empire's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmpireAiProfile {
    /// Ordered list of preferred research domains, strongest to weakest.
    pub research_focus: &'static [TechDomain],
    /// Whether the AI should prioritise science ships once they become available.
    pub prefers_science_ships: bool,
    /// Whether the AI should prioritise troop transports once they become available.
    pub prefers_troop_transports: bool,
    /// Whether the AI should prefer Scientific/Balanced colonies over aggressive roles.
    pub prefers_stable_colonies: bool,
    /// Whether the AI should favour Military roles on high-output worlds.
    pub prefers_military_roles: bool,
    /// Whether the AI should prioritise Fast Scouts over standard scouts.
    pub prefers_fast_scouts: bool,
    /// Whether the AI should prioritise Colony Arks over standard colony ships.
    pub prefers_colony_arks: bool,
    /// Whether the AI should prioritise combat ships (Destroyer, Missile Frigate).
    pub prefers_combat_ships: bool,
    /// Whether the AI should prioritise defensive ships (Escort Frigate, Patrol Corvette).
    pub prefers_defensive_ships: bool,
}

impl Default for EmpireAiProfile {
    fn default() -> Self {
        Self::none()
    }
}

impl EmpireAiProfile {
    pub const fn none() -> Self {
        Self {
            research_focus: &[],
            prefers_science_ships: false,
            prefers_troop_transports: false,
            prefers_stable_colonies: false,
            prefers_military_roles: false,
            prefers_fast_scouts: false,
            prefers_colony_arks: false,
            prefers_combat_ships: false,
            prefers_defensive_ships: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        research_focus: &'static [TechDomain],
        prefers_science_ships: bool,
        prefers_troop_transports: bool,
        prefers_stable_colonies: bool,
        prefers_military_roles: bool,
        prefers_fast_scouts: bool,
        prefers_colony_arks: bool,
        prefers_combat_ships: bool,
        prefers_defensive_ships: bool,
    ) -> Self {
        Self {
            research_focus,
            prefers_science_ships,
            prefers_troop_transports,
            prefers_stable_colonies,
            prefers_military_roles,
            prefers_fast_scouts,
            prefers_colony_arks,
            prefers_combat_ships,
            prefers_defensive_ships,
        }
    }
}

/// Static definition of a playable empire faction.
///
/// These are compile-time records — not serialised.  An empire's chosen
/// definition is referenced by `EmpireDefinitionId` stored in `Empire`.
pub struct EmpireDefinition {
    /// Stable numeric identifier.
    pub id: EmpireDefinitionId,
    /// Display name (original IP — not derived from other 4X titles).
    pub name: &'static str,
    /// One-line flavour description shown during setup and in diplomacy.
    pub short_description: &'static str,
    /// Short tonal description used in diplomacy and empire overview displays.
    pub tone: &'static str,
    /// Single-character symbol used in compact map display.
    pub symbol: char,
    /// Flat per-colony yield bonuses applied every turn.
    pub trait_modifiers: EmpireTraitModifiers,
    /// Ordered list of playstyle orientation tags.
    pub playstyle: &'static [PlaystyleTag],
    /// One-line summary of how the faction tends to play.
    pub playstyle_summary: &'static str,
    /// Deterministic diplomacy posture.
    pub diplomacy_profile: EmpireDiplomacyProfile,
    /// Deterministic military/economy modifiers.
    pub military_modifiers: EmpireMilitaryModifiers,
    /// Deterministic AI preference profile.
    pub ai_profile: EmpireAiProfile,
    /// Deterministic AI doctrine weights for scoring priorities.
    pub doctrine_weights: &'static [EmpireDoctrineWeight],
}

impl EmpireDefinition {
    /// Human-readable effect summaries for setup and diplomacy displays.
    pub fn effect_summaries(&self) -> Vec<String> {
        let mut effects = Vec::new();
        let mods = self.trait_modifiers;
        if mods.industry_per_colony != 0 {
            effects.push(format!("{:+} industry/colony", mods.industry_per_colony));
        }
        if mods.science_per_colony != 0 {
            effects.push(format!("{:+} science/colony", mods.science_per_colony));
        }
        if mods.credits_per_colony != 0 {
            effects.push(format!("{:+} credits/colony", mods.credits_per_colony));
        }
        if mods.food_per_colony != 0 {
            effects.push(format!("{:+} food/colony", mods.food_per_colony));
        }

        let military = self.military_modifiers;
        if military.scout_cost_modifier_pct != 0 {
            effects.push(format!(
                "{:+}% scout cost",
                military.scout_cost_modifier_pct
            ));
        }
        if military.science_ship_cost_modifier_pct != 0 {
            effects.push(format!(
                "{:+}% science ship cost",
                military.science_ship_cost_modifier_pct
            ));
        }
        if military.troop_transport_cost_modifier_pct != 0 {
            effects.push(format!(
                "{:+}% troop transport cost",
                military.troop_transport_cost_modifier_pct
            ));
        }
        if military.shipyard_cost_modifier_pct != 0 {
            effects.push(format!(
                "{:+}% shipyard cost",
                military.shipyard_cost_modifier_pct
            ));
        }
        if military.fleet_maintenance_modifier_per_fleet != 0 {
            effects.push(format!(
                "{:+} fleet maint/fleet",
                military.fleet_maintenance_modifier_per_fleet
            ));
        }
        if military.invasion_strength_bonus_per_transport != 0 {
            effects.push(format!(
                "+{} invasion/transport",
                military.invasion_strength_bonus_per_transport
            ));
        }
        if military.combat_ship_cost_modifier_pct != 0 {
            effects.push(format!(
                "{:+}% combat ship cost",
                military.combat_ship_cost_modifier_pct
            ));
        }

        if self.diplomacy_profile.first_contact_status != RelationshipStatus::Contacted {
            effects.push(format!(
                "First contact starts {}",
                self.diplomacy_profile.first_contact_status.label()
            ));
        }

        effects
    }

    pub fn doctrine_weight(&self, doctrine: AiDoctrine) -> u8 {
        self.doctrine_weights
            .iter()
            .find(|entry| entry.doctrine == doctrine)
            .map(|entry| entry.weight)
            .unwrap_or(0)
    }

    pub fn doctrine_short_summary(&self) -> String {
        let mut top: [Option<EmpireDoctrineWeight>; 3] = [None, None, None];
        for entry in self
            .doctrine_weights
            .iter()
            .copied()
            .filter(|e| e.weight > 0)
        {
            let mut insert_at = None;
            for (idx, slot) in top.iter().enumerate() {
                let outranks = match slot {
                    Some(existing) => {
                        entry.weight > existing.weight
                            || (entry.weight == existing.weight
                                && entry.doctrine.label() < existing.doctrine.label())
                    }
                    None => true,
                };
                if outranks {
                    insert_at = Some(idx);
                    break;
                }
            }
            if let Some(idx) = insert_at {
                for shift in (idx + 1..top.len()).rev() {
                    top[shift] = top[shift - 1];
                }
                top[idx] = Some(entry);
            }
        }

        let mut summary = String::new();
        for (idx, entry) in top.iter().flatten().enumerate() {
            if idx > 0 {
                summary.push('/');
            }
            summary.push_str(entry.doctrine.short_code());
            summary.push_str(&entry.weight.to_string());
        }
        summary
    }
}

/// All available empire definitions in stable ID order.
///
/// # Original IP
/// All names, descriptions, and symbols are original.  No content is derived
/// from Master of Orion or any other published 4X title.
pub fn all_empire_definitions() -> &'static [EmpireDefinition] {
    &EMPIRE_DEFINITIONS
}

/// Look up an empire definition by its ID.  Returns `None` if not found.
pub fn empire_definition_by_id(id: EmpireDefinitionId) -> Option<&'static EmpireDefinition> {
    EMPIRE_DEFINITIONS.iter().find(|d| d.id == id)
}

static EMPIRE_DEFINITIONS: [EmpireDefinition; 8] = [
    EmpireDefinition {
        id: EmpireDefinitionId(0),
        name: "Ashveran Compact",
        short_description: "A federation of heavy-industry worlds united by supply-chain treaties.",
        tone: "Pragmatic industrial coalition",
        symbol: '⚙',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 1,
            science_per_colony: 0,
            credits_per_colony: 0,
            food_per_colony: 0,
        },
        playstyle: &[PlaystyleTag::Industrial],
        playstyle_summary: "Reliable infrastructure empire with steady production and logistics.",
        diplomacy_profile: EmpireDiplomacyProfile::standard(),
        military_modifiers: EmpireMilitaryModifiers::none(),
        ai_profile: EmpireAiProfile::new(
            &[TechDomain::Engineering, TechDomain::Economy],
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            true, // prefers_defensive_ships: patrol corvettes for security
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Industrialist,
                weight: 9,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Isolationist,
                weight: 7,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(1),
        name: "Luminal Traverse",
        short_description: "Explorers driven by an obsession with mapping the unknown.",
        tone: "Restless pathfinders",
        symbol: '◎',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 0,
            science_per_colony: 1,
            credits_per_colony: 0,
            food_per_colony: 0,
        },
        playstyle: &[PlaystyleTag::Expansionist, PlaystyleTag::Scientific],
        playstyle_summary: "Fast early exploration with a research-led expansion curve.",
        diplomacy_profile: EmpireDiplomacyProfile::standard(),
        military_modifiers: EmpireMilitaryModifiers {
            scout_cost_modifier_pct: -10,
            ..EmpireMilitaryModifiers::none()
        },
        ai_profile: EmpireAiProfile::new(
            &[TechDomain::Exploration, TechDomain::Economy],
            false,
            false,
            false,
            false,
            true,
            false,
            false,
            false, // prefers_fast_scouts: pathfinder identity
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Explorer,
                weight: 9,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Merchant,
                weight: 7,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Expansionist,
                weight: 6,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(2),
        name: "Sylvaran Accord",
        short_description: "A biosphere-first collective that values growth and ecological balance.",
        tone: "Patient ecological stewards",
        symbol: '✿',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 0,
            science_per_colony: 0,
            credits_per_colony: 0,
            food_per_colony: 2,
        },
        playstyle: &[PlaystyleTag::Agrarian],
        playstyle_summary: "Food-rich colonies that favour long-term population growth.",
        diplomacy_profile: EmpireDiplomacyProfile {
            first_contact_status: RelationshipStatus::Neutral,
            resting_status: RelationshipStatus::Neutral,
            border_tension_status: RelationshipStatus::Tense,
            severe_border_tension_status: RelationshipStatus::Hostile,
        },
        military_modifiers: EmpireMilitaryModifiers::none(),
        ai_profile: EmpireAiProfile::new(
            &[TechDomain::Biology, TechDomain::Economy],
            false,
            false,
            true,
            false,
            false,
            true,
            false,
            false, // prefers_colony_arks: population-first doctrine
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Expansionist,
                weight: 9,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Biologist,
                weight: 9,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(3),
        name: "Thalori Exchange",
        short_description: "A merchant alliance that turned commerce into a form of galactic power.",
        tone: "Opportunistic commercial brokers",
        symbol: '◈',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 0,
            science_per_colony: 0,
            credits_per_colony: 2,
            food_per_colony: 0,
        },
        playstyle: &[PlaystyleTag::Diplomatic, PlaystyleTag::Industrial],
        playstyle_summary: "Credit-rich empire that prefers secure trade and measured growth.",
        diplomacy_profile: EmpireDiplomacyProfile {
            first_contact_status: RelationshipStatus::Neutral,
            resting_status: RelationshipStatus::Neutral,
            border_tension_status: RelationshipStatus::Tense,
            severe_border_tension_status: RelationshipStatus::Hostile,
        },
        military_modifiers: EmpireMilitaryModifiers::none(),
        ai_profile: EmpireAiProfile::new(
            &[TechDomain::Economy, TechDomain::Engineering],
            false,
            false,
            true,
            false,
            false,
            false,
            false,
            true, // prefers_defensive_ships: trade-lane security corvettes
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Merchant,
                weight: 10,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Expansionist,
                weight: 8,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(4),
        name: "Vorath Dominion",
        short_description: "A martial confederation bound by oaths of mutual defense and conquest.",
        tone: "Martial frontier hegemony",
        symbol: '⚔',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 0,
            science_per_colony: 0,
            credits_per_colony: 1,
            food_per_colony: 0,
        },
        playstyle: &[PlaystyleTag::Militarist],
        playstyle_summary: "Pressure-oriented power that turns frontier tension into war readiness.",
        diplomacy_profile: EmpireDiplomacyProfile {
            first_contact_status: RelationshipStatus::Tense,
            resting_status: RelationshipStatus::Tense,
            border_tension_status: RelationshipStatus::Hostile,
            severe_border_tension_status: RelationshipStatus::War,
        },
        military_modifiers: EmpireMilitaryModifiers {
            troop_transport_cost_modifier_pct: -10,
            invasion_strength_bonus_per_transport: 2,
            combat_ship_cost_modifier_pct: -10,
            ..EmpireMilitaryModifiers::none()
        },
        ai_profile: EmpireAiProfile::new(
            &[TechDomain::Military, TechDomain::Engineering],
            false,
            true,
            false,
            true,
            false,
            false,
            true,
            false, // prefers_combat_ships: militarist warfleet doctrine
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Militarist,
                weight: 10,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Imperial,
                weight: 8,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(5),
        name: "Elarith Confluence",
        short_description: "A technocratic council that views scientific advancement as the highest law.",
        tone: "Measured technocracy",
        symbol: '⟁',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 0,
            science_per_colony: 2,
            credits_per_colony: 0,
            food_per_colony: 0,
        },
        playstyle: &[PlaystyleTag::Scientific],
        playstyle_summary: "Pure research specialists that convert safe worlds into laboratories.",
        diplomacy_profile: EmpireDiplomacyProfile::standard(),
        military_modifiers: EmpireMilitaryModifiers {
            science_ship_cost_modifier_pct: -10,
            ..EmpireMilitaryModifiers::none()
        },
        ai_profile: EmpireAiProfile::new(
            &[
                TechDomain::Exploration,
                TechDomain::Biology,
                TechDomain::Economy,
            ],
            true,
            false,
            true,
            false,
            true,
            false,
            false,
            false, // prefers_fast_scouts + prefers_science_ships
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Industrialist,
                weight: 8,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Technologist,
                weight: 10,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Isolationist,
                weight: 8,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(6),
        name: "Terran Concord",
        short_description: "An open Terran union that treats science, dialogue, and exploration as shared civic duties.",
        tone: "Optimistic, pluralist, science-forward federation",
        symbol: '☼',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: -1,
            science_per_colony: 1,
            credits_per_colony: 0,
            food_per_colony: 0,
        },
        playstyle: &[
            PlaystyleTag::Diplomatic,
            PlaystyleTag::Scientific,
            PlaystyleTag::Expansionist,
        ],
        playstyle_summary: "Cooperative explorers that open with better relations, lean into research, and keep colonies stable before committing to war.",
        diplomacy_profile: EmpireDiplomacyProfile {
            first_contact_status: RelationshipStatus::Neutral,
            resting_status: RelationshipStatus::Neutral,
            border_tension_status: RelationshipStatus::Tense,
            severe_border_tension_status: RelationshipStatus::Hostile,
        },
        military_modifiers: EmpireMilitaryModifiers {
            scout_cost_modifier_pct: -20,
            science_ship_cost_modifier_pct: -20,
            ..EmpireMilitaryModifiers::none()
        },
        ai_profile: EmpireAiProfile::new(
            &[
                TechDomain::Exploration,
                TechDomain::Economy,
                TechDomain::Biology,
            ],
            true,
            false,
            true,
            false,
            true,
            false,
            false,
            false, // prefers_fast_scouts: exploration mandate
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Explorer,
                weight: 10,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Technologist,
                weight: 9,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Merchant,
                weight: 7,
            },
        ],
    },
    EmpireDefinition {
        id: EmpireDefinitionId(7),
        name: "Terran Dominion",
        short_description: "A hardline Terran hierarchy that secures frontier order through rapid militarisation and coercive expansion.",
        tone: "Authoritarian, expansionist, order-through-force empire",
        symbol: '▲',
        trait_modifiers: EmpireTraitModifiers {
            industry_per_colony: 1,
            science_per_colony: 0,
            credits_per_colony: 0,
            food_per_colony: 0,
        },
        playstyle: &[
            PlaystyleTag::Militarist,
            PlaystyleTag::Industrial,
            PlaystyleTag::Expansionist,
        ],
        playstyle_summary: "Militarised colonisers that accept worse first contact, cheaper war logistics, and faster escalation when borders tighten.",
        diplomacy_profile: EmpireDiplomacyProfile {
            first_contact_status: RelationshipStatus::Tense,
            resting_status: RelationshipStatus::Tense,
            border_tension_status: RelationshipStatus::Hostile,
            severe_border_tension_status: RelationshipStatus::War,
        },
        military_modifiers: EmpireMilitaryModifiers {
            troop_transport_cost_modifier_pct: -20,
            shipyard_cost_modifier_pct: -10,
            fleet_maintenance_modifier_per_fleet: -1,
            invasion_strength_bonus_per_transport: 4,
            combat_ship_cost_modifier_pct: -15,
            ..EmpireMilitaryModifiers::none()
        },
        ai_profile: EmpireAiProfile::new(
            &[
                TechDomain::Military,
                TechDomain::Engineering,
                TechDomain::Exploration,
            ],
            false,
            true,
            false,
            true,
            false,
            true,
            true,
            false, // prefers_colony_arks + prefers_combat_ships
        ),
        doctrine_weights: &[
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Imperial,
                weight: 10,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Militarist,
                weight: 9,
            },
            EmpireDoctrineWeight {
                doctrine: AiDoctrine::Industrialist,
                weight: 8,
            },
        ],
    },
];
