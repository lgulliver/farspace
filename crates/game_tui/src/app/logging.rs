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
            CoreEvent::WarDeclared { attacker, defender } => {
                let attacker_name = self.empire_display_name(*attacker);
                let defender_name = self.empire_display_name(*defender);
                format!("WAR DECLARED: {attacker_name} declared war on {defender_name}")
            }
            CoreEvent::PeaceSigned {
                with_empire,
                truce_expires_turn,
            } => {
                let name = self.empire_display_name(*with_empire);
                format!("Peace signed with {name} (truce until turn {truce_expires_turn})")
            }
            CoreEvent::TreatySigned {
                with_empire,
                treaty_type,
                expires_turn,
            } => {
                let name = self.empire_display_name(*with_empire);
                format!(
                    "{} signed with {} (until turn {})",
                    treaty_type.label(),
                    name,
                    expires_turn
                )
            }
            CoreEvent::TreatyExpired {
                with_empire,
                treaty_type,
            } => {
                let name = self.empire_display_name(*with_empire);
                format!("{} with {} expired", treaty_type.label(), name)
            }
            CoreEvent::TreatyCancelled {
                with_empire,
                treaty_type,
            } => {
                let name = self.empire_display_name(*with_empire);
                format!("{} with {} cancelled", treaty_type.label(), name)
            }
            CoreEvent::WarningIssued {
                from_empire,
                to_empire,
            } => {
                let from_name = self.empire_display_name(*from_empire);
                let to_name = self.empire_display_name(*to_empire);
                format!("{from_name} issued warning to {to_name}")
            }
            CoreEvent::TributeDemanded {
                from_empire,
                to_empire,
            } => {
                let from_name = self.empire_display_name(*from_empire);
                let to_name = self.empire_display_name(*to_empire);
                format!("{from_name} demanded tribute from {to_name}")
            }
            CoreEvent::TributeRefused {
                from_empire,
                to_empire,
            } => {
                let from_name = self.empire_display_name(*from_empire);
                let to_name = self.empire_display_name(*to_empire);
                format!("{to_name} refused tribute demanded by {from_name}")
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
            CoreEvent::TreatyProposed {
                from_empire,
                to_empire,
                ..
            }
            | CoreEvent::TreatyAccepted {
                from_empire,
                to_empire,
                ..
            }
            | CoreEvent::TreatyRejected {
                from_empire,
                to_empire,
                ..
            }
            | CoreEvent::WarningIssued {
                from_empire,
                to_empire,
            }
            | CoreEvent::TributeDemanded {
                from_empire,
                to_empire,
            }
            | CoreEvent::TributeRefused {
                from_empire,
                to_empire,
            }
            | CoreEvent::WarDeclared {
                attacker: from_empire,
                defender: to_empire,
            } => self.empire_is_known(*from_empire) && self.empire_is_known(*to_empire),
            CoreEvent::TreatySigned { with_empire, .. }
            | CoreEvent::TreatyExpired { with_empire, .. }
            | CoreEvent::TreatyCancelled { with_empire, .. }
            | CoreEvent::PeaceSigned { with_empire, .. } => self.empire_is_known(*with_empire),
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
        let mut combats = 0usize;
        let mut retreats = 0usize;
        let mut victory_milestones = 0usize;
        let mut victories = 0usize;
        let mut treaty_events = 0usize;
        let mut war_events = 0usize;
        let mut peace_events = 0usize;

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
                CoreEvent::CombatResolved { .. } => combats += 1,
                CoreEvent::FleetRetreatTriggered { .. } => retreats += 1,
                CoreEvent::VictoryProgressMilestone { .. } => victory_milestones += 1,
                CoreEvent::VictoryAchieved { .. } => victories += 1,
                CoreEvent::TreatySigned { .. }
                | CoreEvent::TreatyExpired { .. }
                | CoreEvent::TreatyCancelled { .. } => treaty_events += 1,
                CoreEvent::WarDeclared { .. } => war_events += 1,
                CoreEvent::PeaceSigned { .. } => peace_events += 1,
                CoreEvent::Error { .. } => errors += 1,
                _ => {}
            }
        }

        format!(
            "Turn {} global summary (all empires): explored {}, surveyed {}, colonized {}, research {}, queued starts {}, arrivals {}, combats {}, retreats {}, invasions won {}, invasions failed {}, treaties {}, wars {}, peaces {}, victory milestones {}, victories {}, warnings {}, isolated {}, reconnected {}, errors {}.",
            turn,
            explored,
            surveyed,
            colonized,
            research_completed,
            queue_transitions_started,
            fleets_arrived,
            combats,
            retreats,
            invasions_won,
            invasions_failed,
            treaty_events,
            war_events,
            peace_events,
            victory_milestones,
            victories,
            warnings,
            newly_isolated,
            reconnected,
            errors
        )
    }
}
