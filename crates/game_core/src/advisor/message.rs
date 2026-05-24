use crate::state::{ColonyId, FleetId, StarId, TechId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AdvisorMessageId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AdvisorRuleId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TutorialId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MechanicId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdvisorCategory {
    Tutorial,
    Economy,
    Colony,
    Military,
    Science,
    Diplomacy,
    Exploration,
    System,
}

impl AdvisorCategory {
    pub(crate) const fn sort_key(self) -> u8 {
        match self {
            AdvisorCategory::Tutorial => 0,
            AdvisorCategory::Economy => 1,
            AdvisorCategory::Colony => 2,
            AdvisorCategory::Military => 3,
            AdvisorCategory::Science => 4,
            AdvisorCategory::Diplomacy => 5,
            AdvisorCategory::Exploration => 6,
            AdvisorCategory::System => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdvisorSeverity {
    Info,
    Suggestion,
    Warning,
    Critical,
}

impl AdvisorSeverity {
    pub(crate) const fn sort_key(self) -> u8 {
        match self {
            AdvisorSeverity::Info => 0,
            AdvisorSeverity::Suggestion => 1,
            AdvisorSeverity::Warning => 2,
            AdvisorSeverity::Critical => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdvisorPersona {
    Guide,
    EconomicAdvisor,
    ScienceDirector,
    MilitaryCommand,
    DiplomaticEnvoy,
    ColonialOffice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdvisorTarget {
    Empire,
    System(StarId),
    Colony(ColonyId),
    Fleet(FleetId),
    Tech(TechId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AdvisorMessageKey {
    pub rule_id: AdvisorRuleId,
    pub target: Option<AdvisorTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScreenRef {
    Galaxy,
    Colony,
    Research,
    Fleets,
    Diplomacy,
    AdvisorHistory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisorAction {
    OpenScreen(ScreenRef),
    FocusSystem(StarId),
    FocusColony(ColonyId),
    FocusFleet(FleetId),
    OpenResearch,
    OpenShipyard(ColonyId),
    OpenCommandPalette(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisorMessage {
    pub id: AdvisorMessageId,
    pub key: AdvisorMessageKey,
    pub category: AdvisorCategory,
    pub persona: AdvisorPersona,
    pub severity: AdvisorSeverity,
    pub title: String,
    pub body: String,
    pub turn_created: u32,
    pub expires_on_turn: Option<u32>,
    pub actions: Vec<AdvisorAction>,
    pub dismissible: bool,
    pub tutorial_id: Option<TutorialId>,
    pub target: Option<AdvisorTarget>,
}
