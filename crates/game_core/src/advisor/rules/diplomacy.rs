use crate::advisor::rules::AdvisorRule;
use crate::advisor::{AdvisorContext, AdvisorMessage, AdvisorRuleId};

/// Placeholder for future diplomacy-focused advisor guidance.
pub struct DiplomacyRulesStub;

impl AdvisorRule for DiplomacyRulesStub {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("diplomacy_stub")
    }

    fn evaluate(&self, _ctx: &AdvisorContext<'_>, _out: &mut Vec<AdvisorMessage>) {}
}
