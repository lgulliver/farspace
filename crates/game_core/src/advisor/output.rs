use crate::advisor::message::AdvisorMessage;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdvisorOutput {
    pub active: Vec<AdvisorMessage>,
}
