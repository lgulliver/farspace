use crate::advisor::message::AdvisorCategory;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AdvisorPreferences {
    pub enabled: bool,
    pub muted_categories: HashSet<AdvisorCategory>,
    pub max_messages_per_turn: usize,
}

impl Default for AdvisorPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            muted_categories: HashSet::new(),
            max_messages_per_turn: 5,
        }
    }
}
