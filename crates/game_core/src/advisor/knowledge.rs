use crate::advisor::message::{AdvisorMessageKey, MechanicId, TutorialId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerKnowledge {
    pub dismissed_message_keys: BTreeSet<AdvisorMessageKey>,
    pub dismissed_tutorials: BTreeSet<TutorialId>,
    pub seen_mechanics: BTreeSet<MechanicId>,
}
