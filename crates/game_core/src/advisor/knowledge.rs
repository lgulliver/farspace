use crate::advisor::message::{AdvisorMessageKey, MechanicId, TutorialId};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerKnowledge {
    pub dismissed_message_keys: HashSet<AdvisorMessageKey>,
    pub dismissed_tutorials: HashSet<TutorialId>,
    pub seen_mechanics: HashSet<MechanicId>,
}
