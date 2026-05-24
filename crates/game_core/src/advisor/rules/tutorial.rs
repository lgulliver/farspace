use crate::advisor::rules::AdvisorRule;
use crate::advisor::{
    AdvisorAction, AdvisorCategory, AdvisorContext, AdvisorMessage, AdvisorMessageId,
    AdvisorMessageKey, AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget, MechanicId,
    TutorialId,
};
use crate::state::{available_tech_ids, Colony, ColonyId, Star, StarId};

pub const TUTORIAL_FIRST_COLONY: TutorialId = TutorialId("advisor.tutorial.first_colony");
pub const TUTORIAL_FIRST_RESEARCH: TutorialId = TutorialId("advisor.tutorial.first_research");
pub const TUTORIAL_FIRST_FLEET: TutorialId = TutorialId("advisor.tutorial.first_fleet");
pub const TUTORIAL_FIRST_IDLE_COLONY: TutorialId = TutorialId("advisor.tutorial.first_idle_colony");
pub const TUTORIAL_FIRST_UNEXPLORED_SYSTEM: TutorialId =
    TutorialId("advisor.tutorial.first_unexplored_system");

pub const MECHANIC_FIRST_COLONY: MechanicId = MechanicId("advisor.mechanic.first_colony");
pub const MECHANIC_FIRST_RESEARCH: MechanicId = MechanicId("advisor.mechanic.first_research");
pub const MECHANIC_FIRST_FLEET: MechanicId = MechanicId("advisor.mechanic.first_fleet");
pub const MECHANIC_FIRST_IDLE_COLONY: MechanicId = MechanicId("advisor.mechanic.first_idle_colony");
pub const MECHANIC_FIRST_UNEXPLORED_SYSTEM: MechanicId =
    MechanicId("advisor.mechanic.first_unexplored_system");

pub struct FirstColonyManagementRule;

impl AdvisorRule for FirstColonyManagementRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("first_colony_management")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        if ctx.knowledge.seen_mechanics.contains(&MECHANIC_FIRST_COLONY)
            || ctx.knowledge.dismissed_tutorials.contains(&TUTORIAL_FIRST_COLONY)
        {
            return;
        }
        let Some((&colony_id, _)) = ctx
            .state
            .colonies
            .iter()
            .find(|(_, colony)| colony.owner == ctx.state.player_empire)
        else {
            return;
        };
        out.push(build_tutorial_message(
            self.id(),
            Some(AdvisorTarget::Colony(colony_id)),
            AdvisorPersona::ColonialOffice,
            AdvisorSeverity::Info,
            "Your first colony",
            "Colonies turn population into food, industry, science, and credits.",
            vec![AdvisorAction::FocusColony(colony_id)],
            TUTORIAL_FIRST_COLONY,
            ctx.turn,
        ));
    }
}

pub struct FirstResearchChoiceRule;

impl AdvisorRule for FirstResearchChoiceRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("first_research_choice")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        if ctx.knowledge.seen_mechanics.contains(&MECHANIC_FIRST_RESEARCH)
            || ctx.knowledge.dismissed_tutorials.contains(&TUTORIAL_FIRST_RESEARCH)
        {
            return;
        }
        let Some(empire) = ctx.state.empires.get(&ctx.state.player_empire) else {
            return;
        };
        if empire.research.current_tech.is_some() {
            return;
        }
        if available_tech_ids(&empire.research.completed).is_empty() {
            return;
        }

        out.push(build_tutorial_message(
            self.id(),
            Some(AdvisorTarget::Empire),
            AdvisorPersona::ScienceDirector,
            AdvisorSeverity::Suggestion,
            "Select first research project",
            "Choose a technology to start long-term research momentum.",
            vec![AdvisorAction::OpenResearch],
            TUTORIAL_FIRST_RESEARCH,
            ctx.turn,
        ));
    }
}

pub struct FirstFleetRule;

impl AdvisorRule for FirstFleetRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("first_fleet")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        if ctx.knowledge.seen_mechanics.contains(&MECHANIC_FIRST_FLEET)
            || ctx.knowledge.dismissed_tutorials.contains(&TUTORIAL_FIRST_FLEET)
        {
            return;
        }
        let Some((&fleet_id, _)) = ctx
            .state
            .fleets
            .iter()
            .find(|(_, fleet)| fleet.owner == ctx.state.player_empire && fleet.ships > 0)
        else {
            return;
        };
        out.push(build_tutorial_message(
            self.id(),
            Some(AdvisorTarget::Fleet(fleet_id)),
            AdvisorPersona::MilitaryCommand,
            AdvisorSeverity::Suggestion,
            "Fleet ready for orders",
            "Use fleets to scout lanes, survey systems, and project power.",
            vec![AdvisorAction::FocusFleet(fleet_id)],
            TUTORIAL_FIRST_FLEET,
            ctx.turn,
        ));
    }
}

pub struct FirstIdleColonyRule;

impl AdvisorRule for FirstIdleColonyRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("first_idle_colony")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        if ctx
            .knowledge
            .seen_mechanics
            .contains(&MECHANIC_FIRST_IDLE_COLONY)
            || ctx
                .knowledge
                .dismissed_tutorials
                .contains(&TUTORIAL_FIRST_IDLE_COLONY)
        {
            return;
        }
        let Some((&colony_id, _)) = ctx.state.colonies.iter().find(|(_, colony)| {
            colony.owner == ctx.state.player_empire && colony.build_queue.is_empty()
        }) else {
            return;
        };

        out.push(build_tutorial_message(
            self.id(),
            Some(AdvisorTarget::Colony(colony_id)),
            AdvisorPersona::ColonialOffice,
            AdvisorSeverity::Suggestion,
            "Idle colony queue",
            "Colonies usually perform best with a queued build plan.",
            vec![AdvisorAction::FocusColony(colony_id)],
            TUTORIAL_FIRST_IDLE_COLONY,
            ctx.turn,
        ));
    }
}

pub struct FirstUnexploredNearbySystemRule;

impl AdvisorRule for FirstUnexploredNearbySystemRule {
    fn id(&self) -> AdvisorRuleId {
        AdvisorRuleId("first_unexplored_nearby_system")
    }

    fn evaluate(&self, ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
        if ctx
            .knowledge
            .seen_mechanics
            .contains(&MECHANIC_FIRST_UNEXPLORED_SYSTEM)
            || ctx
                .knowledge
                .dismissed_tutorials
                .contains(&TUTORIAL_FIRST_UNEXPLORED_SYSTEM)
        {
            return;
        }
        let Some(target_system) = nearest_unexplored_system(ctx.state) else {
            return;
        };
        out.push(build_tutorial_message(
            self.id(),
            Some(AdvisorTarget::System(target_system)),
            AdvisorPersona::Guide,
            AdvisorSeverity::Suggestion,
            "Unexplored system nearby",
            "Send a scout to reveal system data and expansion options.",
            vec![AdvisorAction::FocusSystem(target_system)],
            TUTORIAL_FIRST_UNEXPLORED_SYSTEM,
            ctx.turn,
        ));
    }
}

fn nearest_unexplored_system(state: &crate::state::GameState) -> Option<StarId> {
    let anchor = first_player_colony(state)
        .and_then(|colony| state.stars.get(&colony.star))
        .or_else(|| {
            state
                .empires
                .get(&state.player_empire)
                .and_then(|empire| state.stars.get(&empire.home_star))
        })?;
    state
        .stars
        .iter()
        .filter(|(star_id, _)| !state.explored_stars.contains(star_id))
        .map(|(star_id, star)| {
            let dx = i64::from(star.x) - i64::from(anchor.x);
            let dy = i64::from(star.y) - i64::from(anchor.y);
            let dist_sq = dx * dx + dy * dy;
            (*star_id, dist_sq)
        })
        .min_by_key(|(star_id, dist_sq)| (*dist_sq, *star_id))
        .map(|(star_id, _)| star_id)
}

fn first_player_colony(state: &crate::state::GameState) -> Option<&Colony> {
    state
        .colonies
        .values()
        .find(|colony| colony.owner == state.player_empire)
}

fn build_tutorial_message(
    rule_id: AdvisorRuleId,
    target: Option<AdvisorTarget>,
    persona: AdvisorPersona,
    severity: AdvisorSeverity,
    title: &str,
    body: &str,
    actions: Vec<AdvisorAction>,
    tutorial_id: TutorialId,
    turn: u32,
) -> AdvisorMessage {
    let key = AdvisorMessageKey { rule_id, target };
    AdvisorMessage {
        id: AdvisorMessageId(message_id(&key)),
        key,
        category: AdvisorCategory::Tutorial,
        persona,
        severity,
        title: title.to_string(),
        body: body.to_string(),
        turn_created: turn,
        expires_on_turn: None,
        actions,
        dismissible: true,
        tutorial_id: Some(tutorial_id),
        target: key.target,
    }
}

fn message_id(key: &AdvisorMessageKey) -> String {
    match key.target {
        Some(AdvisorTarget::Empire) => format!("{}:empire", key.rule_id.0),
        Some(AdvisorTarget::System(id)) => format!("{}:system:{}", key.rule_id.0, id.0),
        Some(AdvisorTarget::Colony(id)) => format!("{}:colony:{}", key.rule_id.0, id.0),
        Some(AdvisorTarget::Fleet(id)) => format!("{}:fleet:{}", key.rule_id.0, id.0),
        Some(AdvisorTarget::Tech(id)) => format!("{}:tech:{}", key.rule_id.0, id.0),
        None => key.rule_id.0.to_string(),
    }
}
