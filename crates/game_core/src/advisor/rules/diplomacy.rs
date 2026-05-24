use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget, ScreenRef,
};
use crate::state::IntelLevel;

pub struct LowIntelRule;

impl AdvisorRule for LowIntelRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("low_intel")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        let mut low_intel_empires = Vec::new();
        for (&empire_id, status) in &ctx.state.diplomacy {
            if *status == crate::state::RelationshipStatus::Unknown {
                continue;
            }
            let intel = ctx.state.intel_level_for_empire(empire_id);
            if intel >= IntelLevel::Informed {
                continue;
            }

            let name = ctx
                .state
                .empires
                .get(&empire_id)
                .map(|empire| empire.name.clone())
                .unwrap_or_else(|| format!("Empire {}", empire_id.0));
            low_intel_empires.push((name, intel));
        }

        if low_intel_empires.is_empty() {
            return;
        }

        low_intel_empires.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
        let target = Some(AdvisorTarget::Empire);
        let key = AdvisorMessageKey {
            rule_id: self.id(),
            target,
        };
        let names: Vec<_> = low_intel_empires
            .iter()
            .map(|(name, intel)| format!("{name} ({})", intel.label()))
            .collect();
        let title = if names.len() == 1 {
            format!("Intel thin on {}", low_intel_empires[0].0)
        } else {
            format!("Intel thin on {} empires", names.len())
        };
        out.push(AdvisorMessage {
            id: AdvisorMessageId(format!("{}:empire", self.id().0)),
            key,
            category: AdvisorCategory::Diplomacy,
            persona: AdvisorPersona::DiplomaticEnvoy,
            severity: AdvisorSeverity::Suggestion,
            title,
            body: format!(
                "Gather Intelligence, keep fleets nearby, or deepen treaties. Low-intel contacts: {}.",
                names.join(", ")
            ),
            turn_created: ctx.turn,
            expires_on_turn: None,
            actions: vec![AdvisorAction::OpenScreen(ScreenRef::Diplomacy)],
            dismissible: true,
            tutorial_id: None,
            target,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{AdvisorPreferences, PlayerKnowledge};
    use crate::state::{Empire, EmpireId, EmpireIntel, GameState, RelationshipStatus, StarId};

    #[test]
    fn low_intel_rule_aggregates_contacted_empires() {
        let mut state = GameState {
            player_empire: EmpireId(1),
            turn: 7,
            ..GameState::default()
        };
        state.empires.insert(
            EmpireId(1),
            Empire {
                id: EmpireId(1),
                name: "Player".to_string(),
                credits: 100,
                research_points: 0,
                home_star: StarId(1),
                research: Default::default(),
                food: 0,
                empire_def: None,
            },
        );
        for (empire_id, name, status, level) in [
            (
                EmpireId(2),
                "Aurora Combine",
                RelationshipStatus::Contacted,
                IntelLevel::Contacted,
            ),
            (
                EmpireId(3),
                "Brass League",
                RelationshipStatus::Neutral,
                IntelLevel::Basic,
            ),
            (
                EmpireId(4),
                "Clear Skies Pact",
                RelationshipStatus::Neutral,
                IntelLevel::Informed,
            ),
        ] {
            state.empires.insert(
                empire_id,
                Empire {
                    id: empire_id,
                    name: name.to_string(),
                    credits: 0,
                    research_points: 0,
                    home_star: StarId(empire_id.0),
                    research: Default::default(),
                    food: 0,
                    empire_def: None,
                },
            );
            state.diplomacy.insert(empire_id, status);
            state.empire_intel.insert(
                empire_id,
                EmpireIntel {
                    level,
                    points: 0,
                    last_gather_turn: None,
                },
            );
        }

        let knowledge = PlayerKnowledge::default();
        let preferences = AdvisorPreferences::default();
        let ctx = AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &knowledge,
            preferences: &preferences,
            turn: state.turn,
        };
        let mut out = Vec::new();
        let rule = LowIntelRule;
        rule.evaluate(&ctx, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "Intel thin on 2 empires");
        assert!(out[0]
            .body
            .contains("Aurora Combine (Contacted), Brass League (Basic)"));
        assert!(!out[0].body.contains("Clear Skies Pact"));
    }
}
