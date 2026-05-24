use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget,
};
use crate::events::Event;
use crate::state::Colony;

pub struct IdleColonyProductionRule;

impl AdvisorRule for IdleColonyProductionRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("idle_colony_production")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        for (&colony_id, colony) in &ctx.state.colonies {
            if colony.owner != ctx.state.player_empire || !colony.build_queue.is_empty() {
                continue;
            }
            out.push(build_colony_message(
                self.id(),
                colony_id,
                ctx.turn,
                AdvisorSeverity::Warning,
                "Idle colony production",
                "Queue is empty; colony output may be wasted.",
                vec![AdvisorAction::FocusColony(colony_id)],
            ));
        }
    }
}

pub struct ColonyFoodDeficitRule;

impl AdvisorRule for ColonyFoodDeficitRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("colony_food_deficit")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        for event in ctx.events {
            let Event::ColonyStatusWarning {
                colony,
                food_deficit,
                ..
            } = event
            else {
                continue;
            };
            let Some(colony_state) = ctx.state.colonies.get(colony) else {
                continue;
            };
            if colony_state.owner != ctx.state.player_empire || *food_deficit <= 0 {
                continue;
            }
            let severity = if *food_deficit >= 3 {
                AdvisorSeverity::Critical
            } else {
                AdvisorSeverity::Warning
            };
            out.push(build_colony_message(
                self.id(),
                *colony,
                ctx.turn,
                severity,
                "Food deficit detected",
                "Population pressure rising; increase food production soon.",
                vec![AdvisorAction::FocusColony(*colony)],
            ));
        }
    }
}

pub struct ColonyUnrestRule;

impl AdvisorRule for ColonyUnrestRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("colony_unrest")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        for (&colony_id, colony) in &ctx.state.colonies {
            if colony.owner != ctx.state.player_empire || !colony.is_unrest() {
                continue;
            }
            let severity = if colony.stability < 40 {
                AdvisorSeverity::Critical
            } else {
                AdvisorSeverity::Warning
            };
            out.push(build_colony_message(
                self.id(),
                colony_id,
                ctx.turn,
                severity,
                "Low colony stability",
                "Unrest reduces output and may trigger local disorder.",
                vec![AdvisorAction::FocusColony(colony_id)],
            ));
        }
    }
}

pub struct UndefendedColonyRule;

impl AdvisorRule for UndefendedColonyRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("undefended_colony")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        for (&colony_id, colony) in &ctx.state.colonies {
            if colony.owner != ctx.state.player_empire {
                continue;
            }
            let has_defense = ctx
                .state
                .fleets
                .values()
                .any(|fleet| fleet.owner == ctx.state.player_empire && fleet.location == colony.star);
            if has_defense {
                continue;
            }
            out.push(build_colony_message(
                self.id(),
                colony_id,
                ctx.turn,
                AdvisorSeverity::Warning,
                "Colony lacks fleet cover",
                "No friendly fleet currently stationed at this colony.",
                vec![AdvisorAction::FocusColony(colony_id)],
            ));
        }
    }
}

fn build_colony_message(
    rule_id: AdvisorRuleId,
    colony_id: crate::state::ColonyId,
    turn: u32,
    severity: AdvisorSeverity,
    title: &str,
    body: &str,
    actions: Vec<AdvisorAction>,
) -> AdvisorMessage {
    let target = Some(AdvisorTarget::Colony(colony_id));
    let key = AdvisorMessageKey { rule_id, target };
    AdvisorMessage {
        id: AdvisorMessageId(format!("{}:colony:{}", rule_id.0, colony_id.0)),
        key,
        category: AdvisorCategory::Colony,
        persona: AdvisorPersona::ColonialOffice,
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
