use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget,
};
use crate::events::Event;

pub struct TreasuryDepletionRiskRule;

impl AdvisorRule for TreasuryDepletionRiskRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("treasury_depletion_risk")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        let Some(empire) = ctx.state.empires.get(&ctx.state.player_empire) else {
            return;
        };
        let mut projected: Option<i64> = None;
        for event in ctx.events {
            let Event::EconomySummary {
                empire: event_empire,
                credits_income,
                maintenance,
                ..
            } = event
            else {
                continue;
            };
            if *event_empire != ctx.state.player_empire {
                continue;
            }
            projected = Some(empire.credits + *credits_income - *maintenance);
            break;
        }
        if projected.is_none() {
            projected = ctx.events.iter().find_map(|event| {
                if let Event::CreditDeficit {
                    empire: event_empire,
                    ..
                } = event
                {
                    if *event_empire == ctx.state.player_empire {
                        return Some(empire.credits - 1);
                    }
                }
                None
            });
        }
        let Some(projected_credits) = projected else {
            return;
        };
        if projected_credits > 0 {
            return;
        }
        let severity = if projected_credits < 0 {
            AdvisorSeverity::Critical
        } else {
            AdvisorSeverity::Warning
        };
        let target = Some(AdvisorTarget::Empire);
        let key = AdvisorMessageKey {
            rule_id: self.id(),
            target,
        };
        out.push(AdvisorMessage {
            id: AdvisorMessageId(format!("{}:empire", self.id().0)),
            key,
            category: AdvisorCategory::Economy,
            persona: AdvisorPersona::EconomicAdvisor,
            severity,
            title: "Treasury risk detected".to_string(),
            body: "Credits projected to hit zero next turn at current upkeep.".to_string(),
            turn_created: ctx.turn,
            expires_on_turn: None,
            actions: vec![AdvisorAction::OpenCommandPalette(Some("economy".to_string()))],
            dismissible: true,
            tutorial_id: None,
            target,
        });
    }
}
