//! Galactic Dispatch — turn-based news bulletin system.
//!
//! A `GalacticDispatch` bulletin is generated each cadence turn (every
//! `DISPATCH_CADENCE` turns) or whenever urgent / historic events occur.
//! All generation is deterministic — same inputs always yield the same output.

use crate::state::{EmpireId, RelationshipStatus, StarId};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// How often (in turns) a dispatch is unconditionally emitted.
pub const DISPATCH_CADENCE: u32 = 5;

/// Maximum number of dispatch bulletins retained in `GameState::galactic_dispatches`.
pub const DISPATCH_MAX_HISTORY: usize = 10;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Category of a dispatch item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DispatchCategory {
    Exploration,
    Colonization,
    Research,
    Economy,
    Diplomacy,
    War,
    Blockades,
    Invasions,
    Trade,
    VictoryRace,
    MinorFactions,
}

/// Severity of a dispatch item (higher = more important).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DispatchSeverity {
    Notice,
    Notable,
    Urgent,
    Historic,
}

impl DispatchSeverity {
    /// Numeric key used for descending sort (Historic = 3, … Notice = 0).
    fn sort_key(self) -> u8 {
        match self {
            DispatchSeverity::Notice => 0,
            DispatchSeverity::Notable => 1,
            DispatchSeverity::Urgent => 2,
            DispatchSeverity::Historic => 3,
        }
    }
}

/// A single item in a Galactic Dispatch bulletin.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DispatchItem {
    pub category: DispatchCategory,
    pub severity: DispatchSeverity,
    pub headline: String,
    pub body: String,
    pub related_empire_id: Option<EmpireId>,
    pub related_star_id: Option<StarId>,
    pub related_planet_index: Option<usize>,
}

/// A full Galactic Dispatch bulletin.
///
/// `turn` is the 0-indexed *completed* turn that triggered this dispatch
/// (i.e. the value of `GameState::turn` *before* it was incremented).
/// The display turn shown to the player is `turn + 1`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GalacticDispatch {
    pub turn: u32,
    pub title: String,
    pub items: Vec<DispatchItem>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the player knows of `empire_id` (i.e. has made contact
/// or the empire *is* the player).
fn player_knows_empire(state: &crate::state::GameState, empire_id: EmpireId) -> bool {
    if empire_id == state.player_empire {
        return true;
    }
    matches!(
        state.diplomacy.get(&empire_id),
        Some(
            RelationshipStatus::Contacted
                | RelationshipStatus::Neutral
                | RelationshipStatus::Tense
                | RelationshipStatus::Hostile
                | RelationshipStatus::War
        )
    )
}

/// Returns the star's display name if the player has explored it, otherwise `None`.
///
/// Use this instead of a direct lookup to avoid revealing the location of
/// systems the player has not yet visited.
fn star_name_if_known(state: &crate::state::GameState, star_id: StarId) -> Option<&str> {
    if state.explored_stars.contains(&star_id) {
        state.stars.get(&star_id).map(|s| s.name.as_str())
    } else {
        None
    }
}

/// Returns the empire's display name, or `"Unknown Empire"` if not found.
fn empire_name_for_id(state: &crate::state::GameState, empire_id: EmpireId) -> &str {
    state
        .empires
        .get(&empire_id)
        .map(|e| e.name.as_str())
        .unwrap_or("Unknown Empire")
}

/// Simple constructor for a dispatch item.
fn item(
    category: DispatchCategory,
    severity: DispatchSeverity,
    headline: impl Into<String>,
    body: impl Into<String>,
    empire: Option<EmpireId>,
    star: Option<StarId>,
    planet: Option<usize>,
) -> DispatchItem {
    DispatchItem {
        category,
        severity,
        headline: headline.into(),
        body: body.into(),
        related_empire_id: empire,
        related_star_id: star,
        related_planet_index: planet,
    }
}

// ---------------------------------------------------------------------------
// Core generation logic
// ---------------------------------------------------------------------------

/// Generate a Galactic Dispatch from the completed turn's events and game state.
///
/// Returns `None` on non-cadence turns with no qualifying events.
/// Items are in a stable sorted order (severity desc, then category, then headline).
/// Public-information rules apply:
/// - Only empire details known to the player are surfaced
///   (`RelationshipStatus != Unknown`).
/// - Unsurveyed planet specials are never referenced.
pub fn generate_dispatch(
    completed_turn: u32,
    events: &[crate::events::Event],
    state: &crate::state::GameState,
) -> Option<GalacticDispatch> {
    use crate::events::Event;

    let mut items: Vec<DispatchItem> = Vec::new();

    for event in events {
        match event {
            // --- Exploration ---
            Event::SystemExplored { star } => {
                items.push(item(
                    DispatchCategory::Exploration,
                    DispatchSeverity::Notice,
                    "Survey Crews Chart New Frontier Worlds",
                    "Scout vessels have confirmed a new system entry in the star charts.",
                    None,
                    Some(*star),
                    None,
                ));
            }

            Event::PlanetSurveyCompleted { star, .. } => {
                items.push(item(
                    DispatchCategory::Exploration,
                    DispatchSeverity::Notice,
                    "Orbital Surveys Reveal New World Data",
                    "Survey teams have completed detailed orbital scans of a planetary body.",
                    None,
                    Some(*star),
                    None,
                ));
            }

            Event::AncientRuinsDiscovered { star, .. } => {
                if state.explored_stars.contains(star) {
                    items.push(item(
                        DispatchCategory::Research,
                        DispatchSeverity::Historic,
                        "Ancient Ruins Discovered — Archaeological Teams Mobilize",
                        "Pre-spacefaring relics have been unearthed, attracting scientific expeditions from across the empire.",
                        None,
                        Some(*star),
                        None,
                    ));
                }
            }

            // --- Colonization ---
            Event::ColonizationCompleted {
                empire,
                star,
                planet_index,
                colony: _,
                ..
            } => {
                let star_name = state
                    .stars
                    .get(star)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown System");
                items.push(item(
                    DispatchCategory::Colonization,
                    DispatchSeverity::Notable,
                    format!("Colonists Establish Foothold in {star_name} System"),
                    "A new colonial charter has been issued and settlers have made landfall.",
                    Some(*empire),
                    Some(*star),
                    Some(*planet_index),
                ));
            }

            Event::AiColonized {
                empire,
                star,
                planet_index,
                colony: _,
            } => {
                if player_knows_empire(state, *empire) {
                    let (headline, related_star) =
                        if let Some(star_name) = star_name_if_known(state, *star) {
                            (
                                format!("Colonists Establish Foothold in {star_name} System"),
                                Some(*star),
                            )
                        } else {
                            (
                                "Rival Empire Establishes Colony in Distant Territory".to_string(),
                                None,
                            )
                        };
                    items.push(item(
                        DispatchCategory::Colonization,
                        DispatchSeverity::Notable,
                        headline,
                        "A rival empire has established a new colonial presence.",
                        Some(*empire),
                        related_star,
                        Some(*planet_index),
                    ));
                } else {
                    items.push(item(
                        DispatchCategory::Colonization,
                        DispatchSeverity::Notable,
                        "Unknown Forces Establish Remote Colony",
                        "Unidentified vessels have been observed completing colonization operations at an uncharted location.",
                        None,
                        None,
                        None,
                    ));
                }
            }

            // --- Research ---
            Event::ResearchCompleted { tech } => {
                let tech_name = crate::state::tech_by_id(*tech)
                    .map(|t| t.name)
                    .unwrap_or("Unknown Technology");
                items.push(item(
                    DispatchCategory::Research,
                    DispatchSeverity::Notable,
                    format!("Researchers Announce Breakthrough in {tech_name}"),
                    "Imperial research teams have successfully concluded a major research programme.",
                    Some(state.player_empire),
                    None,
                    None,
                ));
            }

            Event::AiResearchSelected { empire, tech: _ } => {
                if player_knows_empire(state, *empire) {
                    let empire_name = empire_name_for_id(state, *empire);
                    items.push(item(
                        DispatchCategory::Research,
                        DispatchSeverity::Notice,
                        format!("{empire_name} Labs Redirect Research Priorities"),
                        "Intelligence sources indicate a change in rival research direction.",
                        Some(*empire),
                        None,
                        None,
                    ));
                }
            }

            // --- Combat ---
            Event::CombatResolved {
                star,
                empire_a,
                empire_b,
                ..
            } => {
                let player = state.player_empire;
                let player_involved = *empire_a == player || *empire_b == player;
                let a_known = player_knows_empire(state, *empire_a);
                let b_known = player_knows_empire(state, *empire_b);

                if !a_known && !b_known && !player_involved {
                    // Both completely unknown — skip
                } else {
                    let known_star = star_name_if_known(state, *star);
                    let (headline, severity) = if player_involved {
                        let h = if let Some(star_name) = known_star {
                            format!("Combat Reported in {star_name} Sector")
                        } else {
                            "Combat Reported in Contested Space".to_string()
                        };
                        (h, DispatchSeverity::Urgent)
                    } else if a_known && b_known {
                        let h = if let Some(star_name) = known_star {
                            format!("Combat Reported in {star_name} Sector")
                        } else {
                            "Combat Reported in Distant Space".to_string()
                        };
                        (h, DispatchSeverity::Notable)
                    } else {
                        let h = if let Some(star_name) = known_star {
                            format!("Unidentified Fleet Engages Forces Near {star_name}")
                        } else {
                            "Unidentified Fleet Engages Forces in Unknown Space".to_string()
                        };
                        (h, DispatchSeverity::Notable)
                    };
                    // Only expose the star if it is known to the player
                    let related_star = if known_star.is_some() {
                        Some(*star)
                    } else {
                        None
                    };
                    items.push(item(
                        DispatchCategory::War,
                        severity,
                        headline,
                        "Fleet engagement has been detected by long-range sensors.",
                        None,
                        related_star,
                        None,
                    ));
                }
            }

            // --- Blockades ---
            Event::BlockadeStarted {
                colony,
                star,
                by_empire,
            } => {
                let is_player_colony = state
                    .colonies
                    .get(colony)
                    .map(|c| c.owner == state.player_empire)
                    .unwrap_or(false);
                let empire_known = player_knows_empire(state, *by_empire);
                // The player knows the star only if they have explored it OR it's their own colony
                let star_known = is_player_colony || star_name_if_known(state, *star).is_some();
                let known_star_name = if star_known {
                    star_name_if_known(state, *star)
                        .or_else(|| state.stars.get(star).map(|s| s.name.as_str()))
                } else {
                    None
                };

                let headline = if is_player_colony || empire_known {
                    if let Some(star_name) = known_star_name {
                        format!("Hostile Vessels Impose Blockade at {star_name}")
                    } else {
                        "Hostile Vessels Impose Blockade on Colonial Outpost".to_string()
                    }
                } else {
                    "Unidentified Fleet Blockades Contested Colony".to_string()
                };
                // Only expose related IDs when the player can legitimately see them
                let related_empire = if empire_known { Some(*by_empire) } else { None };
                let related_star = if star_known { Some(*star) } else { None };
                items.push(item(
                    DispatchCategory::Blockades,
                    DispatchSeverity::Urgent,
                    headline,
                    "Warships have established a blocking cordon around a colonial settlement.",
                    related_empire,
                    related_star,
                    None,
                ));
            }

            // --- Invasions ---
            Event::InvasionSucceeded {
                attacker,
                defender,
                star,
                planet_index,
                colony: _,
                ..
            } => {
                let player = state.player_empire;
                let player_involved = *attacker == player || *defender == player;
                let attacker_known = player_knows_empire(state, *attacker);
                let known_star = star_name_if_known(state, *star);

                let (headline, severity) = if player_involved {
                    let h = if let Some(star_name) = known_star {
                        format!("Colony Falls to Invading Forces at {star_name}")
                    } else {
                        "Colony Falls to Invading Forces".to_string()
                    };
                    (h, DispatchSeverity::Historic)
                } else if attacker_known {
                    let attacker_name = empire_name_for_id(state, *attacker);
                    let h = if let Some(star_name) = known_star {
                        format!("{attacker_name} Forces Capture Colony at {star_name}")
                    } else {
                        format!("{attacker_name} Forces Capture Distant Colony")
                    };
                    (h, DispatchSeverity::Urgent)
                } else {
                    (
                        "Invaders Seize Colonial Outpost".to_string(),
                        DispatchSeverity::Urgent,
                    )
                };
                let related_star = if known_star.is_some() {
                    Some(*star)
                } else {
                    None
                };
                let related_attacker = if attacker_known || player_involved {
                    Some(*attacker)
                } else {
                    None
                };
                items.push(item(
                    DispatchCategory::Invasions,
                    severity,
                    headline,
                    "Ground assault operations have concluded with a change of colonial ownership.",
                    related_attacker,
                    related_star,
                    Some(*planet_index),
                ));
            }

            Event::InvasionFailed { attacker, star, .. } => {
                let known_star = star_name_if_known(state, *star);
                let headline = if let Some(star_name) = known_star {
                    format!("Defenders Repel Invasion Attempt at {star_name}")
                } else {
                    "Defenders Repel Invasion Attempt".to_string()
                };
                let attacker_known = player_knows_empire(state, *attacker);
                let related_star = if known_star.is_some() {
                    Some(*star)
                } else {
                    None
                };
                let related_attacker = if attacker_known {
                    Some(*attacker)
                } else {
                    None
                };
                items.push(item(
                    DispatchCategory::Invasions,
                    DispatchSeverity::Notable,
                    headline,
                    "Colonial defenders successfully repulsed a ground assault.",
                    related_attacker,
                    related_star,
                    None,
                ));
            }

            // --- Economy ---
            Event::EconomySummary {
                empire,
                credits_income,
                maintenance,
                ..
            } => {
                if *empire == state.player_empire && credits_income - maintenance < 0 {
                    items.push(item(
                        DispatchCategory::Economy,
                        DispatchSeverity::Notable,
                        "Economic Pressure Strains Imperial Coffers",
                        "Imperial treasury reports indicate expenditures are outpacing revenue.",
                        Some(*empire),
                        None,
                        None,
                    ));
                }
            }

            Event::FoodShortage { empire, .. } => {
                if *empire == state.player_empire {
                    items.push(item(
                        DispatchCategory::Economy,
                        DispatchSeverity::Urgent,
                        "Food Reserves Depleted — Population Facing Shortfall",
                        "Supply chain analysis confirms food stockpiles have fallen to critical levels.",
                        Some(*empire),
                        None,
                        None,
                    ));
                }
            }

            Event::CreditDeficit { empire, .. } => {
                if *empire == state.player_empire {
                    items.push(item(
                        DispatchCategory::Economy,
                        DispatchSeverity::Urgent,
                        "Imperial Treasury Reports Credit Shortfall",
                        "Credit reserves have fallen below zero — maintenance is outpacing income.",
                        Some(*empire),
                        None,
                        None,
                    ));
                }
            }

            // --- Trade ---
            Event::ColonyIsolated { colony } => {
                let is_player_colony = state
                    .colonies
                    .get(colony)
                    .map(|c| c.owner == state.player_empire)
                    .unwrap_or(false);
                if is_player_colony {
                    items.push(item(
                        DispatchCategory::Trade,
                        DispatchSeverity::Notable,
                        "Trade Network Disrupted — Colony Loses Connectivity",
                        "A colonial settlement has lost its connection to the imperial trade network.",
                        None,
                        None,
                        None,
                    ));
                }
            }

            // --- Diplomacy ---
            Event::FirstContact { with_empire } => {
                let empire_name = empire_name_for_id(state, *with_empire);
                items.push(item(
                    DispatchCategory::Diplomacy,
                    DispatchSeverity::Notable,
                    format!("New Contact Established with {empire_name}"),
                    "Diplomatic channels have been opened for the first time with a new interstellar power.",
                    Some(*with_empire),
                    None,
                    None,
                ));
            }

            // --- Victory ---
            Event::VictoryProgressMilestone {
                path,
                empire,
                progress_percent,
            } => {
                let severity = if *progress_percent >= 80 {
                    DispatchSeverity::Historic
                } else {
                    DispatchSeverity::Notable
                };
                let path_label = path.label();
                items.push(item(
                    DispatchCategory::VictoryRace,
                    severity,
                    format!("Victory Analysts Track Rising {path_label} Momentum"),
                    "Galactic observers note significant progress toward a civilisational milestone.",
                    Some(*empire),
                    None,
                    None,
                ));
            }

            Event::VictoryAchieved { winner, path, .. } => {
                let winner_name = empire_name_for_id(state, *winner);
                let path_label = path.label();
                items.push(item(
                    DispatchCategory::VictoryRace,
                    DispatchSeverity::Historic,
                    format!("{winner_name} Claims Victory via {path_label}"),
                    "The galactic community acknowledges the decisive ascendancy of a singular power.",
                    Some(*winner),
                    None,
                    None,
                ));
            }

            // All other events are not surfaced in the dispatch.
            _ => {}
        }
    }

    // Deduplicate: keep only one item per (category, headline) pair so that
    // e.g. multiple PlanetSurveyCompleted in the same turn don't spam.
    let mut seen: std::collections::BTreeSet<(DispatchCategory, String)> = Default::default();
    items.retain(|it| seen.insert((it.category, it.headline.clone())));

    // Sort: severity descending, then category, then headline.
    items.sort_by_key(|it| {
        (
            // Reverse severity: Historic(3)→0, Urgent(2)→1, Notable(1)→2, Notice(0)→3
            255u8 - it.severity.sort_key(),
            it.category,
            it.headline.clone(),
        )
    });

    let is_cadence = completed_turn.is_multiple_of(DISPATCH_CADENCE);
    let has_urgent_or_historic = items.iter().any(|i| {
        matches!(
            i.severity,
            DispatchSeverity::Urgent | DispatchSeverity::Historic
        )
    });

    if !is_cadence && !has_urgent_or_historic {
        return None;
    }

    let title = format!("Galactic Dispatch — Turn {}", completed_turn + 1);
    Some(GalacticDispatch {
        turn: completed_turn,
        title,
        items,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;
    use crate::state::{
        Colony, ColonyId, Empire, EmpireId, FleetId, GameState, Planet, PlanetClass, PlanetSize,
        RelationshipStatus, Star, StarId, TechId, VictoryPath,
    };
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::{BTreeMap, VecDeque};

    /// Minimal `GameState` for testing dispatch generation.
    fn minimal_state() -> GameState {
        let player_id = EmpireId(1);
        let star_id = StarId(10);
        let mut stars = BTreeMap::new();
        stars.insert(
            star_id,
            Star {
                id: star_id,
                sector: Default::default(),
                name: "Alpha Centauri".to_string(),
                x: 0,
                y: 0,
                spectral_class: crate::state::SpectralClass::G,
                planets: vec![Planet {
                    name: "Alpha Centauri I".to_string(),
                    class: PlanetClass::Terran,
                    size: PlanetSize::Medium,
                    colony: None,
                    habitable: true,
                    surveyed: false,
                    specials: Vec::new(),
                    resources: Vec::new(),
                    ancient_ruins_collected: false,
                }],
            },
        );
        let mut empires = BTreeMap::new();
        empires.insert(
            player_id,
            Empire {
                id: player_id,
                name: "Terran Directorate".to_string(),
                credits: 100,
                research_points: 0,
                home_star: star_id,
                research: Default::default(),
                food: 10,
                empire_def: None,
            },
        );
        GameState {
            seed: 42,
            turn: 0,
            sectors: Default::default(),
            stars,
            empires,
            colonies: Default::default(),
            fleets: Default::default(),
            player_empire: player_id,
            rng: ChaCha8Rng::seed_from_u64(42),
            event_log: Vec::new(),
            next_colony_id: 1,
            next_fleet_id: 1,
            explored_stars: Default::default(),
            scout_missions: Default::default(),
            survey_missions: Default::default(),
            fleet_missions: Default::default(),
            ai_empire: None,
            ai_explored_stars: Default::default(),
            diplomacy: Default::default(),
            hyperspace_lanes: Default::default(),
            known_hyperspace_lanes: Default::default(),
            fleet_orders: Default::default(),
            scenario: None,
            ai_empires: Vec::new(),
            colony_supply: Default::default(),
            colony_blockade: Default::default(),
            victory_status: Default::default(),
            galactic_dispatches: VecDeque::new(),
        }
    }

    // ------------------------------------------------------------------
    // Cadence behaviour
    // ------------------------------------------------------------------

    #[test]
    fn dispatch_generated_on_cadence_turn() {
        let state = minimal_state();
        // completed_turn=0 → 0 % 5 == 0, always emit
        let result = generate_dispatch(0, &[], &state);
        assert!(result.is_some(), "expected dispatch on cadence turn 0");
        let d = result.unwrap();
        assert_eq!(d.turn, 0);
        assert_eq!(d.title, "Galactic Dispatch — Turn 1");
    }

    #[test]
    fn dispatch_not_generated_on_non_cadence_turn_without_urgent_events() {
        let state = minimal_state();
        // turn=2 (0-indexed), no events → None
        let result = generate_dispatch(2, &[], &state);
        assert!(result.is_none());
    }

    #[test]
    fn non_cadence_turn_with_urgent_event_generates_dispatch() {
        let mut state = minimal_state();
        let colony_id = ColonyId(1);
        let star_id = StarId(10);
        // Add a player-owned colony so blockade triggers player-colony path
        state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: state.player_empire,
                population: 1,
                production: 0,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
                role: Default::default(),
                rally_point: None,
            },
        );
        let events = vec![Event::BlockadeStarted {
            colony: colony_id,
            star: star_id,
            by_empire: EmpireId(2),
        }];
        // turn=2 has no cadence but urgent event → Some
        let result = generate_dispatch(2, &events, &state);
        assert!(result.is_some());
        let d = result.unwrap();
        assert!(!d.items.is_empty());
        assert_eq!(d.items[0].severity, DispatchSeverity::Urgent);
    }

    // ------------------------------------------------------------------
    // Determinism
    // ------------------------------------------------------------------

    #[test]
    fn same_state_events_produce_same_dispatch() {
        let state = minimal_state();
        let events = vec![
            Event::SystemExplored { star: StarId(10) },
            Event::ResearchCompleted { tech: TechId(1) },
        ];
        let a = generate_dispatch(0, &events, &state);
        let b = generate_dispatch(0, &events, &state);
        assert_eq!(a, b);
    }

    #[test]
    fn dispatch_items_are_deterministically_ordered() {
        let mut state = minimal_state();
        // Add second empire as known
        state
            .diplomacy
            .insert(EmpireId(2), RelationshipStatus::Contacted);
        let events = vec![
            Event::SystemExplored { star: StarId(10) },
            Event::ResearchCompleted { tech: TechId(1) },
            Event::FirstContact {
                with_empire: EmpireId(2),
            },
        ];
        let a = generate_dispatch(0, &events, &state).unwrap();
        let b = generate_dispatch(0, &events, &state).unwrap();
        assert_eq!(a.items, b.items);
        // Research (Notable) should come before Exploration (Notice)
        let research_pos = a
            .items
            .iter()
            .position(|i| i.category == DispatchCategory::Research)
            .expect("Research item should be present");
        let exploration_pos = a
            .items
            .iter()
            .position(|i| i.category == DispatchCategory::Exploration)
            .expect("Exploration item should be present");
        assert!(
            research_pos < exploration_pos,
            "Notable should sort before Notice"
        );
    }

    // ------------------------------------------------------------------
    // Information hiding
    // ------------------------------------------------------------------

    #[test]
    fn uncontacted_empire_details_not_leaked() {
        let state = minimal_state();
        // EmpireId(99) has no diplomacy entry → Unknown
        let events = vec![Event::AiColonized {
            empire: EmpireId(99),
            star: StarId(10),
            planet_index: 0,
            colony: ColonyId(5),
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let colonization_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Colonization)
            .collect();
        assert!(!colonization_items.is_empty());
        // Headline must not mention empire names or star names for unknown empire
        let headline = &colonization_items[0].headline;
        assert!(
            headline.contains("Unknown"),
            "headline should use vague wording for unknown empire: {headline}"
        );
        // Empire ID must not be leaked
        assert_eq!(colonization_items[0].related_empire_id, None);
    }

    #[test]
    fn hidden_planet_special_not_leaked() {
        // ColonizationCompleted should not reference unsurveyed planet specials.
        // The dispatch body is generic and does not access planet.special.
        let state = minimal_state();
        let events = vec![Event::ColonizationCompleted {
            empire: state.player_empire,
            fleet: FleetId(1),
            star: StarId(10),
            planet_index: 0,
            colony: ColonyId(1),
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let colonization_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Colonization)
            .collect();
        assert!(!colonization_items.is_empty());
        // Body must not mention anything special about the planet
        assert!(
            !colonization_items[0]
                .body
                .to_lowercase()
                .contains("special"),
            "body must not leak unsurveyed planet specials"
        );
    }

    // ------------------------------------------------------------------
    // Per-event type tests
    // ------------------------------------------------------------------

    #[test]
    fn combat_event_produces_war_item_when_visible() {
        let state = minimal_state();
        let enemy = EmpireId(2);
        let events = vec![Event::CombatResolved {
            star: StarId(10),
            fleet_a: FleetId(1),
            empire_a: state.player_empire,
            fleet_b: FleetId(2),
            empire_b: enemy,
            strength_a: 10,
            strength_b: 10,
            integrity_a_remaining: 5,
            integrity_b_remaining: 0,
            fleet_a_destroyed: false,
            fleet_b_destroyed: true,
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let war_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::War)
            .collect();
        assert!(!war_items.is_empty(), "expected at least one War item");
        assert_eq!(
            war_items[0].severity,
            DispatchSeverity::Urgent,
            "player-involved combat should be Urgent"
        );
    }

    #[test]
    fn research_event_produces_research_item() {
        let state = minimal_state();
        let events = vec![Event::ResearchCompleted { tech: TechId(1) }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let research_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Research)
            .collect();
        assert!(!research_items.is_empty());
        assert_eq!(research_items[0].severity, DispatchSeverity::Notable);
    }

    #[test]
    fn colonization_produces_colonization_item() {
        let state = minimal_state();
        let events = vec![Event::ColonizationCompleted {
            empire: state.player_empire,
            fleet: FleetId(1),
            star: StarId(10),
            planet_index: 0,
            colony: ColonyId(1),
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Colonization)
            .collect();
        assert!(!items.is_empty());
        assert_eq!(items[0].severity, DispatchSeverity::Notable);
        assert!(items[0].headline.contains("Alpha Centauri"));
    }

    #[test]
    fn blockade_produces_urgent_item() {
        let mut state = minimal_state();
        let colony_id = ColonyId(1);
        let star_id = StarId(10);
        state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: state.player_empire,
                population: 1,
                production: 0,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
                role: Default::default(),
                rally_point: None,
            },
        );
        let events = vec![Event::BlockadeStarted {
            colony: colony_id,
            star: star_id,
            by_empire: EmpireId(2),
        }];
        let d = generate_dispatch(1, &events, &state).unwrap();
        let blockade_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Blockades)
            .collect();
        assert!(!blockade_items.is_empty());
        assert_eq!(blockade_items[0].severity, DispatchSeverity::Urgent);
        assert!(blockade_items[0].headline.contains("Alpha Centauri"));
    }

    #[test]
    fn invasion_produces_historic_item_when_player_involved() {
        let mut state = minimal_state();
        // Add the star to explored so the star name can be used
        state.explored_stars.insert(StarId(10));
        let events = vec![Event::InvasionSucceeded {
            attacker: EmpireId(2),
            defender: state.player_empire,
            fleet: FleetId(1),
            star: StarId(10),
            planet_index: 0,
            colony: ColonyId(1),
            transports_lost: 0,
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Invasions)
            .collect();
        assert!(!items.is_empty(), "expected an Invasions dispatch item");
        assert_eq!(
            items[0].severity,
            DispatchSeverity::Historic,
            "player-involved invasion should be Historic"
        );
    }

    #[test]
    fn credit_deficit_produces_urgent_economy_item() {
        let state = minimal_state();
        let events = vec![Event::CreditDeficit {
            empire: state.player_empire,
            deficit: 10,
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let economy_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Economy)
            .collect();
        assert!(
            !economy_items.is_empty(),
            "expected an Economy dispatch item for CreditDeficit"
        );
        assert_eq!(
            economy_items[0].severity,
            DispatchSeverity::Urgent,
            "credit deficit should be Urgent"
        );
    }

    #[test]
    fn unknown_empire_blockade_does_not_leak_star_or_empire_id() {
        let mut state = minimal_state();
        let colony_id = ColonyId(1);
        let star_id = StarId(10);
        // Colony owned by known enemy empire (not player)
        state.colonies.insert(
            colony_id,
            Colony {
                id: colony_id,
                star: star_id,
                planet_index: 0,
                owner: EmpireId(99),
                population: 1,
                production: 0,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 100,
                role: Default::default(),
                rally_point: None,
            },
        );
        // Empire 88 is completely unknown to the player
        let events = vec![Event::BlockadeStarted {
            colony: colony_id,
            star: star_id,
            by_empire: EmpireId(88),
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let blockade_items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::Blockades)
            .collect();
        assert!(
            !blockade_items.is_empty(),
            "expected a Blockades dispatch item"
        );
        // Unknown empire → no empire ID leaked
        assert_eq!(
            blockade_items[0].related_empire_id, None,
            "unknown blockading empire ID must not be leaked"
        );
        // Star not explored → no star ID leaked
        assert_eq!(
            blockade_items[0].related_star_id, None,
            "unknown star ID must not be leaked for unvisited system"
        );
    }

    #[test]
    fn victory_milestone_produces_victory_item() {
        let state = minimal_state();
        let events = vec![Event::VictoryProgressMilestone {
            path: VictoryPath::Dominion,
            empire: state.player_empire,
            progress_percent: 50,
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::VictoryRace)
            .collect();
        assert!(!items.is_empty());
        assert_eq!(items[0].severity, DispatchSeverity::Notable);
        assert!(items[0].headline.contains("Dominion"));
    }

    #[test]
    fn victory_milestone_at_80_percent_is_historic() {
        let state = minimal_state();
        let events = vec![Event::VictoryProgressMilestone {
            path: VictoryPath::Discovery,
            empire: state.player_empire,
            progress_percent: 80,
        }];
        let d = generate_dispatch(0, &events, &state).unwrap();
        let items: Vec<_> = d
            .items
            .iter()
            .filter(|i| i.category == DispatchCategory::VictoryRace)
            .collect();
        assert!(!items.is_empty());
        assert_eq!(items[0].severity, DispatchSeverity::Historic);
    }
}
