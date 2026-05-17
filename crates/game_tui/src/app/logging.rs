use super::*;

impl App {
    pub(super) fn empire_display_name(&self, empire_id: game_core::EmpireId) -> String {
        self.engine
            .as_ref()
            .and_then(|engine| engine.state.empires.get(&empire_id))
            .map(|empire| empire.name.clone())
            .unwrap_or_else(|| format!("Empire {}", empire_id.0))
    }

    pub(super) fn format_core_event_for_log(&self, event: &CoreEvent) -> String {
        match event {
            CoreEvent::FirstContact { with_empire } => {
                let name = self.empire_display_name(*with_empire);
                let tone = self
                    .engine
                    .as_ref()
                    .and_then(|engine| engine.state.empires.get(with_empire))
                    .and_then(|empire| empire.empire_def)
                    .and_then(empire_definition_by_id)
                    .map(|def| def.tone)
                    .unwrap_or("Unknown stance");
                format!("First contact established with {name} — {tone}")
            }
            CoreEvent::AiResearchSelected { empire, tech } => {
                let name = self.empire_display_name(*empire);
                let doctrine = self.empire_doctrine_marker(*empire);
                let tech_name = tech_by_id(*tech)
                    .map(|record| record.name)
                    .unwrap_or("Unknown Tech");
                format!("{name} {doctrine} redirected its labs to {tech_name}")
            }
            CoreEvent::AiBuildQueued {
                empire,
                colony,
                item,
            } => {
                let name = self.empire_display_name(*empire);
                let doctrine = self.empire_doctrine_marker(*empire);
                format!(
                    "{name} {doctrine} queued {} at colony {}",
                    item.name(),
                    colony.0
                )
            }
            CoreEvent::AiScoutDispatched {
                empire,
                fleet,
                destination,
            } => {
                let name = self.empire_display_name(*empire);
                let doctrine = self.empire_doctrine_marker(*empire);
                format!(
                    "{name} {doctrine} dispatched scout {} to system {}",
                    fleet.0, destination.0
                )
            }
            CoreEvent::AiColonized {
                empire,
                star,
                planet_index,
                colony,
            } => {
                let name = self.empire_display_name(*empire);
                let doctrine = self.empire_doctrine_marker(*empire);
                format!(
                    "{name} {doctrine} founded colony {} at system {} orbit {}",
                    colony.0,
                    star.0,
                    planet_index + 1
                )
            }
            CoreEvent::AiColonyRoleAssigned {
                empire,
                colony,
                role,
            } => {
                let name = self.empire_display_name(*empire);
                let doctrine = self.empire_doctrine_marker(*empire);
                format!(
                    "{name} {doctrine} reorganized colony {} as {}",
                    colony.0,
                    role.name()
                )
            }
            _ => event.to_log_message(),
        }
    }

    pub(super) fn empire_doctrine_marker(&self, empire_id: game_core::EmpireId) -> String {
        let doctrine = self
            .engine
            .as_ref()
            .and_then(|engine| engine.state.empires.get(&empire_id))
            .and_then(|empire| empire.empire_def)
            .and_then(empire_definition_by_id)
            .map(|def| def.doctrine_short_summary())
            .unwrap_or_else(|| "N/A".to_string());
        format!("[DOC {doctrine}]")
    }

    fn empire_is_known(&self, empire_id: game_core::EmpireId) -> bool {
        self.engine
            .as_ref()
            .map(|engine| {
                engine
                    .state
                    .diplomacy
                    .get(&empire_id)
                    .map(|status| *status != game_core::RelationshipStatus::Unknown)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn colony_is_player_owned(&self, colony_id: game_core::ColonyId) -> bool {
        self.engine
            .as_ref()
            .and_then(|engine| {
                engine
                    .state
                    .colonies
                    .get(&colony_id)
                    .map(|colony| (engine, colony))
            })
            .map(|(engine, colony)| colony.owner == engine.state.player_empire)
            .unwrap_or(false)
    }

    fn star_is_player_explored(&self, star_id: game_core::StarId) -> bool {
        self.engine
            .as_ref()
            .map(|engine| engine.state.explored_stars.contains(&star_id))
            .unwrap_or(true)
    }

    fn event_visible_to_player(&self, event: &CoreEvent) -> bool {
        let Some(engine) = self.engine.as_ref() else {
            return true;
        };

        match event {
            CoreEvent::EconomySummary { empire, .. }
            | CoreEvent::FoodShortage { empire, .. }
            | CoreEvent::CreditDeficit { empire, .. } => *empire == engine.state.player_empire,
            CoreEvent::ColonyStatusWarning { colony, .. }
            | CoreEvent::PopulationGrew { colony, .. }
            | CoreEvent::ColonyIsolated { colony }
            | CoreEvent::ColonyReconnected { colony } => self.colony_is_player_owned(*colony),
            CoreEvent::SystemExplored { star }
            | CoreEvent::PlanetSurveyCompleted { star, .. }
            | CoreEvent::AncientRuinsDiscovered { star, .. } => self.star_is_player_explored(*star),
            CoreEvent::AiResearchSelected { empire, .. }
            | CoreEvent::AiBuildQueued { empire, .. }
            | CoreEvent::AiScoutDispatched { empire, .. }
            | CoreEvent::AiColonized { empire, .. }
            | CoreEvent::AiColonyRoleAssigned { empire, .. } => self.empire_is_known(*empire),
            _ => true,
        }
    }

    pub(super) fn push_core_event_to_log(&mut self, event: &CoreEvent) {
        if !self.event_visible_to_player(event) {
            return;
        }
        let message = self.format_core_event_for_log(event);
        let kind = LogEntryKind::from_message(&message);
        self.state.log.push_with_kind(kind, message);
    }

    pub(super) fn push_status(&mut self, kind: LogEntryKind, message: impl Into<String>) {
        let message = message.into();
        self.state.log.push_with_kind(kind, message.clone());
        self.state.status_message = Some(message);
    }

    pub(super) fn push_error_status(&mut self, message: impl Into<String>) {
        self.push_status(LogEntryKind::Error, message);
    }

    pub(super) fn build_end_turn_report(turn: u32, events: &[CoreEvent]) -> String {
        let mut explored = 0usize;
        let mut surveyed = 0usize;
        let mut colonized = 0usize;
        let mut research_completed = 0usize;
        let mut queue_transitions_started = 0usize;
        let mut fleets_arrived = 0usize;
        let mut warnings = 0usize;
        let mut errors = 0usize;
        let mut newly_isolated = 0usize;
        let mut reconnected = 0usize;
        let mut invasions_won = 0usize;
        let mut invasions_failed = 0usize;

        for event in events {
            match event {
                CoreEvent::SystemExplored { .. } => explored += 1,
                CoreEvent::PlanetSurveyCompleted { .. } => surveyed += 1,
                CoreEvent::ColonizationCompleted { .. } => colonized += 1,
                CoreEvent::ResearchCompleted { .. } => research_completed += 1,
                CoreEvent::ResearchCompletedWithQueueTransition {
                    started: Some(_), ..
                } => queue_transitions_started += 1,
                CoreEvent::FleetArrived { .. } => fleets_arrived += 1,
                CoreEvent::FoodShortage { .. } | CoreEvent::CreditDeficit { .. } => warnings += 1,
                CoreEvent::ColonyIsolated { .. } => newly_isolated += 1,
                CoreEvent::ColonyReconnected { .. } => reconnected += 1,
                CoreEvent::InvasionSucceeded { .. } => invasions_won += 1,
                CoreEvent::InvasionFailed { .. } => invasions_failed += 1,
                CoreEvent::Error { .. } => errors += 1,
                _ => {}
            }
        }

        format!(
            "Turn {} global summary (all empires): explored {}, surveyed {}, colonized {}, research {}, queued starts {}, arrivals {}, invasions won {}, invasions failed {}, warnings {}, isolated {}, reconnected {}, errors {}.",
            turn,
            explored,
            surveyed,
            colonized,
            research_completed,
            queue_transitions_started,
            fleets_arrived,
            invasions_won,
            invasions_failed,
            warnings,
            newly_isolated,
            reconnected,
            errors
        )
    }
}
