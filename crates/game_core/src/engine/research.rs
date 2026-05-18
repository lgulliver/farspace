use super::*;

impl Engine {
    fn player_research_state(&self, events: &mut Vec<Event>) -> Option<&ResearchState> {
        self.state
            .empires
            .get(&self.state.player_empire)
            .map(|empire| &empire.research)
            .or_else(|| {
                events.push(Event::error("Player empire not found"));
                None
            })
    }

    pub(super) fn process_select_research(&mut self, tech_id: TechId, events: &mut Vec<Event>) {
        let empire_id = self.state.player_empire;
        let tech = match tech_by_id(tech_id) {
            Some(tech) => tech,
            None => {
                events.push(Event::error(format!("Tech {} not found", tech_id.0)));
                return;
            }
        };

        let Some(research) = self.player_research_state(events) else {
            return;
        };
        if research.completed.contains(&tech_id) {
            events.push(Event::error(format!(
                "Tech {} is already completed",
                tech_id.0
            )));
            return;
        }
        if !is_tech_available(&research.completed, tech_id) {
            let missing: Vec<&str> = tech
                .prerequisites
                .iter()
                .filter(|req| !research.completed.contains(req))
                .filter_map(|req| tech_by_id(*req).map(|record| record.name))
                .collect();
            let message = if missing.is_empty() {
                format!("Tech {} is locked", tech_id.0)
            } else {
                format!(
                    "Tech {} is locked — requires {}",
                    tech_id.0,
                    missing.join(", ")
                )
            };
            events.push(Event::error(message));
            return;
        }

        if let Some(empire) = self.state.empires.get_mut(&empire_id) {
            if let Some(active_tech) = empire.research.current_tech {
                if active_tech != tech_id {
                    empire.research.progress = 0;
                }
            }
            empire.research.current_tech = Some(tech_id);
            empire.research.queue.retain(|queued| *queued != tech_id);
        }

        events.push(Event::ResearchSelected { tech: tech_id });
    }

    pub(super) fn process_queue_research(&mut self, tech_id: TechId, events: &mut Vec<Event>) {
        if tech_by_id(tech_id).is_none() {
            events.push(Event::error(format!("Tech {} not found", tech_id.0)));
            return;
        }

        let Some(research) = self.player_research_state(events) else {
            return;
        };
        if research.completed.contains(&tech_id) {
            events.push(Event::error(format!(
                "Tech {} is already completed",
                tech_id.0
            )));
            return;
        }
        if research.current_tech == Some(tech_id) {
            events.push(Event::error(format!(
                "Tech {} is already active research",
                tech_id.0
            )));
            return;
        }
        if research.queue.contains(&tech_id) {
            events.push(Event::error(format!(
                "Tech {} is already queued",
                tech_id.0
            )));
            return;
        }

        if let Some(empire) = self.state.empires.get_mut(&self.state.player_empire) {
            empire.research.queue.push(tech_id);
        }
        events.push(Event::ResearchQueued { tech: tech_id });
    }

    pub(super) fn process_remove_queued_research(
        &mut self,
        tech_id: TechId,
        events: &mut Vec<Event>,
    ) {
        let Some(research) = self.player_research_state(events) else {
            return;
        };
        let Some(index) = research.queue.iter().position(|queued| *queued == tech_id) else {
            events.push(Event::error(format!(
                "Tech {} is not in research queue",
                tech_id.0
            )));
            return;
        };

        if let Some(empire) = self.state.empires.get_mut(&self.state.player_empire) {
            empire.research.queue.remove(index);
        }
        events.push(Event::ResearchQueueRemoved { tech: tech_id });
    }

    pub(super) fn process_move_queued_research_up(
        &mut self,
        tech_id: TechId,
        events: &mut Vec<Event>,
    ) {
        let Some(research) = self.player_research_state(events) else {
            return;
        };
        let Some(from_index) = research.queue.iter().position(|queued| *queued == tech_id) else {
            events.push(Event::error(format!(
                "Tech {} is not in research queue",
                tech_id.0
            )));
            return;
        };
        if from_index == 0 {
            events.push(Event::error(format!(
                "Tech {} is already at top of research queue",
                tech_id.0
            )));
            return;
        }

        let to_index = from_index - 1;
        if let Some(empire) = self.state.empires.get_mut(&self.state.player_empire) {
            empire.research.queue.swap(from_index, to_index);
        }
        events.push(Event::ResearchQueueReordered {
            tech: tech_id,
            from_index,
            to_index,
        });
    }

    pub(super) fn process_move_queued_research_down(
        &mut self,
        tech_id: TechId,
        events: &mut Vec<Event>,
    ) {
        let Some(research) = self.player_research_state(events) else {
            return;
        };
        let Some(from_index) = research.queue.iter().position(|queued| *queued == tech_id) else {
            events.push(Event::error(format!(
                "Tech {} is not in research queue",
                tech_id.0
            )));
            return;
        };
        if from_index + 1 >= research.queue.len() {
            events.push(Event::error(format!(
                "Tech {} is already at bottom of research queue",
                tech_id.0
            )));
            return;
        }

        let to_index = from_index + 1;
        if let Some(empire) = self.state.empires.get_mut(&self.state.player_empire) {
            empire.research.queue.swap(from_index, to_index);
        }
        events.push(Event::ResearchQueueReordered {
            tech: tech_id,
            from_index,
            to_index,
        });
    }

    pub(super) fn process_clear_research_queue(&mut self, events: &mut Vec<Event>) {
        let Some(research) = self.player_research_state(events) else {
            return;
        };
        let removed_count = research.queue.len();

        if let Some(empire) = self.state.empires.get_mut(&self.state.player_empire) {
            empire.research.queue.clear();
        }
        events.push(Event::ResearchQueueCleared { removed_count });
    }

    pub(super) fn process_research_completion_with_queue(
        &mut self,
        empire_id: EmpireId,
        completed_tech: TechId,
        overflow: i64,
        events: &mut Vec<Event>,
    ) {
        if let Some(empire) = self.state.empires.get_mut(&empire_id) {
            if !empire.research.completed.contains(&completed_tech) {
                empire.research.completed.push(completed_tech);
            }
            empire.research.current_tech = None;
            empire.research.progress = overflow;
        }
        events.push(Event::ResearchCompleted {
            tech: completed_tech,
        });
        if completed_tech == TechId::HYPERSPACE_CARTOGRAPHY {
            events.push(Event::HyperspaceCartographyUnlocked { empire: empire_id });
        }

        // If this is an AI empire, regenerate ship designs now that new techs are available.
        if self.state.ai_empires.contains(&empire_id) {
            crate::ai::ai_generate_designs(&mut self.state, empire_id);
        }

        let mut transition_source = completed_tech;
        loop {
            let next_started = self.dequeue_next_valid_queued_research(empire_id, events);
            events.push(Event::ResearchCompletedWithQueueTransition {
                completed: transition_source,
                started: next_started,
            });

            let Some(started) = next_started else {
                if let Some(empire) = self.state.empires.get_mut(&empire_id) {
                    empire.research.current_tech = None;
                }
                return;
            };

            events.push(Event::QueuedResearchStarted { tech: started });

            let Some(cost) = tech_by_id(started).map(|tech| tech.cost) else {
                events.push(Event::QueuedResearchSkipped {
                    tech: started,
                    reason: "unknown technology".to_string(),
                });
                continue;
            };

            let current_progress = match self.state.empires.get(&empire_id) {
                Some(empire) => empire.research.progress,
                None => return,
            };

            if current_progress >= cost {
                let remaining_overflow = current_progress - cost;
                if let Some(empire) = self.state.empires.get_mut(&empire_id) {
                    if !empire.research.completed.contains(&started) {
                        empire.research.completed.push(started);
                    }
                    empire.research.current_tech = None;
                    empire.research.progress = remaining_overflow;
                }
                events.push(Event::ResearchCompleted { tech: started });
                if started == TechId::HYPERSPACE_CARTOGRAPHY {
                    events.push(Event::HyperspaceCartographyUnlocked { empire: empire_id });
                }
                transition_source = started;
                continue;
            }

            if let Some(empire) = self.state.empires.get_mut(&empire_id) {
                empire.research.current_tech = Some(started);
            }
            return;
        }
    }

    fn dequeue_next_valid_queued_research(
        &mut self,
        empire_id: EmpireId,
        events: &mut Vec<Event>,
    ) -> Option<TechId> {
        loop {
            let candidate = {
                let empire = self.state.empires.get_mut(&empire_id)?;
                if empire.research.queue.is_empty() {
                    return None;
                }
                empire.research.queue.remove(0)
            };

            let reason = match tech_by_id(candidate) {
                None => Some("unknown technology".to_string()),
                Some(_) => {
                    let completed = &self.state.empires.get(&empire_id)?.research.completed;
                    if completed.contains(&candidate) {
                        Some("already completed".to_string())
                    } else if !is_tech_available(completed, candidate) {
                        Some("prerequisites not met".to_string())
                    } else {
                        None
                    }
                }
            };

            if let Some(reason) = reason {
                events.push(Event::QueuedResearchSkipped {
                    tech: candidate,
                    reason,
                });
                continue;
            }

            return Some(candidate);
        }
    }
}
