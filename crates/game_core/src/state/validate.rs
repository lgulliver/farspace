//! Referential-integrity validation for `GameState`.
//!
//! The engine trusts that cross-references between top-level collections are
//! consistent (e.g. a fleet's location star exists) and panics when they are
//! not. State built by the engine always satisfies this, but a loaded save
//! file is external input: `game_save` runs this check after deserialising so
//! a corrupted or hand-edited save fails at load time with a description
//! instead of crashing mid-turn.

use super::GameState;

impl GameState {
    /// Check that cross-references between top-level state collections are
    /// consistent. Returns the first inconsistency found.
    ///
    /// Mission `origin` fields are deliberately not checked: they default to
    /// `StarId(0)` for saves migrated from older schemas and are only used
    /// for progress display.
    pub fn validate_integrity(&self) -> Result<(), String> {
        if !self.empires.contains_key(&self.player_empire) {
            return Err(format!(
                "player empire {} does not exist",
                self.player_empire.0
            ));
        }
        if let Some(ai) = self.ai_empire
            && !self.empires.contains_key(&ai)
        {
            return Err(format!("ai empire {} does not exist", ai.0));
        }

        for (id, empire) in &self.empires {
            if !self.stars.contains_key(&empire.home_star) {
                return Err(format!(
                    "empire {} home star {} does not exist",
                    id.0, empire.home_star.0
                ));
            }
        }

        for colony in self.colonies.values() {
            if !self.empires.contains_key(&colony.owner) {
                return Err(format!(
                    "colony {} owner empire {} does not exist",
                    colony.id.0, colony.owner.0
                ));
            }
            let Some(star) = self.stars.get(&colony.star) else {
                return Err(format!(
                    "colony {} star {} does not exist",
                    colony.id.0, colony.star.0
                ));
            };
            if colony.planet_index >= star.planets.len() {
                return Err(format!(
                    "colony {} planet index {} out of bounds for star {} ({} planets)",
                    colony.id.0,
                    colony.planet_index,
                    colony.star.0,
                    star.planets.len()
                ));
            }
        }

        for star in self.stars.values() {
            for (index, planet) in star.planets.iter().enumerate() {
                if let Some(colony_id) = planet.colony
                    && !self.colonies.contains_key(&colony_id)
                {
                    return Err(format!(
                        "star {} planet {} references missing colony {}",
                        star.id.0, index, colony_id.0
                    ));
                }
            }
        }

        for fleet in self.fleets.values() {
            if !self.empires.contains_key(&fleet.owner) {
                return Err(format!(
                    "fleet {} owner empire {} does not exist",
                    fleet.id.0, fleet.owner.0
                ));
            }
            if !self.stars.contains_key(&fleet.location) {
                return Err(format!(
                    "fleet {} location star {} does not exist",
                    fleet.id.0, fleet.location.0
                ));
            }
        }

        for (fleet_id, mission) in &self.scout_missions {
            if !self.fleets.contains_key(fleet_id) {
                return Err(format!(
                    "scout mission references missing fleet {}",
                    fleet_id.0
                ));
            }
            if !self.stars.contains_key(&mission.destination) {
                return Err(format!(
                    "scout mission for fleet {} references missing star {}",
                    fleet_id.0, mission.destination.0
                ));
            }
        }

        for (fleet_id, mission) in &self.fleet_missions {
            if !self.fleets.contains_key(fleet_id) {
                return Err(format!(
                    "fleet mission references missing fleet {}",
                    fleet_id.0
                ));
            }
            if !self.stars.contains_key(&mission.destination) {
                return Err(format!(
                    "fleet mission for fleet {} references missing star {}",
                    fleet_id.0, mission.destination.0
                ));
            }
        }

        for (fleet_id, mission) in &self.survey_missions {
            if !self.fleets.contains_key(fleet_id) {
                return Err(format!(
                    "survey mission references missing fleet {}",
                    fleet_id.0
                ));
            }
            let Some(star) = self.stars.get(&mission.star) else {
                return Err(format!(
                    "survey mission for fleet {} references missing star {}",
                    fleet_id.0, mission.star.0
                ));
            };
            if mission.planet_index >= star.planets.len() {
                return Err(format!(
                    "survey mission for fleet {} planet index {} out of bounds for star {} ({} planets)",
                    fleet_id.0,
                    mission.planet_index,
                    mission.star.0,
                    star.planets.len()
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Engine;
    use crate::state::{ColonyId, EmpireId, StarId};

    #[test]
    fn freshly_generated_state_is_valid() {
        let mut engine = Engine::new(42);
        engine.state.validate_integrity().expect("new game valid");
        engine.apply_turn(vec![crate::Command::EndTurn]);
        engine
            .state
            .validate_integrity()
            .expect("state after a turn valid");
    }

    #[test]
    fn missing_player_empire_is_rejected() {
        let mut engine = Engine::new(42);
        engine.state.player_empire = EmpireId(9999);
        let err = engine.state.validate_integrity().unwrap_err();
        assert!(err.contains("player empire"), "got: {err}");
    }

    #[test]
    fn fleet_pointing_at_missing_star_is_rejected() {
        let mut engine = Engine::new(42);
        let fleet_id = *engine.state.fleets.keys().next().expect("fleet exists");
        engine.state.fleets.get_mut(&fleet_id).unwrap().location = StarId(u64::MAX);
        let err = engine.state.validate_integrity().unwrap_err();
        assert!(err.contains("location star"), "got: {err}");
    }

    #[test]
    fn colony_with_out_of_bounds_planet_index_is_rejected() {
        let mut engine = Engine::new(42);
        let colony_id = *engine.state.colonies.keys().next().expect("colony exists");
        engine
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .planet_index = usize::MAX;
        let err = engine.state.validate_integrity().unwrap_err();
        assert!(err.contains("planet index"), "got: {err}");
    }

    #[test]
    fn planet_backref_to_missing_colony_is_rejected() {
        let mut engine = Engine::new(42);
        let star_id = *engine.state.stars.keys().next().expect("star exists");
        engine.state.stars.get_mut(&star_id).unwrap().planets[0].colony = Some(ColonyId(424_242));
        let err = engine.state.validate_integrity().unwrap_err();
        assert!(err.contains("missing colony"), "got: {err}");
    }

    #[test]
    fn mission_for_missing_fleet_is_rejected() {
        let mut engine = Engine::new(42);
        let star_id = *engine.state.stars.keys().next().expect("star exists");
        engine.state.scout_missions.insert(
            crate::FleetId(987_654),
            crate::ScoutMission {
                fleet: crate::FleetId(987_654),
                destination: star_id,
                turns_remaining: 1,
                origin: star_id,
                total_duration: 1,
            },
        );
        let err = engine.state.validate_integrity().unwrap_err();
        assert!(err.contains("missing fleet"), "got: {err}");
    }
}
