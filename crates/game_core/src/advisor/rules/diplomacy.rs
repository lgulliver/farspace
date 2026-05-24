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
                .map(|empire| empire.name.as_str())
                .unwrap_or("contacted empire");
            let target = Some(AdvisorTarget::Empire);
            let key = AdvisorMessageKey {
                rule_id: self.id(),
                target,
            };
            out.push(AdvisorMessage {
                id: AdvisorMessageId(format!("{}:{}", self.id().0, empire_id.0)),
                key,
                category: AdvisorCategory::Diplomacy,
                persona: AdvisorPersona::DiplomaticEnvoy,
                severity: AdvisorSeverity::Suggestion,
                title: format!("Intel thin on {name}"),
                body: format!(
                    "Gather Intelligence, keep fleets nearby, or deepen treaties to improve intel beyond {}.",
                    intel.label()
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
}
