use std::collections::BTreeMap;

use crate::theme::Theme;
use game_core::{empire_definition_by_id, EmpireId, GameState, SectorId, StarId};
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionVisual {
    pub color: Color,
    pub territory: Color,
    pub symbol: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FogState {
    Unexplored,
    Explored,
    Visible,
}

pub fn empire_visual(game_state: &GameState, empire_id: EmpireId) -> FactionVisual {
    let def_id = game_state
        .empires
        .get(&empire_id)
        .and_then(|empire| empire.empire_def);
    FactionVisual {
        color: Theme::faction_color(def_id, empire_id),
        territory: Theme::faction_territory_color(def_id, empire_id),
        symbol: game_state
            .empires
            .get(&empire_id)
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
            .map(|definition| definition.symbol)
            .unwrap_or('*'),
    }
}

pub fn star_owner(game_state: &GameState, star_id: StarId) -> Option<EmpireId> {
    game_state
        .colonies
        .values()
        .find(|colony| colony.star == star_id)
        .map(|colony| colony.owner)
}

pub fn fleets_by_star(game_state: &GameState) -> BTreeMap<StarId, Vec<EmpireId>> {
    let mut fleets = BTreeMap::<StarId, Vec<EmpireId>>::new();
    for fleet in game_state.fleets.values() {
        fleets.entry(fleet.location).or_default().push(fleet.owner);
    }
    fleets
}

pub fn sector_dominant_owner(game_state: &GameState, sector_id: SectorId) -> Option<EmpireId> {
    let mut counts = BTreeMap::<EmpireId, usize>::new();
    for colony in game_state.colonies.values() {
        let Some(star) = game_state.stars.get(&colony.star) else {
            continue;
        };
        if star.sector == sector_id {
            *counts.entry(colony.owner).or_default() += 1;
        }
    }

    counts
        .into_iter()
        .max_by_key(|(empire_id, count)| (*count, std::cmp::Reverse(empire_id.0)))
        .map(|(empire_id, _)| empire_id)
}

pub fn star_is_capital(game_state: &GameState, star_id: StarId) -> bool {
    game_state
        .empires
        .values()
        .any(|empire| empire.home_star == star_id)
}

pub fn visible_star_ids(
    game_state: &GameState,
    sector_id: SectorId,
) -> std::collections::BTreeSet<StarId> {
    let mut visible = std::collections::BTreeSet::<StarId>::new();

    for colony in game_state.colonies.values() {
        if colony.owner == game_state.player_empire
            && game_state
                .stars
                .get(&colony.star)
                .is_some_and(|star| star.sector == sector_id)
        {
            visible.insert(colony.star);
        }
    }

    for fleet in game_state.fleets.values() {
        if fleet.owner == game_state.player_empire
            && game_state
                .stars
                .get(&fleet.location)
                .is_some_and(|star| star.sector == sector_id)
        {
            visible.insert(fleet.location);
        }
    }

    let mut expanded = visible.clone();
    for lane in &game_state.known_hyperspace_lanes {
        let (a, b) = lane.endpoints();
        let Some(a_star) = game_state.stars.get(&a) else {
            continue;
        };
        let Some(b_star) = game_state.stars.get(&b) else {
            continue;
        };
        if a_star.sector != sector_id || b_star.sector != sector_id {
            continue;
        }
        if visible.contains(&a) {
            expanded.insert(b);
        }
        if visible.contains(&b) {
            expanded.insert(a);
        }
    }

    expanded
}

pub fn sector_fog_state(game_state: &GameState, sector_id: SectorId) -> FogState {
    if visible_star_ids(game_state, sector_id)
        .iter()
        .any(|star_id| game_state.explored_stars.contains(star_id))
    {
        FogState::Visible
    } else if game_state
        .stars
        .values()
        .filter(|star| star.sector == sector_id)
        .any(|star| game_state.explored_stars.contains(&star.id))
    {
        FogState::Explored
    } else {
        FogState::Unexplored
    }
}

pub fn star_fog_state(
    game_state: &GameState,
    visible_stars: &std::collections::BTreeSet<StarId>,
    star_id: StarId,
) -> FogState {
    if visible_stars.contains(&star_id) && game_state.explored_stars.contains(&star_id) {
        FogState::Visible
    } else if game_state.explored_stars.contains(&star_id) {
        FogState::Explored
    } else {
        FogState::Unexplored
    }
}
