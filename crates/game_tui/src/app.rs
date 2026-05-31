//! Application state and main run loop

mod logging;

use crate::animation::{ScreenTransition, TransitionState};
use crate::components::{
    render_battle_reports, render_dispatch, render_help, render_palette, EventLog, LogEntryKind,
    PaletteCommand,
};
use crate::keys::KeyMap;
use crate::screens::empire_overview::{derive_empire_overview, EmpireOverviewData, OverviewSort};
use crate::screens::menu::{menu_action_count, MenuAction};
use crate::screens::research::{
    filtered_research_techs, RESEARCH_DOMAIN_FILTER_COUNT, RESEARCH_ERA_FILTER_COUNT,
    RESEARCH_STATUS_FILTER_COUNT,
};
use crate::screens::ship_designer::{DesignerMode, DesignerPanel, ShipDesignerState};
use crate::screens::Screen;
use crate::update::{UpdateChannel, UpdateConfirmKind, UpdateInfo, UpdateState};
use crate::visual_mode::{map_symbol_for_mode, user_config_path, VisualMode};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::advisor::{
    AdvisorContext, AdvisorEngine, AdvisorOutput, AdvisorPreferences, PlayerKnowledge,
};
use game_core::{
    empire_definition_by_id, tech_by_id, BuildingType, ColonyId, ColonyRole, Command, ComponentId,
    Engine, Event as CoreEvent, FleetFormation, FleetId, FleetKind, FleetRole, GalaxySize,
    OrbitalStructureType, ScenarioSetup, SectorId, StarId, TechId, TreatyType,
};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::path::Path;
use std::time::Duration;

/// Default save file path
const DEFAULT_SAVE_PATH: &str = "farspace.sav";

/// Render ticks for the subtle fade when a campaign launches. UI-only.
const CAMPAIGN_ENTRY_TRANSITION_TICKS: u16 = 6;

/// Short, human-readable explanation of an IO failure (no debug noise).
fn io_error_reason(err: &std::io::Error) -> &'static str {
    use std::io::ErrorKind;
    match err.kind() {
        ErrorKind::NotFound => "the file could not be found.",
        ErrorKind::PermissionDenied => "permission denied.",
        _ => "the file could not be read from disk.",
    }
}

/// Map a [`game_save::SaveError`] onto a calm, player-facing sentence. Raw Rust
/// error text never reaches the UI; the full error is still suitable for logs.
fn friendly_load_error(err: &game_save::SaveError) -> String {
    use game_save::SaveError;
    let reason = match err {
        SaveError::UnsupportedVersion { .. } => {
            "save version is newer than this build.".to_string()
        }
        SaveError::MigrationFailed { .. } => {
            "it could not be upgraded from an older version.".to_string()
        }
        SaveError::Empty
        | SaveError::CorruptedSave { .. }
        | SaveError::MissingField { .. }
        | SaveError::Json(_) => "file is corrupted or incomplete.".to_string(),
        SaveError::Io(io) => io_error_reason(io).to_string(),
    };
    format!("Could not load campaign: {reason}")
}

/// Main application state
pub struct App {
    state: AppState,
    engine: Option<Engine>,
    /// Receives the result of the background update check (Ok(Some) = update available, Ok(None) = up to date, Err = check failed).
    check_rx: Option<std::sync::mpsc::Receiver<Result<Option<UpdateInfo>, String>>>,
    /// Sends an update request to the background download worker.
    download_tx: Option<std::sync::mpsc::SyncSender<UpdateInfo>>,
    /// Receives download completion (Ok(version) = staged, Err = failed).
    download_rx: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    /// Set to true when the user confirms "apply update and restart".
    /// The binary crate checks this after `run()` returns and re-execs the process.
    restart_requested: bool,
}

/// UI state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub(crate) active: Screen,
    /// The screen to return to when closing Ship Designer with Esc.
    pub(crate) previous_screen: Screen,
    pub(crate) overlay: OverlayState,
    pub(crate) navigation: NavigationState,
    pub(crate) sector_overview: SectorOverviewState,
    pub(crate) colony: ColonyScreenState,
    pub(crate) research: ResearchScreenState,
    pub(crate) overview: EmpireOverviewScreenState,
    pub(crate) diplomacy: DiplomacyScreenState,
    pub(crate) new_game_setup: NewGameSetupState,
    pub(crate) log: EventLog,
    pub(crate) quit: bool,
    /// Monotonically-increasing frame counter, incremented once per render loop iteration.
    /// Used only for low-frequency UI animations; never affects simulation state.
    pub(crate) tick_count: u64,
    /// When true, all fleet travel animations are suppressed (accessibility / low-motion).
    pub(crate) reduced_motion: bool,
    /// Active screen transition (UI-only, tick-driven). Scaffolding for future
    /// transition compositing; never affects simulation state.
    pub(crate) transition: TransitionState,
    /// Status line shown in contextual footer hints.
    pub(crate) status_message: Option<String>,
    /// Latest deterministic advisor guidance, recomputed on new game and each
    /// turn end. Drives the turn brief and contextual advisor strip.
    pub(crate) advisor_output: AdvisorOutput,
    /// What the player has already seen/dismissed; gates one-shot tutorial tips.
    pub(crate) advisor_knowledge: PlayerKnowledge,
    /// Advisor display preferences (enabled, muted categories, message cap).
    pub(crate) advisor_prefs: AdvisorPreferences,
    pub(crate) ship_designer: ShipDesignerState,
    /// Terminal glyph mode for rendering text and icons.
    pub(crate) visual_mode: VisualMode,
    /// Which release channel to track for updates.
    pub(crate) update_channel: UpdateChannel,
    /// Whether to automatically download and stage updates when found.
    pub(crate) auto_update: bool,
    /// Current state of the update lifecycle.
    pub(crate) update_state: UpdateState,
    /// Cursor position on the Settings screen.
    pub(crate) settings_cursor: usize,
    /// Cursor position on the menu screen.
    pub(crate) menu_cursor: usize,
    /// Whether at least one readable save exists, gating the "Continue" menu
    /// action. Refreshed when the menu is shown and after save/delete.
    pub(crate) can_continue: bool,
}

/// UI overlay state shared by all screens.
#[derive(Debug, Clone, Default)]
pub(crate) struct OverlayState {
    pub(crate) show_help: bool,
    pub(crate) show_palette: bool,
    pub(crate) palette_input: String,
    /// Whether the Galactic Dispatch modal is open
    pub(crate) show_dispatch: bool,
    /// Index into the dispatch history (0 = oldest shown)
    pub(crate) dispatch_history_index: usize,
    /// Whether the battle report modal is open.
    pub(crate) show_battle_reports: bool,
    /// Selected battle report index in history.
    pub(crate) battle_report_index: usize,
    /// Toggle between list and detailed inspect mode.
    pub(crate) battle_report_inspect: bool,
    /// Whether the Settings modal is open.
    pub(crate) show_settings: bool,
    /// Update confirmation dialog — `Some` means the dialog is showing.
    pub(crate) update_confirm: Option<UpdateConfirmKind>,
    /// Campaign Archives (save browser) overlay state.
    pub(crate) archives: ArchivesState,
}

/// Campaign Archives overlay state. Holds the scanned save summaries plus the
/// browser cursor and modal flags. The summaries are a UI snapshot, refreshed
/// each time the overlay is opened.
#[derive(Debug, Clone, Default)]
pub(crate) struct ArchivesState {
    pub(crate) open: bool,
    pub(crate) entries: Vec<game_save::SaveSlotSummary>,
    pub(crate) cursor: usize,
    pub(crate) confirm_delete: bool,
    pub(crate) show_help: bool,
    pub(crate) error: Option<String>,
}

/// Cross-screen map/system selection state.
#[derive(Debug, Clone, Default)]
pub(crate) struct NavigationState {
    pub(crate) selected_sector: Option<SectorId>,
    pub(crate) selected_star: Option<StarId>,
    /// Selected planet index when inspecting a system.
    pub(crate) selected_planet_index: usize,
    /// Selected fleet index when inspecting fleet posture in a system.
    pub(crate) selected_fleet_index: usize,
}

/// Sector overview screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct SectorOverviewState {
    /// Toggle for rendering inter-sector hyperspace lanes on galaxy overview.
    pub(crate) show_inter_sector_lanes: bool,
}

/// Colony screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ColonyScreenState {
    /// Currently viewed colony (set when entering the colony screen).
    pub(crate) selected_colony: Option<ColonyId>,
    /// Cursor index for the build-picker on the colony screen.
    pub(crate) build_cursor: usize,
    /// Cursor index for the role selector on the colony screen.
    pub(crate) role_cursor: usize,
    /// Whether the role selector is the active panel (true) or build picker (false).
    pub(crate) role_panel_active: bool,
    /// Colony for which a rally point is being picked.
    ///
    /// When `Some`, the sector map shows a prompt and pressing 'R' or Enter
    /// while navigating will confirm the selected star as the rally destination.
    /// Pressing Esc cancels.
    pub(crate) pending_rally_colony: Option<ColonyId>,
}

/// Research screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResearchScreenState {
    /// Cursor index for the tech list on the research screen.
    pub(crate) cursor: usize,
    /// Domain filter index (0 = all, then domain order).
    pub(crate) domain_filter: usize,
    /// Era filter index (0 = all, then Era 1..6).
    pub(crate) era_filter: usize,
    /// Status filter index (0 = all, then available/locked/active/completed).
    pub(crate) status_filter: usize,
    /// Case-insensitive text query for technology name/description/tag.
    pub(crate) query: String,
    /// True while text input mode is active for query editing.
    pub(crate) query_input: bool,
}

/// Empire overview screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct EmpireOverviewScreenState {
    /// Cursor index for selected row on empire overview.
    pub(crate) cursor: usize,
    /// Current sort mode for empire overview.
    pub(crate) sort: OverviewSort,
    /// Optional overview filter query.
    pub(crate) filter: String,
    /// Whether filter input mode is active on the overview screen.
    pub(crate) filter_input: bool,
}

/// New Game Setup screen state.
#[derive(Debug, Clone)]
pub(crate) struct NewGameSetupState {
    /// Galaxy size selected on the setup screen.
    pub(crate) galaxy_size: GalaxySize,
    /// Number of AI empires selected on the setup screen (1–4).
    pub(crate) ai_count: u8,
    /// Seed string shown on the setup screen (ASCII digits only).
    pub(crate) seed_str: String,
    /// Which field is currently highlighted on the setup screen.
    /// 0 = empire selection, 1 = galaxy size, 2 = AI count, 3 = seed.
    pub(crate) cursor: usize,
    /// Whether the seed text-edit mode is active.
    pub(crate) seed_editing: bool,
    /// Snapshot of `setup_seed_str` taken when seed editing begins, used to
    /// restore the original value if the user presses Esc to discard changes.
    pub(crate) seed_pre_edit: String,
    /// Index into `all_empire_definitions()` for the empire the player has chosen.
    pub(crate) empire_cursor: usize,
}

/// Diplomacy screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiplomacyScreenState {
    /// Cursor index for selected foreign empire.
    pub(crate) selected_empire_index: usize,
    /// Cursor index for selected communication response.
    pub(crate) selected_response_index: usize,
    /// Whether the communication modal is open.
    pub(crate) show_communication_modal: bool,
    /// Selected communication index within player-targeted pending messages.
    pub(crate) selected_communication_index: usize,
}

impl Default for NewGameSetupState {
    fn default() -> Self {
        Self {
            galaxy_size: GalaxySize::Medium,
            ai_count: 1,
            seed_str: "42".to_string(),
            cursor: 0,
            seed_editing: false,
            seed_pre_edit: String::new(),
            empire_cursor: 0,
        }
    }
}

fn fleet_is_idle(state: &game_core::GameState, fleet_id: FleetId) -> bool {
    !state.scout_missions.contains_key(&fleet_id)
        && !state.survey_missions.contains_key(&fleet_id)
        && !state.fleet_missions.contains_key(&fleet_id)
}

fn first_idle_player_fleet(
    state: &game_core::GameState,
    kind: Option<FleetKind>,
    location: Option<StarId>,
) -> Option<FleetId> {
    state
        .fleets
        .values()
        .find(|fleet| {
            fleet.owner == state.player_empire
                && kind.is_none_or(|kind| fleet.kind == kind)
                && location.is_none_or(|location| fleet.location == location)
                && fleet_is_idle(state, fleet.id)
        })
        .map(|fleet| fleet.id)
}

/// Parsed contents of the user config file.
struct AppConfig {
    visual_mode: VisualMode,
    update_channel: UpdateChannel,
    auto_update: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            visual_mode: VisualMode::default(),
            update_channel: UpdateChannel::default(),
            auto_update: true,
        }
    }
}

impl App {
    fn load_config_from_path(path: &std::path::Path) -> AppConfig {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return AppConfig::default();
        };
        let mut cfg = AppConfig::default();
        for line in contents.lines() {
            let mut parts = line.splitn(2, '=');
            let Some(key) = parts.next().map(str::trim) else {
                continue;
            };
            let Some(val) = parts.next().map(str::trim) else {
                continue;
            };
            match key {
                "visual_mode" => {
                    if let Some(m) = VisualMode::from_config_value(val) {
                        cfg.visual_mode = m;
                    }
                }
                "update_channel" => {
                    if let Some(c) = UpdateChannel::from_config_value(val) {
                        cfg.update_channel = c;
                    }
                }
                "auto_update" => {
                    let v = val.trim().to_ascii_lowercase();
                    cfg.auto_update = v != "false" && v != "0" && v != "off";
                }
                _ => {}
            }
        }
        cfg
    }

    fn persist_config_to_path(path: &std::path::Path, state: &AppState) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let auto_update_str = if state.auto_update { "true" } else { "false" };
        std::fs::write(
            path,
            format!(
                "visual_mode={}\nupdate_channel={}\nauto_update={}\n",
                state.visual_mode.config_value(),
                state.update_channel.config_value(),
                auto_update_str,
            ),
        )
    }

    fn load_config() -> AppConfig {
        let Some(path) = user_config_path() else {
            return AppConfig::default();
        };
        Self::load_config_from_path(&path)
    }

    fn persist_config(&mut self) -> std::io::Result<()> {
        let Some(path) = user_config_path() else {
            return Ok(());
        };
        Self::persist_config_to_path(&path, &self.state)
    }

    fn cycle_visual_mode_with_path(&mut self, path: Option<&Path>) {
        self.state.visual_mode = self.state.visual_mode.next();
        let message = format!(
            "Visual mode: {} ({})",
            self.state.visual_mode.label(),
            self.state.visual_mode.preview_sample()
        );
        let persist_result = match path {
            Some(path) => Self::persist_config_to_path(path, &self.state),
            None => self.persist_config(),
        };
        match persist_result {
            Ok(()) => self.push_status(LogEntryKind::Other, message),
            Err(err) => self.push_error_status(format!("{message} — config save failed: {err}")),
        }
    }

    fn cycle_visual_mode(&mut self) {
        self.cycle_visual_mode_with_path(None);
    }

    fn apply_visual_mode_fallback(&self, frame: &mut Frame) {
        // Global pass is intentional: every widget/path gets guaranteed fallback
        // coverage without per-component branching. NerdFont mode bypasses this.
        if self.state.visual_mode == VisualMode::NerdFont {
            return;
        }
        let area = frame.area();
        let buffer = frame.buffer_mut();
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buffer.cell_mut((x, y)) {
                    let mapped = map_symbol_for_mode(self.state.visual_mode, cell.symbol());
                    if let std::borrow::Cow::Owned(mapped) = mapped {
                        cell.set_symbol(&mapped);
                    }
                }
            }
        }
    }

    /// Create a new application
    pub fn new() -> Self {
        let cfg = Self::load_config();
        let mut app = App {
            state: AppState {
                visual_mode: cfg.visual_mode,
                update_channel: cfg.update_channel,
                auto_update: cfg.auto_update,
                ..AppState::default()
            },
            engine: None,
            check_rx: None,
            download_tx: None,
            download_rx: None,
            restart_requested: false,
        };
        app.refresh_continue_availability();
        app
    }

    /// Directory scanned for campaign saves. Single-file saves are written to the
    /// process working directory, so that is where the archives live too.
    fn saves_dir(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(".")
    }

    /// Refresh whether the "Continue" menu action is available by checking for at
    /// least one readable save on disk.
    fn refresh_continue_availability(&mut self) {
        self.state.can_continue = game_save::list_saves(&self.saves_dir())
            .iter()
            .any(|s| s.readable);
    }

    /// Returns the update channel the user has configured, for use by the binary crate.
    pub fn update_channel(&self) -> UpdateChannel {
        self.state.update_channel
    }

    /// Returns true if the user confirmed "apply update and restart".
    /// The binary crate should check this after `run()` returns.
    pub fn restart_requested(&self) -> bool {
        self.restart_requested
    }

    /// Wire in the update channels from the binary crate's update system.
    pub fn set_update_channels(
        &mut self,
        check_rx: std::sync::mpsc::Receiver<Result<Option<UpdateInfo>, String>>,
        download_tx: std::sync::mpsc::SyncSender<UpdateInfo>,
        download_rx: std::sync::mpsc::Receiver<Result<String, String>>,
    ) {
        self.check_rx = Some(check_rx);
        self.download_tx = Some(download_tx);
        self.download_rx = Some(download_rx);
        self.state.update_state = UpdateState::Checking;
    }

    /// Start a new game with the given seed (default setup, 1 AI empire).
    pub fn new_game(&mut self, seed: u64) {
        let setup = ScenarioSetup::default_for_seed(seed);
        self.new_game_from_setup(setup);
    }

    /// Start a new game from a `ScenarioSetup`.  Logs an error if the setup is invalid.
    pub fn new_game_from_setup(&mut self, setup: ScenarioSetup) {
        if let Err(e) = setup.validate() {
            self.state.log.push(format!("Error: Invalid setup: {}", e));
            return;
        }
        let engine = Engine::new_from_setup(setup);

        // Select the first sector and first star by default
        self.state.navigation.selected_sector = engine.state.sectors.keys().next().copied();
        self.state.navigation.selected_star = engine.state.stars.keys().next().copied();
        self.state.navigation.selected_planet_index = 0;

        // Add initial log entry with setup summary and playability hints
        let scenario_summary = if let Some(s) = &engine.state.scenario {
            let player_faction = engine
                .state
                .empires
                .get(&engine.state.player_empire)
                .and_then(|empire| empire.empire_def)
                .and_then(empire_definition_by_id)
                .map(|def| def.name)
                .unwrap_or("Unaligned");
            format!(
                "Game started — {} galaxy, {} AI empire(s), seed {}, faction {}",
                s.galaxy_size.label(),
                s.ai_empire_count,
                s.seed,
                player_faction,
            )
        } else {
            "Game started".to_string()
        };
        self.state.log.push(scenario_summary);
        self.state.log.push(
            "What to do next: Enter Sector Map, scout (S), survey (System:S), then colonize (C)."
                .to_string(),
        );
        self.state.status_message = Some(
            "First turn: Enter sector map, scout systems, choose research, queue a build."
                .to_string(),
        );

        self.engine = Some(engine);
        self.state.active = Screen::SectorOverview;
        self.recompute_advisor(&[]);
        // Subtle entry transition into the campaign. Inert under reduced motion.
        if !self.state.reduced_motion {
            self.state
                .transition
                .start(ScreenTransition::Fade, CAMPAIGN_ENTRY_TRANSITION_TICKS);
        }
    }

    /// Recompute advisor guidance from current game state and the given events.
    /// No-op when no game is loaded. Deterministic: same state + events ⇒ same
    /// guidance.
    fn recompute_advisor(&mut self, events: &[CoreEvent]) {
        let Some(engine) = &self.engine else {
            self.state.advisor_output = AdvisorOutput::default();
            return;
        };
        self.state.advisor_output = AdvisorEngine::new().evaluate(&AdvisorContext {
            state: &engine.state,
            events,
            knowledge: &self.state.advisor_knowledge,
            preferences: &self.state.advisor_prefs,
            turn: engine.state.turn,
        });
    }

    /// Save the current game to the given path. Returns an error message on failure.
    pub fn save_game(&mut self, path: &std::path::Path) -> Result<(), String> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| "No game in progress".to_string())?;
        game_save::save_to_file(&engine.state, path)
            .map_err(|e| format!("Error: Save failed ({}): {}", path.display(), e))?;
        Ok(())
    }

    /// Load a game from the given path. Returns an error message on failure.
    pub fn load_game(&mut self, path: &std::path::Path) -> Result<(), String> {
        let state = game_save::load_from_file(path)
            .map_err(|e| format!("Error: Load failed ({}): {}", path.display(), e))?;
        let selected_star = state.stars.keys().next().copied();
        let selected_sector = state.sectors.keys().next().copied();
        self.engine = Some(Engine::from_state(state));
        self.state.navigation.selected_sector = selected_sector;
        self.state.navigation.selected_star = selected_star;
        self.state.navigation.selected_planet_index = 0;
        self.state.active = Screen::SectorOverview;
        self.recompute_advisor(&[]);
        Ok(())
    }

    /// Open the Campaign Archives overlay, scanning the saves directory fresh.
    fn open_archives(&mut self) {
        let entries = game_save::list_saves(&self.saves_dir());
        self.state.overlay.archives = ArchivesState {
            open: true,
            entries,
            cursor: 0,
            confirm_delete: false,
            show_help: false,
            error: None,
        };
    }

    /// Load the most-recently-played readable campaign. Used by "Continue".
    fn continue_latest_campaign(&mut self) {
        let Some(summary) = game_save::list_saves(&self.saves_dir())
            .into_iter()
            .find(|s| s.readable)
        else {
            let msg = "No saved campaign to continue.".to_string();
            self.state.log.push(msg.clone());
            self.state.status_message = Some(msg);
            return;
        };
        self.load_selected_path(&summary.path.clone());
    }

    /// Load the campaign currently selected in the Archives overlay.
    fn load_selected_archive(&mut self) {
        let Some(entry) = self
            .state
            .overlay
            .archives
            .entries
            .get(self.state.overlay.archives.cursor)
            .cloned()
        else {
            return;
        };
        if !entry.readable {
            self.state.overlay.archives.error = Some(
                "Could not load campaign: file is corrupted or from a newer build.".to_string(),
            );
            return;
        }
        let path = entry.path.clone();
        match game_save::load_from_file(&path) {
            Ok(state) => {
                self.adopt_loaded_state(state);
                self.state.overlay.archives.open = false;
                let msg = format!("Loaded campaign \"{}\".", entry.display_name);
                self.state.log.push(msg.clone());
                self.state.status_message = Some(msg);
            }
            Err(e) => {
                self.state.overlay.archives.error = Some(friendly_load_error(&e));
            }
        }
    }

    /// Load a save path directly (no overlay), reporting via status/log.
    fn load_selected_path(&mut self, path: &Path) {
        match game_save::load_from_file(path) {
            Ok(state) => {
                self.adopt_loaded_state(state);
                let msg = format!("Loaded campaign from {}.", path.display());
                self.state.log.push(msg.clone());
                self.state.status_message = Some(msg);
            }
            Err(e) => {
                let msg = friendly_load_error(&e);
                self.state.log.push(msg.clone());
                self.state.status_message = Some(msg);
            }
        }
    }

    /// Install a freshly-loaded game state and reset navigation.
    fn adopt_loaded_state(&mut self, state: game_core::state::GameState) {
        let selected_star = state.stars.keys().next().copied();
        let selected_sector = state.sectors.keys().next().copied();
        self.engine = Some(Engine::from_state(state));
        self.state.navigation.selected_sector = selected_sector;
        self.state.navigation.selected_star = selected_star;
        self.state.navigation.selected_planet_index = 0;
        self.state.active = Screen::SectorOverview;
        self.recompute_advisor(&[]);
    }

    /// Delete the campaign currently selected in the Archives overlay, then
    /// refresh the listing and Continue availability.
    fn delete_selected_archive(&mut self) {
        let Some(entry) = self
            .state
            .overlay
            .archives
            .entries
            .get(self.state.overlay.archives.cursor)
            .cloned()
        else {
            return;
        };
        match game_save::delete_save(&entry.path) {
            Ok(()) => {
                let msg = format!("Deleted campaign \"{}\".", entry.display_name);
                self.state.log.push(msg);
                self.state.overlay.archives.entries = game_save::list_saves(&self.saves_dir());
                let len = self.state.overlay.archives.entries.len();
                self.state.overlay.archives.cursor = self
                    .state
                    .overlay
                    .archives
                    .cursor
                    .min(len.saturating_sub(1));
                self.state.overlay.archives.error = None;
                self.refresh_continue_availability();
            }
            Err(e) => {
                let reason = match &e {
                    game_save::SaveError::Io(io) => io_error_reason(io),
                    _ => "the file could not be removed.",
                };
                self.state.overlay.archives.error =
                    Some(format!("Could not delete campaign: {reason}"));
            }
        }
    }

    /// Parse and execute palette input (e.g. "save", ":save").
    fn execute_palette_input(&mut self, input: &str) {
        let command = match PaletteCommand::parse(input) {
            Ok(Some(command)) => command,
            Ok(None) => return,
            Err(err) => {
                self.push_error_status(format!("Error: Unknown command: {}", err.command()));
                return;
            }
        };
        self.execute_palette_command(command);
    }

    fn execute_palette_command(&mut self, cmd: PaletteCommand) {
        let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
        self.execute_palette_command_with_path(cmd, &path);
    }

    fn execute_palette_command_with_path(&mut self, cmd: PaletteCommand, path: &Path) {
        match cmd {
            PaletteCommand::Save => match self.save_game(path) {
                Ok(()) => {
                    let msg = format!("Save: wrote {}", path.display());
                    self.push_status(LogEntryKind::SaveLoad, msg);
                }
                Err(e) => {
                    self.push_error_status(e);
                }
            },
            PaletteCommand::Load => match self.load_game(path) {
                Ok(()) => {
                    let msg = format!("Load: loaded {}", path.display());
                    self.push_status(LogEntryKind::SaveLoad, msg);
                }
                Err(e) => {
                    self.push_error_status(e);
                }
            },
            PaletteCommand::ClearRally => {
                self.clear_rally_point();
            }
            PaletteCommand::VisualMode => {
                self.cycle_visual_mode_with_path(Some(path));
            }
            PaletteCommand::Dispatch | PaletteCommand::News => {
                self.open_latest_dispatch();
            }
        }
    }

    /// Run the main event loop
    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
        while !self.state.quit {
            self.poll_update_channels();

            terminal.draw(|frame| self.render(frame))?;
            self.state.tick_count = self.state.tick_count.wrapping_add(1);
            self.state.transition.advance();

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }

        Ok(self.restart_requested)
    }

    fn poll_update_channels(&mut self) {
        if let Some(rx) = &self.check_rx {
            if let Ok(result) = rx.try_recv() {
                self.check_rx = None;
                match result {
                    Ok(Some(info)) => {
                        if self.state.auto_update {
                            if let Some(tx) = &self.download_tx {
                                if tx.try_send(info.clone()).is_ok() {
                                    self.state.update_state = UpdateState::Downloading;
                                } else {
                                    self.state.update_state = UpdateState::Available(info);
                                }
                            } else {
                                self.state.update_state = UpdateState::Available(info);
                            }
                        } else {
                            self.state.update_state = UpdateState::Available(info);
                        }
                    }
                    Ok(None) => {
                        self.state.update_state = UpdateState::Idle;
                    }
                    Err(e) => {
                        self.state.update_state = UpdateState::Error(e);
                    }
                }
            }
        }

        if let Some(rx) = &self.download_rx {
            if let Ok(result) = rx.try_recv() {
                self.download_rx = None;
                match result {
                    Ok(version) => {
                        self.state.update_state = UpdateState::Staged { version };
                    }
                    Err(err) => {
                        self.state.update_state = UpdateState::Error(err);
                    }
                }
            }
        }
    }

    /// Render the current state
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Render the active screen
        let game_state = self.engine.as_ref().map(|e| &e.state);
        self.state
            .active
            .render(frame, area, &self.state, game_state);

        // Render overlays
        if self.state.overlay.show_help {
            render_help(frame, area, &self.state.active);
        }

        if self.state.overlay.show_palette {
            render_palette(
                frame,
                area,
                &self.state.overlay.palette_input,
                self.state.visual_mode,
            );
        }

        if self.state.overlay.show_dispatch {
            if let Some(engine) = &self.engine {
                let dispatches = &engine.state.galactic_dispatches;
                if !dispatches.is_empty() {
                    let idx = self
                        .state
                        .overlay
                        .dispatch_history_index
                        .min(dispatches.len().saturating_sub(1));
                    render_dispatch(
                        frame,
                        area,
                        &dispatches[idx],
                        idx,
                        dispatches.len(),
                        self.state.visual_mode,
                    );
                }
            }
        }

        if self.state.overlay.show_battle_reports {
            if let Some(engine) = &self.engine {
                render_battle_reports(
                    frame,
                    area,
                    &engine.state.battle_reports,
                    self.state.overlay.battle_report_index,
                    self.state.overlay.battle_report_inspect,
                    self.state.visual_mode,
                );
            }
        }

        if self.state.overlay.show_settings {
            crate::screens::settings::render_settings(frame, area, &self.state);
        }

        if self.state.overlay.archives.open {
            let archives = &self.state.overlay.archives;
            crate::screens::archives::render_archives(
                frame,
                area,
                &archives.entries,
                archives.cursor,
                archives.confirm_delete,
                archives.show_help,
                archives.error.as_deref(),
            );
        }

        if let Some(confirm) = &self.state.overlay.update_confirm {
            render_update_confirm(frame, area, confirm);
        }

        self.apply_visual_mode_fallback(frame);
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) {
        // Update confirm dialog has highest priority — nothing else should fire underneath it.
        if self.state.overlay.update_confirm.is_some() {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    let kind = self.state.overlay.update_confirm.take();
                    match kind {
                        Some(UpdateConfirmKind::Download(info)) => {
                            if let Some(tx) = &self.download_tx {
                                let _ = tx.try_send(info);
                                self.state.update_state = UpdateState::Downloading;
                            }
                        }
                        Some(UpdateConfirmKind::ApplyAndRestart { .. }) => {
                            self.restart_requested = true;
                            self.state.quit = true;
                        }
                        None => {}
                    }
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.state.overlay.update_confirm = None;
                }
                _ => {}
            }
            return;
        }

        // Campaign Archives overlay — handled before global keys so its own
        // bindings (Esc/?/N/D) take precedence over the menu shortcuts beneath.
        if self.state.overlay.archives.open {
            self.handle_archives_key(key);
            return;
        }

        // Handle overlays first
        if self.state.overlay.show_battle_reports {
            match key.code {
                KeyCode::Esc | KeyCode::Char('B') | KeyCode::Char('b') => {
                    self.state.overlay.show_battle_reports = false;
                    self.state.overlay.battle_report_inspect = false;
                }
                KeyCode::Up | KeyCode::Char('k') if self.state.overlay.battle_report_index > 0 => {
                    self.state.overlay.battle_report_index -= 1;
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if let Some(engine) = &self.engine {
                        let max = engine.state.battle_reports.len().saturating_sub(1);
                        if self.state.overlay.battle_report_index < max {
                            self.state.overlay.battle_report_index += 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    self.state.overlay.battle_report_inspect =
                        !self.state.overlay.battle_report_inspect;
                }
                _ => {}
            }
            return;
        }

        if self.state.overlay.show_dispatch {
            match key.code {
                KeyCode::Esc | KeyCode::Char('N') | KeyCode::Char('n') => {
                    self.state.overlay.show_dispatch = false;
                }
                KeyCode::Left | KeyCode::Char('h')
                    if self.state.overlay.dispatch_history_index > 0 =>
                {
                    self.state.overlay.dispatch_history_index -= 1;
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(engine) = &self.engine {
                        let max = engine.state.galactic_dispatches.len().saturating_sub(1);
                        if self.state.overlay.dispatch_history_index < max {
                            self.state.overlay.dispatch_history_index += 1;
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        if self.state.overlay.show_settings {
            self.handle_settings_key(key);
            return;
        }

        if self.state.overlay.show_help {
            if KeyMap::is_help(key) || KeyMap::is_escape(key) {
                self.state.overlay.show_help = false;
            }
            return;
        }

        if self.state.overlay.show_palette {
            match key.code {
                KeyCode::Esc => {
                    self.state.overlay.show_palette = false;
                    self.state.overlay.palette_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.state.overlay.palette_input.clone();
                    self.state.overlay.show_palette = false;
                    self.state.overlay.palette_input.clear();
                    self.execute_palette_input(&cmd);
                }
                KeyCode::Backspace => {
                    self.state.overlay.palette_input.pop();
                }
                KeyCode::Char(c) => {
                    self.state.overlay.palette_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Global keys
        if KeyMap::is_help(key) {
            self.state.overlay.show_help = true;
            return;
        }

        if KeyMap::is_palette(key) {
            self.state.overlay.show_palette = true;
            return;
        }

        if KeyMap::is_quit(key) {
            self.state.quit = true;
            return;
        }

        if key.code == KeyCode::Char('N') && self.engine.is_some() {
            self.open_latest_dispatch();
            return;
        }
        if matches!(key.code, KeyCode::Char('B') | KeyCode::Char('b')) && self.engine.is_some() {
            self.open_latest_battle_report();
            return;
        }

        if matches!(
            key.code,
            KeyCode::Char('O') | KeyCode::Char('o') | KeyCode::Char('V') | KeyCode::Char('v')
        ) && self.engine.is_some()
        {
            self.state.active = Screen::EmpireOverview;
            self.state.overview.filter_input = false;
            return;
        }

        // 'W' opens Ship Designer from any game screen
        if key.code == KeyCode::Char('W') && self.engine.is_some() {
            self.state.previous_screen = self.state.active;
            self.state.active = Screen::ShipDesigner;
            self.state.ship_designer.reset_to_browse();
            return;
        }

        // Screen-specific handling
        match self.state.active {
            Screen::Menu => self.handle_menu_key(key),
            Screen::Settings => self.handle_settings_key(key),
            Screen::EmpireSelect => self.handle_empire_select_key(key),
            Screen::NewGameSetup => self.handle_new_game_setup_key(key),
            Screen::SectorOverview => self.handle_sector_overview_key(key),
            Screen::SectorMap => self.handle_sector_map_key(key),
            Screen::System => self.handle_system_key(key),
            Screen::Colony => self.handle_colony_key(key),
            Screen::EmpireOverview => self.handle_empire_overview_key(key),
            Screen::Research => self.handle_research_key(key),
            Screen::Diplomacy => self.handle_diplomacy_key(key),
            Screen::ShipDesigner => self.handle_ship_designer_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C')) {
            self.state.menu_cursor = 0;
            self.activate_menu_action(MenuAction::Continue);
        } else if KeyMap::is_new_game(key) {
            self.state.menu_cursor = 1;
            self.activate_menu_action(MenuAction::NewGame);
        } else if matches!(key.code, KeyCode::Char('j') | KeyCode::Down) {
            self.state.menu_cursor = (self.state.menu_cursor + 1) % menu_action_count();
        } else if matches!(key.code, KeyCode::Char('k') | KeyCode::Up) {
            self.state.menu_cursor = (self.state.menu_cursor
                + menu_action_count().saturating_sub(1))
                % menu_action_count();
        } else if matches!(key.code, KeyCode::Enter) {
            self.activate_menu_action(MenuAction::from_cursor(self.state.menu_cursor));
        } else if matches!(
            key.code,
            KeyCode::Tab | KeyCode::Char('v') | KeyCode::Char('V')
        ) {
            self.cycle_visual_mode();
        } else if matches!(
            key.code,
            KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Char('o') | KeyCode::Char('O')
        ) {
            self.state.menu_cursor = 3;
            self.activate_menu_action(MenuAction::Options);
        } else if KeyMap::is_load_game(key) {
            self.state.menu_cursor = 2;
            self.activate_menu_action(MenuAction::LoadGame);
        } else if matches!(key.code, KeyCode::Char('u') | KeyCode::Char('U')) {
            // Open update confirm dialog
            let confirm = match &self.state.update_state {
                UpdateState::Available(info) => Some(UpdateConfirmKind::Download(info.clone())),
                UpdateState::Staged { version } => Some(UpdateConfirmKind::ApplyAndRestart {
                    version: version.clone(),
                }),
                _ => None,
            };
            if let Some(kind) = confirm {
                self.state.overlay.update_confirm = Some(kind);
            }
        } else if KeyMap::is_escape(key) {
            self.state.menu_cursor = 4;
            self.activate_menu_action(MenuAction::Quit);
        }
    }

    fn handle_archives_key(&mut self, key: KeyEvent) {
        // Delete confirmation captures all input until resolved.
        if self.state.overlay.archives.confirm_delete {
            match key.code {
                KeyCode::Enter => {
                    self.delete_selected_archive();
                    self.state.overlay.archives.confirm_delete = false;
                }
                KeyCode::Esc => {
                    self.state.overlay.archives.confirm_delete = false;
                }
                _ => {}
            }
            return;
        }

        // Inline help captures the next key to dismiss it.
        if self.state.overlay.archives.show_help {
            self.state.overlay.archives.show_help = false;
            return;
        }

        let count = self.state.overlay.archives.entries.len();
        match key.code {
            KeyCode::Esc => {
                self.state.overlay.archives.open = false;
            }
            KeyCode::Char('?') => {
                self.state.overlay.archives.show_help = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.state.overlay.archives.open = false;
                self.state.active = Screen::NewGameSetup;
            }
            KeyCode::Down | KeyCode::Char('j') if count > 0 => {
                self.state.overlay.archives.cursor =
                    (self.state.overlay.archives.cursor + 1) % count;
            }
            KeyCode::Up | KeyCode::Char('k') if count > 0 => {
                self.state.overlay.archives.cursor =
                    (self.state.overlay.archives.cursor + count - 1) % count;
            }
            KeyCode::Enter if count > 0 => {
                self.load_selected_archive();
            }
            KeyCode::Char('d') | KeyCode::Char('D') if count > 0 => {
                self.state.overlay.archives.confirm_delete = true;
            }
            _ => {}
        }
    }

    fn activate_menu_action(&mut self, action: MenuAction) {
        match action {
            MenuAction::Continue => {
                if self.state.can_continue {
                    self.continue_latest_campaign();
                }
            }
            MenuAction::NewGame => {
                self.state.active = Screen::NewGameSetup;
            }
            MenuAction::LoadGame => {
                self.open_archives();
            }
            MenuAction::Options => {
                self.state.overlay.show_settings = true;
                self.state.settings_cursor = 0;
            }
            MenuAction::Quit => {
                self.state.quit = true;
            }
        }
    }

    fn handle_settings_key(&mut self, key: KeyEvent) {
        let count = crate::screens::settings::settings_cursor_count();
        match key.code {
            KeyCode::Esc => {
                let _ = self.persist_config();
                // Close overlay; if somehow on old Settings screen navigate back to Menu
                self.state.overlay.show_settings = false;
                if self.state.active == Screen::Settings {
                    self.state.active = Screen::Menu;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.settings_cursor = (self.state.settings_cursor + 1) % count;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.settings_cursor = (self.state.settings_cursor + count - 1) % count;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.cycle_settings_item();
            }
            _ => {}
        }
    }

    fn cycle_settings_item(&mut self) {
        match self.state.settings_cursor {
            0 => self.cycle_visual_mode(),
            1 => {
                self.state.update_channel = self.state.update_channel.next();
            }
            2 => {
                self.state.auto_update = !self.state.auto_update;
            }
            _ => {}
        }
    }

    fn handle_empire_select_key(&mut self, key: KeyEvent) {
        let all_defs = game_core::all_empire_definitions();
        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::Menu;
            }
            KeyCode::Enter => {
                self.state.active = Screen::NewGameSetup;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.new_game_setup.empire_cursor = (self.state.new_game_setup.empire_cursor
                    + 1)
                .min(all_defs.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up if self.state.new_game_setup.empire_cursor > 0 => {
                self.state.new_game_setup.empire_cursor -= 1;
            }
            _ => {}
        }
    }

    /// Handle keyboard input on the New Game Setup screen.
    fn handle_new_game_setup_key(&mut self, key: KeyEvent) {
        use crate::screens::new_game_setup::FIELD_SEED;
        const NUM_FIELDS: usize = 4;

        // Seed editing mode intercepts most keys.
        if self.state.new_game_setup.seed_editing {
            match key.code {
                KeyCode::Esc => {
                    // Cancel editing — restore the value that was active when editing began.
                    self.state.new_game_setup.seed_str =
                        self.state.new_game_setup.seed_pre_edit.clone();
                    self.state.new_game_setup.seed_editing = false;
                }
                KeyCode::Enter => {
                    // Confirm edit.
                    self.state.new_game_setup.seed_editing = false;
                    // Ensure seed string is valid (non-empty).
                    if self.state.new_game_setup.seed_str.is_empty() {
                        self.state.new_game_setup.seed_str = "0".to_string();
                    }
                }
                KeyCode::Backspace => {
                    self.state.new_game_setup.seed_str.pop();
                }
                // Limit to 18 digits to stay within u64::MAX (20 digits).
                KeyCode::Char(c)
                    if c.is_ascii_digit() && self.state.new_game_setup.seed_str.len() < 18 =>
                {
                    self.state.new_game_setup.seed_str.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::Menu;
            }
            KeyCode::Enter => {
                let cursor = self.state.new_game_setup.cursor;
                if cursor == FIELD_SEED {
                    // Enter edit mode for seed field — snapshot the current value so Esc can restore it.
                    self.state.new_game_setup.seed_pre_edit =
                        self.state.new_game_setup.seed_str.clone();
                    self.state.new_game_setup.seed_editing = true;
                } else {
                    // Start the game from the setup screen.
                    self.start_game_from_setup();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.new_game_setup.cursor =
                    (self.state.new_game_setup.cursor + 1) % NUM_FIELDS;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.new_game_setup.cursor =
                    (self.state.new_game_setup.cursor + NUM_FIELDS - 1) % NUM_FIELDS;
            }
            // Cycle left/decrease
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('-') => {
                self.setup_cycle_field(false);
            }
            // Cycle right/increase
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Char('+') => {
                self.setup_cycle_field(true);
            }
            KeyCode::Char('S') | KeyCode::Char('s') => {
                // Start the game from the setup screen (shortcut).
                self.start_game_from_setup();
            }
            _ => {}
        }
    }

    /// Cycle the currently selected setup field forward (true) or backward (false).
    fn setup_cycle_field(&mut self, forward: bool) {
        use crate::screens::new_game_setup::{FIELD_AI_COUNT, FIELD_EMPIRE, FIELD_GALAXY_SIZE};
        use game_core::GalaxySize;
        let all_sizes = GalaxySize::all();
        match self.state.new_game_setup.cursor {
            FIELD_EMPIRE => {
                let all_defs = game_core::all_empire_definitions();
                if forward {
                    self.state.new_game_setup.empire_cursor =
                        (self.state.new_game_setup.empire_cursor + 1)
                            .min(all_defs.len().saturating_sub(1));
                } else {
                    self.state.new_game_setup.empire_cursor =
                        self.state.new_game_setup.empire_cursor.saturating_sub(1);
                }
            }
            FIELD_GALAXY_SIZE => {
                let idx = all_sizes
                    .iter()
                    .position(|s| *s == self.state.new_game_setup.galaxy_size)
                    .unwrap_or(0);
                let new_idx = if forward {
                    (idx + 1).min(all_sizes.len() - 1)
                } else {
                    idx.saturating_sub(1)
                };
                self.state.new_game_setup.galaxy_size = all_sizes[new_idx];
            }
            FIELD_AI_COUNT => {
                if forward {
                    if self.state.new_game_setup.ai_count < 4 {
                        self.state.new_game_setup.ai_count += 1;
                    }
                } else if self.state.new_game_setup.ai_count > 1 {
                    self.state.new_game_setup.ai_count -= 1;
                }
            }
            _ => {}
        }
    }

    /// Build a `ScenarioSetup` from the current setup screen state and start the game.
    fn start_game_from_setup(&mut self) {
        let seed: u64 = match self.state.new_game_setup.seed_str.parse() {
            Ok(v) => v,
            Err(_) => {
                self.state.log.push(format!(
                    "Invalid seed '{}' — using 0",
                    self.state.new_game_setup.seed_str
                ));
                self.state.new_game_setup.seed_str = "0".to_string();
                0
            }
        };
        let all_defs = game_core::all_empire_definitions();
        let player_empire_def = all_defs
            .get(self.state.new_game_setup.empire_cursor)
            .map(|d| d.id);
        let setup = ScenarioSetup {
            seed,
            galaxy_size: self.state.new_game_setup.galaxy_size,
            ai_empire_count: self.state.new_game_setup.ai_count,
            sector_count_override: None,
            difficulty: game_core::DifficultyLevel::Standard,
            player_empire_def,
            victory_settings: game_core::VictorySettings::default_v1(),
        };
        self.new_game_from_setup(setup);
    }

    fn handle_sector_overview_key(&mut self, key: KeyEvent) {
        if let Some((dx, dy)) = KeyMap::movement(key) {
            self.move_sector_selection(dx, dy);
            return;
        }

        if key.code == KeyCode::Enter {
            self.state.active = Screen::SectorMap;
            return;
        }

        if key.code == KeyCode::Char('r') {
            self.state.active = Screen::Research;
            self.state.research.cursor = 0;
            return;
        }

        if key.code == KeyCode::Char('D') {
            self.state.active = Screen::Diplomacy;
            return;
        }

        if key.code == KeyCode::Char('L') {
            self.state.sector_overview.show_inter_sector_lanes =
                !self.state.sector_overview.show_inter_sector_lanes;
            return;
        }

        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_sector_map_key(&mut self, key: KeyEvent) {
        // Rally-point picking mode: 'R' or Enter confirms, Esc cancels
        if self.state.colony.pending_rally_colony.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.state.colony.pending_rally_colony = None;
                    self.state
                        .log
                        .push("Rally point selection cancelled.".to_string());
                    return;
                }
                KeyCode::Char('R') | KeyCode::Enter => {
                    self.confirm_rally_point();
                    return;
                }
                _ => {}
            }
            // Allow normal navigation in pick mode
            if let Some((dx, dy)) = KeyMap::movement(key) {
                self.move_star_selection_in_sector(dx, dy);
            }
            return;
        }

        if key.code == KeyCode::Esc {
            self.state.active = Screen::SectorOverview;
            return;
        }

        if let Some((dx, dy)) = KeyMap::movement(key) {
            self.move_star_selection_in_sector(dx, dy);
            return;
        }

        if key.code == KeyCode::Enter {
            self.state.active = Screen::System;
            self.state.navigation.selected_planet_index = 0;
            return;
        }

        if key.code == KeyCode::Char('c') {
            if !self.try_enter_colony() {
                let msg = "Unavailable: open colony — no player colony in selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
            }
            return;
        }

        if key.code == KeyCode::Char('S') {
            self.dispatch_scout();
            return;
        }

        if key.code == KeyCode::Char('M') {
            self.move_fleet();
            return;
        }

        if key.code == KeyCode::Char('r') {
            self.state.active = Screen::Research;
            self.state.research.cursor = 0;
            return;
        }

        if key.code == KeyCode::Char('D') {
            self.state.active = Screen::Diplomacy;
            return;
        }

        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_system_key(&mut self, key: KeyEvent) {
        let planet_count = self
            .engine
            .as_ref()
            .and_then(|engine| {
                self.state
                    .navigation
                    .selected_star
                    .and_then(|star_id| engine.state.stars.get(&star_id))
            })
            .map(|star| star.planets.len())
            .unwrap_or(0);

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            KeyCode::Enter => {
                if !self.try_enter_colony() {
                    let msg = "Unavailable: open colony — no player colony in selected system.";
                    self.state.log.push(msg.to_string());
                    self.state.status_message = Some(msg.to_string());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if planet_count > 0 {
                    self.state.navigation.selected_planet_index =
                        (self.state.navigation.selected_planet_index + 1) % planet_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if planet_count > 0 {
                    self.state.navigation.selected_planet_index =
                        (self.state.navigation.selected_planet_index + planet_count - 1)
                            % planet_count;
                }
            }
            KeyCode::Char('C') => {
                self.colonize_selected_planet();
            }
            KeyCode::Char('S') => {
                self.survey_selected_planet();
            }
            KeyCode::Char('I') => {
                self.invade_selected_planet();
            }
            KeyCode::Char('f') => {
                self.cycle_system_fleet_focus();
            }
            KeyCode::Char('R') => {
                self.cycle_selected_fleet_role();
            }
            KeyCode::Char('F') => {
                self.cycle_selected_fleet_formation();
            }
            KeyCode::Char('c') => {
                if !self.try_enter_colony() {
                    let msg = "Unavailable: open colony — no player colony in selected system.";
                    self.state.log.push(msg.to_string());
                    self.state.status_message = Some(msg.to_string());
                }
            }
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_colony_key(&mut self, key: KeyEvent) {
        match key.code {
            // Return to sector map
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
                self.state.colony.selected_colony = None;
                self.state.colony.role_panel_active = false;
                self.state.colony.role_cursor = 0;
            }
            // Tab: switch active panel (build picker ↔ role selector)
            KeyCode::Tab => {
                self.state.colony.role_panel_active = !self.state.colony.role_panel_active;
            }
            // Navigate the active panel
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.colony.role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony.role_cursor = (self.state.colony.role_cursor + 1) % count;
                } else {
                    let count = self.visible_build_count();
                    self.state.colony.build_cursor = (self.state.colony.build_cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.colony.role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony.role_cursor =
                        (self.state.colony.role_cursor + count.saturating_sub(1)) % count;
                } else {
                    let count = self.visible_build_count();
                    self.state.colony.build_cursor =
                        (self.state.colony.build_cursor + count.saturating_sub(1)) % count;
                }
            }
            // Enter: confirm selection in the active panel
            KeyCode::Enter => {
                if self.state.colony.role_panel_active {
                    self.set_colony_role();
                } else {
                    self.queue_building();
                }
            }
            // R: start rally-point picking — return to Sector Map in pick mode
            KeyCode::Char('R') => {
                if let Some(colony_id) = self.state.colony.selected_colony {
                    self.state.colony.pending_rally_colony = Some(colony_id);
                    self.state.active = Screen::SectorMap;
                    self.state.colony.selected_colony = None;
                    self.state.colony.role_panel_active = false;
                    self.state.log.push(
                        "Select a star and press R to set rally point. Esc to cancel.".to_string(),
                    );
                }
            }
            // X: clear rally point for the active colony
            KeyCode::Char('X') => {
                self.clear_rally_point();
            }
            // End turn from colony screen
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_empire_overview_key(&mut self, key: KeyEvent) {
        if self.state.overview.filter_input {
            match key.code {
                KeyCode::Esc => {
                    self.state.overview.filter_input = false;
                }
                KeyCode::Enter => {
                    self.state.overview.filter_input = false;
                }
                KeyCode::Backspace => {
                    self.state.overview.filter.pop();
                    self.state.overview.cursor = 0;
                }
                KeyCode::Char(c) => {
                    self.state.overview.filter.push(c);
                    self.state.overview.cursor = 0;
                }
                _ => {}
            }
            return;
        }

        let overview_data = self.engine.as_ref().map(|engine| {
            derive_empire_overview(
                &engine.state,
                engine.state.player_empire,
                self.state.overview.sort,
                &self.state.overview.filter,
            )
        });
        let visible_count = overview_data.as_ref().map(|d| d.rows.len()).unwrap_or(0);

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
                self.state.overview.filter_input = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if visible_count > 0 {
                    self.state.overview.cursor = (self.state.overview.cursor + 1) % visible_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if visible_count > 0 {
                    self.state.overview.cursor =
                        (self.state.overview.cursor + visible_count - 1) % visible_count;
                }
            }
            KeyCode::Enter => {
                self.jump_overview_to_colony(overview_data.as_ref());
            }
            KeyCode::Char('S') => {
                self.jump_overview_to_system(overview_data.as_ref());
            }
            KeyCode::Char('s') => {
                self.state.overview.sort = self.state.overview.sort.next();
                self.state.overview.cursor = 0;
            }
            _ if KeyMap::is_search(key) => {
                self.state.overview.filter_input = true;
            }
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn jump_overview_to_colony(&mut self, data: Option<&EmpireOverviewData>) {
        let data = match data {
            Some(d) if !d.rows.is_empty() => d,
            _ => {
                let msg = "Unavailable: open colony — no colonies match current overview filter.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };
        let selected = self
            .state
            .overview
            .cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index, colony_id) = (row.star_id, row.planet_index, row.colony_id);

        self.state.navigation.selected_star = Some(star_id);
        self.state.navigation.selected_planet_index = planet_index;
        self.state.colony.selected_colony = Some(colony_id);
        self.state.colony.build_cursor = 0;
        self.state.active = Screen::Colony;
    }

    fn jump_overview_to_system(&mut self, data: Option<&EmpireOverviewData>) {
        let data = match data {
            Some(d) if !d.rows.is_empty() => d,
            _ => {
                let msg = "Unavailable: open system — no colonies match current overview filter.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };
        let selected = self
            .state
            .overview
            .cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index) = (row.star_id, row.planet_index);

        self.state.navigation.selected_star = Some(star_id);
        self.state.navigation.selected_planet_index = planet_index;
        self.state.active = Screen::System;
    }

    /// Number of build picker items visible to the player (excludes tech-locked items).
    fn visible_build_count(&self) -> usize {
        let completed = self
            .engine
            .as_ref()
            .and_then(|e| e.state.empires.get(&e.state.player_empire))
            .map(|emp| emp.research.completed.as_slice())
            .unwrap_or(&[]);
        let orbital_count = OrbitalStructureType::all()
            .iter()
            .filter(|ot| {
                ot.required_tech()
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .count();
        let ship_count = game_core::all_ship_designs()
            .iter()
            .filter(|d| {
                d.required_tech
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .count();
        BuildingType::all().len() + orbital_count + ship_count
    }

    /// Try to enter the colony screen for the selected star.
    /// Returns true if a player colony was found and the screen transitioned.
    fn try_enter_colony(&mut self) -> bool {
        let engine = match &self.engine {
            Some(e) => e,
            None => return false,
        };

        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => return false,
        };

        let star = match engine.state.stars.get(&star_id) {
            Some(s) => s,
            None => return false,
        };

        if !star.planets.is_empty() {
            let selected_planet = self
                .state
                .navigation
                .selected_planet_index
                .min(star.planets.len().saturating_sub(1));
            if let Some(colony_id) = star.planets[selected_planet].colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.colony.selected_colony = Some(colony_id);
                        self.state.colony.build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        // Fallback: find the first planet at this star that has a player-owned colony.
        for planet in &star.planets {
            if let Some(colony_id) = planet.colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.colony.selected_colony = Some(colony_id);
                        self.state.colony.build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_research_key(&mut self, key: KeyEvent) {
        if self.state.research.query_input {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.research.query_input = false;
                }
                KeyCode::Backspace => {
                    self.state.research.query.pop();
                }
                KeyCode::Char(c) => {
                    self.state.research.query.push(c);
                }
                _ => {}
            }
            self.state.research.cursor = 0;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            KeyCode::Char('/') => {
                self.state.research.query_input = true;
            }
            KeyCode::Tab => {
                self.state.research.domain_filter =
                    (self.state.research.domain_filter + 1) % RESEARCH_DOMAIN_FILTER_COUNT;
                self.state.research.cursor = 0;
            }
            KeyCode::Char('[') => {
                self.state.research.era_filter =
                    (self.state.research.era_filter + 1) % RESEARCH_ERA_FILTER_COUNT;
                self.state.research.cursor = 0;
            }
            KeyCode::Char(']') => {
                self.state.research.status_filter =
                    (self.state.research.status_filter + 1) % RESEARCH_STATUS_FILTER_COUNT;
                self.state.research.cursor = 0;
            }
            // Navigate tech list
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.tech_tree_count();
                if count > 0 {
                    self.state.research.cursor = (self.state.research.cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.tech_tree_count();
                if count > 0 {
                    self.state.research.cursor =
                        (self.state.research.cursor + count.saturating_sub(1)) % count;
                }
            }
            // Select the highlighted tech for research
            KeyCode::Enter => {
                self.select_research_tech();
            }
            // Queue highlighted tech for automatic follow-up
            KeyCode::Char('a') => {
                self.queue_research_tech();
            }
            // Remove highlighted tech from research queue
            KeyCode::Char('x') => {
                self.remove_queued_research_tech();
            }
            // Move highlighted queued tech one slot earlier
            KeyCode::Char('u') => {
                self.move_queued_research_up();
            }
            // Move highlighted queued tech one slot later
            KeyCode::Char('d') => {
                self.move_queued_research_down();
            }
            // Clear entire research queue
            KeyCode::Char('c') => {
                self.dispatch_command(Command::ClearResearchQueue);
            }
            // End turn from research screen (excluding Enter, which selects tech)
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_diplomacy_key(&mut self, key: KeyEvent) {
        let player_targeted_messages = self
            .engine
            .as_ref()
            .map(|engine| {
                engine
                    .state
                    .diplomacy_pending_communications
                    .iter()
                    .filter(|msg| msg.receiving_empire == engine.state.player_empire)
                    .count()
            })
            .unwrap_or(0);

        if self.state.diplomacy.show_communication_modal {
            if player_targeted_messages == 0 {
                self.state.diplomacy.show_communication_modal = false;
                self.state.diplomacy.selected_response_index = 0;
                return;
            }
            match key.code {
                KeyCode::Esc => {
                    self.state.diplomacy.show_communication_modal = false;
                    self.state.diplomacy.selected_response_index = 0;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.state.diplomacy.selected_response_index = self
                        .state
                        .diplomacy
                        .selected_response_index
                        .saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.state.diplomacy.selected_response_index = self
                        .state
                        .diplomacy
                        .selected_response_index
                        .saturating_sub(1);
                }
                KeyCode::Tab => {
                    self.state.diplomacy.selected_communication_index =
                        (self.state.diplomacy.selected_communication_index + 1)
                            % player_targeted_messages;
                    self.state.diplomacy.selected_response_index = 0;
                }
                KeyCode::Enter => {
                    self.respond_to_selected_diplomatic_message();
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            KeyCode::Tab => {
                self.cycle_diplomacy_empire();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.cycle_diplomacy_empire();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cycle_diplomacy_empire_reverse();
            }
            KeyCode::Char('c') => {
                if player_targeted_messages == 0 {
                    self.push_error_status("No pending diplomatic communications.");
                } else {
                    self.state.diplomacy.show_communication_modal = true;
                    self.state.diplomacy.selected_response_index = 0;
                    self.state.diplomacy.selected_communication_index = self
                        .state
                        .diplomacy
                        .selected_communication_index
                        .min(player_targeted_messages.saturating_sub(1));
                }
            }
            KeyCode::Char('w') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::DeclareWar { target });
                }
            }
            KeyCode::Char('p') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::OfferPeace { target });
                }
            }
            KeyCode::Char('n') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::ProposeNonAggressionPact { target });
                }
            }
            KeyCode::Char('x') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::CancelTreaty {
                        target,
                        treaty_type: TreatyType::NonAggressionPact,
                    });
                }
            }
            KeyCode::Char('g') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::SendGreeting { target });
                }
            }
            KeyCode::Char('u') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::IssueWarning { target });
                }
            }
            KeyCode::Char('m') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::DemandTribute { target });
                }
            }
            KeyCode::Char('i') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::GatherIntelligence { target });
                }
            }
            KeyCode::Char('z') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::SabotageProduction { target });
                }
            }
            KeyCode::Char('y') => {
                if let Some(target) = self.selected_diplomacy_target() {
                    self.dispatch_command(Command::StealResearch { target });
                }
            }
            // End turn from diplomacy screen
            _ => {
                if KeyMap::is_end_turn(key) {
                    self.end_turn();
                }
            }
        }
    }

    fn diplomacy_target_list(&self) -> Vec<game_core::EmpireId> {
        let Some(engine) = &self.engine else {
            return Vec::new();
        };
        engine
            .state
            .empires
            .keys()
            .copied()
            .filter(|empire_id| *empire_id != engine.state.player_empire)
            .collect()
    }

    fn selected_diplomacy_target(&self) -> Option<game_core::EmpireId> {
        let targets = self.diplomacy_target_list();
        if targets.is_empty() {
            return None;
        }
        let idx = self.state.diplomacy.selected_empire_index % targets.len();
        targets.get(idx).copied()
    }

    fn cycle_diplomacy_empire(&mut self) {
        let targets = self.diplomacy_target_list();
        if !targets.is_empty() {
            self.state.diplomacy.selected_empire_index =
                (self.state.diplomacy.selected_empire_index + 1) % targets.len();
        }
    }

    fn cycle_diplomacy_empire_reverse(&mut self) {
        let targets = self.diplomacy_target_list();
        if !targets.is_empty() {
            self.state.diplomacy.selected_empire_index =
                (self.state.diplomacy.selected_empire_index + targets.len() - 1) % targets.len();
        }
    }

    fn respond_to_selected_diplomatic_message(&mut self) {
        let Some(engine) = &self.engine else {
            return;
        };
        let player = engine.state.player_empire;
        let mut messages: Vec<_> = engine
            .state
            .diplomacy_pending_communications
            .iter()
            .filter(|msg| msg.receiving_empire == player)
            .cloned()
            .collect();
        messages.sort_by_key(|msg| msg.communication_id);
        let Some(message) = messages.get(
            self.state
                .diplomacy
                .selected_communication_index
                .min(messages.len().saturating_sub(1)),
        ) else {
            self.push_error_status("No pending diplomatic communication selected.");
            return;
        };
        if message.available_responses.is_empty() {
            self.push_error_status("Selected communication has no valid responses.");
            return;
        }
        let response = message.available_responses[self
            .state
            .diplomacy
            .selected_response_index
            .min(message.available_responses.len().saturating_sub(1))];
        self.dispatch_command(Command::RespondToCommunication {
            communication_id: message.communication_id,
            response,
        });
        self.state.diplomacy.selected_response_index = 0;
        self.state.diplomacy.show_communication_modal = false;
    }

    fn handle_ship_designer_key(&mut self, key: KeyEvent) {
        use DesignerMode::*;
        use DesignerPanel::*;
        match key.code {
            KeyCode::Esc => match self.state.ship_designer.mode {
                Browse => {
                    self.state.active = self.state.previous_screen;
                }
                _ => {
                    self.state.ship_designer.reset_to_browse();
                }
            },
            KeyCode::Char('n') => {
                self.state.ship_designer.begin_new_design();
            }
            KeyCode::Tab => {
                self.state.ship_designer.panel = match self.state.ship_designer.panel {
                    DesignList => SlotConfig,
                    SlotConfig => Stats,
                    Stats => DesignList,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.ship_designer_nav(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ship_designer_nav(-1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.ship_designer_cycle_component(-1);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.ship_designer_cycle_component(1);
            }
            KeyCode::Enter => {
                self.ship_designer_confirm();
            }
            KeyCode::Char('s') => {
                self.save_ship_design();
            }
            KeyCode::Char('d') => {
                self.delete_ship_design();
            }
            _ => {}
        }
    }

    fn ship_designer_nav(&mut self, delta: i32) {
        use DesignerMode::*;
        let mode = self.state.ship_designer.mode;
        let panel = self.state.ship_designer.panel;
        match (mode, panel) {
            (Browse, DesignerPanel::DesignList) => {
                let count = self.ship_designer_design_count() + 1;
                if count > 0 {
                    let cur = self.state.ship_designer.selected_design_idx;
                    self.state.ship_designer.selected_design_idx =
                        ((cur as i32 + delta).rem_euclid(count as i32)) as usize;
                }
            }
            (NewDesign, _) => {
                let hull_count = game_core::all_hull_templates().len();
                if hull_count > 0 {
                    let cur = self.state.ship_designer.selected_hull_idx;
                    self.state.ship_designer.selected_hull_idx =
                        ((cur as i32 + delta).rem_euclid(hull_count as i32)) as usize;
                }
            }
            (EditSlots, DesignerPanel::SlotConfig) | (ConfirmSave, DesignerPanel::SlotConfig) => {
                let slot_count = self.ship_designer_slot_count();
                if slot_count > 0 {
                    let cur = self.state.ship_designer.active_slot_idx;
                    self.state.ship_designer.active_slot_idx =
                        ((cur as i32 + delta).rem_euclid(slot_count as i32)) as usize;
                    self.state.ship_designer.component_cursor = 0;
                }
            }
            _ => {}
        }
    }

    fn ship_designer_cycle_component(&mut self, delta: i32) {
        if !matches!(self.state.ship_designer.mode, DesignerMode::EditSlots) {
            return;
        }
        let hull_idx = self.state.ship_designer.selected_hull_idx;
        let slot_idx = self.state.ship_designer.active_slot_idx;
        let category = game_core::all_hull_templates()
            .get(hull_idx)
            .and_then(|h| h.slots.get(slot_idx))
            .copied();
        if let Some(cat) = category {
            // Only show components that are unlocked
            let completed: Vec<_> = self
                .engine
                .as_ref()
                .map(|e| {
                    e.state
                        .empires
                        .get(&e.state.player_empire)
                        .map(|emp| emp.research.completed.to_vec())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
            let comps: Vec<_> = game_core::components_for_slot(cat)
                .into_iter()
                .filter(|c| game_core::is_component_unlocked(c.component_id, &completed))
                .collect();
            let count = comps.len();
            if count == 0 {
                return;
            }
            let cur = self.state.ship_designer.component_cursor;
            let new_cursor = ((cur as i32 + delta).rem_euclid(count as i32)) as usize;
            self.state.ship_designer.component_cursor = new_cursor;
            if let Some(comp) = comps.get(new_cursor) {
                if let Some(slot) = self
                    .state
                    .ship_designer
                    .current_components
                    .get_mut(slot_idx)
                {
                    *slot = comp.component_id;
                }
            }
        }
    }

    fn ship_designer_confirm(&mut self) {
        use DesignerMode::*;
        match self.state.ship_designer.mode {
            Browse => {
                if self.state.ship_designer.selected_design_idx == 0 {
                    self.state.ship_designer.begin_new_design();
                }
            }
            NewDesign => {
                if let Some(h) =
                    game_core::all_hull_templates().get(self.state.ship_designer.selected_hull_idx)
                {
                    // Block entry into slot editing if hull tech is not yet unlocked
                    let completed: Vec<_> = self
                        .engine
                        .as_ref()
                        .map(|e| {
                            e.state
                                .empires
                                .get(&e.state.player_empire)
                                .map(|emp| emp.research.completed.to_vec())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default();
                    if let Some(tech) = h.required_tech {
                        if !completed.contains(&tech) {
                            self.push_error_status(
                                "Hull not yet unlocked — research required tech first.".to_string(),
                            );
                            return;
                        }
                    }
                    self.state.ship_designer.begin_edit_slots(h);
                }
            }
            EditSlots => {
                self.ship_designer_cycle_component(1);
            }
            ConfirmSave => {
                self.save_ship_design();
            }
        }
    }

    fn ship_designer_design_count(&self) -> usize {
        self.engine
            .as_ref()
            .map(|e| {
                let player = e.state.player_empire;
                e.state
                    .custom_designs
                    .values()
                    .filter(|d| d.owner == player && !d.obsolete)
                    .count()
            })
            .unwrap_or(0)
    }

    fn ship_designer_slot_count(&self) -> usize {
        game_core::all_hull_templates()
            .get(self.state.ship_designer.selected_hull_idx)
            .map(|h| h.slots.len())
            .unwrap_or(0)
    }

    fn save_ship_design(&mut self) {
        use DesignerMode::*;
        if !matches!(self.state.ship_designer.mode, EditSlots | ConfirmSave) {
            return;
        }
        let hull = game_core::all_hull_templates()
            .get(self.state.ship_designer.selected_hull_idx)
            .copied();
        let hull = match hull {
            Some(h) => h,
            None => {
                self.push_error_status("Error: No hull selected for design.".to_string());
                return;
            }
        };
        let components: Vec<ComponentId> = self.state.ship_designer.current_components.clone();
        let name = self
            .state
            .ship_designer
            .name_input
            .clone()
            .unwrap_or_else(|| format!("{} Design", hull.name));
        let hull_id = hull.hull_id;
        self.state.ship_designer.reset_to_browse();
        // Apply the command immediately and check whether a design was actually created
        let is_end_turn = false;
        let (events, _end_turn_report) = {
            let engine = match &mut self.engine {
                Some(engine) => engine,
                None => return,
            };
            let evs = engine.apply_turn(vec![Command::CreateShipDesign {
                hull_id,
                components,
                name: Some(name),
            }]);
            (evs, is_end_turn.then_some(()))
        };
        for event in &events {
            self.push_core_event_to_log(event);
        }
        // Only show success if a design was actually created
        if events
            .iter()
            .any(|e| matches!(e, CoreEvent::ShipDesignCreated { .. }))
        {
            self.state.status_message = Some("Design saved.".to_string());
        } else if let Some(CoreEvent::Error { message }) = events.iter().find(|e| e.is_error()) {
            self.state.status_message = Some(format!("Error: {}", message));
        } else if events
            .iter()
            .any(|e| matches!(e, CoreEvent::ShipDesignInvalid { .. }))
        {
            self.state.status_message =
                Some("Design invalid — check hull/component requirements.".to_string());
        }
    }

    fn delete_ship_design(&mut self) {
        // Delete only makes sense in Browse mode
        if !matches!(self.state.ship_designer.mode, DesignerMode::Browse) {
            return;
        }
        if self.state.ship_designer.selected_design_idx == 0 {
            return;
        }
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };
        let player = engine.state.player_empire;
        let designs: Vec<_> = engine
            .state
            .custom_designs
            .values()
            .filter(|d| d.owner == player && !d.obsolete)
            .collect();
        let design_idx =
            (self.state.ship_designer.selected_design_idx - 1).min(designs.len().saturating_sub(1));
        if let Some(design) = designs.get(design_idx) {
            let design_id = design.design_id;
            self.state.ship_designer.selected_design_idx = 0;
            self.dispatch_command(Command::DeleteShipDesign { design_id });
        }
    }

    /// Returns the number of technologies in the tree.
    fn tech_tree_count(&self) -> usize {
        let Some(engine) = &self.engine else {
            return 0;
        };
        filtered_research_techs(&self.state, &engine.state).len()
    }

    /// Dispatch one game-core command and centralize event logging/status updates.
    fn dispatch_command(&mut self, command: Command) {
        let is_end_turn = matches!(command, Command::EndTurn);
        let (events, end_turn_report) = {
            let engine = match &mut self.engine {
                Some(engine) => engine,
                None => return,
            };
            let events = engine.apply_turn(vec![command]);
            let report = is_end_turn.then(|| {
                Self::build_end_turn_report_with_state(
                    engine.state.turn,
                    &events,
                    Some(&engine.state),
                )
            });
            (events, report)
        };

        for event in &events {
            self.push_core_event_to_log(event);
        }

        self.recompute_advisor(&events);

        if let Some(report) = end_turn_report {
            self.push_status(LogEntryKind::TurnReport, report);
            // Auto-show dispatch if a new one was generated this turn
            if let Some(engine) = &self.engine {
                if !engine.state.galactic_dispatches.is_empty() {
                    let last_idx = engine.state.galactic_dispatches.len().saturating_sub(1);
                    let last_dispatch = &engine.state.galactic_dispatches[last_idx];
                    // last_dispatch.turn is the completed_turn; state.turn has been incremented
                    if last_dispatch.turn + 1 == engine.state.turn {
                        self.state.overlay.dispatch_history_index = last_idx;
                        self.state.overlay.show_dispatch = true;
                    }
                }
            }
            return;
        }

        if let Some(CoreEvent::Error { message }) = events
            .iter()
            .rev()
            .find(|event| matches!(event, CoreEvent::Error { .. }))
        {
            self.state.status_message = Some(format!("Error: {}", message));
        }
    }

    /// Select the highlighted technology for research
    fn select_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("select research") else {
            return;
        };

        self.dispatch_command(Command::SelectResearch { tech: tech_id });
    }

    fn queue_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("queue research") else {
            return;
        };

        self.dispatch_command(Command::QueueResearch { tech: tech_id });
    }

    fn remove_queued_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("remove queued research") else {
            return;
        };

        self.dispatch_command(Command::RemoveQueuedResearch { tech: tech_id });
    }

    fn move_queued_research_up(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("reorder queued research") else {
            return;
        };

        self.dispatch_command(Command::MoveQueuedResearchUp { tech: tech_id });
    }

    fn move_queued_research_down(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("reorder queued research") else {
            return;
        };

        self.dispatch_command(Command::MoveQueuedResearchDown { tech: tech_id });
    }

    fn highlighted_research_tech(&mut self, action: &str) -> Option<TechId> {
        let Some(engine) = &self.engine else {
            let msg = format!("Unavailable: {} — no game in progress.", action);
            self.state.log.push(msg.clone());
            self.state.status_message = Some(msg);
            return None;
        };

        let visible = filtered_research_techs(&self.state, &engine.state);
        if visible.is_empty() {
            let msg = format!(
                "Unavailable: {} — no technologies match current filters.",
                action
            );
            self.state.log.push(msg.clone());
            self.state.status_message = Some(msg);
            return None;
        }

        let cursor = self.state.research.cursor % visible.len();
        Some(visible[cursor].id)
    }

    /// Queue the currently selected build item at the active colony
    fn queue_building(&mut self) {
        let colony_id = match self.state.colony.selected_colony {
            Some(id) => id,
            None => {
                let msg = "Unavailable: queue build — no colony selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let completed = self
            .engine
            .as_ref()
            .and_then(|e| e.state.empires.get(&e.state.player_empire))
            .map(|emp| emp.research.completed.clone())
            .unwrap_or_default();
        let surface_buildings = BuildingType::all();
        let visible_orbitals: Vec<_> = OrbitalStructureType::all()
            .iter()
            .filter(|ot| {
                ot.required_tech()
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .collect();
        let visible_ships: Vec<_> = game_core::all_ship_designs()
            .iter()
            .filter(|d| {
                d.required_tech
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .collect();

        let total = surface_buildings.len() + visible_orbitals.len() + visible_ships.len();
        if total == 0 {
            let msg = "Unavailable: queue build — no build items available.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let cursor = self.state.colony.build_cursor % total;

        let item = if cursor < surface_buildings.len() {
            game_core::BuildItem::SurfaceStructure(surface_buildings[cursor])
        } else if cursor < surface_buildings.len() + visible_orbitals.len() {
            game_core::BuildItem::OrbitalStructure(
                *visible_orbitals[cursor - surface_buildings.len()],
            )
        } else {
            game_core::BuildItem::Ship(
                visible_ships[cursor - surface_buildings.len() - visible_orbitals.len()].id,
            )
        };

        self.dispatch_command(Command::QueueBuild {
            colony: colony_id,
            item,
        });
    }

    /// Assign the currently highlighted role to the active colony.
    fn set_colony_role(&mut self) {
        let colony_id = match self.state.colony.selected_colony {
            Some(id) => id,
            None => {
                let msg = "Unavailable: set role — no colony selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let roles = ColonyRole::all();
        if roles.is_empty() {
            let msg = "Unavailable: set role — no colony roles are defined.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let cursor = self.state.colony.role_cursor % roles.len();
        let role = roles[cursor];

        self.dispatch_command(Command::SetColonyRole {
            colony: colony_id,
            role,
        });
    }

    /// Confirm the selected star as the rally point for `pending_rally_colony`.
    fn confirm_rally_point(&mut self) {
        let colony_id = match self.state.colony.pending_rally_colony.take() {
            Some(id) => id,
            None => return,
        };
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No star selected for rally point.".to_string());
                return;
            }
        };

        self.dispatch_command(Command::SetRallyPoint {
            colony: colony_id,
            star: star_id,
        });
    }

    /// Clear the rally point for the currently selected colony (or the pending rally colony).
    fn clear_rally_point(&mut self) {
        let colony_id = self
            .state
            .colony
            .selected_colony
            .or(self.state.colony.pending_rally_colony);
        let colony_id = match colony_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No colony selected for rally clear.".to_string());
                return;
            }
        };
        self.state.colony.pending_rally_colony = None;

        self.dispatch_command(Command::ClearRallyPoint { colony: colony_id });
    }

    #[allow(dead_code)]
    fn move_star_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                // Select first star if none selected
                self.state.navigation.selected_star = engine.state.stars.keys().next().copied();
                return;
            }
        };

        let current_star = match engine.state.stars.get(&current) {
            Some(s) => s,
            None => return,
        };

        // Find the nearest star in the given direction
        let mut best: Option<(StarId, i32)> = None;

        for star in engine.state.stars.values() {
            if star.id == current {
                continue;
            }

            let rel_x = star.x - current_star.x;
            let rel_y = star.y - current_star.y;

            // Check if star is in the right direction
            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((star.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((star.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_star = Some(id);
        }
    }

    fn move_sector_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.navigation.selected_sector {
            Some(id) => id,
            None => {
                self.state.navigation.selected_sector = engine.state.sectors.keys().next().copied();
                return;
            }
        };

        let current_sector = match engine.state.sectors.get(&current) {
            Some(s) => s,
            None => return,
        };

        let mut best: Option<(SectorId, i32)> = None;

        for sector in engine.state.sectors.values() {
            if sector.id == current {
                continue;
            }

            let rel_x = sector.x - current_sector.x;
            let rel_y = sector.y - current_sector.y;

            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((sector.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((sector.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_sector = Some(id);
            self.state.navigation.selected_star = engine
                .state
                .stars
                .values()
                .find(|s| s.sector == id)
                .map(|s| s.id);
        }
    }

    fn move_star_selection_in_sector(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let sector_id = match self.state.navigation.selected_sector {
            Some(id) => id,
            None => return,
        };

        let current = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                self.state.navigation.selected_star = engine
                    .state
                    .stars
                    .values()
                    .find(|s| s.sector == sector_id)
                    .map(|s| s.id);
                return;
            }
        };

        let current_star = match engine.state.stars.get(&current) {
            Some(s) if s.sector == sector_id => s,
            _ => {
                self.state.navigation.selected_star = engine
                    .state
                    .stars
                    .values()
                    .find(|s| s.sector == sector_id)
                    .map(|s| s.id);
                return;
            }
        };

        let mut best: Option<(StarId, i32)> = None;

        for star in engine.state.stars.values() {
            if star.id == current || star.sector != sector_id {
                continue;
            }

            let rel_x = star.x - current_star.x;
            let rel_y = star.y - current_star.y;

            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((star.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((star.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_star = Some(id);
        }
    }

    fn end_turn(&mut self) {
        self.dispatch_command(Command::EndTurn);
    }

    /// Open the Galactic Dispatch modal showing the latest dispatch.
    fn open_latest_dispatch(&mut self) {
        if let Some(engine) = &self.engine {
            if !engine.state.galactic_dispatches.is_empty() {
                self.state.overlay.dispatch_history_index =
                    engine.state.galactic_dispatches.len().saturating_sub(1);
                self.state.overlay.show_dispatch = true;
            } else {
                let msg = "No dispatches available yet.";
                self.state.status_message = Some(msg.to_string());
            }
        }
    }

    /// Open Battle Reports modal at the newest report.
    fn open_latest_battle_report(&mut self) {
        if let Some(engine) = &self.engine {
            if !engine.state.battle_reports.is_empty() {
                self.state.overlay.battle_report_index =
                    engine.state.battle_reports.len().saturating_sub(1);
                self.state.overlay.battle_report_inspect = false;
                self.state.overlay.show_battle_reports = true;
            } else {
                let msg = "No battle reports available yet.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
            }
        }
    }

    /// Dispatch an available scout fleet to the currently selected star system.
    /// Logs an error if no fleet is available or the destination is already explored.
    fn dispatch_scout(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: dispatch scout — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Scout), None)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: dispatch scout — no idle scout is available.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        self.dispatch_command(Command::SendScout {
            fleet: fleet_id,
            destination: star_id,
        });
    }

    /// Move the first idle player fleet to the currently selected (explored) star system.
    /// Logs an error if no idle fleet is available or the destination is not explored.
    fn move_fleet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: move fleet — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, None, None)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: move fleet — no idle fleet is available.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        self.dispatch_command(Command::MoveFleet {
            fleet: fleet_id,
            destination: star_id,
        });
    }

    /// Colonize the currently selected planet at the selected star system
    /// using an idle colonizer fleet present at that system.
    fn colonize_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: colonize — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Colonizer), Some(star_id))
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: colonize — no idle colonizer at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };

        if planet_count == 0 {
            let msg = "Unavailable: colonize — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);

        self.dispatch_command(Command::Colonize {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }

    /// Survey the currently selected planet using an idle science ship at the selected star.
    fn survey_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: survey — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(star_id))
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: survey — no idle science ship at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };

        if planet_count == 0 {
            let msg = "Unavailable: survey — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }

        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);

        self.dispatch_command(Command::SurveyPlanet {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }

    /// Invade the currently selected enemy colony using an idle troop transport at this system.
    fn invade_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: invade — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(
                &engine.state,
                Some(FleetKind::TroopTransport),
                Some(star_id),
            )
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: invade — no idle troop transport at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };
        if planet_count == 0 {
            let msg = "Unavailable: invade — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }

        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);
        self.dispatch_command(Command::Invade {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }

    fn player_fleets_at_selected_star(&self) -> Vec<FleetId> {
        let Some(engine) = &self.engine else {
            return Vec::new();
        };
        let Some(star_id) = self.state.navigation.selected_star else {
            return Vec::new();
        };
        engine
            .state
            .fleets
            .values()
            .filter(|fleet| fleet.owner == engine.state.player_empire && fleet.location == star_id)
            .map(|fleet| fleet.id)
            .collect()
    }

    fn cycle_system_fleet_focus(&mut self) {
        let fleets = self.player_fleets_at_selected_star();
        if fleets.is_empty() {
            self.push_error_status("Unavailable: inspect fleet — no player fleets in this system.");
            return;
        }
        self.state.navigation.selected_fleet_index =
            (self.state.navigation.selected_fleet_index + 1) % fleets.len();
        self.inspect_selected_fleet_composition();
    }

    fn inspect_selected_fleet_composition(&mut self) {
        let fleets = self.player_fleets_at_selected_star();
        if fleets.is_empty() {
            return;
        }
        let Some(selected) = fleets
            .get(
                self.state
                    .navigation
                    .selected_fleet_index
                    .min(fleets.len().saturating_sub(1)),
            )
            .copied()
        else {
            return;
        };
        let Some(engine) = &self.engine else {
            return;
        };
        let role = engine.state.fleet_role_for(selected);
        let formation = engine.state.fleet_formation_for(selected);
        let supply = engine.state.fleet_supply_state(selected);
        if let Some(summary) = engine.state.fleet_evaluation(selected) {
            self.push_status(
                LogEntryKind::Other,
                format!(
                    "Fleet {} [{} | {} | {}] off {} def {} inv {} mob {} esc {} blk {} — {}",
                    selected.0,
                    role.label(),
                    formation.label(),
                    supply.label(),
                    summary.offensive,
                    summary.defensive,
                    summary.invasion_capability,
                    summary.mobility,
                    summary.escort_quality,
                    summary.blockade_strength,
                    supply.penalty_summary()
                ),
            );
        }
    }

    fn cycle_selected_fleet_role(&mut self) {
        let fleets = self.player_fleets_at_selected_star();
        if fleets.is_empty() {
            self.push_error_status(
                "Unavailable: set fleet role — no player fleets in this system.",
            );
            return;
        }
        let Some(selected) = fleets
            .get(
                self.state
                    .navigation
                    .selected_fleet_index
                    .min(fleets.len().saturating_sub(1)),
            )
            .copied()
        else {
            return;
        };
        let Some(engine) = &self.engine else {
            return;
        };
        let current = engine.state.fleet_role_for(selected);
        let roles = FleetRole::all();
        let idx = roles.iter().position(|role| *role == current).unwrap_or(0);
        let next = roles[(idx + 1) % roles.len()];
        self.dispatch_command(Command::SetFleetRole {
            fleet: selected,
            role: next,
        });
        self.inspect_selected_fleet_composition();
    }

    fn cycle_selected_fleet_formation(&mut self) {
        let fleets = self.player_fleets_at_selected_star();
        if fleets.is_empty() {
            self.push_error_status(
                "Unavailable: set fleet formation — no player fleets in this system.",
            );
            return;
        }
        let Some(selected) = fleets
            .get(
                self.state
                    .navigation
                    .selected_fleet_index
                    .min(fleets.len().saturating_sub(1)),
            )
            .copied()
        else {
            return;
        };
        let Some(engine) = &self.engine else {
            return;
        };
        let current = engine.state.fleet_formation_for(selected);
        let formations = FleetFormation::all();
        let idx = formations
            .iter()
            .position(|formation| *formation == current)
            .unwrap_or(0);
        let next = formations[(idx + 1) % formations.len()];
        self.dispatch_command(Command::SetFleetFormation {
            fleet: selected,
            formation: next,
        });
        self.inspect_selected_fleet_composition();
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn render_update_confirm(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    confirm: &UpdateConfirmKind,
) {
    use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout},
        style::{Color, Style},
        text::Line,
        widgets::{Block, BorderType, Borders, Clear, Paragraph},
    };

    let box_width = 50u16.min(area.width.saturating_sub(4));
    let box_height = 9u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width.saturating_sub(box_width)) / 2;
    let y = area.y + (area.height.saturating_sub(box_height)) / 2;
    let dialog_area = ratatui::layout::Rect::new(x, y, box_width, box_height);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(format!(" {} ", confirm.title()))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let body = confirm.body();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White)),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            ratatui::text::Span::styled("[Y] Yes", Style::default().fg(Color::Cyan)),
            ratatui::text::Span::raw("    "),
            ratatui::text::Span::styled("[N] No / Esc", Style::default().fg(Color::DarkGray)),
        ]))
        .alignment(Alignment::Center),
        rows[3],
    );
}

#[cfg(test)]
mod tests;
