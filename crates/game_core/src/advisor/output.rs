use crate::advisor::message::AdvisorMessage;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct AdvisorOutput {
    pub active: Vec<AdvisorMessage>,
}
