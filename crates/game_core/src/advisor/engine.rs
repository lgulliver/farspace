use crate::advisor::rules::{
    colony::{
        ColonyFoodDeficitRule, ColonyUnrestRule, IdleColonyProductionRule, UndefendedColonyRule,
    },
    diplomacy::LowIntelRule,
    economy::TreasuryDepletionRiskRule,
    military::IdleFleetRule,
    science::NoActiveResearchRule,
    tutorial::{
        FirstColonyManagementRule, FirstFleetRule, FirstIdleColonyRule, FirstResearchChoiceRule,
        FirstUnexploredNearbySystemRule,
    },
    AdvisorRule,
};
use crate::advisor::{
    AdvisorMessage, AdvisorOutput, AdvisorPreferences, AdvisorSeverity, PlayerKnowledge,
};
use crate::events::Event;
use crate::state::GameState;
use std::collections::BTreeSet;

pub struct AdvisorContext<'a> {
    pub state: &'a GameState,
    pub events: &'a [Event],
    pub knowledge: &'a PlayerKnowledge,
    pub preferences: &'a AdvisorPreferences,
    pub turn: u32,
}

pub struct AdvisorEngine {
    rules: Vec<Box<dyn AdvisorRule>>,
    max_messages_per_turn: usize,
}

impl Default for AdvisorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvisorEngine {
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(FirstColonyManagementRule),
                Box::new(FirstResearchChoiceRule),
                Box::new(FirstFleetRule),
                Box::new(FirstIdleColonyRule),
                Box::new(FirstUnexploredNearbySystemRule),
                Box::new(IdleColonyProductionRule),
                Box::new(ColonyFoodDeficitRule),
                Box::new(ColonyUnrestRule),
                Box::new(UndefendedColonyRule),
                Box::new(IdleFleetRule),
                Box::new(NoActiveResearchRule),
                Box::new(TreasuryDepletionRiskRule),
                Box::new(LowIntelRule),
            ],
            max_messages_per_turn: AdvisorPreferences::default().max_messages_per_turn,
        }
    }

    pub fn evaluate(&self, ctx: &AdvisorContext<'_>) -> AdvisorOutput {
        if !ctx.preferences.enabled {
            return AdvisorOutput::default();
        }

        let mut messages: Vec<AdvisorMessage> = Vec::new();
        for rule in &self.rules {
            rule.evaluate(ctx, &mut messages);
        }

        messages.retain(|message| {
            !ctx.preferences.muted_categories.contains(&message.category)
                && !ctx.knowledge.dismissed_message_keys.contains(&message.key)
                && message
                    .tutorial_id
                    .is_none_or(|id| !ctx.knowledge.dismissed_tutorials.contains(&id))
        });

        let mut seen_keys = BTreeSet::new();
        messages.retain(|message| seen_keys.insert(message.key.clone()));

        messages.sort_by(|a, b| {
            b.severity
                .sort_key()
                .cmp(&a.severity.sort_key())
                .then_with(|| a.category.sort_key().cmp(&b.category.sort_key()))
                .then_with(|| a.id.0.cmp(&b.id.0))
        });

        let cap = ctx
            .preferences
            .max_messages_per_turn
            .min(self.max_messages_per_turn);
        let critical_count = messages
            .iter()
            .filter(|msg| msg.severity == AdvisorSeverity::Critical)
            .count();
        let keep_non_critical = cap.saturating_sub(critical_count);
        let mut non_critical_taken = 0usize;
        messages.retain(|msg| {
            if msg.severity == AdvisorSeverity::Critical {
                true
            } else if non_critical_taken < keep_non_critical {
                non_critical_taken += 1;
                true
            } else {
                false
            }
        });

        AdvisorOutput { active: messages }
    }

    #[cfg(test)]
    fn with_rules(rules: Vec<Box<dyn AdvisorRule>>, max_messages_per_turn: usize) -> Self {
        Self {
            rules,
            max_messages_per_turn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::rules::tutorial::{FirstColonyManagementRule, TUTORIAL_FIRST_COLONY};
    use crate::advisor::{
        AdvisorAction, AdvisorCategory, AdvisorMessage, AdvisorMessageId, AdvisorMessageKey,
        AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget, MechanicId,
    };
    use crate::state::{
        Colony, ColonyId, Empire, EmpireId, Fleet, FleetId, FleetKind, GameState, Planet,
        PlanetClass, PlanetSize, SectorId, SpectralClass, Star, StarId,
    };

    struct StaticRule {
        id: AdvisorRuleId,
        messages: Vec<AdvisorMessage>,
    }

    impl AdvisorRule for StaticRule {
        fn id(&self) -> AdvisorRuleId {
            self.id
        }

        fn evaluate(&self, _ctx: &AdvisorContext<'_>, out: &mut Vec<AdvisorMessage>) {
            out.extend(self.messages.clone());
        }
    }

    fn test_state() -> GameState {
        let player_id = EmpireId(1);
        let star_id = StarId(1);
        let mut state = GameState {
            player_empire: player_id,
            turn: 7,
            ..GameState::default()
        };
        state.stars.insert(
            star_id,
            Star {
                id: star_id,
                sector: SectorId(0),
                name: "Talos".to_string(),
                x: 0,
                y: 0,
                spectral_class: SpectralClass::G,
                planets: vec![Planet {
                    name: "Talos Prime".to_string(),
                    size: PlanetSize::Medium,
                    class: PlanetClass::Terran,
                    colony: Some(ColonyId(1)),
                    habitable: true,
                    surveyed: false,
                    specials: vec![],
                    resources: vec![],
                    anomalies: vec![],
                    ancient_ruins_collected: false,
                }],
            },
        );
        state.empires.insert(
            player_id,
            Empire {
                id: player_id,
                name: "Player".to_string(),
                credits: 100,
                research_points: 0,
                home_star: star_id,
                research: Default::default(),
                food: 0,
                empire_def: None,
            },
        );
        state.colonies.insert(
            ColonyId(1),
            Colony {
                id: ColonyId(1),
                star: star_id,
                planet_index: 0,
                owner: player_id,
                population: 3,
                production: 10,
                prod_pct: 50,
                research_pct: 50,
                build_queue: vec![],
                accumulated_production: 0,
                buildings: vec![],
                surface_installations: vec![],
                orbital_installations: vec![],
                stability: 100,
                role: Default::default(),
                rally_point: None,
            },
        );
        state.fleets.insert(
            FleetId(1),
            Fleet {
                id: FleetId(1),
                owner: player_id,
                location: star_id,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        state
    }

    fn msg(
        id: &str,
        rule_id: &'static str,
        target: Option<AdvisorTarget>,
        category: AdvisorCategory,
        severity: AdvisorSeverity,
    ) -> AdvisorMessage {
        AdvisorMessage {
            id: AdvisorMessageId(id.to_string()),
            key: AdvisorMessageKey {
                rule_id: AdvisorRuleId(rule_id),
                target,
            },
            category,
            persona: AdvisorPersona::Guide,
            severity,
            title: id.to_string(),
            body: id.to_string(),
            turn_created: 1,
            expires_on_turn: None,
            actions: vec![AdvisorAction::OpenResearch],
            dismissible: true,
            tutorial_id: None,
            target,
        }
    }

    #[test]
    fn tutorial_fires_once() {
        let state = test_state();
        let prefs = AdvisorPreferences::default();
        let knowledge = PlayerKnowledge::default();
        let engine = AdvisorEngine::with_rules(vec![Box::new(FirstColonyManagementRule)], 5);
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &knowledge,
            preferences: &prefs,
            turn: state.turn,
        });
        assert_eq!(output.active.len(), 1);

        let mut seen = PlayerKnowledge::default();
        seen.seen_mechanics
            .insert(MechanicId("advisor.mechanic.first_colony"));
        let second = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &seen,
            preferences: &prefs,
            turn: state.turn,
        });
        assert!(second.active.is_empty());
    }

    #[test]
    fn dismissed_tutorial_does_not_reappear() {
        let state = test_state();
        let prefs = AdvisorPreferences::default();
        let mut knowledge = PlayerKnowledge::default();
        knowledge.dismissed_tutorials.insert(TUTORIAL_FIRST_COLONY);
        let engine = AdvisorEngine::with_rules(vec![Box::new(FirstColonyManagementRule)], 5);
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &knowledge,
            preferences: &prefs,
            turn: state.turn,
        });
        assert!(output.active.is_empty());
    }

    #[test]
    fn idle_colony_warning_deduplicates_correctly() {
        let duplicate_a = msg(
            "a",
            "idle_colony",
            Some(AdvisorTarget::Colony(ColonyId(1))),
            AdvisorCategory::Colony,
            AdvisorSeverity::Warning,
        );
        let duplicate_b = msg(
            "b",
            "idle_colony",
            Some(AdvisorTarget::Colony(ColonyId(1))),
            AdvisorCategory::Colony,
            AdvisorSeverity::Warning,
        );
        let engine = AdvisorEngine::with_rules(
            vec![
                Box::new(StaticRule {
                    id: AdvisorRuleId("a"),
                    messages: vec![duplicate_a],
                }),
                Box::new(StaticRule {
                    id: AdvisorRuleId("b"),
                    messages: vec![duplicate_b],
                }),
            ],
            5,
        );
        let state = test_state();
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &PlayerKnowledge::default(),
            preferences: &AdvisorPreferences::default(),
            turn: state.turn,
        });
        assert_eq!(output.active.len(), 1);
        assert_eq!(output.active[0].id.0, "a");
    }

    #[test]
    fn critical_messages_bypass_message_cap() {
        let critical = msg(
            "critical",
            "critical",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::System,
            AdvisorSeverity::Critical,
        );
        let critical_2 = msg(
            "critical-2",
            "critical-2",
            Some(AdvisorTarget::Fleet(FleetId(1))),
            AdvisorCategory::System,
            AdvisorSeverity::Critical,
        );
        let warning = msg(
            "warning",
            "warning",
            Some(AdvisorTarget::Colony(ColonyId(1))),
            AdvisorCategory::Colony,
            AdvisorSeverity::Warning,
        );
        let engine = AdvisorEngine::with_rules(
            vec![Box::new(StaticRule {
                id: AdvisorRuleId("s"),
                messages: vec![warning, critical, critical_2],
            })],
            1,
        );
        let state = test_state();
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &PlayerKnowledge::default(),
            preferences: &AdvisorPreferences {
                max_messages_per_turn: 1,
                ..AdvisorPreferences::default()
            },
            turn: state.turn,
        });
        assert_eq!(output.active.len(), 2);
        assert!(output
            .active
            .iter()
            .all(|m| m.severity == AdvisorSeverity::Critical));
    }

    #[test]
    fn message_sorting_is_deterministic() {
        let info = msg(
            "b",
            "info",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::Science,
            AdvisorSeverity::Info,
        );
        let warning = msg(
            "c",
            "warning",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::Economy,
            AdvisorSeverity::Warning,
        );
        let critical = msg(
            "a",
            "critical",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::Colony,
            AdvisorSeverity::Critical,
        );
        let suggestion = msg(
            "d",
            "suggestion",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::Tutorial,
            AdvisorSeverity::Suggestion,
        );

        let engine = AdvisorEngine::with_rules(
            vec![Box::new(StaticRule {
                id: AdvisorRuleId("s"),
                messages: vec![info, warning, critical, suggestion],
            })],
            10,
        );
        let state = test_state();
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &PlayerKnowledge::default(),
            preferences: &AdvisorPreferences {
                max_messages_per_turn: 10,
                ..AdvisorPreferences::default()
            },
            turn: state.turn,
        });
        let ids: Vec<_> = output.active.iter().map(|m| m.id.0.as_str()).collect();
        assert_eq!(ids, vec!["a", "c", "d", "b"]);
    }

    #[test]
    fn muted_categories_are_filtered() {
        let economy_warning = msg(
            "econ",
            "econ_rule",
            Some(AdvisorTarget::Empire),
            AdvisorCategory::Economy,
            AdvisorSeverity::Warning,
        );
        let engine = AdvisorEngine::with_rules(
            vec![Box::new(StaticRule {
                id: AdvisorRuleId("s"),
                messages: vec![economy_warning],
            })],
            5,
        );
        let state = test_state();
        let mut prefs = AdvisorPreferences::default();
        prefs.muted_categories.insert(AdvisorCategory::Economy);
        let output = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &PlayerKnowledge::default(),
            preferences: &prefs,
            turn: state.turn,
        });
        assert!(output.active.is_empty());
    }

    #[test]
    fn stable_output_for_identical_input() {
        let state = test_state();
        let prefs = AdvisorPreferences::default();
        let knowledge = PlayerKnowledge::default();
        let engine = AdvisorEngine::new();

        let first = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &knowledge,
            preferences: &prefs,
            turn: state.turn,
        });
        let second = engine.evaluate(&AdvisorContext {
            state: &state,
            events: &[],
            knowledge: &knowledge,
            preferences: &prefs,
            turn: state.turn,
        });
        assert_eq!(first, second);
    }
}
