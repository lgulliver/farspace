pub mod engine;
pub mod knowledge;
pub mod message;
pub mod output;
pub mod preferences;
pub mod rules;

pub use engine::{AdvisorContext, AdvisorEngine};
pub use knowledge::PlayerKnowledge;
pub use message::{
    AdvisorAction, AdvisorCategory, AdvisorMessage, AdvisorMessageId, AdvisorMessageKey,
    AdvisorPersona, AdvisorRuleId, AdvisorSeverity, AdvisorTarget, MechanicId, ScreenRef,
    TutorialId,
};
pub use output::AdvisorOutput;
pub use preferences::AdvisorPreferences;
pub use rules::AdvisorRule;

pub type GameEvent = crate::events::Event;
