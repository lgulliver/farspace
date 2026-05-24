use crate::advisor::message::{AdvisorMessageKey, MechanicId, TutorialId};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PlayerKnowledge {
    pub dismissed_message_keys: HashSet<AdvisorMessageKey>,
    pub dismissed_tutorials: HashSet<TutorialId>,
    pub seen_mechanics: HashSet<MechanicId>,
}
