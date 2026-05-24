use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget,
};
use crate::state::FleetOrder;

pub struct IdleFleetRule;

impl AdvisorRule for IdleFleetRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("idle_fleet")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        for (&fleet_id, fleet) in &ctx.state.fleets {
            if fleet.owner != ctx.state.player_empire {
                continue;
            }
            let is_idle = match ctx.state.fleet_orders.get(&fleet_id) {
                None => true,
                Some(FleetOrder::Hold) => true,
                Some(FleetOrder::MoveToSystem(_)) => false,
            };
            if !is_idle {
                continue;
            }
            out.push(build_fleet_message(
                self.id(),
                fleet_id,
                ctx.turn,
                AdvisorSeverity::Warning,
                "Idle fleet awaiting orders",
                "Re-task fleet to explore, defend, or patrol.",
                vec![AdvisorAction::FocusFleet(fleet_id)],
            ));
        }
    }
}

fn build_fleet_message(
    rule_id: AdvisorRuleId,
    fleet_id: crate::state::FleetId,
    turn: u32,
    severity: AdvisorSeverity,
    title: &str,
    body: &str,
    actions: Vec<AdvisorAction>,
) -> AdvisorMessage {
    let target = Some(AdvisorTarget::Fleet(fleet_id));
    let key = AdvisorMessageKey { rule_id, target };
    AdvisorMessage {
        id: AdvisorMessageId(format!("{}:fleet:{}", rule_id.0, fleet_id.0)),
        key,
        category: AdvisorCategory::Military,
        persona: AdvisorPersona::MilitaryCommand,
        severity,
        title: title.to_string(),
        body: body.to_string(),
        turn_created: turn,
        expires_on_turn: None,
        actions,
        dismissible: true,
        tutorial_id: None,
        target,
    }
}
