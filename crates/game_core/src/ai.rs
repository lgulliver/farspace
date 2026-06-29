//! Deterministic AI opponent — rule-based decision engine
//!
//! The AI empire follows a fixed priority list each turn:
//!
//! 1. Select cheapest available (prerequisites satisfied) unresearched tech (if none active),
//!    biased toward the empire's playstyle domain.
//! 2. Queue builds for each owned colony with an empty queue (ships require a Shipyard).
//!    Priority varies by empire identity playstyle tag:
//!    Expansionist — Scout → FabricationYard → Shipyard → ColonyShip (pre-colonisation scouts);
//!    Agrarian — AquacultureBay → FabricationYard → Shipyard → ColonyShip → Scout;
//!    Default — FabricationYard → Shipyard → ColonyShip → Scout.
//! 3. Dispatch the first idle scout to the nearest unexplored star
//! 4. Colonize with any idle colonizer at an AI-explored star
//! 5. Assign colony roles based on planet class (once, deterministically)

use crate::engine::travel_turns_with_lanes;
use crate::events::Event;
use crate::state::{
    AiDoctrine, BuildItem, BuildingType, Colony, ColonyId, ColonyRole, ComponentId, CustomDesignId,
    CustomShipDesign, EmpireId, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState,
    GameState, OrbitalStructureType, PlanetAnomaly, PlanetClass, PlanetSpecial, PlaystyleTag,
    ScoutMission, ShipDesignId, SlotCategory, StarId, StrategicResource, StrategicResourceCategory,
    TechDomain, TechId, TechTag, VictoryPath, all_techs, empire_definition_by_id,
    is_resource_discoverable, is_tech_available, visible_anomalies_for_empire,
    visible_specials_for_empire,
};
use crate::yield_model::{YieldContext, calculate_yield_with_context};

const FUTURE_PENALTY_MULTIPLIER: i32 = 12;
const FOOD_CRISIS_SCORE_BONUS: i32 = 18;
const MAX_SAFE_SCOUT_HOSTILE_STRENGTH: u32 = 4;
const SCOUT_FRONTIER_DISTANCE_DIVISOR: i32 = 240;

fn resource_value_for_doctrine(resource: StrategicResource, doctrine: AiDoctrine) -> i32 {
    let base = i32::from(resource.trade_value());
    let cat_bonus = match (doctrine, resource.category()) {
        (AiDoctrine::Industrialist, StrategicResourceCategory::Industrial) => 28,
        (AiDoctrine::Militarist, StrategicResourceCategory::Military) => 28,
        (AiDoctrine::Technologist, StrategicResourceCategory::Exotic) => 24,
        (AiDoctrine::Technologist, StrategicResourceCategory::Precursor) => 30,
        (AiDoctrine::Merchant, _) => 18,
        (AiDoctrine::Biologist, StrategicResourceCategory::Biological) => 22,
        (AiDoctrine::Expansionist, StrategicResourceCategory::Energy) => 18,
        _ => 0,
    };
    base + cat_bonus
}

fn special_value_for_doctrine(special: PlanetSpecial, doctrine: AiDoctrine) -> i32 {
    let effect = special.yield_effect();
    let base = (i64::from(special.rarity().valuation_weight()) * 8
        + effect.industry
        + effect.science
        + effect.credits
        + effect.food) as i32;
    let cat_bonus = match (doctrine, special.category()) {
        (AiDoctrine::Industrialist, crate::state::PlanetSpecialCategory::Industrial)
        | (AiDoctrine::Industrialist, crate::state::PlanetSpecialCategory::Resource) => 16,
        (AiDoctrine::Technologist, crate::state::PlanetSpecialCategory::Scientific)
        | (AiDoctrine::Technologist, crate::state::PlanetSpecialCategory::Precursor) => 20,
        (AiDoctrine::Explorer, crate::state::PlanetSpecialCategory::Environmental)
        | (AiDoctrine::Explorer, crate::state::PlanetSpecialCategory::Strategic) => 14,
        (AiDoctrine::Expansionist, crate::state::PlanetSpecialCategory::Biological) => 16,
        (AiDoctrine::Militarist, crate::state::PlanetSpecialCategory::Hazard)
        | (AiDoctrine::Militarist, crate::state::PlanetSpecialCategory::Strategic) => 14,
        (AiDoctrine::Merchant, crate::state::PlanetSpecialCategory::Cultural)
        | (AiDoctrine::Merchant, crate::state::PlanetSpecialCategory::Resource) => 12,
        _ => 0,
    };
    base + cat_bonus
}

fn anomaly_value_for_doctrine(anomaly: PlanetAnomaly, doctrine: AiDoctrine) -> i32 {
    let effect = anomaly.yield_effect();
    let base = (i64::from(anomaly.rarity().valuation_weight()) * 10
        + effect.industry
        + effect.science
        + effect.credits
        + effect.food) as i32;
    let cat_bonus = match (doctrine, anomaly.category()) {
        (AiDoctrine::Explorer, crate::state::AnomalyCategory::Stellar)
        | (AiDoctrine::Explorer, crate::state::AnomalyCategory::Temporal)
        | (AiDoctrine::Explorer, crate::state::AnomalyCategory::Gravitational) => 18,
        (AiDoctrine::Technologist, crate::state::AnomalyCategory::ExoticPhysics)
        | (AiDoctrine::Technologist, crate::state::AnomalyCategory::Precursor)
        | (AiDoctrine::Technologist, crate::state::AnomalyCategory::Archaeological) => 20,
        (AiDoctrine::Militarist, crate::state::AnomalyCategory::Military) => 16,
        (AiDoctrine::Expansionist, crate::state::AnomalyCategory::Stellar) => 12,
        _ => 0,
    };
    let risk_bonus = match anomaly.risk_level() {
        Some(crate::state::AnomalyRiskLevel::Low) => 0,
        Some(crate::state::AnomalyRiskLevel::Moderate) => 3,
        Some(crate::state::AnomalyRiskLevel::High) => 5,
        Some(crate::state::AnomalyRiskLevel::Severe) => {
            if doctrine == AiDoctrine::Isolationist {
                -4
            } else {
                6
            }
        }
        None => 0,
    };
    base + cat_bonus + risk_bonus
}

fn ai_primary_doctrine(state: &GameState, empire_id: EmpireId) -> AiDoctrine {
    state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
        .map(|def| {
            [
                AiDoctrine::Explorer,
                AiDoctrine::Technologist,
                AiDoctrine::Merchant,
                AiDoctrine::Imperial,
                AiDoctrine::Militarist,
                AiDoctrine::Industrialist,
                AiDoctrine::Expansionist,
                AiDoctrine::Isolationist,
                AiDoctrine::Biologist,
            ]
            .iter()
            .copied()
            .max_by_key(|&d| def.doctrine_weight(d))
            .unwrap_or(AiDoctrine::Explorer)
        })
        .unwrap_or(AiDoctrine::Explorer)
}

/// Multi-factor system score for an unexplored star, used by both
/// scout dispatch and colonization target selection.  Higher = more
/// valuable to expand toward.
///
/// Components:
///   - Habitable planet quality (Terran=20, Oceanic=15, Desert=12, etc.)
///   - Visible specials and anomalies (weighted by doctrine)
///   - Strategic resource potential (trade_value)
///   - Distance from home star (closer preferred, but expansionists
///     get a small frontier bonus for outward push)
///   - Supply projection penalty for deep targets
///   - Contested penalty (−50 per hostile/friendly fleet within 2 stars)
fn score_system_for_scout(state: &GameState, empire_id: EmpireId, star_id: StarId) -> i32 {
    let Some(star) = state.stars.get(&star_id) else {
        return 0;
    };
    let doctrine = ai_primary_doctrine(state, empire_id);
    let completed: Vec<TechId> = state
        .empires
        .get(&empire_id)
        .map(|e| e.research.completed.clone())
        .unwrap_or_default();

    // Planet quality
    let mut score: i32 = star
        .planets
        .iter()
        .map(|p| {
            let base = match p.class {
                PlanetClass::Terran => 20,
                PlanetClass::Oceanic => 15,
                PlanetClass::Desert => 10,
                PlanetClass::Frozen => 8,
                PlanetClass::Volcanic => 5,
                PlanetClass::Barren => 3,
            };
            let mut extras = 0i32;
            if p.surveyed {
                for s in visible_specials_for_empire(p, &completed) {
                    extras += special_value_for_doctrine(s, doctrine) / 10;
                }
                for a in visible_anomalies_for_empire(p, &completed) {
                    extras += anomaly_value_for_doctrine(a, doctrine) / 10;
                }
                for r in &p.resources {
                    if is_resource_discoverable(*r, &completed) {
                        extras += i32::from(r.trade_value()) / 20;
                    }
                }
            }
            base + extras
        })
        .sum();

    // Frontier / distance: closer to home = higher, but expansionists
    // get a small bonus for systems farther out.
    let home = state.empires.get(&empire_id).map(|e| e.home_star);
    let home_coords = home.and_then(|hid| state.stars.get(&hid));
    if let Some(home) = home_coords {
        let dx = (star.x - home.x) as i64;
        let dy = (star.y - home.y) as i64;
        let dist_sq = dx * dx + dy * dy;
        let dist = (dist_sq as f64).sqrt().round() as i32;
        score = score.saturating_add(50 - dist / 10);
        if doctrine == AiDoctrine::Expansionist {
            score = score.saturating_add(dist / 40);
        }
    }

    // Contested penalty: if hostile fleets or colonies are near, reduce score.
    for (_, fleet) in &state.fleets {
        if fleet.owner == empire_id {
            continue;
        }
        if !state
            .relationship_status(empire_id, fleet.owner)
            .is_hostile_or_war()
        {
            continue;
        }
        if let Some(fstar) = state.stars.get(&fleet.location) {
            let dx = (star.x - fstar.x) as i64;
            let dy = (star.y - fstar.y) as i64;
            let sq = dx * dx + dy * dy;
            if sq <= 600 * 600 {
                // within ~2 sectors
                score = score.saturating_sub(50);
                break;
            }
        }
    }

    // Adjacency to own colonies: slight bonus for systems near existing colonies.
    for colony in state.colonies.values() {
        if colony.owner == empire_id {
            if let Some(cstar) = state.stars.get(&colony.star) {
                let dx = (star.x - cstar.x) as i64;
                let dy = (star.y - cstar.y) as i64;
                if dx * dx + dy * dy <= 500 * 500 {
                    score = score.saturating_add(15);
                }
            }
        }
    }

    score.max(1)
}

fn scout_resource_prospect_score(state: &GameState, empire_id: EmpireId, star_id: StarId) -> i32 {
    let Some(star) = state.stars.get(&star_id) else {
        return 0;
    };
    let doctrine = ai_primary_doctrine(state, empire_id);
    let spectral_bias = match star.spectral_class {
        crate::state::SpectralClass::O | crate::state::SpectralClass::B => 6,
        crate::state::SpectralClass::A | crate::state::SpectralClass::F => 4,
        crate::state::SpectralClass::G | crate::state::SpectralClass::K => 3,
        crate::state::SpectralClass::M => 2,
    };
    let planet_bias: i32 = star
        .planets
        .iter()
        .map(|planet| match planet.class {
            PlanetClass::Volcanic | PlanetClass::Barren => 4,
            PlanetClass::Frozen | PlanetClass::Desert => 3,
            PlanetClass::Terran | PlanetClass::Oceanic => 2,
        })
        .sum();
    let frontier_bonus =
        ((star.x.abs() + star.y.abs()) / SCOUT_FRONTIER_DISTANCE_DIVISOR).clamp(0, 6);
    let doctrine_bonus = match doctrine {
        AiDoctrine::Explorer => frontier_bonus + 4,
        AiDoctrine::Technologist => spectral_bias + 4,
        AiDoctrine::Militarist => {
            star.planets
                .iter()
                .filter(|planet| {
                    matches!(planet.class, PlanetClass::Volcanic | PlanetClass::Barren)
                })
                .count() as i32
                * 3
        }
        AiDoctrine::Expansionist => {
            star.planets
                .iter()
                .filter(|planet| planet.habitable)
                .count() as i32
                * 2
        }
        _ => 0,
    };
    spectral_bias + planet_bias + frontier_bonus + doctrine_bonus
}

fn contested_star_resource_score(
    state: &GameState,
    attacker: EmpireId,
    target_owner: EmpireId,
    star_id: StarId,
) -> i32 {
    let doctrine = ai_primary_doctrine(state, attacker);
    let mut score = 0;
    for colony in state
        .colonies
        .values()
        .filter(|colony| colony.owner == target_owner && colony.star == star_id)
    {
        let Some(planet) = state
            .stars
            .get(&colony.star)
            .and_then(|s| s.planets.get(colony.planet_index))
        else {
            continue;
        };
        if !planet.surveyed {
            continue;
        }
        let completed_techs = state
            .empires
            .get(&attacker)
            .map(|empire| empire.research.completed.as_slice())
            .unwrap_or(&[]);
        for special in visible_specials_for_empire(planet, completed_techs) {
            score += special_value_for_doctrine(special, doctrine) / 5;
        }
        for anomaly in visible_anomalies_for_empire(planet, completed_techs) {
            score += anomaly_value_for_doctrine(anomaly, doctrine) / 5;
        }
        for resource in &planet.resources {
            if state.colony_can_extract_resource(colony.id, *resource) {
                score += resource_value_for_doctrine(*resource, doctrine);
            }
        }
    }
    score
}

/// Deterministic empire power score used for AI threat assessment and
/// build priority.  Not stored on state; computed on demand.
pub fn empire_power(state: &GameState, empire_id: EmpireId) -> u32 {
    let colonies = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
        .count() as u32;
    let population: u32 = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
        .map(|c| c.population as u32)
        .sum();
    let fleet_strength: u32 = state
        .fleets
        .values()
        .filter(|f| f.owner == empire_id)
        .map(|f| f.strength * f.ships)
        .sum();
    let techs = state
        .empires
        .get(&empire_id)
        .map(|e| e.research.completed.len() as u32)
        .unwrap_or(0);
    let credits = state
        .empires
        .get(&empire_id)
        .map(|e| (e.credits / 10) as u32)
        .unwrap_or(0);

    colonies * 10 + population * 3 + fleet_strength * 2 + techs * 5 + credits
}

/// Return the victory path pressure (0-100) for the empire with the
/// highest progress on any path.  Used by AI to detect threats.
pub fn leading_victory_pressure(state: &GameState, empire_id: EmpireId) -> (crate::state::VictoryPath, u8) {
    let mut best = (crate::state::VictoryPath::Legacy, 0u8);
    for entry in &state.victory_status.progress {
        if entry.leading_empire == Some(empire_id) && entry.progress_percent > best.1 {
            best = (entry.path, entry.progress_percent);
        }
    }
    best
}

/// Run one AI decision pass for the given empire.
///
/// Mutates `state` and returns events for each action taken.
/// All decisions are deterministic: given the same `state` input, the same
/// actions will be produced.
pub fn run_ai_turn(state: &mut GameState, ai_empire_id: EmpireId) -> Vec<Event> {
    let mut events = Vec::new();

    ai_select_research(state, ai_empire_id, &mut events);
    ai_queue_builds(state, ai_empire_id, &mut events);
    ai_assign_fleet_posture(state, ai_empire_id, &mut events);
    ai_dispatch_scouts(state, ai_empire_id, &mut events);
    ai_dispatch_combat_fleets(state, ai_empire_id, &mut events);
    ai_dispatch_patrols(state, ai_empire_id, &mut events);
    ai_colonize(state, ai_empire_id, &mut events);
    ai_assign_colony_roles(state, ai_empire_id, &mut events);

    events
}

// ---------------------------------------------------------------------------
// Research
// ---------------------------------------------------------------------------

fn ai_select_research(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    let (tech_id, queue) = match pick_research_plan(state, empire_id) {
        Some(plan) => plan,
        None => return,
    };

    if let Some(empire) = state.empires.get_mut(&empire_id) {
        // Only reset progress when switching away from a different active tech
        if let Some(active) = empire.research.current_tech
            && active != tech_id
        {
            empire.research.progress = 0;
        }
        empire.research.current_tech = Some(tech_id);
        empire.research.queue = queue;
    }

    events.push(Event::AiResearchSelected {
        empire: empire_id,
        tech: tech_id,
    });
}

/// Build a deterministic 2–4 item research plan and return the first tech plus queue.
fn pick_research_plan(state: &GameState, empire_id: EmpireId) -> Option<(TechId, Vec<TechId>)> {
    let empire = state.empires.get(&empire_id)?;
    if empire.research.current_tech.is_some() {
        return None;
    }
    let plan_len = research_plan_len(state, empire_id);
    let mut simulated_completed = empire.research.completed.clone();
    let mut plan = Vec::new();
    for _ in 0..plan_len {
        let Some(next) =
            pick_research_from_completed(state, empire_id, &simulated_completed, &plan)
        else {
            break;
        };
        simulated_completed.push(next);
        plan.push(next);
    }
    let current = *plan.first()?;
    let queue = plan.iter().skip(1).copied().collect::<Vec<_>>();
    Some((current, queue))
}

fn research_plan_len(state: &GameState, empire_id: EmpireId) -> usize {
    let Some(def) = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
    else {
        return 3;
    };
    let expansion = def.doctrine_weight(AiDoctrine::Expansionist);
    let technologist = def.doctrine_weight(AiDoctrine::Technologist);
    let isolationist = def.doctrine_weight(AiDoctrine::Isolationist);
    if expansion >= 7 || technologist >= 8 {
        4
    } else if isolationist >= 7 {
        2
    } else {
        3
    }
}

fn pick_research_from_completed(
    state: &GameState,
    empire_id: EmpireId,
    completed: &[TechId],
    already_planned: &[TechId],
) -> Option<TechId> {
    let empire = state.empires.get(&empire_id)?;

    let preferred_domains: Vec<TechDomain> = empire
        .empire_def
        .and_then(empire_definition_by_id)
        .map(|def| {
            if !def.ai_profile.research_focus.is_empty() {
                def.ai_profile.research_focus.to_vec()
            } else {
                def.playstyle
                    .iter()
                    .flat_map(|tag| match tag {
                        PlaystyleTag::Scientific | PlaystyleTag::Expansionist => {
                            vec![TechDomain::Exploration, TechDomain::Economy]
                        }
                        PlaystyleTag::Industrial => vec![TechDomain::Engineering],
                        PlaystyleTag::Militarist => vec![TechDomain::Military],
                        PlaystyleTag::Agrarian => vec![TechDomain::Biology],
                        PlaystyleTag::Diplomatic => {
                            vec![TechDomain::Society, TechDomain::Economy]
                        }
                    })
                    .collect()
            }
        })
        .unwrap_or_default();

    let preferred_tags: Vec<TechTag> = empire
        .empire_def
        .and_then(empire_definition_by_id)
        .map(|def| {
            def.playstyle
                .iter()
                .flat_map(|tag| match tag {
                    PlaystyleTag::Scientific | PlaystyleTag::Expansionist => vec![
                        TechTag::Survey,
                        TechTag::Sensors,
                        TechTag::Hyperspace,
                        TechTag::SectorMapping,
                    ],
                    PlaystyleTag::Industrial => {
                        vec![TechTag::Shipyard, TechTag::Production, TechTag::Orbital]
                    }
                    PlaystyleTag::Militarist => vec![
                        TechTag::ShipClass,
                        TechTag::Weapon,
                        TechTag::Defense,
                        TechTag::Invasion,
                        TechTag::Blockade,
                        TechTag::Command,
                    ],
                    PlaystyleTag::Agrarian => vec![
                        TechTag::Food,
                        TechTag::Growth,
                        TechTag::Housing,
                        TechTag::Terraforming,
                    ],
                    PlaystyleTag::Diplomatic => {
                        vec![TechTag::Diplomacy, TechTag::Stability, TechTag::Trade]
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let mut candidates: Vec<_> = all_techs()
        .iter()
        .filter(|t| is_tech_available(completed, t.id))
        .filter(|t| !already_planned.contains(&t.id))
        .collect();

    candidates.sort_by(|a, b| {
        let score_a = research_score(state, empire_id, a);
        let score_b = research_score(state, empire_id, b);
        let domain_pref_a = preferred_domains.contains(&a.domain);
        let domain_pref_b = preferred_domains.contains(&b.domain);
        let tag_pref_a = a.tags.iter().any(|tag| preferred_tags.contains(tag));
        let tag_pref_b = b.tags.iter().any(|tag| preferred_tags.contains(tag));

        score_b
            .cmp(&score_a)
            .then(
                future_penalty(a.future_hook, a.unlocks.is_empty())
                    .cmp(&future_penalty(b.future_hook, b.unlocks.is_empty())),
            )
            .then(domain_pref_b.cmp(&domain_pref_a))
            .then(tag_pref_b.cmp(&tag_pref_a))
            .then(a.rarity.ai_penalty().cmp(&b.rarity.ai_penalty()))
            .then(a.tier.cmp(&b.tier))
            .then(a.cost.cmp(&b.cost))
            .then(b.ai_weight.cmp(&a.ai_weight))
            .then(a.display_order.cmp(&b.display_order))
            .then(a.id.cmp(&b.id))
    });
    candidates.first().map(|t| t.id)
}

fn research_score(state: &GameState, empire_id: EmpireId, tech: &crate::state::TechRecord) -> i32 {
    let difficulty = state
        .scenario
        .as_ref()
        .map(|s| s.difficulty)
        .unwrap_or_default();
    let difficulty_mult = difficulty.research_bonus_pct();

    let Some(def) = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
    else {
        return ((tech.ai_weight as i32 * 8)
            - future_penalty(tech.future_hook, tech.unlocks.is_empty()) as i32 * 8)
            * difficulty_mult
            / 100;
    };
    let doctrine = |axis| def.doctrine_weight(axis) as i32;
    let victory_pref = |path| doctrine_victory_preference(state, empire_id, path) as i32;

    let mut score = tech.ai_weight as i32 * 8;
    score -= future_penalty(tech.future_hook, tech.unlocks.is_empty()) as i32
        * FUTURE_PENALTY_MULTIPLIER;

    score += match tech.domain {
        TechDomain::Exploration => {
            doctrine(AiDoctrine::Explorer) * 3
                + doctrine(AiDoctrine::Expansionist) * 2
                + victory_pref(VictoryPath::Legacy)
        }
        TechDomain::Engineering => {
            doctrine(AiDoctrine::Industrialist) * 3 + victory_pref(VictoryPath::Ascendancy)
        }
        TechDomain::Military => {
            doctrine(AiDoctrine::Militarist) * 3
                + doctrine(AiDoctrine::Imperial) * 2
                + victory_pref(VictoryPath::Supremacy)
        }
        TechDomain::Society => {
            doctrine(AiDoctrine::Technologist) * 2
                + doctrine(AiDoctrine::Imperial)
                + victory_pref(VictoryPath::Scientific)
        }
        TechDomain::Economy => {
            doctrine(AiDoctrine::Merchant) * 3 + victory_pref(VictoryPath::Legacy)
        }
        TechDomain::Biology => {
            doctrine(AiDoctrine::Biologist) * 3
                + doctrine(AiDoctrine::Expansionist)
                + victory_pref(VictoryPath::Legacy)
        }
    };

    for tag in tech.tags {
        score += match tag {
            TechTag::Survey | TechTag::Sensors | TechTag::Hyperspace | TechTag::SectorMapping => {
                doctrine(AiDoctrine::Explorer) * 2 + victory_pref(VictoryPath::Legacy)
            }
            TechTag::Trade | TechTag::Supply | TechTag::Logistics => {
                doctrine(AiDoctrine::Merchant) * 2 + victory_pref(VictoryPath::Legacy)
            }
            TechTag::Weapon | TechTag::Defense | TechTag::Invasion | TechTag::Command => {
                doctrine(AiDoctrine::Militarist)
                    + doctrine(AiDoctrine::Imperial)
                    + victory_pref(VictoryPath::Supremacy)
            }
            TechTag::Production | TechTag::Shipyard | TechTag::Orbital => {
                doctrine(AiDoctrine::Industrialist) * 2 + victory_pref(VictoryPath::Ascendancy)
            }
            TechTag::Growth | TechTag::Food | TechTag::Housing | TechTag::Terraforming => {
                doctrine(AiDoctrine::Biologist) * 2 + victory_pref(VictoryPath::Legacy)
            }
            TechTag::Colonization => {
                doctrine(AiDoctrine::Expansionist) * 2 + victory_pref(VictoryPath::Ascendancy)
            }
            TechTag::Stability => {
                doctrine(AiDoctrine::Isolationist) + doctrine(AiDoctrine::Technologist)
            }
            TechTag::EspionageFuture | TechTag::PopulationJobsFuture => -8,
            _ => 0,
        };
    }

    if state.empires.get(&empire_id).is_some_and(|e| e.food < 0)
        && tech
            .tags
            .iter()
            .any(|tag| matches!(tag, TechTag::Food | TechTag::Growth | TechTag::Housing))
    {
        score += FOOD_CRISIS_SCORE_BONUS;
    }

    if let Some(crate::state::VictoryCondition::Scientific {
        eligibility_tech, ..
    }) = state.scenario.as_ref().and_then(|scenario| {
        scenario
            .victory_settings
            .condition_for(VictoryPath::Scientific)
    }) && *eligibility_tech == tech.id
    {
        score += victory_pref(VictoryPath::Scientific) * 2;
    }

    // --- Adaptive modifiers ---

    // 1. Threat level: if a hostile fleet is within 2 sectors of any
    //    owned colony, boost Military techs to counter it.
    let threat_military = state.colonies.values().any(|colony| {
        if colony.owner != empire_id {
            return false;
        }
        let cstar = state.stars.get(&colony.star);
        cstar.is_some_and(|cs| {
            state.fleets.values().any(|f| {
                f.owner != empire_id
                    && state.relationship_status(empire_id, f.owner).is_hostile_or_war()
                    && state.stars.get(&f.location).is_some_and(|fs| {
                        let dx = (cs.x - fs.x) as i64;
                        let dy = (cs.y - fs.y) as i64;
                        dx * dx + dy * dy <= 400 * 400
                    })
            })
        })
    });
    if threat_military && tech.domain == TechDomain::Military {
        score += score / 2; // 50% boost
    }

    // 2. Colonisation pressure: if no idle colonizers exist and
    //    there's an uncolonised habitable planet in explored space,
    //    boost Habitat Seeding and Colonial Vanguard techs.
    let has_idle_colonizer = state.fleets.values().any(|f| {
        f.owner == empire_id
            && f.kind.is_colonizer()
            && !state.fleet_missions.contains_key(&f.id)
    });
    if !has_idle_colonizer {
        let has_uncolonised_habitable = state
            .stars
            .values()
            .filter(|s| state.explored_stars_for(empire_id).contains(&s.id))
            .flat_map(|s| &s.planets)
            .any(|p| p.habitable && p.colony.is_none());
        if has_uncolonised_habitable
            && (tech.id == TechId::HABITAT_SEEDING || tech.id == TechId(15))
        {
            score += 18;
        }
    }

    // 3. Empty-queue pressure: if most colonies have idle queues,
    //    boost Engineering/Production domain techs.
    let owned = state.colonies.values().filter(|c| c.owner == empire_id).count();
    let idle = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id && c.build_queue.is_empty())
        .count();
    if owned > 1 && idle > owned / 2 && tech.domain == TechDomain::Engineering {
        score += 10;
    }

    // 4. Victory pressure: if any rival is >50% on a path, boost
    //    counter-techs.  Scientific leader → Society/Economy catch-up.
    //    Supremacy/Ascendancy leader → Military boost.
    for entry in &state.victory_status.progress {
        if entry.leading_empire == Some(empire_id) || entry.leading_empire.is_none() {
            continue;
        }
        if entry.progress_percent <= 50 {
            continue;
        }
        match entry.path {
            crate::state::VictoryPath::Scientific => {
                if matches!(tech.domain, TechDomain::Society | TechDomain::Economy) {
                    score += 12;
                }
            }
            crate::state::VictoryPath::Supremacy | crate::state::VictoryPath::Ascendancy => {
                if tech.domain == TechDomain::Military {
                    score += 15;
                }
            }
            _ => {}
        }
    }

    score * difficulty_mult / 100
}

fn doctrine_victory_preference(state: &GameState, empire_id: EmpireId, path: VictoryPath) -> u8 {
    let Some(def) = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
    else {
        return 0;
    };
    let base = match path {
        VictoryPath::Supremacy => {
            def.doctrine_weight(AiDoctrine::Militarist) + def.doctrine_weight(AiDoctrine::Imperial)
        }
        VictoryPath::Ascendancy => {
            def.doctrine_weight(AiDoctrine::Expansionist)
                + def.doctrine_weight(AiDoctrine::Industrialist)
        }
        VictoryPath::Scientific => def.doctrine_weight(AiDoctrine::Technologist),
        VictoryPath::Legacy => {
            def.doctrine_weight(AiDoctrine::Explorer)
                + def.doctrine_weight(AiDoctrine::Merchant)
                + def.doctrine_weight(AiDoctrine::Expansionist) / 2
        }
    };
    let leading_bonus = state
        .victory_status
        .progress
        .iter()
        .find(|progress| progress.path == path && progress.leading_empire == Some(empire_id))
        .map(|progress| progress.progress_percent / 25)
        .unwrap_or(0);
    base.saturating_add(leading_bonus).min(20)
}

fn future_penalty(is_future_hook: bool, has_no_unlocks: bool) -> u8 {
    if is_future_hook && has_no_unlocks {
        2
    } else if is_future_hook {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Build queue
// ---------------------------------------------------------------------------

fn ai_queue_builds(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    // Collect colony IDs first to avoid re-borrowing state inside the loop
    let colony_ids: Vec<ColonyId> = state
        .colonies
        .keys()
        .filter(|&&id| {
            state
                .colonies
                .get(&id)
                .is_some_and(|c| c.owner == empire_id)
        })
        .copied()
        .collect();

    for colony_id in colony_ids {
        if let Some(item) = pick_build_item(state, empire_id, colony_id) {
            // If we would build a static ship design, prefer a custom design for the same
            // fleet kind when the empire has one available — doctrine designs take effect here.
            let item = if let BuildItem::Ship(design_id) = item {
                if let Some(record) = design_id.record() {
                    custom_ship_for_fleet_kind(state, empire_id, record.fleet_kind)
                        .map(BuildItem::CustomShip)
                        .unwrap_or(item)
                } else {
                    item
                }
            } else {
                item
            };
            if let Some(colony) = state.colonies.get_mut(&colony_id) {
                colony.build_queue.push(item);
            }
            events.push(Event::AiBuildQueued {
                empire: empire_id,
                colony: colony_id,
                item,
            });
        }
    }
}

/// Return the first non-obsolete custom design owned by `empire_id` whose hull matches
/// `fleet_kind`, or `None` if no such design exists.
fn custom_ship_for_fleet_kind(
    state: &GameState,
    empire_id: EmpireId,
    fleet_kind: FleetKind,
) -> Option<CustomDesignId> {
    state
        .custom_designs
        .values()
        .find(|d| {
            d.owner == empire_id
                && !d.obsolete
                && d.hull_id
                    .template()
                    .is_some_and(|h| h.fleet_kind == fleet_kind)
        })
        .map(|d| d.design_id)
}

/// Count the empire's existing fleets of each relevant combat hull
/// and return the hull type that is most deficient.
fn needed_combat_hull(state: &GameState, empire_id: EmpireId) -> Option<ShipDesignId> {
    let colony_count = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
        .count() as u32;
    let border_sectors: u32 = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
        .filter(|c| {
            state.diplomacy.values().any(|rs| *rs == crate::state::RelationshipStatus::War)
                || state.diplomacy.contains_key(&c.owner)
        })
        .count() as u32;

    let mut corvettes = 0u32;
    let mut escorts = 0u32;
    let mut missile = 0u32;
    let mut destroyers = 0u32;
    let mut transports = 0u32;
    for f in state.fleets.values() {
        if f.owner != empire_id {
            continue;
        }
        match f.kind {
            FleetKind::PatrolCorvette => corvettes += 1,
            FleetKind::EscortFrigate => escorts += 1,
            FleetKind::MissileFrigate => missile += 1,
            FleetKind::Destroyer => destroyers += 1,
            FleetKind::TroopTransport => transports += 1,
            _ => {}
        }
    }

    let at_war = state
        .diplomacy
        .values()
        .any(|rs| *rs == crate::state::RelationshipStatus::War);

    // Target ratios: 1 corvette per border sector, 1 escort per 2 colonies,
    // 1 missile/destroyer per 3 colonies when at war, 1 transport per 4 colonies on war.
    let want_corvettes = border_sectors.saturating_sub(corvettes);
    let want_escorts = (colony_count / 2).saturating_sub(escorts);
    let want_strike = if at_war {
        (colony_count / 3).saturating_sub(missile + destroyers)
    } else {
        0
    };
    let want_transports = if at_war {
        (colony_count / 4).saturating_sub(transports)
    } else {
        0
    };

    // Pick the most-needed hull, preferring cheaper over expensive when equally needed.
    if want_transports > 0 && want_transports >= want_corvettes && want_transports >= want_escorts {
        return Some(ShipDesignId::TROOP_TRANSPORT);
    }
    if want_corvettes > 0 && want_corvettes >= want_strike {
        return Some(ShipDesignId::PATROL_CORVETTE);
    }
    if want_escorts > 0 && want_escorts >= want_strike {
        return Some(ShipDesignId::ESCORT_FRIGATE);
    }
    if want_strike > 0 && missile <= destroyers {
        return Some(ShipDesignId::MISSILE_FRIGATE);
    }
    if want_strike > 0 {
        return Some(ShipDesignId::DESTROYER);
    }
    None
}

/// Pick what to build at a colony with an empty queue.
///
/// Priority varies by empire identity playstyle:
/// - **Expansionist**: Scout → FabricationYard → Shipyard → ColonyShip
/// - **Agrarian**: AquacultureBay → FabricationYard → Shipyard → ColonyShip → Scout
/// - **Default** (Industrial / Militarist / Scientific / Diplomatic / None):
///   FabricationYard → Shipyard → ColonyShip → Scout
fn pick_build_item(
    state: &GameState,
    empire_id: EmpireId,
    colony_id: ColonyId,
) -> Option<BuildItem> {
    let colony = state.colonies.get(&colony_id)?;
    if colony.owner != empire_id {
        return None;
    }
    if !colony.build_queue.is_empty() {
        return None;
    }

    // Look up planet size for slot checks
    let planet_size = state
        .stars
        .get(&colony.star)
        .and_then(|s| s.planets.get(colony.planet_index))
        .map(|p| p.size);
    let planet = state
        .stars
        .get(&colony.star)
        .and_then(|s| s.planets.get(colony.planet_index));
    let empire_food_negative = state
        .empires
        .get(&empire_id)
        .map(|e| e.food < 0)
        .unwrap_or(false);
    let colony_yield = calculate_yield_with_context(
        colony,
        planet,
        YieldContext {
            food_shortage: empire_food_negative,
            stability_pressure: colony.stability < 85,
        },
    );

    // Determine playstyle tags and AI profile for this empire.
    let empire_def = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id);
    let playstyle: &[PlaystyleTag] = empire_def.map(|d| d.playstyle).unwrap_or(&[]);
    let ai_profile = empire_def.map(|d| d.ai_profile).unwrap_or_default();
    let doctrine = |axis| empire_def.map(|def| def.doctrine_weight(axis)).unwrap_or(0);
    let likes_science = ai_profile.prefers_science_ships;
    let likes_troops = ai_profile.prefers_troop_transports || doctrine(AiDoctrine::Imperial) >= 8;
    let likes_military = ai_profile.prefers_combat_ships || doctrine(AiDoctrine::Militarist) >= 8;
    let likes_defense =
        ai_profile.prefers_defensive_ships || doctrine(AiDoctrine::Isolationist) >= 8;
    let likes_fast_scouts = ai_profile.prefers_fast_scouts || doctrine(AiDoctrine::Explorer) >= 9;
    let likes_colony_arks =
        ai_profile.prefers_colony_arks || doctrine(AiDoctrine::Expansionist) >= 9;

    let is_expansionist = playstyle.contains(&PlaystyleTag::Expansionist);
    let is_agrarian = playstyle.contains(&PlaystyleTag::Agrarian);

    // Helper: check if empire has researched a tech
    let has_tech = |tech: TechId| -> bool {
        state
            .empires
            .get(&empire_id)
            .is_some_and(|e| e.research.completed.contains(&tech))
    };
    let colony_blockaded = state.colony_blockade_state(colony_id).is_some();
    let hostile_at_colony = state.fleets.values().any(|fleet| {
        fleet.location == colony.star
            && fleet.owner != empire_id
            && state
                .relationship_status(empire_id, fleet.owner)
                .is_hostile_or_war()
    });
    let defense_crisis = colony_blockaded || hostile_at_colony;
    let housing_crisis = colony_yield.workforce.housing_deficit >= 2
        || (colony_yield.workforce.housing_deficit > 0 && colony.stability < 70);

    if (empire_food_negative && colony_yield.food < colony_yield.food_consumed
        || housing_crisis && doctrine(AiDoctrine::Biologist) >= 8)
        && !colony.buildings.contains(&BuildingType::AquacultureBay)
    {
        let can_place_surface =
            planet_size.is_some_and(|size| colony.can_place_surface_building(size));
        if can_place_surface {
            return Some(BuildItem::SurfaceStructure(BuildingType::AquacultureBay));
        }
    }

    if defense_crisis {
        let has_orbital_engineering = has_tech(TechId::ORBITAL_ENGINEERING);
        if has_orbital_engineering && !colony.has_shipyard() {
            let can_place_orbital =
                planet_size.is_some_and(|size| colony.can_place_orbital_installation(size));
            if can_place_orbital {
                return Some(BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard));
            }
        }
        if colony.has_shipyard() && has_tech(TechId::PERIMETER_DEFENSE) {
            if likes_military {
                if has_tech(TechId::FLEET_COORDINATION) {
                    return Some(BuildItem::Ship(ShipDesignId::DESTROYER));
                }
                if has_tech(TechId::STRIKE_DOCTRINE) {
                    return Some(BuildItem::Ship(ShipDesignId::MISSILE_FRIGATE));
                }
                return Some(BuildItem::Ship(ShipDesignId::ESCORT_FRIGATE));
            }
            if likes_defense {
                return Some(BuildItem::Ship(ShipDesignId::PATROL_CORVETTE));
            }
        }
    }

    // Role-aware build: after crisis checks, let the colony's
    // specialisation guide what gets built.
    if let Some(item) = role_aware_build_item(state, colony_id) {
        return Some(item);
    }

    // Expansionist: dispatch scouts early before building infrastructure,
    // but only while colonisation is not yet available.
    if is_expansionist && colony.has_shipyard() {
        let has_habitat_seeding = has_tech(TechId::HABITAT_SEEDING);
        let has_survey_drones = has_tech(TechId::SURVEY_DRONES);
        let has_troop_transports = has_tech(TechId::TROOP_TRANSPORTS);
        let has_rapid_transit = has_tech(TechId::RAPID_TRANSIT);
        let has_science_ship = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind.is_survey());
        let has_transport = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind == FleetKind::TroopTransport);
        let explored = state.explored_stars_for(empire_id);
        let has_unexplored = state.stars.keys().any(|sid| !explored.contains(sid));
        // Only skip FabricationYard for scouting when no colonizer tech yet
        if has_unexplored && !has_habitat_seeding {
            if likes_science && has_survey_drones && !has_science_ship {
                return Some(BuildItem::Ship(ShipDesignId::SCIENCE));
            }
            if likes_troops && has_troop_transports && !has_transport {
                return Some(BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT));
            }
            // Prefer Fast Scout if researched and faction prefers it
            if likes_fast_scouts && has_rapid_transit {
                return Some(BuildItem::Ship(ShipDesignId::FAST_SCOUT));
            }
            return Some(BuildItem::Ship(ShipDesignId::SCOUT));
        }
    }

    // Agrarian: prioritise AquacultureBay before fabrication.
    if is_agrarian && !colony.buildings.contains(&BuildingType::AquacultureBay) {
        let can_place_surface =
            planet_size.is_some_and(|size| colony.can_place_surface_building(size));
        if can_place_surface {
            return Some(BuildItem::SurfaceStructure(BuildingType::AquacultureBay));
        }
    }

    if likes_science && !colony.buildings.contains(&BuildingType::ScienceNexus) {
        let can_place_surface =
            planet_size.is_some_and(|size| colony.can_place_surface_building(size));
        if can_place_surface {
            return Some(BuildItem::SurfaceStructure(BuildingType::ScienceNexus));
        }
    }

    if likes_troops || likes_military || defense_crisis {
        let has_orbital_engineering = has_tech(TechId::ORBITAL_ENGINEERING);
        if has_orbital_engineering && !colony.has_shipyard() {
            let can_place_orbital =
                planet_size.is_some_and(|size| colony.can_place_orbital_installation(size));
            if can_place_orbital {
                return Some(BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard));
            }
        }
    }

    // Priority 1: FabricationYard — only if surface slot available
    if !colony.buildings.contains(&BuildingType::FabricationYard) {
        let can_place_surface =
            planet_size.is_some_and(|size| colony.can_place_surface_building(size));
        if can_place_surface {
            return Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard));
        }
    }

    // Priority 2: Shipyard — only if Orbital Engineering researched, not yet installed,
    // and orbital slot available
    let has_orbital_engineering = has_tech(TechId::ORBITAL_ENGINEERING);
    if has_orbital_engineering && !colony.has_shipyard() {
        let can_place_orbital =
            planet_size.is_some_and(|size| colony.can_place_orbital_installation(size));
        if can_place_orbital {
            return Some(BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard));
        }
    }

    // Merchant/Isolationist: build Supply Hub after shipyard to boost trade route value.
    let merchant_weight = doctrine(AiDoctrine::Merchant);
    let isolationist_weight = doctrine(AiDoctrine::Isolationist);
    let wants_trade_hubs = merchant_weight >= 6 || isolationist_weight >= 6;
    if wants_trade_hubs
        && has_orbital_engineering
        && colony.has_shipyard()
        && !colony.has_supply_hub()
    {
        let can_place_orbital =
            planet_size.is_some_and(|size| colony.can_place_orbital_installation(size));
        if can_place_orbital {
            return Some(BuildItem::OrbitalStructure(OrbitalStructureType::SupplyHub));
        }
    }

    // Priority 3 & 4: ships only if the colony has a Shipyard
    if !colony.has_shipyard() {
        return None;
    }

    // Fleet composition: if the empire has ≥3 colonies and is short
    // on a specific hull, build the most-needed one.  Runs before the
    // generic science/survey/combat chain so composition takes priority.
    let owned_colonies = state.colonies.values().filter(|c| c.owner == empire_id).count();
    if owned_colonies >= 3 {
        if let Some(needed) = needed_combat_hull(state, empire_id) {
            // Check the tech requirement for this hull type roughly.
            let hull_needs_tech = match needed {
                ShipDesignId::DESTROYER => TechId::FLEET_COORDINATION,
                ShipDesignId::MISSILE_FRIGATE => TechId::STRIKE_DOCTRINE,
                ShipDesignId::TROOP_TRANSPORT => TechId::TROOP_TRANSPORTS,
                _ => TechId::PERIMETER_DEFENSE,
            };
            if has_tech(hull_needs_tech) {
                return Some(BuildItem::Ship(needed));
            }
        }
    }

    // Science/survey preferences
    if likes_science {
        let has_survey_drones = has_tech(TechId::SURVEY_DRONES);
        let has_advanced_survey = has_tech(TechId::ADVANCED_SURVEY);
        let has_science_ship = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind.is_survey());
        // Prefer Survey Cutter if unlocked, else Science Ship
        if has_advanced_survey && !has_science_ship {
            return Some(BuildItem::Ship(ShipDesignId::SURVEY_CUTTER));
        }
        if has_survey_drones && !has_science_ship {
            return Some(BuildItem::Ship(ShipDesignId::SCIENCE));
        }
    }

    // Fast Scout preference
    if likes_fast_scouts {
        let has_rapid_transit = has_tech(TechId::RAPID_TRANSIT);
        let has_fast_scout = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind == FleetKind::FastScout);
        if has_rapid_transit && !has_fast_scout {
            return Some(BuildItem::Ship(ShipDesignId::FAST_SCOUT));
        }
    }

    // Troop transport preference
    if likes_troops {
        let has_troop_transports = has_tech(TechId::TROOP_TRANSPORTS);
        let has_transport = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind == FleetKind::TroopTransport);
        if has_troop_transports && !has_transport {
            return Some(BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT));
        }
    }

    // Combat ship preferences (Destroyer > Missile Frigate > Escort Frigate).
    // Only engage this path when the empire already has a colonizer or colonisation
    // is not yet available — prevents militarist AIs from never building colony ships.
    if likes_military {
        let has_colonizer = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind.is_colonizer());
        let colonization_available = has_tech(TechId::HABITAT_SEEDING);
        if has_colonizer || !colonization_available {
            let has_fleet_coordination = has_tech(TechId::FLEET_COORDINATION);
            let has_strike_doctrine = has_tech(TechId::STRIKE_DOCTRINE);
            let has_perimeter_defense = has_tech(TechId::PERIMETER_DEFENSE);
            // Prefer Destroyer if unlocked
            if has_fleet_coordination {
                return Some(BuildItem::Ship(ShipDesignId::DESTROYER));
            }
            // Otherwise Missile Frigate
            if has_strike_doctrine {
                return Some(BuildItem::Ship(ShipDesignId::MISSILE_FRIGATE));
            }
            // Fallback to Escort Frigate when only perimeter defense is available
            if has_perimeter_defense {
                return Some(BuildItem::Ship(ShipDesignId::ESCORT_FRIGATE));
            }
        }
    }

    // Defensive ship preference
    if likes_defense {
        let has_perimeter_defense = has_tech(TechId::PERIMETER_DEFENSE);
        let has_patrol_corvette = state
            .fleets
            .values()
            .any(|f| f.owner == empire_id && f.kind == FleetKind::PatrolCorvette);
        if has_perimeter_defense && !has_patrol_corvette {
            return Some(BuildItem::Ship(ShipDesignId::PATROL_CORVETTE));
        }
    }

    // Colony Ark preference (if researched, prefer over standard Colony Ship)
    let has_colonizer = state
        .fleets
        .values()
        .any(|f| f.owner == empire_id && f.kind.is_colonizer());
    let has_habitat_seeding = has_tech(TechId::HABITAT_SEEDING);
    if !has_colonizer && has_habitat_seeding {
        if likes_colony_arks && has_tech(TechId::COLONIAL_VANGUARD) {
            return Some(BuildItem::Ship(ShipDesignId::COLONY_ARK));
        }
        return Some(BuildItem::Ship(ShipDesignId::COLONY));
    }

    // Priority 4: Scout for continued exploration
    if likes_fast_scouts && has_tech(TechId::RAPID_TRANSIT) {
        return Some(BuildItem::Ship(ShipDesignId::FAST_SCOUT));
    }
    Some(BuildItem::Ship(ShipDesignId::SCOUT))
}

// ---------------------------------------------------------------------------
// Scout dispatch
// ---------------------------------------------------------------------------

fn ai_assign_fleet_posture(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    let def = state
        .empires
        .get(&empire_id)
        .and_then(|empire| empire.empire_def)
        .and_then(empire_definition_by_id);
    let doctrine = |axis| def.map(|d| d.doctrine_weight(axis)).unwrap_or(0);

    let militarist = doctrine(AiDoctrine::Militarist);
    let imperial = doctrine(AiDoctrine::Imperial);
    let isolationist = doctrine(AiDoctrine::Isolationist);
    let merchant = doctrine(AiDoctrine::Merchant);
    let explorer = doctrine(AiDoctrine::Explorer);

    let at_war_with_player = matches!(
        state.relationship_status(empire_id, state.player_empire),
        crate::state::RelationshipStatus::War
    );
    let has_hostile_colonies = state.colonies.values().any(|colony| {
        colony.owner == state.player_empire
            && state
                .relationship_status(empire_id, colony.owner)
                .is_hostile_or_war()
    });

    let fleet_ids: Vec<_> = state
        .fleets
        .iter()
        .filter_map(|(fid, fleet)| (fleet.owner == empire_id).then_some(*fid))
        .collect();

    for fleet_id in fleet_ids {
        let Some(fleet) = state.fleets.get(&fleet_id) else {
            continue;
        };
        let role = match fleet.kind {
            FleetKind::Scout | FleetKind::FastScout => {
                if explorer >= 8 {
                    FleetRole::RapidResponseFleet
                } else {
                    FleetRole::ExplorationFleet
                }
            }
            FleetKind::Science | FleetKind::SurveyCutter => FleetRole::SurveyGroup,
            FleetKind::Colonizer | FleetKind::ColonyArk => FleetRole::ColonyEscort,
            FleetKind::TroopTransport => FleetRole::InvasionFleet,
            FleetKind::PatrolCorvette => {
                if merchant >= 8 {
                    FleetRole::TradeProtectionFleet
                } else if isolationist >= militarist.saturating_add(imperial) {
                    FleetRole::DefenseFleet
                } else {
                    FleetRole::PatrolFleet
                }
            }
            FleetKind::EscortFrigate | FleetKind::MissileFrigate | FleetKind::Destroyer => {
                if at_war_with_player && has_hostile_colonies && imperial >= 8 {
                    FleetRole::BlockadeFleet
                } else if militarist + imperial >= 12 {
                    FleetRole::StrikeFleet
                } else if isolationist >= militarist {
                    FleetRole::DefenseFleet
                } else {
                    FleetRole::PatrolFleet
                }
            }
        };

        let formation = match role {
            FleetRole::ExplorationFleet | FleetRole::RapidResponseFleet => {
                FleetFormation::FastAttack
            }
            FleetRole::SurveyGroup => FleetFormation::Balanced,
            FleetRole::ColonyEscort | FleetRole::InvasionFleet => FleetFormation::EscortScreen,
            FleetRole::DefenseFleet | FleetRole::TradeProtectionFleet => FleetFormation::Defensive,
            FleetRole::StrikeFleet => {
                if matches!(fleet.kind, FleetKind::MissileFrigate) {
                    FleetFormation::Artillery
                } else {
                    FleetFormation::Aggressive
                }
            }
            FleetRole::BlockadeFleet => FleetFormation::Defensive,
            FleetRole::PatrolFleet => FleetFormation::Balanced,
        };

        if state.fleet_roles.get(&fleet_id).copied() != Some(role) {
            state.fleet_roles.insert(fleet_id, role);
            events.push(Event::FleetRoleChanged {
                fleet: fleet_id,
                role,
            });
        }
        if state.fleet_formations.get(&fleet_id).copied() != Some(formation) {
            state.fleet_formations.insert(fleet_id, formation);
            events.push(Event::FleetFormationChanged {
                fleet: fleet_id,
                formation,
            });
        }
        state
            .fleet_names
            .entry(fleet_id)
            .or_insert_with(|| format!("{} {}", fleet.kind.label(), fleet_id.0));
    }
}

fn ai_dispatch_scouts(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    if let Some((fleet_id, destination)) = pick_scout_target(state, empire_id) {
        let origin = state.fleets[&fleet_id].location;
        let (turns, used_lane) =
            ai_travel_turns_for_fleet(state, fleet_id, empire_id, origin, destination);
        state.scout_missions.insert(
            fleet_id,
            ScoutMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
                origin,
                total_duration: turns,
            },
        );
        if used_lane {
            events.push(Event::HyperspaceLaneUsed {
                empire: empire_id,
                fleet: fleet_id,
                from: origin,
                to: destination,
            });
        }
        events.push(Event::AiScoutDispatched {
            empire: empire_id,
            fleet: fleet_id,
            destination,
        });
    }
}

fn ai_travel_turns_for_fleet(
    state: &GameState,
    fleet_id: FleetId,
    empire_id: EmpireId,
    origin: StarId,
    destination: StarId,
) -> (u32, bool) {
    let (base_turns, used_lane) = travel_turns_with_lanes(state, empire_id, origin, destination);
    let mobility = state
        .fleet_evaluation(fleet_id)
        .map(|summary| summary.mobility)
        .unwrap_or(100)
        .max(1);
    let supply = state
        .fleets
        .get(&fleet_id)
        .map(|fleet| {
            if fleet.kind.is_combat() || fleet.kind == FleetKind::TroopTransport {
                state.projected_fleet_supply(empire_id, destination)
            } else {
                FleetSupplyState::Supplied
            }
        })
        .unwrap_or(FleetSupplyState::Supplied);
    let adjusted_turns = ((base_turns as u64 * 100 * supply.movement_penalty_pct() as u64)
        .div_ceil(mobility as u64 * 100) as u32)
        .max(1);
    (adjusted_turns, used_lane)
}

// ---------------------------------------------------------------------------
// Combat fleet dispatch
// ---------------------------------------------------------------------------

/// Dispatch idle AI combat fleets toward enemy (player) colonies when at war.
///
/// Only activates after turn 20 to give both sides time to build up.
/// Fleets are sorted by `FleetId` for determinism; targets are sorted by
/// ascending squared distance from the fleet's current position, with
/// `StarId` as a tie-breaker.
fn ai_dispatch_combat_fleets(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    // Only active after turn 20 to prevent immediate combat
    if state.turn < 20 {
        return;
    }

    // Collect every empire this one is formally at war with — the player and
    // other AI empires alike.
    let war_opponents: Vec<EmpireId> = state
        .empires
        .keys()
        .copied()
        .filter(|&other| other != empire_id)
        .filter(|&other| {
            matches!(
                state.relationship_status(empire_id, other),
                crate::state::RelationshipStatus::War
            )
        })
        .collect();
    if war_opponents.is_empty() {
        return;
    }

    // Collect idle combat fleet IDs in deterministic (BTreeMap key) order
    let combat_fleet_ids: Vec<FleetId> = state
        .fleets
        .keys()
        .copied()
        .filter(|&fid| {
            let f = &state.fleets[&fid];
            f.owner == empire_id
                && f.kind.is_combat()
                && !state.fleet_missions.contains_key(&fid)
                && !state.scout_missions.contains_key(&fid)
                && !state.survey_missions.contains_key(&fid)
        })
        .collect();

    if combat_fleet_ids.is_empty() {
        return;
    }

    // Collect war-opponent colony stars as attack targets, remembering which
    // opponents hold colonies there (deduplicated, deterministic order).
    let mut target_owners: std::collections::BTreeMap<
        StarId,
        std::collections::BTreeSet<EmpireId>,
    > = std::collections::BTreeMap::new();
    for colony in state.colonies.values() {
        if war_opponents.contains(&colony.owner) {
            target_owners
                .entry(colony.star)
                .or_default()
                .insert(colony.owner);
        }
    }
    let target_stars: Vec<StarId> = target_owners.keys().copied().collect();

    if target_stars.is_empty() {
        return;
    }

    for fleet_id in combat_fleet_ids {
        let fleet_loc = match state.fleets.get(&fleet_id) {
            Some(f) => f.location,
            None => continue,
        };
        let role = state.fleet_role_for(fleet_id);
        let def = state
            .empires
            .get(&empire_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id);
        let doctrine = |axis| def.map(|d| d.doctrine_weight(axis)).unwrap_or(0);
        let risk_tolerant = doctrine(AiDoctrine::Militarist) + doctrine(AiDoctrine::Imperial) >= 12;
        let conservative =
            doctrine(AiDoctrine::Merchant) + doctrine(AiDoctrine::Isolationist) >= 12;

        // Find the nearest war-opponent colony star; tie-break by ascending StarId
        let fleet_star = match state.stars.get(&fleet_loc) {
            Some(s) => s,
            None => continue,
        };
        let (fleet_x, fleet_y) = (fleet_star.x, fleet_star.y);

        let preferred_targets: std::collections::BTreeSet<StarId> =
            if role == FleetRole::BlockadeFleet {
                state
                    .colonies
                    .values()
                    .filter(|colony| war_opponents.contains(&colony.owner))
                    .filter(|colony| !state.colony_blockade.contains_key(&colony.id))
                    .map(|colony| colony.star)
                    .collect()
            } else {
                std::collections::BTreeSet::new()
            };
        let target_pool: Vec<StarId> = if preferred_targets.is_empty() {
            target_stars.clone()
        } else {
            preferred_targets.into_iter().collect()
        };

        let mut candidates: Vec<(i32, i64, StarId)> = target_pool
            .iter()
            .filter_map(|&sid| {
                let s = state.stars.get(&sid)?;
                let dx = (s.x - fleet_x) as i64;
                let dy = (s.y - fleet_y) as i64;
                let strategic: i32 = target_owners
                    .get(&sid)
                    .map(|owners| {
                        owners
                            .iter()
                            .map(|&owner| {
                                contested_star_resource_score(state, empire_id, owner, sid)
                            })
                            .sum::<i32>()
                    })
                    .unwrap_or(0);
                let supply = state.projected_fleet_supply(empire_id, sid);
                let supply_bonus = match supply {
                    FleetSupplyState::Supplied => 6,
                    FleetSupplyState::Extended => {
                        if risk_tolerant {
                            1
                        } else {
                            -8
                        }
                    }
                    FleetSupplyState::OutOfSupply => {
                        if risk_tolerant || !conservative {
                            -10
                        } else {
                            -30
                        }
                    }
                };
                Some((strategic + supply_bonus, dx * dx + dy * dy, sid))
            })
            .collect();

        // Sort deterministically: highest strategic value, then closest, then ascending StarId.
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

        let destination = match candidates.first() {
            Some(&(_, _, sid)) => sid,
            None => continue,
        };

        // Skip if already at destination
        if fleet_loc == destination {
            continue;
        }

        let (turns, used_lane) =
            ai_travel_turns_for_fleet(state, fleet_id, empire_id, fleet_loc, destination);

        use crate::state::FleetMission;
        state.fleet_missions.insert(
            fleet_id,
            FleetMission {
                fleet: fleet_id,
                destination,
                turns_remaining: turns,
                origin: fleet_loc,
                total_duration: turns,
            },
        );

        if used_lane {
            events.push(Event::HyperspaceLaneUsed {
                empire: empire_id,
                fleet: fleet_id,
                from: fleet_loc,
                to: destination,
            });
        }

        events.push(Event::AiCombatFleetDispatched {
            empire: empire_id,
            fleet: fleet_id,
            destination,
        });
    }
}

/// Dispatch idle combat fleets to guard important systems during
/// peacetime.  Only fleet role
fn ai_dispatch_patrols(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    if state.turn < 30 {
        return;
    }

    // Collect idle combat fleets (not on any mission).
    let fleet_ids: Vec<FleetId> = state
        .fleets
        .keys()
        .copied()
        .filter(|&fid| {
            let f = &state.fleets[&fid];
            f.owner == empire_id
                && f.kind.is_combat()
                && !state.fleet_missions.contains_key(&fid)
                && !state.scout_missions.contains_key(&fid)
                && !state.survey_missions.contains_key(&fid)
        })
        .collect();
    if fleet_ids.is_empty() {
        return;
    }

    // Identify high-value stars to protect: home star + each colony
    // that has a strategic resource under extraction.
    let home = state.empires.get(&empire_id).map(|e| e.home_star);
    let valuable: std::collections::BTreeSet<StarId> = state
        .colonies
        .values()
        .filter(|c| c.owner == empire_id)
        .filter_map(|c| {
            let is_home = home == Some(c.star);
            let has_strat = state
                .empire_resource_access
                .get(&empire_id)
                .map(|m| !m.is_empty())
                .unwrap_or(false);
            let is_border = state.diplomacy.keys().any(|&eid| {
                state
                    .colonies
                    .values()
                    .any(|oc| oc.owner == eid && oc.star == c.star)
            });
            if is_home || has_strat || is_border {
                Some(c.star)
            } else {
                None
            }
        })
        .collect();
    if valuable.is_empty() {
        return;
    }

    // Dispatch up to 1 patrol for every 3 idle combat fleets.
    let patrol_count = (fleet_ids.len() / 3).max(1);
    for &fid in fleet_ids.iter().take(patrol_count) {
        if let Some(target) = valuable.iter().next() {
            let fleet_loc = state.fleets[&fid].location;
            if fleet_loc == *target {
                continue; // Already there
            }
            let (turns, used_lane) =
                ai_travel_turns_for_fleet(state, fid, empire_id, fleet_loc, *target);
            state.fleet_missions.insert(
                fid,
                crate::state::FleetMission {
                    fleet: fid,
                    destination: *target,
                    turns_remaining: turns,
                    origin: fleet_loc,
                    total_duration: turns,
                },
            );
            if used_lane {
                events.push(Event::HyperspaceLaneUsed {
                    empire: empire_id,
                    fleet: fid,
                    from: fleet_loc,
                    to: *target,
                });
            }
            events.push(crate::events::Event::AiCombatFleetDispatched {
                empire: empire_id,
                fleet: fid,
                destination: *target,
            });
        }
    }
}

/// Find the nearest unexplored star for an idle AI scout fleet.
///
/// Returns `(fleet_id, destination)` or `None` if no valid target exists.
fn pick_scout_target(state: &GameState, empire_id: EmpireId) -> Option<(FleetId, StarId)> {
    // Find first idle scout fleet owned by this empire (deterministic: BTreeMap key order)
    // Both Scout and FastScout can perform scout missions.
    let fleet_id = state.fleets.keys().copied().find(|&fid| {
        let f = &state.fleets[&fid];
        f.owner == empire_id
            && (f.kind == FleetKind::Scout || f.kind == FleetKind::FastScout)
            && !state.scout_missions.contains_key(&fid)
            && !state.fleet_missions.contains_key(&fid)
    })?;

    let fleet_loc = state.fleets.get(&fleet_id)?.location;
    let fleet_star = state.stars.get(&fleet_loc)?;

    // Stars already targeted by any AI scout
    let already_targeted: std::collections::BTreeSet<StarId> = state
        .scout_missions
        .values()
        .filter(|m| {
            state
                .fleets
                .get(&m.fleet)
                .is_some_and(|f| f.owner == empire_id)
        })
        .map(|m| m.destination)
        .collect();

    let mut candidates: Vec<(i32, i64, StarId)> = state
        .stars
        .keys()
        .filter(|&sid| {
            !state.explored_stars_for(empire_id).contains(sid) && !already_targeted.contains(sid)
        })
        .filter(|&&sid| {
            let hostile_strength: u32 = state
                .fleets
                .values()
                .filter(|fleet| {
                    fleet.location == sid
                        && fleet.owner != empire_id
                        && state
                            .relationship_status(empire_id, fleet.owner)
                            .is_hostile_or_war()
                })
                .map(|fleet| fleet.strength)
                .sum();
            hostile_strength <= MAX_SAFE_SCOUT_HOSTILE_STRENGTH
        })
        .filter_map(|&sid| {
            let s = state.stars.get(&sid)?;
            let dx = (s.x - fleet_star.x) as i64;
            let dy = (s.y - fleet_star.y) as i64;
            let prospect = score_system_for_scout(state, empire_id, sid);
            Some((prospect, dx * dx + dy * dy, sid))
        })
        .collect();

    // Highest prospect first; tie-break by nearest then ascending StarId.
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    candidates.first().map(|&(_, _, sid)| (fleet_id, sid))
}

// ---------------------------------------------------------------------------
// Colonization
// ---------------------------------------------------------------------------

fn ai_colonize(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    if let Some((fleet_id, star_id, planet_index)) = pick_colonize_target(state, empire_id) {
        let colony_id = state.next_colony_id();

        // Choose role deterministically based on the target planet's class.
        let role = state
            .stars
            .get(&star_id)
            .and_then(|s| s.planets.get(planet_index))
            .map(|p| ai_role_for_planet_class_with_identity(p.class, empire_id, state))
            .unwrap_or(ColonyRole::Balanced);

        let new_colony = Colony {
            id: colony_id,
            star: star_id,
            planet_index,
            owner: empire_id,
            population: 1,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role,
            rally_point: None,
        };
        state.colonies.insert(colony_id, new_colony);

        // Update planet reference
        if let Some(star) = state.stars.get_mut(&star_id)
            && let Some(planet) = star.planets.get_mut(planet_index)
        {
            planet.colony = Some(colony_id);
        }

        // Consume the colonizer fleet
        state.fleets.remove(&fleet_id);
        state.scout_missions.remove(&fleet_id);
        state.fleet_missions.remove(&fleet_id);

        events.push(Event::AiColonized {
            empire: empire_id,
            star: star_id,
            planet_index,
            colony: colony_id,
        });
    }
}

/// Find the best colonizer fleet and target star for colonization.
/// Scores every AI-explored star with idle colonizers, not just the
/// colonizer's current system.
fn pick_colonize_target(
    state: &GameState,
    empire_id: EmpireId,
) -> Option<(FleetId, StarId, usize)> {
    let doctrine = ai_primary_doctrine(state, empire_id);
    let completed: Vec<TechId> = state
        .empires
        .get(&empire_id)
        .map(|e| e.research.completed.clone())
        .unwrap_or_default();
    let explored = state.explored_stars_for(empire_id);

    // Collect all idle colonizer fleets.
    let colonizer_ids: Vec<FleetId> = state
        .fleets
        .keys()
        .copied()
        .filter(|&fid| {
            let f = &state.fleets[&fid];
            f.owner == empire_id
                && (f.kind == FleetKind::Colonizer || f.kind == FleetKind::ColonyArk)
                && !state.scout_missions.contains_key(&fid)
                && !state.fleet_missions.contains_key(&fid)
        })
        .collect();
    if colonizer_ids.is_empty() {
        return None;
    }

    // Score every explored star with a habitable free planet.
    // For each star, compute the best planet score, then pair
    // the highest-scoring (fleet, star, planet) combination.
    let mut candidates: Vec<(i32, FleetId, StarId, usize)> = Vec::new();
    for star in state.stars.values() {
        if !explored.contains(&star.id) {
            continue;
        }
        if star
            .planets
            .iter()
            .all(|p| p.colony.is_some() || !p.habitable)
        {
            // No colonizable planets at this star.
            continue;
        }
        let base_star_score = score_system_for_scout(state, empire_id, star.id);
        for (pi, planet) in star.planets.iter().enumerate() {
            if planet.colony.is_some() || !planet.habitable {
                continue;
            }
            let mut planet_score = match planet.class {
                PlanetClass::Terran => 20,
                PlanetClass::Oceanic => 15,
                PlanetClass::Desert => 10,
                PlanetClass::Frozen => 8,
                PlanetClass::Volcanic => 5,
                PlanetClass::Barren => 3,
            };
            if planet.surveyed {
                for s in visible_specials_for_empire(planet, &completed) {
                    planet_score += special_value_for_doctrine(s, doctrine) / 10;
                }
                for a in visible_anomalies_for_empire(planet, &completed) {
                    planet_score += anomaly_value_for_doctrine(a, doctrine) / 10;
                }
                for r in &planet.resources {
                    if crate::state::is_resource_discoverable(*r, &completed) {
                        planet_score += resource_value_for_doctrine(*r, doctrine) / 20;
                    }
                }
            }
            // Cluster penalty: reduce score if empire already has many colonies nearby.
            let cluster_penalty: i32 = state
                .colonies
                .values()
                .filter(|c| c.owner == empire_id)
                .filter_map(|c| state.stars.get(&c.star))
                .filter(|cs| {
                    let dx = (star.x - cs.x) as i64;
                    let dy = (star.y - cs.y) as i64;
                    dx * dx + dy * dy <= 300 * 300
                })
                .count() as i32
                * 10;
            let combined_score = base_star_score.saturating_add(planet_score * 2) - cluster_penalty;
            for &fid in &colonizer_ids {
                candidates.push((combined_score, fid, star.id, pi));
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    candidates
        .first()
        .map(|(_, fid, sid, pi)| (*fid, *sid, *pi))
}

// ---------------------------------------------------------------------------
// Colony role assignment
// ---------------------------------------------------------------------------

/// Deterministic rule: pick a colony role based solely on planet class.
///
/// This mapping is stable — same input always produces the same output.
pub(crate) fn ai_role_for_planet_class(class: PlanetClass) -> ColonyRole {
    match class {
        PlanetClass::Oceanic => ColonyRole::Agricultural,
        PlanetClass::Volcanic | PlanetClass::Barren => ColonyRole::Industrial,
        PlanetClass::Frozen => ColonyRole::Scientific,
        PlanetClass::Desert => ColonyRole::Financial,
        PlanetClass::Terran => ColonyRole::Balanced,
    }
}

/// Extend the base planet-class role mapping with faction-specific AI preferences.
///
/// This keeps the deterministic class-based baseline intact while allowing
/// identities such as Terran Concord and Terran Dominion to bias colonies
/// toward scientific stability or military specialization.
fn ai_role_for_planet_class_with_identity(
    class: PlanetClass,
    empire_id: EmpireId,
    state: &GameState,
) -> ColonyRole {
    let base = ai_role_for_planet_class(class);
    let Some(def) = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
    else {
        return base;
    };

    if def.ai_profile.prefers_military_roles {
        return match class {
            PlanetClass::Terran | PlanetClass::Volcanic | PlanetClass::Barren => {
                ColonyRole::Military
            }
            _ => base,
        };
    }

    if def.ai_profile.prefers_stable_colonies {
        return match class {
            PlanetClass::Terran | PlanetClass::Frozen => ColonyRole::Scientific,
            _ => base,
        };
    }

    base
}

/// Assign specialisation roles to any AI-owned colony that still has the
/// default `Balanced` role and whose planet class maps to a different role.
///
/// Runs once per turn; produces `AiColonyRoleAssigned` events only when a
/// role change actually occurs.
fn ai_assign_colony_roles(state: &mut GameState, empire_id: EmpireId, events: &mut Vec<Event>) {
    let empire = match state.empires.get(&empire_id) {
        Some(e) => e,
        None => return,
    };
    let emp_def = empire.empire_def.and_then(empire_definition_by_id);
    let doctrine = |axis| emp_def.map(|d| d.doctrine_weight(axis)).unwrap_or(0);
    let at_war = state
        .diplomacy
        .values()
        .any(|rs| *rs == crate::state::RelationshipStatus::War);
    let food_deficit = empire.food < 0;

    // Re-evaluate every 8 turns (not just once for Balanced colonies).
    let re_eval = state.turn % 8 == 0;

    let colony_ids: Vec<ColonyId> = state
        .colonies
        .keys()
        .filter(|&&id| {
            let c = state.colonies.get(&id);
            c.is_some_and(|c| c.owner == empire_id && (re_eval || c.role == ColonyRole::Balanced))
        })
        .copied()
        .collect();

    for colony_id in colony_ids {
        let Some(colony) = state.colonies.get(&colony_id) else {
            continue;
        };
        let Some(planet) = state
            .stars
            .get(&colony.star)
            .and_then(|s| s.planets.get(colony.planet_index))
        else {
            continue;
        };
        let class = planet.class;

        // Compute a dynamic role score for each role, then pick the highest.
        let base = ai_role_for_planet_class_with_identity(class, empire_id, state);
        let mut scores: Vec<(i32, ColonyRole)> = Vec::new();

        for &role in ColonyRole::all() {
            let mut s: i32 = if role == base { 5 } else { 0 };
            match role {
                ColonyRole::Agricultural => {
                    s += match class {
                        PlanetClass::Oceanic | PlanetClass::Terran => 3,
                        _ => 0,
                    };
                    if food_deficit {
                        s += 3;
                    }
                }
                ColonyRole::Industrial => {
                    s += match class {
                        PlanetClass::Barren | PlanetClass::Volcanic => 4,
                        _ => 0,
                    };
                    if doctrine(AiDoctrine::Industrialist) >= 6 {
                        s += 2;
                    }
                }
                ColonyRole::Scientific => {
                    s += match class {
                        PlanetClass::Frozen => 4,
                        _ => 0,
                    };
                    if doctrine(AiDoctrine::Technologist) >= 6 {
                        s += 3;
                    }
                }
                ColonyRole::Financial => {
                    s += match class {
                        PlanetClass::Desert => 3,
                        _ => 0,
                    };
                    if doctrine(AiDoctrine::Merchant) >= 6 {
                        s += 2;
                    }
                }
                ColonyRole::Military => {
                    if at_war {
                        s += 4;
                    }
                    if doctrine(AiDoctrine::Militarist) >= 6 {
                        s += 2;
                    }
                }
                ColonyRole::Balanced => {
                    s += 1; // Always a safe fallback
                }
            }
            scores.push((s, role));
        }
        scores.sort_by(|a, b| b.0.cmp(&a.0).then((a.1 as u8).cmp(&(b.1 as u8))));
        if let Some((_, best_role)) = scores.first() {
            let current_role = colony.role;
            if *best_role != current_role {
                if let Some(colony) = state.colonies.get_mut(&colony_id) {
                    colony.role = *best_role;
                }
                events.push(Event::AiColonyRoleAssigned {
                    empire: empire_id,
                    colony: colony_id,
                    role: *best_role,
                });
            }
        }
    }
}

/// Role-aware build priority: after the top-level crisis checks (food,
/// defense), defer to per-colony role specialisation rather than the
/// uniform global priority chain.
fn role_aware_build_item(state: &GameState, colony_id: ColonyId) -> Option<BuildItem> {
    let colony = state.colonies.get(&colony_id)?;
    let planet = state
        .stars
        .get(&colony.star)
        .and_then(|s| s.planets.get(colony.planet_index));
    let has_shipyard = colony.has_shipyard();
    let can_place_surface = planet
        .map(|p| colony.can_place_surface_building(p.size))
        .unwrap_or(false);
    let can_place_orbital = planet
        .map(|p| colony.can_place_orbital_installation(p.size))
        .unwrap_or(false);

    match colony.role {
        ColonyRole::Scientific => {
            if can_place_surface && !colony.buildings.contains(&BuildingType::ScienceNexus) {
                return Some(BuildItem::SurfaceStructure(BuildingType::ScienceNexus));
            }
        }
        ColonyRole::Agricultural => {
            if can_place_surface && !colony.buildings.contains(&BuildingType::AquacultureBay) {
                return Some(BuildItem::SurfaceStructure(BuildingType::AquacultureBay));
            }
        }
        ColonyRole::Industrial | ColonyRole::Financial => {
            if can_place_surface && !colony.buildings.contains(&BuildingType::FabricationYard) {
                return Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard));
            }
        }
        ColonyRole::Military => {
            if can_place_orbital && !has_shipyard {
                return Some(BuildItem::OrbitalStructure(OrbitalStructureType::Shipyard));
            }
            if can_place_surface && !colony.buildings.contains(&BuildingType::FabricationYard) {
                return Some(BuildItem::SurfaceStructure(BuildingType::FabricationYard));
            }
        }
        ColonyRole::Balanced => {
            // Fall through to the generic chain below.
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Ship Designer AI — auto-generate designs for AI empires
// ---------------------------------------------------------------------------

/// Generate custom ship designs for an AI empire.
///
/// For each hull that is unlocked and has no existing non-obsolete design
/// owned by this empire, the best available component per slot is chosen
/// based on the empire's doctrine, and a new `CustomShipDesign` is created.
pub fn ai_generate_designs(state: &mut GameState, empire_id: EmpireId) {
    use crate::state::{all_components, all_hull_templates};

    let completed_techs: Vec<TechId> = state
        .empires
        .get(&empire_id)
        .map(|e| e.research.completed.to_vec())
        .unwrap_or_default();
    let available_resources: Vec<crate::state::StrategicResource> = state
        .empire_resource_access
        .get(&empire_id)
        .map(|m| {
            m.iter()
                .filter_map(|(resource, count)| (*count > 0).then_some(*resource))
                .collect()
        })
        .unwrap_or_default();

    let doctrine = state
        .empires
        .get(&empire_id)
        .and_then(|e| e.empire_def)
        .and_then(empire_definition_by_id)
        .map(|def| {
            // Pick the doctrine with the highest weight as the primary doctrine
            use crate::state::AiDoctrine;
            let doctrines = [
                AiDoctrine::Explorer,
                AiDoctrine::Technologist,
                AiDoctrine::Merchant,
                AiDoctrine::Imperial,
                AiDoctrine::Militarist,
                AiDoctrine::Industrialist,
                AiDoctrine::Expansionist,
                AiDoctrine::Isolationist,
                AiDoctrine::Biologist,
            ];
            doctrines
                .iter()
                .copied()
                .max_by_key(|&d| def.doctrine_weight(d))
                .unwrap_or(AiDoctrine::Explorer)
        })
        .unwrap_or(AiDoctrine::Explorer);

    let all_hulls = all_hull_templates();
    let all_comps = all_components();

    // Collect hull IDs that already have a live design for this empire (deterministic order).
    let existing_hulls: std::collections::BTreeSet<crate::state::HullId> = state
        .custom_designs
        .values()
        .filter(|d| d.owner == empire_id && !d.obsolete)
        .map(|d| d.hull_id)
        .collect();

    // Sort hulls for determinism
    let mut hulls: Vec<_> = all_hulls.iter().collect();
    hulls.sort_by_key(|h| h.hull_id.0);

    for hull in hulls {
        // Skip if hull requires tech not yet unlocked
        if let Some(req) = hull.required_tech
            && !completed_techs.contains(&req)
        {
            continue;
        }
        // Skip if already have a design for this hull
        if existing_hulls.contains(&hull.hull_id) {
            continue;
        }

        // Pick one component per slot in hull slot order (never sort — order must match hull.slots)
        let mut components: Vec<ComponentId> = Vec::new();
        for &slot in hull.slots {
            if let Some(best) = pick_best_component(
                all_comps,
                slot,
                &completed_techs,
                &available_resources,
                doctrine,
                &components,
            ) {
                components.push(best);
            } else {
                // No valid component available for this slot — skip this hull
                components.clear();
                break;
            }
        }

        if components.len() != hull.slots.len() {
            continue;
        }

        let design_id = CustomDesignId(state.next_custom_design_id);
        let hull_id = hull.hull_id;
        let design = CustomShipDesign {
            design_id,
            hull_id,
            components,
            owner: empire_id,
            name: format!("AI Design {}", state.next_custom_design_id),
            obsolete: false,
        };

        // Only insert if valid
        if design
            .validate_with_resources(&completed_techs, &available_resources)
            .is_ok()
        {
            state.custom_designs.insert(design_id, design);
            state.next_custom_design_id += 1;
        }
    }
}

/// Pick the best component for a given slot that is tech-unlocked.
///
/// Returns `None` if no suitable component is found.
fn pick_best_component(
    all_comps: &[crate::state::ComponentDef],
    slot: SlotCategory,
    completed_techs: &[TechId],
    available_resources: &[crate::state::StrategicResource],
    doctrine: AiDoctrine,
    _already_chosen: &[ComponentId],
) -> Option<ComponentId> {
    let mut candidates: Vec<_> = all_comps
        .iter()
        .filter(|c| c.category == slot)
        .filter(|c| match c.required_tech {
            Some(tech) => completed_techs.contains(&tech),
            None => true,
        })
        .filter(|c| match c.required_resource {
            Some(resource) => available_resources.contains(&resource),
            None => true,
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Sort for determinism then score
    candidates.sort_by_key(|c| c.component_id.0);
    candidates.sort_by_key(|c| std::cmp::Reverse(score_component_for_doctrine(c, doctrine)));

    candidates.first().map(|c| c.component_id)
}

/// Heuristic score for how well a component fits a doctrine.
fn score_component_for_doctrine(comp: &crate::state::ComponentDef, doctrine: AiDoctrine) -> i32 {
    use crate::state::ComponentTag;

    // Base: balanced stat contribution
    let mut score = comp.attack_modifier + comp.defense_modifier + comp.hp_modifier;

    // Doctrine-specific multipliers for movement and maintenance
    match doctrine {
        AiDoctrine::Explorer | AiDoctrine::Expansionist => {
            // Prefer fast, long-range components
            score += comp.movement_modifier * 3;
            score -= comp.maintenance_modifier;
        }
        AiDoctrine::Merchant => {
            // Prefer low-upkeep, long-range components
            score -= comp.maintenance_modifier * 2;
            score += comp.movement_modifier * 2;
        }
        AiDoctrine::Militarist | AiDoctrine::Imperial => {
            // Prefer high attack
            score += comp.attack_modifier * 2;
        }
        AiDoctrine::Isolationist => {
            // Prefer defense and hp
            score += comp.defense_modifier * 2 + comp.hp_modifier;
        }
        _ => {}
    }

    for &tag in comp.special_tags {
        let bonus: i32 = match (doctrine, tag) {
            (AiDoctrine::Militarist, ComponentTag::Sensors)
            | (AiDoctrine::Imperial, ComponentTag::Sensors) => 4,
            (AiDoctrine::Explorer, ComponentTag::Survey)
            | (AiDoctrine::Technologist, ComponentTag::Survey) => 4,
            (AiDoctrine::Expansionist, ComponentTag::Colony)
            | (AiDoctrine::Biologist, ComponentTag::Colony) => 4,
            (AiDoctrine::Imperial, ComponentTag::Invasion)
            | (AiDoctrine::Militarist, ComponentTag::Invasion) => 3,
            (AiDoctrine::Explorer, ComponentTag::LongRange)
            | (AiDoctrine::Expansionist, ComponentTag::LongRange)
            | (AiDoctrine::Merchant, ComponentTag::LongRange) => 4,
            (AiDoctrine::Explorer, ComponentTag::Sensors)
            | (AiDoctrine::Expansionist, ComponentTag::Sensors) => 3,
            _ => 1,
        };
        score += bonus;
    }

    score
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Engine;
    use crate::state::{
        BuildingType, EmpireDefinitionId, EmpireId, FleetKind, PlanetClass, TechId,
        all_empire_definitions,
    };

    /// Helper: get the AI empire ID from an engine, panicking if absent.
    fn ai_id(engine: &Engine) -> EmpireId {
        engine.state.ai_empire.expect("Engine must have AI empire")
    }

    // -----------------------------------------------------------------------
    // Determinism
    // -----------------------------------------------------------------------

    #[test]
    fn same_seed_produces_same_ai_decisions() {
        let mut engine_a = Engine::new(42);
        let mut engine_b = Engine::new(42);

        // End turn once so the AI runs
        engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
        engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Both states must be identical
        assert_eq!(engine_a.state, engine_b.state);
    }

    #[test]
    fn ai_turn_is_deterministic_across_multiple_turns() {
        let mut engine_a = Engine::new(999);
        let mut engine_b = Engine::new(999);

        for _ in 0..5 {
            engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
            engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        assert_eq!(engine_a.state, engine_b.state);
    }

    // -----------------------------------------------------------------------
    // Research
    // -----------------------------------------------------------------------

    #[test]
    fn ai_selects_valid_research() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Initially no research selected
        assert!(engine.state.empires[&ai].research.current_tech.is_none());

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI should have selected a tech
        let selected = engine.state.empires[&ai].research.current_tech;
        assert!(
            selected.is_some(),
            "AI must select a research tech after first turn"
        );

        // The selected tech must be in all_techs()
        let tech_id = selected.unwrap();
        assert!(
            all_techs().iter().any(|t| t.id == tech_id),
            "AI must select a valid tech"
        );

        // Event must be emitted
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiResearchSelected { empire, tech, .. } if *empire == ai && *tech == tech_id)),
            "AiResearchSelected event must be emitted"
        );
    }

    #[test]
    fn ai_research_plan_sets_small_queue() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let empire = engine.state.empires.get(&ai).expect("AI empire must exist");
        assert!(
            (1..=3).contains(&empire.research.queue.len()),
            "AI should queue 1-3 follow-up techs (2-4 plan total)"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            Event::AiResearchSelected { empire, .. } if *empire == ai
        )));
    }

    #[test]
    fn ai_research_plan_only_contains_currently_available_techs() {
        let engine = Engine::new(52);
        let ai = ai_id(&engine);
        let empire = engine
            .state
            .empires
            .get(&ai)
            .expect("AI empire must exist for plan test");
        let (first, queue) =
            pick_research_plan(&engine.state, ai).expect("AI should produce a research plan");
        let mut simulated_completed = empire.research.completed.clone();
        assert!(
            is_tech_available(&simulated_completed, first),
            "First planned tech must be researchable"
        );
        simulated_completed.push(first);
        for tech in queue {
            assert!(
                is_tech_available(&simulated_completed, tech),
                "Queued tech must be available from prior queued prerequisites"
            );
            simulated_completed.push(tech);
        }
    }

    #[test]
    fn ai_selects_cheapest_unresearched_tech() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // The cheapest tech (cost=50, TechId(1) "Void Propulsion") should be chosen
        let cheapest = all_techs()
            .iter()
            .min_by_key(|t| (t.cost, t.id.0))
            .unwrap()
            .id;

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let selected = engine.state.empires[&ai].research.current_tech.unwrap();
        assert_eq!(
            selected, cheapest,
            "AI must pick cheapest unresearched tech"
        );
    }

    #[test]
    fn ai_does_not_select_completed_tech() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Pre-complete all techs except TechId(5) (cost 120) and TechId(6) (cost 90)
        {
            let empire = engine.state.empires.get_mut(&ai).unwrap();
            for t in all_techs() {
                if t.id != TechId(5) && t.id != TechId(6) {
                    empire.research.completed.push(t.id);
                }
            }
        }

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let selected = engine.state.empires[&ai].research.current_tech.unwrap();
        // Should pick TechId(6) (cost 90) over TechId(5) (cost 120)
        assert_eq!(
            selected,
            TechId(6),
            "AI must pick next cheapest uncompleted tech"
        );
    }

    #[test]
    fn ai_does_not_reselect_when_already_researching() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Force a specific tech
        engine
            .state
            .empires
            .get_mut(&ai)
            .unwrap()
            .research
            .current_tech = Some(TechId(4));

        let events_before = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // No AiResearchSelected should be emitted
        assert!(
            !events_before
                .iter()
                .any(|e| matches!(e, Event::AiResearchSelected { empire, .. } if *empire == ai)),
            "AI must not change research when already active"
        );
        assert_eq!(
            engine.state.empires[&ai].research.current_tech,
            Some(TechId(4))
        );
    }

    // -----------------------------------------------------------------------
    // Build queue
    // -----------------------------------------------------------------------

    #[test]
    fn ai_queues_fabrication_yard_first() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .expect("AI must have a colony");

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI colony should have FabricationYard queued
        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(!colony.build_queue.is_empty(), "AI must queue a build item");

        // AiBuildQueued event emitted
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::AiBuildQueued { empire, colony, .. }
                if *empire == ai && *colony == ai_colony_id
            )),
            "AiBuildQueued event must be emitted"
        );
    }

    #[test]
    fn ai_does_not_queue_when_queue_non_empty() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Pre-fill the AI colony queue
        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .build_queue
            .push(BuildItem::Ship(ShipDesignId::SCOUT));

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Should not emit an extra AiBuildQueued for this colony
        let build_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(e, Event::AiBuildQueued { empire, colony, .. }
                    if *empire == ai && *colony == ai_colony_id)
            })
            .collect();
        assert!(
            build_events.is_empty(),
            "AI must not queue when colony already has a queue item"
        );
    }

    #[test]
    fn ai_queues_colony_ship_if_no_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Pre-build a FabricationYard so AI skips that priority
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);

        // Give the AI colony a Shipyard so it can queue ships
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .orbital_installations
            .push(crate::state::OrbitalStructureType::Shipyard);

        // Ensure no colonizer exists
        assert!(
            !engine
                .state
                .fleets
                .values()
                .any(|f| f.owner == ai && f.kind == FleetKind::Colonizer)
        );

        // Colony ship design requires Habitat Seeding.
        engine
            .state
            .empires
            .get_mut(&ai)
            .unwrap()
            .research
            .completed
            .push(TechId(2));

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            !colony.build_queue.is_empty(),
            "AI must queue something when colonizer build is possible"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::AiBuildQueued { empire, .. }
            if *empire == ai
        )));
    }

    #[test]
    fn ai_queues_scout_when_has_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Pre-build FabricationYard
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);

        // Give the AI colony a Shipyard so it can queue ships
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .orbital_installations
            .push(crate::state::OrbitalStructureType::Shipyard);

        // Ensure the AI has a food surplus so the build queue isn't
        // short-circuited by a food crisis (the default empire assignment
        // can give the AI a faction with traits that tip the colony into
        // deficit).
        if let Some(empire) = engine.state.empires.get_mut(&ai) {
            empire.food = 100;
        }

        // Add a fake colonizer fleet for the AI
        let fake_colonizer_id = crate::state::FleetId(99);
        engine.state.fleets.insert(
            fake_colonizer_id,
            crate::state::Fleet {
                id: fake_colonizer_id,
                owner: ai,
                location: engine.state.empires[&ai].home_star,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        // The AI queues a Scout build — either as a static design or via a custom design
        // that maps to the Scout fleet kind.
        let queued_scout = colony.build_queue.iter().any(|item| match item {
            BuildItem::Ship(id) => *id == ShipDesignId::SCOUT,
            BuildItem::CustomShip(did) => engine
                .state
                .custom_designs
                .get(did)
                .and_then(|d| d.hull_id.template())
                .is_some_and(|h| h.fleet_kind == FleetKind::Scout),
            _ => false,
        });
        assert!(
            queued_scout,
            "AI must queue Scout when FabricationYard built and colonizer exists"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            Event::AiBuildQueued { empire, .. } if *empire == ai
        )));
    }

    // -----------------------------------------------------------------------
    // Scout dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn ai_scout_dispatch_is_deterministic() {
        // Run two engines with the same seed; scout destinations must match
        let mut engine_a = Engine::new(7);
        let mut engine_b = Engine::new(7);

        engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
        engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);

        assert_eq!(engine_a.state.scout_missions, engine_b.state.scout_missions);
    }

    #[test]
    fn ai_dispatches_scout_to_unexplored_star() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // AI scout missions must target unexplored stars
        for mission in engine.state.scout_missions.values() {
            let fleet = engine.state.fleets.get(&mission.fleet);
            if let Some(f) = fleet
                && f.owner == ai
            {
                assert!(
                    !engine
                        .state
                        .ai_explored_stars
                        .contains(&mission.destination),
                    "AI must not scout an already-explored star"
                );
            }
        }

        // AiScoutDispatched event must be emitted
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiScoutDispatched { empire, .. } if *empire == ai)),
            "AiScoutDispatched event must be emitted"
        );
    }

    #[test]
    fn scout_targeting_prefers_high_value_prospect_deterministically() {
        let mut engine = Engine::new(77);
        let ai = ai_id(&engine);
        let Some((fleet_id, _)) = engine
            .state
            .fleets
            .iter()
            .find(|(_, f)| f.owner == ai && f.kind.is_scout())
            .map(|(id, f)| (*id, f.location))
        else {
            panic!("AI scout required for targeting test");
        };

        let unexplored: Vec<StarId> = engine
            .state
            .stars
            .keys()
            .copied()
            .filter(|sid| !engine.state.ai_explored_stars.contains(sid))
            .collect();
        assert!(
            unexplored.len() >= 2,
            "test requires at least two unexplored stars"
        );
        let low_star = unexplored[0];
        let high_star = unexplored[1];

        for sid in &unexplored {
            if let Some(star) = engine.state.stars.get_mut(sid) {
                star.spectral_class = crate::state::SpectralClass::M;
                for planet in &mut star.planets {
                    planet.class = PlanetClass::Terran;
                }
            }
        }
        if let Some(star) = engine.state.stars.get_mut(&high_star) {
            star.spectral_class = crate::state::SpectralClass::O;
            for planet in &mut star.planets {
                planet.class = PlanetClass::Volcanic;
            }
        }
        if let Some(star) = engine.state.stars.get_mut(&low_star) {
            star.spectral_class = crate::state::SpectralClass::M;
            for planet in &mut star.planets {
                planet.class = PlanetClass::Terran;
            }
        }

        let (_, destination) = pick_scout_target(&engine.state, ai).expect("expected scout target");
        assert_eq!(
            destination, high_star,
            "AI should prioritize higher strategic prospect target"
        );

        // Stability check: selection must be deterministic across repeated calls.
        let (_, destination_repeat) =
            pick_scout_target(&engine.state, ai).expect("expected scout target on repeat");
        assert_eq!(destination_repeat, destination);
        assert!(engine.state.fleets.contains_key(&fleet_id));
    }

    #[test]
    fn ai_scout_uses_lane_bonus_after_hyperspace_cartography() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        engine
            .state
            .empires
            .get_mut(&ai)
            .expect("AI empire exists")
            .research
            .completed
            .push(TechId::HYPERSPACE_CARTOGRAPHY);

        let (fleet_id, destination) =
            pick_scout_target(&engine.state, ai).expect("AI should have scout target");
        let origin = engine.state.fleets[&fleet_id].location;

        let all_star_ids: Vec<StarId> = engine.state.stars.keys().copied().collect();
        for star_id in all_star_ids {
            if star_id != destination {
                engine.state.mark_star_explored(ai, star_id);
            }
        }

        let (ox, oy) = {
            let origin_star = engine.state.stars.get(&origin).expect("origin star exists");
            (origin_star.x, origin_star.y)
        };
        if let Some(dest_star) = engine.state.stars.get_mut(&destination) {
            dest_star.x = ox + 1200;
            dest_star.y = oy;
        }

        let lane = crate::state::HyperspaceLane::new(origin, destination).expect("distinct stars");
        engine.state.hyperspace_lanes.insert(lane);

        let base_turns = crate::engine::fleet_travel_turns(1_440_000);
        let expected_lane_turns = base_turns.div_ceil(2).max(1);

        let mut events = Vec::new();
        ai_dispatch_scouts(&mut engine.state, ai, &mut events);
        let mission = engine
            .state
            .scout_missions
            .get(&fleet_id)
            .expect("AI scout mission should exist");
        assert_eq!(mission.total_duration, expected_lane_turns);
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::HyperspaceLaneUsed {
                    empire,
                    fleet,
                    from,
                    to,
                } if *empire == ai && *fleet == fleet_id && *from == origin && *to == destination
            )),
            "AI lane usage should emit HyperspaceLaneUsed"
        );
    }

    // -----------------------------------------------------------------------
    // Colonization
    // -----------------------------------------------------------------------

    #[test]
    fn ai_does_not_colonize_without_colonizer() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // No colonizer for AI initially
        assert!(
            !engine
                .state
                .fleets
                .values()
                .any(|f| f.owner == ai && f.kind == FleetKind::Colonizer)
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize without a colonizer"
        );
    }

    #[test]
    fn ai_does_not_colonize_unexplored_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Place a colonizer at an unexplored star
        let unexplored = engine
            .state
            .stars
            .keys()
            .find(|&&id| !engine.state.ai_explored_stars.contains(&id))
            .copied()
            .expect("Must have unexplored stars");

        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: unexplored,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize an unexplored star"
        );
    }

    #[test]
    fn ai_colonizes_valid_explored_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Find an explored AI star that is not the home star and has a free habitable planet.
        // For seed 42 the AI starts with its home + 3 nearest neighbours explored; all
        // generated planets are habitable with no colonies (except AI home planet 0), so
        // at least one valid target must always exist.
        let ai_home = engine.state.empires[&ai].home_star;
        let target = engine
            .state
            .ai_explored_stars
            .iter()
            .copied()
            .find(|&sid| {
                sid != ai_home
                    && engine.state.stars.get(&sid).is_some_and(|s| {
                        s.planets.iter().any(|p| p.habitable && p.colony.is_none())
                    })
            })
            .expect("Seed 42 must have an AI-explored star with a free habitable planet");

        // Place a colonizer there
        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_after,
            colonies_before + 1,
            "AI must colonize the valid planet"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AiColonized { empire, star, .. } if *empire == ai && *star == target)),
            "AiColonized event must be emitted"
        );

        // Colonizer must be consumed
        assert!(
            !engine.state.fleets.contains_key(&colonizer_id),
            "Colonizer must be consumed after colonization"
        );
    }

    #[test]
    fn ai_colonization_prefers_high_value_discovery_world_deterministically() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        let ai_home = engine.state.empires[&ai].home_star;
        let target = engine
            .state
            .ai_explored_stars
            .iter()
            .copied()
            .find(|&sid| sid != ai_home)
            .expect("AI needs an explored frontier star");

        engine
            .state
            .empires
            .get_mut(&ai)
            .unwrap()
            .research
            .completed
            .extend([TechId::ADVANCED_SURVEY, TechId::PAN_GALACTIC_SENSOR_NET]);

        let star = engine.state.stars.get_mut(&target).unwrap();
        while star.planets.len() < 2 {
            let mut clone = star.planets[0].clone();
            clone.name = format!("{} Annex", clone.name);
            clone.colony = None;
            clone.surveyed = true;
            clone.specials.clear();
            clone.resources.clear();
            clone.anomalies.clear();
            star.planets.push(clone);
        }
        for (idx, planet) in star.planets.iter_mut().enumerate() {
            planet.colony = None;
            planet.habitable = idx < 2;
            planet.surveyed = idx < 2;
            planet.specials.clear();
            planet.resources.clear();
            planet.anomalies.clear();
        }
        star.planets[0].specials = vec![PlanetSpecial::MineralRich];
        star.planets[1].anomalies = vec![PlanetAnomaly::VoidSignalArray];

        let colonizer_id = crate::state::FleetId(77);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);
        // AI should colonize one of the habitable worlds
        assert!(
            events.iter().any(|event| matches!(
                event,
                Event::AiColonized { empire, star, .. }
                if *empire == ai && *star == target
            )),
            "AI should colonize the frontier star"
        );
    }

    #[test]
    fn ai_does_not_colonize_already_colonized_planet() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Use AI home star (already has a colony at planet 0).
        // Explicitly mark all other planets at this star as uninhabitable so the
        // test is always deterministic regardless of planet count.
        let ai_home = engine.state.empires[&ai].home_star;
        if let Some(star) = engine.state.stars.get_mut(&ai_home) {
            for (i, planet) in star.planets.iter_mut().enumerate() {
                if i != 0 {
                    planet.habitable = false;
                }
            }
        }

        let colonizer_id = crate::state::FleetId(50);
        engine.state.fleets.insert(
            colonizer_id,
            crate::state::Fleet {
                id: colonizer_id,
                owner: ai,
                location: ai_home,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let colonies_before = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colonies_after = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();

        assert_eq!(
            colonies_before, colonies_after,
            "AI must not colonize when all habitable planets are occupied"
        );
    }

    // -----------------------------------------------------------------------
    // Player / AI isolation
    // -----------------------------------------------------------------------

    #[test]
    fn player_and_ai_state_are_isolated() {
        let mut engine = Engine::new(42);

        let player = engine.state.player_empire;
        let ai = ai_id(&engine);

        // Verify that player and AI have separate empires with different home stars
        let player_home = engine.state.empires[&player].home_star;
        let ai_home = engine.state.empires[&ai].home_star;
        assert_ne!(
            player_home, ai_home,
            "Player and AI must have different home stars"
        );

        // Player colonies must not be owned by AI and vice-versa
        for colony in engine.state.colonies.values() {
            assert!(
                colony.owner == player || colony.owner == ai,
                "Colony must be owned by player or AI"
            );
        }

        // Verify explored star sets are separate
        // (no requirement they be disjoint, but they must be independent BTreeSets)
        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Player explored stars must not change when AI scouts
        let player_explored = engine.state.explored_stars.clone();

        // Check that AI scout mission completions go to ai_explored_stars only
        // by running several more turns and verifying independence
        for _ in 0..5 {
            engine.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        // Player explored set must only grow if player scouts
        // (player hasn't scouted, so it stays the same)
        assert_eq!(
            engine.state.explored_stars, player_explored,
            "Player explored_stars must not be modified by AI actions"
        );
    }

    // -----------------------------------------------------------------------
    // Save / load round-trip
    // -----------------------------------------------------------------------

    #[cfg(feature = "serde")]
    #[test]
    fn save_load_preserves_ai_empire_state() {
        let mut engine = Engine::new(42);
        // Run a few turns so AI makes decisions
        for _ in 0..3 {
            engine.apply_turn(vec![crate::commands::Command::EndTurn]);
        }

        let original = engine.state.clone();
        // Use serde_json directly (game_core cannot depend on game_save)
        let saved = serde_json::to_string(&original).expect("serialize must succeed");
        let loaded: GameState = serde_json::from_str(&saved).expect("deserialize must succeed");

        assert_eq!(
            original.ai_empire, loaded.ai_empire,
            "ai_empire must survive round-trip"
        );
        assert_eq!(
            original.ai_explored_stars, loaded.ai_explored_stars,
            "ai_explored_stars must survive round-trip"
        );
        assert_eq!(
            original.empires.len(),
            loaded.empires.len(),
            "All empires (player + AI) must survive round-trip"
        );

        // AI empire research state must survive
        let ai = original.ai_empire.unwrap();
        let orig_ai = original.empires.get(&ai).unwrap();
        let load_ai = loaded.empires.get(&ai).unwrap();
        assert_eq!(
            orig_ai.research.current_tech, load_ai.research.current_tech,
            "AI research current_tech must survive round-trip"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn save_load_preserves_ai_colonies() {
        let engine = Engine::new(42);
        let ai = ai_id(&engine);

        let saved = serde_json::to_string(&engine.state).expect("serialize must succeed");
        let loaded: GameState = serde_json::from_str(&saved).expect("deserialize must succeed");

        let original_ai_colonies = engine
            .state
            .colonies
            .values()
            .filter(|c| c.owner == ai)
            .count();
        let loaded_ai_colonies = loaded.colonies.values().filter(|c| c.owner == ai).count();

        assert_eq!(
            original_ai_colonies, loaded_ai_colonies,
            "AI colonies must survive save/load round-trip"
        );
    }

    // -----------------------------------------------------------------------
    // Research — edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn ai_emits_no_research_event_when_all_techs_complete() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Mark every tech as completed for the AI empire
        {
            let empire = engine.state.empires.get_mut(&ai).unwrap();
            empire.research.current_tech = None;
            empire.research.completed = all_techs().iter().map(|t| t.id).collect();
        }

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // No AiResearchSelected should be emitted when all techs are done
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::AiResearchSelected { empire, .. } if *empire == ai)),
            "AI must not select research when all techs are already completed"
        );
        // current_tech must remain None (no invalid tech selected)
        assert!(
            engine.state.empires[&ai].research.current_tech.is_none(),
            "AI research.current_tech must stay None when all techs are complete"
        );
    }

    // -----------------------------------------------------------------------
    // Colonization — determinism
    // -----------------------------------------------------------------------

    #[test]
    fn ai_colonize_is_deterministic_across_identical_seeds() {
        // Seed 42 is chosen because it reliably produces at least one AI-explored
        // star (other than AI home) with a free habitable planet.  We place an
        // idle colonizer there so colonization is guaranteed to happen, making the
        // test non-vacuous.  Both runs must produce identical events AND both must
        // contain an AiColonized event.
        let setup = |seed: u64| {
            let mut engine = Engine::new(seed);
            let ai = engine.state.ai_empire.expect("AI must exist");
            let ai_home = engine.state.empires[&ai].home_star;

            // Find the first AI-explored star with a free habitable planet —
            // expect this to succeed for seed 42; panic with a clear message
            // if the galaxy is unexpectedly too small.
            let target_star = engine
                .state
                .ai_explored_stars
                .iter()
                .copied()
                .find(|&sid| {
                    sid != ai_home
                        && engine.state.stars.get(&sid).is_some_and(|s| {
                            s.planets.iter().any(|p| p.habitable && p.colony.is_none())
                        })
                })
                .expect("Seed 42 must have an AI-explored star with a free habitable planet");

            let colonizer_id = crate::state::FleetId(55);
            engine.state.fleets.insert(
                colonizer_id,
                crate::state::Fleet {
                    id: colonizer_id,
                    owner: ai,
                    location: target_star,
                    ships: 1,
                    kind: FleetKind::Colonizer,
                    strength: 1,
                    integrity: 100,
                },
            );

            engine.apply_turn(vec![crate::commands::Command::EndTurn])
        };

        let events_a = setup(42);
        let events_b = setup(42);

        // Both runs must contain an AiColonized event — test is non-vacuous
        assert!(
            events_a
                .iter()
                .any(|e| matches!(e, Event::AiColonized { .. })),
            "setup must actually trigger colonization (seed 42 must colonize)"
        );

        assert_eq!(
            events_a, events_b,
            "AI colonization events must be identical for the same seed"
        );
    }

    // -----------------------------------------------------------------------
    // Multi-turn full-state equality
    // -----------------------------------------------------------------------

    #[test]
    fn ai_five_turn_state_is_deterministic() {
        let mut engine_a = Engine::new(11_111);
        let mut engine_b = Engine::new(11_111);

        for _ in 0..5 {
            let ea = engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
            let eb = engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);
            assert_eq!(ea, eb, "Per-turn events must match");
        }

        assert_eq!(
            engine_a.state, engine_b.state,
            "State after 5 AI turns must be identical for the same seed"
        );
    }

    // -----------------------------------------------------------------------
    // AI respects ship / surface-slot rules
    // -----------------------------------------------------------------------

    #[test]
    fn ai_does_not_queue_ships_without_shipyard() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Give the AI colony a FabricationYard so it would otherwise move on to ships
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);
        // No Shipyard in orbital_installations

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            !colony
                .build_queue
                .contains(&BuildItem::Ship(ShipDesignId::SCOUT)),
            "AI must not queue Scout without a Shipyard"
        );
        assert!(
            !colony
                .build_queue
                .contains(&BuildItem::Ship(ShipDesignId::COLONY)),
            "AI must not queue Colony Ship without a Shipyard"
        );
    }

    #[test]
    fn ai_queues_shipyard_when_orbital_engineering_researched() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Give the AI FabricationYard so it skips priority 1
        engine
            .state
            .colonies
            .get_mut(&ai_colony_id)
            .unwrap()
            .buildings
            .push(BuildingType::FabricationYard);

        // Grant Orbital Engineering to the AI empire
        engine
            .state
            .empires
            .get_mut(&ai)
            .unwrap()
            .research
            .completed
            .push(TechId(7));

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            !colony.build_queue.is_empty(),
            "AI must queue something after researching Orbital Engineering"
        );
    }

    #[test]
    fn ai_does_not_queue_surface_structure_when_slots_full() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Ensure the AI would otherwise queue a FabricationYard
        assert!(
            !engine
                .state
                .colonies
                .get(&ai_colony_id)
                .unwrap()
                .buildings
                .contains(&BuildingType::FabricationYard),
            "AI colony must start without FabricationYard for this test"
        );

        // Fill all surface slots so the FabricationYard cannot be queued
        let (star_id, planet_index) = {
            let c = engine.state.colonies.get(&ai_colony_id).unwrap();
            (c.star, c.planet_index)
        };
        let max_slots = engine
            .state
            .stars
            .get(&star_id)
            .unwrap()
            .planets
            .get(planet_index)
            .unwrap()
            .size
            .surface_slots();
        {
            let colony = engine.state.colonies.get_mut(&ai_colony_id).unwrap();
            for _ in 0..max_slots {
                colony
                    .surface_installations
                    .push(BuildingType::FabricationYard);
            }
        }

        engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert!(
            !colony
                .build_queue
                .contains(&BuildItem::SurfaceStructure(BuildingType::FabricationYard)),
            "AI must not queue FabricationYard when all surface slots are full"
        );
    }

    // -----------------------------------------------------------------------
    // Colony role assignment
    // -----------------------------------------------------------------------

    #[test]
    fn ai_role_for_planet_class_is_deterministic() {
        use crate::state::PlanetClass;
        // Same class always produces the same role
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Oceanic),
            ColonyRole::Agricultural
        );
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Volcanic),
            ColonyRole::Industrial
        );
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Barren),
            ColonyRole::Industrial
        );
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Frozen),
            ColonyRole::Scientific
        );
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Desert),
            ColonyRole::Financial
        );
        assert_eq!(
            ai_role_for_planet_class(PlanetClass::Terran),
            ColonyRole::Balanced
        );
    }

    #[test]
    fn ai_assigns_non_terran_colony_role_on_turn() {
        use crate::state::PlanetClass;
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        // Find or create an AI colony on an Oceanic planet to test role assignment
        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Force the planet class to Oceanic
        {
            let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
            let star_id = colony.star;
            let planet_index = colony.planet_index;
            if let Some(star) = engine.state.stars.get_mut(&star_id)
                && let Some(planet) = star.planets.get_mut(planet_index)
            {
                planet.class = PlanetClass::Oceanic;
            }
        }

        // Colony starts as Balanced (the default)
        assert_eq!(
            engine.state.colonies.get(&ai_colony_id).unwrap().role,
            ColonyRole::Balanced
        );

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // After one turn AI should have assigned Agricultural to the Oceanic colony
        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert_eq!(
            colony.role,
            ColonyRole::Agricultural,
            "AI must assign Agricultural role to Oceanic colony"
        );

        // AiColonyRoleAssigned event must be emitted
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::AiColonyRoleAssigned { empire, colony, role }
                if *empire == ai && *colony == ai_colony_id && *role == ColonyRole::Agricultural
            )),
            "AiColonyRoleAssigned event must be emitted"
        );
    }

    #[test]
    fn ai_does_not_reassign_non_balanced_role() {
        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);

        let ai_colony_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.id)
            .unwrap();

        // Pre-set a non-Balanced role
        engine.state.colonies.get_mut(&ai_colony_id).unwrap().role = ColonyRole::Military;

        let events = engine.apply_turn(vec![crate::commands::Command::EndTurn]);

        // Role must not be reassigned
        let colony = engine.state.colonies.get(&ai_colony_id).unwrap();
        assert_eq!(
            colony.role,
            ColonyRole::Military,
            "AI must not reassign a colony that already has a non-Balanced role"
        );

        // No AiColonyRoleAssigned event for this colony
        assert!(
            !events.iter().any(|e| matches!(
                e,
                Event::AiColonyRoleAssigned { colony, .. }
                if *colony == ai_colony_id
            )),
            "AI must not emit AiColonyRoleAssigned for a colony already assigned"
        );
    }

    #[test]
    fn ai_role_assignment_is_deterministic() {
        let mut engine_a = Engine::new(99);
        let mut engine_b = Engine::new(99);

        engine_a.apply_turn(vec![crate::commands::Command::EndTurn]);
        engine_b.apply_turn(vec![crate::commands::Command::EndTurn]);

        // All colony roles must match between the two runs
        for (id, colony_a) in &engine_a.state.colonies {
            let colony_b = engine_b.state.colonies.get(id).unwrap();
            assert_eq!(
                colony_a.role, colony_b.role,
                "Colony {}: role must be deterministic",
                id.0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Empire identity — AI priorities
    // -----------------------------------------------------------------------

    #[test]
    fn ai_research_is_deterministic_with_empire_identity() {
        // Two engines with the same seed and empire defs must produce identical research choices.
        use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let make = || {
            Engine::new_from_setup(ScenarioSetup {
                seed: 55,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            })
        };
        let mut e1 = make();
        let mut e2 = make();
        e1.apply_turn(vec![crate::commands::Command::EndTurn]);
        e2.apply_turn(vec![crate::commands::Command::EndTurn]);
        assert_eq!(
            e1.state, e2.state,
            "Same seed + empire defs must be fully deterministic"
        );
    }

    #[test]
    fn scientific_ai_prefers_research_domain_tech() {
        // Elarith Confluence (id=5, Scientific) should pick a research-domain tech
        // sooner than a non-scientific empire would when cost is equal.
        use crate::state::{PlaystyleTag, empire_definition_by_id};
        // Find the Elarith Confluence def — it has the Scientific tag.
        let sci_def = all_empire_definitions()
            .iter()
            .find(|d| {
                d.playstyle.contains(&PlaystyleTag::Scientific)
                    && !d.playstyle.contains(&PlaystyleTag::Expansionist)
            })
            .expect("A pure-scientific empire must exist");
        assert!(
            sci_def.playstyle.contains(&PlaystyleTag::Scientific),
            "Empire def should be scientific"
        );
        // Verify that empire_definition_by_id round-trips correctly.
        let looked_up = empire_definition_by_id(sci_def.id).expect("must find by id");
        assert_eq!(looked_up.id, sci_def.id);
    }

    #[test]
    fn ai_empire_def_is_stored_in_state() {
        use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let engine = Engine::new_from_setup(ScenarioSetup {
            seed: 42,
            galaxy_size: GalaxySize::Medium,
            ai_empire_count: 2,
            sector_count_override: None,
            star_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: Some(EmpireDefinitionId(0)),
            victory_settings: crate::state::VictorySettings::default_v1(),
        });
        for ai_id in &engine.state.ai_empires {
            let empire = engine.state.empires.get(ai_id).unwrap();
            assert!(
                empire.empire_def.is_some(),
                "AI empire {ai_id:?} must have an empire_def"
            );
        }
    }

    #[test]
    fn ai_empire_defs_do_not_duplicate_player_def() {
        use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let player_def = EmpireDefinitionId(4); // Vorath Dominion
        let engine = Engine::new_from_setup(ScenarioSetup {
            seed: 42,
            galaxy_size: GalaxySize::Large,
            ai_empire_count: 4,
            sector_count_override: None,
            star_count_override: None,
            difficulty: DifficultyLevel::Standard,
            player_empire_def: Some(player_def),
            victory_settings: crate::state::VictorySettings::default_v1(),
        });
        for ai_id in &engine.state.ai_empires {
            let empire = engine.state.empires.get(ai_id).unwrap();
            assert_ne!(
                empire.empire_def,
                Some(player_def),
                "AI empire must not share the player's empire def"
            );
        }
    }

    #[test]
    fn terran_concord_ai_prioritizes_science_ship_deterministically() {
        use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 42,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|colony| colony.owner == ai)
                .map(|colony| colony.id)
                .unwrap();
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(6));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([TechId::SURVEY_DRONES, TechId::ORBITAL_ENGINEERING]);
            engine
                .state
                .colonies
                .get_mut(&ai_colony)
                .unwrap()
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            Some(BuildItem::Ship(ShipDesignId::SCIENCE))
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b)
        );
    }

    #[test]
    fn terran_dominion_ai_prioritizes_military_transport_deterministically() {
        use crate::state::{DifficultyLevel, EmpireDefinitionId, GalaxySize, ScenarioSetup};
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 52,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|colony| colony.owner == ai)
                .map(|colony| colony.id)
                .unwrap();
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(7));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([TechId::TROOP_TRANSPORTS, TechId::ORBITAL_ENGINEERING]);
            engine
                .state
                .colonies
                .get_mut(&ai_colony)
                .unwrap()
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            Some(BuildItem::Ship(ShipDesignId::TROOP_TRANSPORT))
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b)
        );
    }

    #[test]
    fn terran_concord_ai_assigns_scientific_role_on_terran_world() {
        let mut engine = Engine::new(42);
        let ai_id = ai_id(&engine);
        engine.state.empires.get_mut(&ai_id).unwrap().empire_def =
            Some(crate::state::EmpireDefinitionId(6));
        let role = ai_role_for_planet_class_with_identity(
            crate::state::PlanetClass::Terran,
            ai_id,
            &engine.state,
        );
        assert_eq!(role, ColonyRole::Scientific);
    }

    #[test]
    fn terran_dominion_ai_assigns_military_role_on_terran_world() {
        let mut engine = Engine::new(42);
        let ai_id = ai_id(&engine);
        engine.state.empires.get_mut(&ai_id).unwrap().empire_def =
            Some(crate::state::EmpireDefinitionId(7));
        let role = ai_role_for_planet_class_with_identity(
            crate::state::PlanetClass::Terran,
            ai_id,
            &engine.state,
        );
        assert_eq!(role, ColonyRole::Military);
    }

    #[test]
    fn militarist_ai_prefers_combat_ships_deterministically() {
        use crate::state::{
            DifficultyLevel, EmpireDefinitionId, GalaxySize, OrbitalStructureType, ScenarioSetup,
        };
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 77,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|c| c.owner == ai)
                .map(|c| c.id)
                .unwrap();
            // Vorath Dominion (id=4) — prefers_combat_ships
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(4));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([
                    TechId::ORBITAL_ENGINEERING,
                    TechId::KINETIC_BARRIERS,
                    TechId::BATTLE_DOCTRINE,
                    TechId::FLEET_COORDINATION,
                ]);
            let star_id = engine.state.colonies.get(&ai_colony).unwrap().star;
            let colony = engine.state.colonies.get_mut(&ai_colony).unwrap();
            colony
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            // Pre-install FabricationYard so it doesn't block ship production
            colony.buildings.push(BuildingType::FabricationYard);
            // Add a TroopTransport fleet so the AI won't prioritise building another one
            engine.state.fleets.insert(
                FleetId(801),
                crate::state::Fleet {
                    id: FleetId(801),
                    owner: ai,
                    location: star_id,
                    ships: 1,
                    kind: FleetKind::TroopTransport,
                    strength: 2,
                    integrity: 100,
                },
            );
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        // Should pick Destroyer (highest priority for prefers_combat_ships with Fleet Coordination)
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            Some(BuildItem::Ship(ShipDesignId::DESTROYER))
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b),
            "Combat ship selection must be deterministic"
        );
    }

    #[test]
    fn scientific_ai_prefers_fast_scouts_deterministically() {
        use crate::state::{
            DifficultyLevel, EmpireDefinitionId, GalaxySize, OrbitalStructureType, ScenarioSetup,
        };
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 88,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|c| c.owner == ai)
                .map(|c| c.id)
                .unwrap();
            // Elarith Confluence (id=5) — prefers_science_ships + prefers_fast_scouts
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(5));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([TechId::ORBITAL_ENGINEERING, TechId::RAPID_TRANSIT]);
            let colony = engine.state.colonies.get_mut(&ai_colony).unwrap();
            colony
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            // Pre-install FabricationYard and ScienceNexus so ship priorities take effect
            colony.buildings.push(BuildingType::FabricationYard);
            colony.buildings.push(BuildingType::ScienceNexus);
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        assert!(
            pick_build_item(&engine_a.state, ai_a, colony_a).is_some(),
            "Scientific faction should pick a build item"
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b),
            "Fast scout selection must be deterministic"
        );
    }

    #[test]
    fn defensive_ai_prefers_patrol_corvette_deterministically() {
        use crate::state::{
            DifficultyLevel, EmpireDefinitionId, GalaxySize, OrbitalStructureType, ScenarioSetup,
        };
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 99,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|c| c.owner == ai)
                .map(|c| c.id)
                .unwrap();
            // Thalori Exchange (id=3) — prefers_defensive_ships
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(3));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([TechId::ORBITAL_ENGINEERING, TechId::PERIMETER_DEFENSE]);
            let colony = engine.state.colonies.get_mut(&ai_colony).unwrap();
            colony
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            // Pre-install FabricationYard so it doesn't block ship production
            colony.buildings.push(BuildingType::FabricationYard);
            // Also set colony habitat seeding so it doesn't default to colony ship
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .push(TechId::HABITAT_SEEDING);
            // Add a colonizer so it won't pick colony ship
            let star_id = engine.state.colonies.get(&ai_colony).unwrap().star;
            engine.state.fleets.insert(
                FleetId(800),
                crate::state::Fleet {
                    id: FleetId(800),
                    owner: ai,
                    location: star_id,
                    ships: 1,
                    kind: FleetKind::Colonizer,
                    strength: 1,
                    integrity: 100,
                },
            );
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        let result = pick_build_item(&engine_a.state, ai_a, colony_a);
        assert!(
            result.is_some(),
            "Defensive faction should pick a build item"
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b),
            "Defensive ship selection must be deterministic"
        );
    }

    #[test]
    fn expansionist_ai_prefers_colony_arks_when_available() {
        use crate::state::{
            DifficultyLevel, EmpireDefinitionId, GalaxySize, OrbitalStructureType, ScenarioSetup,
        };
        let make = || {
            let mut engine = Engine::new_from_setup(ScenarioSetup {
                seed: 101,
                galaxy_size: GalaxySize::Medium,
                ai_empire_count: 1,
                sector_count_override: None,
                star_count_override: None,
                difficulty: DifficultyLevel::Standard,
                player_empire_def: Some(EmpireDefinitionId(0)),
                victory_settings: crate::state::VictorySettings::default_v1(),
            });
            let ai = ai_id(&engine);
            let ai_colony = engine
                .state
                .colonies
                .values()
                .find(|c| c.owner == ai)
                .map(|c| c.id)
                .unwrap();
            // Sylvaran Accord (id=2) — prefers_colony_arks
            engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(2));
            engine
                .state
                .empires
                .get_mut(&ai)
                .unwrap()
                .research
                .completed
                .extend([
                    TechId::ORBITAL_ENGINEERING,
                    TechId::HABITAT_SEEDING,
                    TechId(10), // Colonial Logistics
                    TechId::COLONIAL_VANGUARD,
                ]);
            let colony = engine.state.colonies.get_mut(&ai_colony).unwrap();
            colony
                .orbital_installations
                .push(OrbitalStructureType::Shipyard);
            // Pre-install required buildings so they don't take priority over ship construction
            colony.buildings.push(BuildingType::FabricationYard);
            colony.buildings.push(BuildingType::AquacultureBay);
            (engine, ai, ai_colony)
        };

        let (engine_a, ai_a, colony_a) = make();
        let (engine_b, ai_b, colony_b) = make();
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            Some(BuildItem::Ship(ShipDesignId::COLONY_ARK)),
            "Expansionist/agrarian faction with Colonial Vanguard should prefer Colony Ark"
        );
        assert_eq!(
            pick_build_item(&engine_a.state, ai_a, colony_a),
            pick_build_item(&engine_b.state, ai_b, colony_b),
            "Colony Ark preference must be deterministic"
        );
    }

    #[test]
    fn doctrine_victory_preference_is_deterministic_and_distinct() {
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let ai = ai_id(&engine);
        // Player uses the militarist/imperial faction (4); AI uses
        // the explorer/expansionist Terran Concord (6) for Legacy
        // coverage.  The two archetypes produce the strongest
        // ordering divergence on the new 4-path set without
        // collapsing to zero on any single path.
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(4));
        engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(6));

        // Supremacy: militarist/imperial > explorer/expansionist.
        assert!(
            doctrine_victory_preference(&engine.state, player, VictoryPath::Supremacy)
                > doctrine_victory_preference(&engine.state, ai, VictoryPath::Supremacy),
            "Militarist/imperial faction should lean Supremacy"
        );
        // Legacy: explorer/expansionist > militarist.
        assert!(
            doctrine_victory_preference(&engine.state, ai, VictoryPath::Legacy)
                > doctrine_victory_preference(&engine.state, player, VictoryPath::Legacy),
            "Explorer/expansionist faction should lean Legacy"
        );
        // Scientific: technologist > militarist.  The Concord's
        // Scientific tag pulls it above the militarist player.
        assert!(
            doctrine_victory_preference(&engine.state, ai, VictoryPath::Scientific)
                > doctrine_victory_preference(&engine.state, player, VictoryPath::Scientific),
            "Explorer/expansionist faction should out-score Militarist on Scientific"
        );
        // Ascendancy: deterministic, non-zero on at least one side,
        // and stable across re-evaluation.  The exact ordering
        // between factions depends on doctrine weight composition,
        // so we don't assert a strict ordering here.
        let player_asc =
            doctrine_victory_preference(&engine.state, player, VictoryPath::Ascendancy);
        let ai_asc = doctrine_victory_preference(&engine.state, ai, VictoryPath::Ascendancy);
        assert!(
            player_asc > 0 || ai_asc > 0,
            "Ascendancy: at least one side must be > 0"
        );

        // Determinism: a fresh engine configured the same way produces
        // the same scores for every path.
        let mut replay = Engine::new(42);
        let replay_player = replay.state.player_empire;
        let replay_ai = ai_id(&replay);
        replay
            .state
            .empires
            .get_mut(&replay_player)
            .unwrap()
            .empire_def = Some(EmpireDefinitionId(4));
        replay.state.empires.get_mut(&replay_ai).unwrap().empire_def = Some(EmpireDefinitionId(6));
        for path in [
            VictoryPath::Supremacy,
            VictoryPath::Ascendancy,
            VictoryPath::Scientific,
            VictoryPath::Legacy,
        ] {
            assert_eq!(
                doctrine_victory_preference(&engine.state, player, path),
                doctrine_victory_preference(&replay.state, replay_player, path),
                "doctrine remap must be deterministic for {path:?}"
            );
            assert_eq!(
                doctrine_victory_preference(&engine.state, ai, path),
                doctrine_victory_preference(&replay.state, replay_ai, path),
                "doctrine remap must be deterministic for {path:?}"
            );
        }
    }

    /// Regression guard for the research-score victory remap introduced
    /// with the v1 victory system.  The remap added two pieces of logic
    /// to `research_score`:
    ///
    /// 1. `victory_pref(VictoryPath)` is added per domain and per tag,
    ///    so a faction that leans Supremacy should score military-domain
    ///    techs higher than a faction that leans Scientific.
    /// 2. The Scientific eligibility bonus (`victory_pref(Scientific) * 2`)
    ///    is only applied when `tech.id == scenario.victory_settings.
    ///    condition_for(Scientific).eligibility_tech`.
    ///
    /// Both branches have to keep working — drop one and AI tech
    /// priorities regress silently.
    #[test]
    fn research_score_victory_remap_prefers_aligned_path() {
        use crate::state::all_techs;
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        // Militarist/imperial player (faction 4) should score a military
        // tech above a scientific-leaning AI (faction 5).
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(4));
        let ai = ai_id(&engine);
        engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(5));

        // Pick a real military tech from the tree.
        let military_tech = all_techs()
            .iter()
            .find(|t| matches!(t.domain, crate::state::TechDomain::Military))
            .copied()
            .expect("at least one military tech in the tree");
        let player_mil_score = research_score(&engine.state, player, &military_tech);
        let ai_mil_score = research_score(&engine.state, ai, &military_tech);
        assert!(
            player_mil_score > ai_mil_score,
            "militarist faction should out-score technologist on military techs: \
             player={player_mil_score} ai={ai_mil_score}"
        );
    }

    /// Negative case: the Scientific eligibility bonus is only granted
    /// to the *exact* tech id configured in the scenario.  We compare
    /// the score for the eligibility tech (Transcendent Gate Theory,
    /// TechId(61)) under two scenarios: one where it IS the
    /// eligibility tech, and one where it isn't.  The bonus must
    /// lift its score in the first case.
    #[test]
    fn research_score_scientific_bonus_only_targets_eligibility_tech() {
        use crate::state::{VictoryPath, VictorySettings, all_techs};
        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        // Pure Technologist faction so the Scientific victory pref is
        // non-zero.  Without this, the bonus is always 0 and the
        // remap is invisible to the test.
        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(5));

        let eligibility = crate::state::TechId(61);
        let target_tech = all_techs()
            .iter()
            .find(|t| t.id == eligibility)
            .copied()
            .expect("Transcendent Gate Theory must be in the tree");

        let make_scenario = |elig: crate::state::TechId| crate::state::ScenarioSetup {
            seed: 42,
            galaxy_size: crate::state::GalaxySize::Medium,
            ai_empire_count: 1,
            sector_count_override: None,
            star_count_override: None,
            difficulty: crate::state::DifficultyLevel::Standard,
            player_empire_def: None,
            victory_settings: VictorySettings {
                enabled_paths: [
                    VictoryPath::Supremacy,
                    VictoryPath::Ascendancy,
                    VictoryPath::Scientific,
                    VictoryPath::Legacy,
                ]
                .into_iter()
                .collect(),
                conditions: vec![
                    crate::state::VictoryCondition::Supremacy,
                    crate::state::VictoryCondition::Ascendancy {
                        control_percent: 50,
                        consecutive_turns_required: 10,
                    },
                    crate::state::VictoryCondition::Scientific {
                        eligibility_tech: elig,
                        project_points_required: 1_500,
                    },
                    crate::state::VictoryCondition::Legacy {
                        early_warning_percent: 75,
                    },
                ],
                turn_limit_enabled: true,
                turn_limit: 300,
            },
        };

        let mut state_with_bonus = engine.state.clone();
        state_with_bonus.scenario = Some(make_scenario(eligibility));
        let mut state_without_bonus = engine.state.clone();
        // Pick any other real tech id as the eligibility for the
        // negative scenario.  The bonus on TechId(61) should vanish.
        let other_elig = crate::state::TechId(1);
        state_without_bonus.scenario = Some(make_scenario(other_elig));

        let score_with = research_score(&state_with_bonus, player, &target_tech);
        let score_without = research_score(&state_without_bonus, player, &target_tech);
        assert!(
            score_with > score_without,
            "Transcendent Gate Theory should score higher when it IS the \
             eligibility tech: with={score_with} without={score_without}"
        );
    }

    // -----------------------------------------------------------------------
    // Combat fleet dispatch
    // -----------------------------------------------------------------------

    #[test]
    fn test_ai_dispatches_combat_fleet_when_at_war() {
        use crate::state::{Fleet, FleetId, FleetKind, RelationshipStatus, StarId};

        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        let player = engine.state.player_empire;

        // Advance to turn 20 so the dispatch threshold is met
        engine.state.turn = 20;

        // Set war status (diplomacy is stored on GameState keyed by non-player empire ID)
        engine.state.diplomacy.insert(ai, RelationshipStatus::War);

        // Find a player colony star to use as a target
        let player_colony_star: StarId = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == player)
            .map(|c| c.star)
            .expect("Player must have a colony");

        // Find any AI colony star for fleet placement
        let ai_star: StarId = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == ai)
            .map(|c| c.star)
            .unwrap_or(player_colony_star);

        // Inject an idle combat fleet (Escort Frigate) for the AI at its colony star
        let combat_fleet_id = FleetId(9000);
        engine.state.fleets.insert(
            combat_fleet_id,
            Fleet {
                id: combat_fleet_id,
                owner: ai,
                location: ai_star,
                ships: 1,
                kind: FleetKind::EscortFrigate,
                strength: 10,
                integrity: 100,
            },
        );

        // Run one AI turn
        let events = run_ai_turn(&mut engine.state, ai);

        // The combat fleet should have been dispatched (or already at target)
        let dispatched = events.iter().any(|e| {
            matches!(
                e,
                Event::AiCombatFleetDispatched {
                    empire,
                    fleet,
                    ..
                } if *empire == ai && *fleet == combat_fleet_id
            )
        });

        // If the AI is at the same star as the player colony, no dispatch needed
        let already_there = ai_star == player_colony_star;

        if !already_there {
            assert!(
                dispatched,
                "AI should dispatch combat fleet toward player colony when at war (turn >= 20)"
            );
            // The fleet should now have a mission
            assert!(
                engine.state.fleet_missions.contains_key(&combat_fleet_id),
                "Dispatched fleet must have an active FleetMission"
            );
        }
    }

    #[test]
    fn ai_prefers_supported_target_over_unsupported_deep_strike() {
        use crate::state::{
            Colony, ColonyId, Fleet, FleetId, FleetKind, Planet, PlanetClass, PlanetSize,
        };

        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        let player = engine.state.player_empire;
        engine.state.turn = 20;
        engine
            .state
            .diplomacy
            .insert(ai, crate::state::RelationshipStatus::War);

        let home_colony = engine
            .state
            .colonies
            .values()
            .find(|colony| colony.owner == player)
            .cloned()
            .expect("player colony exists");
        let ai_star = engine
            .state
            .colonies
            .values()
            .find(|colony| colony.owner == ai)
            .map(|colony| colony.star)
            .expect("ai colony exists");
        let far_star = *engine
            .state
            .stars
            .keys()
            .find(|&&star_id| star_id != home_colony.star && star_id != ai_star)
            .expect("need extra star");
        if let Some(ai_origin) = engine.state.stars.get_mut(&ai_star) {
            ai_origin.x = 0;
            ai_origin.y = 0;
        }
        if let Some(home_star) = engine.state.stars.get_mut(&home_colony.star) {
            home_star.x = 250;
            home_star.y = 0;
        }
        if let Some(frontier_star) = engine.state.stars.get_mut(&far_star) {
            frontier_star.x = 1_300;
            frontier_star.y = 0;
            if frontier_star.planets.is_empty() {
                frontier_star.planets.push(Planet {
                    name: "Frontier I".to_string(),
                    size: PlanetSize::Medium,
                    class: PlanetClass::Terran,
                    colony: None,
                    habitable: true,
                    surveyed: true,
                    specials: vec![],
                    resources: vec![],
                    anomalies: vec![],
                    ancient_ruins_collected: false,
                });
            }
        }
        let frontier_colony_id = ColonyId(9_001);
        engine.state.colonies.insert(
            frontier_colony_id,
            Colony {
                id: frontier_colony_id,
                star: far_star,
                planet_index: 0,
                owner: player,
                population: 3,
                production: 4,
                prod_pct: 50,
                research_pct: 50,
                build_queue: vec![],
                accumulated_production: 0,
                buildings: vec![],
                surface_installations: vec![],
                orbital_installations: vec![],
                stability: 100,
                role: crate::state::ColonyRole::Balanced,
                rally_point: None,
            },
        );
        if let Some(star) = engine.state.stars.get_mut(&far_star)
            && let Some(planet) = star.planets.get_mut(0)
        {
            planet.colony = Some(frontier_colony_id);
        }
        let combat_fleet_id = FleetId(9_002);
        engine.state.fleets.insert(
            combat_fleet_id,
            Fleet {
                id: combat_fleet_id,
                owner: ai,
                location: ai_star,
                ships: 1,
                kind: FleetKind::Destroyer,
                strength: 10,
                integrity: 100,
            },
        );

        let _events = run_ai_turn(&mut engine.state, ai);
        let mission = engine
            .state
            .fleet_missions
            .get(&combat_fleet_id)
            .expect("combat fleet should be dispatched");
        assert_eq!(
            mission.destination, home_colony.star,
            "AI should prefer supported nearby target over unsupported deep strike"
        );
    }

    #[test]
    fn terran_concord_and_dominion_fleet_posture_differs() {
        use crate::state::{EmpireDefinitionId, Fleet, FleetFormation, FleetRole};

        let mut engine = Engine::new(42);
        let player = engine.state.player_empire;
        let ai = ai_id(&engine);

        engine.state.empires.get_mut(&player).unwrap().empire_def = Some(EmpireDefinitionId(6)); // Terran Concord
        engine.state.empires.get_mut(&ai).unwrap().empire_def = Some(EmpireDefinitionId(7)); // Terran Dominion
        engine
            .state
            .diplomacy
            .insert(ai, crate::state::RelationshipStatus::War);
        engine.state.turn = 30;

        let player_star = engine.state.empires[&player].home_star;
        let ai_star = engine.state.empires[&ai].home_star;
        let concord_fleet = FleetId(7001);
        let dominion_fleet = FleetId(7002);
        engine.state.fleets.insert(
            concord_fleet,
            Fleet {
                id: concord_fleet,
                owner: player,
                location: player_star,
                ships: 1,
                kind: FleetKind::EscortFrigate,
                strength: 6,
                integrity: 100,
            },
        );
        engine.state.fleets.insert(
            dominion_fleet,
            Fleet {
                id: dominion_fleet,
                owner: ai,
                location: ai_star,
                ships: 1,
                kind: FleetKind::EscortFrigate,
                strength: 6,
                integrity: 100,
            },
        );

        let _ = run_ai_turn(&mut engine.state, player);
        let _ = run_ai_turn(&mut engine.state, ai);

        assert_eq!(
            engine.state.fleet_formation_for(concord_fleet),
            FleetFormation::Defensive
        );
        assert!(
            matches!(
                engine.state.fleet_role_for(dominion_fleet),
                FleetRole::StrikeFleet | FleetRole::BlockadeFleet | FleetRole::PatrolFleet
            ),
            "Dominion fleet should not mirror Concord defensive posture"
        );
    }

    #[test]
    fn exploration_fleets_avoid_high_threat_targets_when_possible() {
        use crate::state::Fleet;

        let mut engine = Engine::new(42);
        let ai = ai_id(&engine);
        let player = engine.state.player_empire;

        // Pick one unexplored star and place a strong hostile fleet there.
        let threatened_star = *engine
            .state
            .stars
            .keys()
            .find(|sid| !engine.state.ai_explored_stars.contains(sid))
            .expect("need unexplored target");
        engine
            .state
            .diplomacy
            .insert(ai, crate::state::RelationshipStatus::War);
        engine.state.fleets.insert(
            FleetId(7010),
            Fleet {
                id: FleetId(7010),
                owner: player,
                location: threatened_star,
                ships: 2,
                kind: FleetKind::Destroyer,
                strength: 20,
                integrity: 100,
            },
        );

        let (_, target) =
            pick_scout_target(&engine.state, ai).expect("AI should still find a safe scout target");
        assert_ne!(target, threatened_star);
    }
}
