pub mod colony;
pub mod diplomacy;
pub mod economy;
pub mod military;
pub mod science;
pub mod tutorial;

use crate::advisor::{AdvisorContext, AdvisorMessage, AdvisorRuleId};

pub trait AdvisorRule {
    fn id(&self) -> AdvisorRuleId;

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>);
}
