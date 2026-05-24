use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget,
};
use crate::state::available_tech_ids;

pub struct NoActiveResearchRule;

impl AdvisorRule for NoActiveResearchRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("no_active_research")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        let Some(empire) = ctx.state.empires.get(&ctx.state.player_empire) else {
            return;
        };
        if empire.research.current_tech.is_some() || !empire.research.queue.is_empty() {
            return;
        }
        if available_tech_ids(&empire.research.completed).is_empty() {
            return;
        }
        let target = Some(AdvisorTarget::Empire);
        let key = AdvisorMessageKey {
            rule_id: self.id(),
            target,
        };
        out.push(AdvisorMessage {
            id: AdvisorMessageId(format!("{}:empire", self.id().0)),
            key,
            category: AdvisorCategory::Science,
            persona: AdvisorPersona::ScienceDirector,
            severity: AdvisorSeverity::Warning,
            title: "No active research".to_string(),
            body: "Select a technology to resume scientific progress.".to_string(),
            turn_created: ctx.turn,
            expires_on_turn: None,
            actions: vec![AdvisorAction::OpenResearch],
            dismissible: true,
            tutorial_id: None,
            target,
        });
    }
}
