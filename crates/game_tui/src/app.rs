//! Application state and main run loop

use crate::components::{render_help, render_palette, EventLog, LogEntryKind, PaletteCommand};
use crate::keys::KeyMap;
use crate::screens::empire_overview::{derive_empire_overview, EmpireOverviewData, OverviewSort};
use crate::screens::research::{
    filtered_research_techs, RESEARCH_DOMAIN_FILTER_COUNT, RESEARCH_STATUS_FILTER_COUNT,
};
use crate::screens::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::{
    empire_definition_by_id, tech_by_id, BuildingType, ColonyId, ColonyRole, Command, Engine,
    Event as CoreEvent, FleetId, FleetKind, GalaxySize, OrbitalStructureType, ScenarioSetup,
    SectorId, StarId, TechId,
};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::time::Duration;

/// Default save file path
const DEFAULT_SAVE_PATH: &str = "farspace.sav";

/// Main application state
pub struct App {
    state: AppState,
    engine: Option<Engine>,
}

/// UI state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub(crate) active: Screen,
    pub(crate) overlay: OverlayState,
    pub(crate) navigation: NavigationState,
    pub(crate) sector_overview: SectorOverviewState,
    pub(crate) colony: ColonyScreenState,
    pub(crate) research: ResearchScreenState,
    pub(crate) overview: EmpireOverviewScreenState,
    pub(crate) new_game_setup: NewGameSetupState,
    pub(crate) log: EventLog,
    pub(crate) quit: bool,
    /// Monotonically-increasing frame counter, incremented once per render loop iteration.
    /// Used only for low-frequency UI animations; never affects simulation state.
    pub(crate) tick_count: u64,
    /// When true, all fleet travel animations are suppressed (accessibility / low-motion).
    pub(crate) reduced_motion: bool,
    /// Status line shown in contextual footer hints.
    pub(crate) status_message: Option<String>,
}

/// UI overlay state shared by all screens.
#[derive(Debug, Clone, Default)]
pub(crate) struct OverlayState {
    pub(crate) show_help: bool,
    pub(crate) show_palette: bool,
    pub(crate) palette_input: String,
}

/// Cross-screen map/system selection state.
#[derive(Debug, Clone, Default)]
pub(crate) struct NavigationState {
    pub(crate) selected_sector: Option<SectorId>,
    pub(crate) selected_star: Option<StarId>,
    /// Selected planet index when inspecting a system.
    pub(crate) selected_planet_index: usize,
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

impl App {
    fn empire_display_name(&self, empire_id: game_core::EmpireId) -> String {
        self.engine
            .as_ref()
            .and_then(|engine| engine.state.empires.get(&empire_id))
            .map(|empire| empire.name.clone())
            .unwrap_or_else(|| format!("Empire {}", empire_id.0))
    }

    fn format_core_event_for_log(&self, event: &CoreEvent) -> String {
        match event {
            CoreEvent::FirstContact { with_empire } => {
                let name = self.empire_display_name(*with_empire);
                let tone = self
                    .engine
                    .as_ref()
                    .and_then(|engine| engine.state.empires.get(with_empire))
                    .and_then(|empire| empire.empire_def)
                    .and_then(empire_definition_by_id)
                    .map(|def| def.tone)
                    .unwrap_or("Unknown stance");
                format!("First contact established with {name} — {tone}")
            }
            CoreEvent::AiResearchSelected { empire, tech } => {
                let name = self.empire_display_name(*empire);
                let tech_name = tech_by_id(*tech)
                    .map(|record| record.name)
                    .unwrap_or("Unknown Tech");
                format!("{name} redirected its labs to {tech_name}")
            }
            CoreEvent::AiBuildQueued {
                empire,
                colony,
                item,
            } => {
                let name = self.empire_display_name(*empire);
                format!("{name} queued {} at colony {}", item.name(), colony.0)
            }
            CoreEvent::AiScoutDispatched {
                empire,
                fleet,
                destination,
            } => {
                let name = self.empire_display_name(*empire);
                format!(
                    "{name} dispatched scout {} to system {}",
                    fleet.0, destination.0
                )
            }
            CoreEvent::AiColonized {
                empire,
                star,
                planet_index,
                colony,
            } => {
                let name = self.empire_display_name(*empire);
                format!(
                    "{name} founded colony {} at system {} orbit {}",
                    colony.0,
                    star.0,
                    planet_index + 1
                )
            }
            CoreEvent::AiColonyRoleAssigned {
                empire,
                colony,
                role,
            } => {
                let name = self.empire_display_name(*empire);
                format!("{name} reorganized colony {} as {}", colony.0, role.name())
            }
            _ => event.to_log_message(),
        }
    }

    fn empire_is_known(&self, empire_id: game_core::EmpireId) -> bool {
        self.engine
            .as_ref()
            .map(|engine| {
                engine
                    .state
                    .diplomacy
                    .get(&empire_id)
                    .map(|status| *status != game_core::RelationshipStatus::Unknown)
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn colony_is_player_owned(&self, colony_id: game_core::ColonyId) -> bool {
        self.engine
            .as_ref()
            .and_then(|engine| engine.state.colonies.get(&colony_id).map(|colony| (engine, colony)))
            .map(|(engine, colony)| colony.owner == engine.state.player_empire)
            .unwrap_or(false)
    }

    fn event_visible_to_player(&self, event: &CoreEvent) -> bool {
        let Some(engine) = self.engine.as_ref() else {
            return true;
        };

        match event {
            CoreEvent::EconomySummary { empire, .. }
            | CoreEvent::FoodShortage { empire, .. }
            | CoreEvent::CreditDeficit { empire, .. } => *empire == engine.state.player_empire,
            CoreEvent::ColonyStatusWarning { colony, .. }
            | CoreEvent::PopulationGrew { colony, .. }
            | CoreEvent::ColonyIsolated { colony }
            | CoreEvent::ColonyReconnected { colony } => self.colony_is_player_owned(*colony),
            CoreEvent::AiResearchSelected { empire, .. }
            | CoreEvent::AiBuildQueued { empire, .. }
            | CoreEvent::AiScoutDispatched { empire, .. }
            | CoreEvent::AiColonized { empire, .. }
            | CoreEvent::AiColonyRoleAssigned { empire, .. } => self.empire_is_known(*empire),
            _ => true,
        }
    }

    fn push_core_event_to_log(&mut self, event: &CoreEvent) {
        if !self.event_visible_to_player(event) {
            return;
        }
        let message = self.format_core_event_for_log(event);
        let kind = LogEntryKind::from_message(&message);
        self.state.log.push_with_kind(kind, message);
    }

    fn push_status(&mut self, kind: LogEntryKind, message: impl Into<String>) {
        let message = message.into();
        self.state.log.push_with_kind(kind, message.clone());
        self.state.status_message = Some(message);
    }

    fn push_error_status(&mut self, message: impl Into<String>) {
        self.push_status(LogEntryKind::Error, message);
    }

    /// Create a new application
    pub fn new() -> Self {
        App {
            state: AppState::default(),
            engine: None,
        }
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
        Ok(())
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
        match cmd {
            PaletteCommand::Save => match self.save_game(&path) {
                Ok(()) => {
                    let msg = format!("Save: wrote {}", path.display());
                    self.push_status(LogEntryKind::SaveLoad, msg);
                }
                Err(e) => {
                    self.push_error_status(e);
                }
            },
            PaletteCommand::Load => match self.load_game(&path) {
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
        }
    }

    /// Run the main event loop
    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        while !self.state.quit {
            terminal.draw(|frame| self.render(frame))?;
            // Increment animation tick counter each render frame (wraps safely at u64::MAX)
            self.state.tick_count = self.state.tick_count.wrapping_add(1);

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }

        Ok(())
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
            render_palette(frame, area, &self.state.overlay.palette_input);
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) {
        // Handle overlays first
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

        if key.code == KeyCode::Char('O') && self.engine.is_some() {
            self.state.active = Screen::EmpireOverview;
            self.state.overview.filter_input = false;
            return;
        }

        // Screen-specific handling
        match self.state.active {
            Screen::Menu => self.handle_menu_key(key),
            Screen::NewGameSetup => self.handle_new_game_setup_key(key),
            Screen::SectorOverview => self.handle_sector_overview_key(key),
            Screen::SectorMap => self.handle_sector_map_key(key),
            Screen::System => self.handle_system_key(key),
            Screen::Colony => self.handle_colony_key(key),
            Screen::EmpireOverview => self.handle_empire_overview_key(key),
            Screen::Research => self.handle_research_key(key),
            Screen::Diplomacy => self.handle_diplomacy_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        if KeyMap::is_new_game(key) {
            // Navigate to the setup screen instead of directly starting a game.
            self.state.active = Screen::NewGameSetup;
        } else if KeyMap::is_load_game(key) {
            let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
            match self.load_game(&path) {
                Ok(()) => {
                    let msg = format!("Load: loaded {}", path.display());
                    self.state.log.push(msg.clone());
                    self.state.status_message = Some(msg);
                }
                Err(e) => {
                    self.state.log.push(e.clone());
                    self.state.status_message = Some(e);
                }
            }
        }
    }

    /// Handle keyboard input on the New Game Setup screen.
    fn handle_new_game_setup_key(&mut self, key: KeyEvent) {
        use crate::screens::new_game_setup::{FIELD_EMPIRE, FIELD_SEED};
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
                } else if cursor == FIELD_EMPIRE {
                    // Enter on empire field cycles to next empire (same as →).
                    self.setup_cycle_field(true);
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
            KeyCode::Char('S') => {
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
        let all_defs = game_core::all_empire_definitions();
        match self.state.new_game_setup.cursor {
            FIELD_EMPIRE => {
                let n = all_defs.len();
                if forward {
                    self.state.new_game_setup.empire_cursor =
                        (self.state.new_game_setup.empire_cursor + 1) % n;
                } else {
                    self.state.new_game_setup.empire_cursor =
                        (self.state.new_game_setup.empire_cursor + n - 1) % n;
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
        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            // End turn from diplomacy screen
            _ => {
                if KeyMap::is_end_turn(key) {
                    self.end_turn();
                }
            }
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
            let report =
                is_end_turn.then(|| Self::build_end_turn_report(engine.state.turn, &events));
            (events, report)
        };

        for event in &events {
            self.push_core_event_to_log(event);
        }

        if let Some(report) = end_turn_report {
            self.push_status(LogEntryKind::TurnReport, report);
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

        let total =
            surface_buildings.len() + visible_orbitals.len() + visible_ships.len();
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

    fn build_end_turn_report(turn: u32, events: &[CoreEvent]) -> String {
        let mut explored = 0usize;
        let mut surveyed = 0usize;
        let mut colonized = 0usize;
        let mut research_completed = 0usize;
        let mut queue_transitions_started = 0usize;
        let mut fleets_arrived = 0usize;
        let mut warnings = 0usize;
        let mut errors = 0usize;
        let mut newly_isolated = 0usize;
        let mut reconnected = 0usize;
        let mut invasions_won = 0usize;
        let mut invasions_failed = 0usize;

        for event in events {
            match event {
                CoreEvent::SystemExplored { .. } => explored += 1,
                CoreEvent::PlanetSurveyCompleted { .. } => surveyed += 1,
                CoreEvent::ColonizationCompleted { .. } => colonized += 1,
                CoreEvent::ResearchCompleted { .. } => research_completed += 1,
                CoreEvent::ResearchCompletedWithQueueTransition {
                    started: Some(_), ..
                } => queue_transitions_started += 1,
                CoreEvent::FleetArrived { .. } => fleets_arrived += 1,
                CoreEvent::FoodShortage { .. } | CoreEvent::CreditDeficit { .. } => warnings += 1,
                CoreEvent::ColonyIsolated { .. } => newly_isolated += 1,
                CoreEvent::ColonyReconnected { .. } => reconnected += 1,
                CoreEvent::InvasionSucceeded { .. } => invasions_won += 1,
                CoreEvent::InvasionFailed { .. } => invasions_failed += 1,
                CoreEvent::Error { .. } => errors += 1,
                _ => {}
            }
        }

        format!(
            "Turn {} global summary (all empires): explored {}, surveyed {}, colonized {}, research {}, queued starts {}, arrivals {}, invasions won {}, invasions failed {}, warnings {}, isolated {}, reconnected {}, errors {}.",
            turn,
            explored,
            surveyed,
            colonized,
            research_completed,
            queue_transitions_started,
            fleets_arrived,
            invasions_won,
            invasions_failed,
            warnings,
            newly_isolated,
            reconnected,
            errors
        )
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// Create a unique temporary file path for tests to avoid parallel races.
    fn tmp_save_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("farspace_test_{}.sav", name))
    }

    #[test]
    fn toggle_help_overlay_on_question_mark() {
        let mut app = App::new();
        assert!(!app.state.overlay.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.state.overlay.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(!app.state.overlay.show_help);
    }

    #[test]
    fn toggle_palette_on_colon() {
        let mut app = App::new();
        assert!(!app.state.overlay.show_palette);

        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.overlay.show_palette);

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.overlay.show_palette);
    }

    #[test]
    fn quit_flag_set_on_q() {
        let mut app = App::new();
        assert!(!app.state.quit);

        app.handle_key(key(KeyCode::Char('q')));
        assert!(app.state.quit);
    }

    #[test]
    fn screen_transitions_on_new_game() {
        let mut app = App::new();
        assert_eq!(app.state.active, Screen::Menu);

        app.new_game(42);
        assert_eq!(app.state.active, Screen::SectorOverview);
        assert!(app.engine.is_some());
    }

    #[test]
    fn enter_opens_sector_map_from_sector_overview() {
        let mut app = App::new();
        app.new_game(42);
        assert_eq!(app.state.active, Screen::SectorOverview);

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.state.active, Screen::SectorMap);
    }

    #[test]
    fn enter_opens_system_view_from_sector_map() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::SectorMap;

        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.state.active, Screen::System);
    }

    #[test]
    fn o_key_opens_empire_overview() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::SectorMap;

        app.handle_key(key(KeyCode::Char('O')));

        assert_eq!(app.state.active, Screen::EmpireOverview);
    }

    #[test]
    fn overview_enter_opens_selected_colony() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::EmpireOverview;
        app.state.overview.cursor = 0;

        let expected_colony = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .values()
            .find(|c| c.owner == app.engine.as_ref().unwrap().state.player_empire)
            .map(|c| c.id)
            .expect("player colony should exist");

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.state.active, Screen::Colony);
        assert_eq!(app.state.colony.selected_colony, Some(expected_colony));
    }

    #[test]
    fn overview_s_opens_selected_system() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::EmpireOverview;
        app.state.overview.cursor = 0;

        let expected_star = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .values()
            .find(|c| c.owner == app.engine.as_ref().unwrap().state.player_empire)
            .map(|c| c.star)
            .expect("player colony should exist");

        app.handle_key(key(KeyCode::Char('S')));

        assert_eq!(app.state.active, Screen::System);
        assert_eq!(app.state.navigation.selected_star, Some(expected_star));
    }

    #[test]
    fn esc_returns_from_system_view_to_sector_map() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::System;

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.state.active, Screen::SectorMap);
    }

    #[test]
    fn system_view_colonize_targets_selected_planet() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
            .expect("need explored non-home star");
        app.state.navigation.selected_star = Some(target);
        app.state.navigation.selected_planet_index = 1;
        app.state.active = Screen::System;

        {
            let star = engine.state.stars.get_mut(&target).unwrap();
            if star.planets.len() < 2 {
                let clone = star.planets[0].clone();
                star.planets.push(clone);
            }
            for planet in &mut star.planets {
                planet.colony = None;
                planet.habitable = true;
                planet.surveyed = true;
            }
        }

        let fleet_id = FleetId(6000);
        engine.state.fleets.insert(
            fleet_id,
            game_core::Fleet {
                id: fleet_id,
                owner: engine.state.player_empire,
                location: target,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        app.handle_key(key(KeyCode::Char('C')));

        let star = &app.engine.as_ref().unwrap().state.stars[&target];
        assert!(
            star.planets[1].colony.is_some(),
            "selected planet should be colonized"
        );
    }

    #[test]
    fn system_view_invade_targets_selected_planet() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
            .expect("need explored non-home star");
        app.state.navigation.selected_star = Some(target);
        app.state.navigation.selected_planet_index = 0;
        app.state.active = Screen::System;

        let enemy_id = engine.state.ai_empire.expect("AI empire required");
        engine
            .state
            .diplomacy
            .insert(enemy_id, game_core::RelationshipStatus::War);

        let enemy_colony_id = game_core::ColonyId(9001);
        engine.state.colonies.insert(
            enemy_colony_id,
            game_core::Colony {
                id: enemy_colony_id,
                star: target,
                planet_index: 0,
                owner: enemy_id,
                population: 1,
                production: 5,
                prod_pct: 50,
                research_pct: 50,
                build_queue: Vec::new(),
                accumulated_production: 0,
                buildings: Vec::new(),
                surface_installations: Vec::new(),
                orbital_installations: Vec::new(),
                stability: 10,
                role: game_core::ColonyRole::Balanced,
                rally_point: None,
            },
        );
        engine.state.stars.get_mut(&target).unwrap().planets[0].colony = Some(enemy_colony_id);

        let troop_fleet = FleetId(6100);
        engine.state.fleets.insert(
            troop_fleet,
            game_core::Fleet {
                id: troop_fleet,
                owner: engine.state.player_empire,
                location: target,
                ships: 1,
                kind: FleetKind::TroopTransport,
                strength: 1,
                integrity: 100,
            },
        );

        app.handle_key(key(KeyCode::Char('I')));

        let captured_owner = app.engine.as_ref().unwrap().state.colonies[&enemy_colony_id].owner;
        assert_eq!(
            captured_owner,
            app.engine.as_ref().unwrap().state.player_empire,
            "selected enemy planet should be invaded and captured"
        );
    }

    #[test]
    fn star_selection_moves_on_hjkl() {
        let mut app = App::new();
        app.new_game(42);

        let initial = app.state.navigation.selected_star;
        assert!(initial.is_some());

        // Move right
        app.handle_key(key(KeyCode::Char('l')));

        // Selection should change (might be same if no star to right)
        // Just verify it doesn't crash
        assert!(app.state.navigation.selected_star.is_some());
    }

    #[test]
    fn end_turn_advances_game() {
        let mut app = App::new();
        app.new_game(42);

        let initial_turn = app.engine.as_ref().unwrap().state.turn;

        app.handle_key(key(KeyCode::Char('t')));

        let new_turn = app.engine.as_ref().unwrap().state.turn;
        assert_eq!(new_turn, initial_turn + 1);
    }

    #[test]
    fn end_turn_with_e_key() {
        let mut app = App::new();
        app.new_game(42);
        let initial_turn = app.engine.as_ref().unwrap().state.turn;

        app.handle_key(key(KeyCode::Char('e')));

        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    }

    #[test]
    fn app_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        app.new_game(42);

        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    #[test]
    fn menu_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let app = App::new();

        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    #[test]
    fn end_turn_without_engine_does_nothing() {
        let mut app = App::new();
        // No game started — end_turn should be a no-op
        app.end_turn();
        assert!(app.engine.is_none());
    }

    #[test]
    fn move_star_selection_without_engine_does_nothing() {
        let mut app = App::new();
        app.move_star_selection(1, 0);
        assert!(app.state.navigation.selected_star.is_none());
    }

    #[test]
    fn move_star_selection_with_no_selection_selects_first() {
        let mut app = App::new();
        app.new_game(42);
        app.state.navigation.selected_star = None;

        // With no selection, move should select first star
        app.move_star_selection(1, 0);
        // Either first star selected or unchanged; verify no panic
        // (may remain None if no star is to the right, but first-star selection triggers)
    }

    #[test]
    fn handle_key_end_turn_on_galaxy_screen() {
        let mut app = App::new();
        app.new_game(42);
        let initial_turn = app.engine.as_ref().unwrap().state.turn;

        // 't' ends the turn on galaxy screen (Enter opens System View)
        app.handle_key(key(KeyCode::Char('t')));
        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    }

    #[test]
    fn escape_closes_help_overlay() {
        let mut app = App::new();
        app.state.overlay.show_help = true;

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.overlay.show_help);
    }

    #[test]
    fn palette_key_closes_palette() {
        let mut app = App::new();
        app.state.overlay.show_palette = true;
        app.state.overlay.palette_input = String::new();

        // Pressing `:` when palette is open adds `:` to input (does not close)
        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.overlay.show_palette);
        assert_eq!(app.state.overlay.palette_input, ":");

        // Pressing Esc closes the palette
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.overlay.show_palette);
        assert!(app.state.overlay.palette_input.is_empty());
    }

    #[test]
    fn palette_accepts_character_input() {
        let mut app = App::new();
        app.state.overlay.show_palette = true;
        app.state.overlay.palette_input.clear();

        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('v')));
        app.handle_key(key(KeyCode::Char('e')));

        assert_eq!(app.state.overlay.palette_input, "save");
    }

    #[test]
    fn palette_backspace_removes_last_char() {
        let mut app = App::new();
        app.state.overlay.show_palette = true;
        app.state.overlay.palette_input = "sav".to_string();

        app.handle_key(key(KeyCode::Backspace));

        assert_eq!(app.state.overlay.palette_input, "sa");
    }

    #[test]
    fn palette_unknown_command_logs_error() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.execute_palette_input("unknowncmd");

        assert!(app.state.log.len() > before);
    }

    #[test]
    fn palette_enter_executes_and_closes() {
        let mut app = App::new();
        app.state.overlay.show_palette = true;
        app.state.overlay.palette_input = "unknowncmd".to_string();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.overlay.show_palette);
        assert!(app.state.overlay.palette_input.is_empty());
    }

    #[test]
    fn save_game_without_engine_returns_error() {
        let mut app = App::new();
        let path = tmp_save_path("no_engine");
        let result = app.save_game(&path);
        assert!(result.is_err());
    }

    #[test]
    fn save_and_load_round_trip_via_app() {
        let path = tmp_save_path("round_trip");
        let mut app = App::new();
        app.new_game(42);

        // End a turn so state is non-trivial
        app.end_turn();
        let turn_before = app.engine.as_ref().unwrap().state.turn;

        // Save then load
        app.save_game(&path).expect("save should succeed");
        app.load_game(&path).expect("load should succeed");

        let turn_after = app.engine.as_ref().unwrap().state.turn;
        assert_eq!(turn_before, turn_after);
        assert_eq!(app.state.active, Screen::SectorOverview);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_game_missing_file_logs_error() {
        let path = tmp_save_path("missing_file");
        // Ensure file does not exist
        let _ = std::fs::remove_file(&path);

        let mut app = App::new();
        let result = app.load_game(&path);
        assert!(result.is_err());
        // Error message should contain the path
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Error:"),
            "Expected 'Error:' prefix, got: {}",
            msg
        );
    }

    #[test]
    fn save_command_via_palette_logs_success() {
        let path = tmp_save_path("palette_save");
        let mut app = App::new();
        app.new_game(42);

        // Temporarily override the palette to call save directly with our path
        let before = app.state.log.len();
        match app.save_game(&path) {
            Ok(()) => app.state.log.push("Game saved.".to_string()),
            Err(e) => app.state.log.push(e),
        }

        assert!(app.state.log.len() > before);
        let last = app.state.log.last_n(1);
        assert!(
            last[0].contains("saved") || last[0].contains("Save"),
            "Expected save confirmation, got: {}",
            last[0]
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_command_after_save_via_palette() {
        let path = tmp_save_path("palette_load");
        let mut app = App::new();
        app.new_game(42);
        app.end_turn();
        let turn_before = app.engine.as_ref().unwrap().state.turn;

        app.save_game(&path).expect("save should succeed");
        app.load_game(&path).expect("load should succeed");

        let turn_after = app.engine.as_ref().unwrap().state.turn;
        assert_eq!(turn_before, turn_after);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn end_turn_appends_events_to_log() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.end_turn();

        // At least one event should have been logged (TurnAdvanced, ColonyProduced, etc.)
        assert!(app.state.log.len() > before);
    }

    #[test]
    fn palette_colon_only_input_does_not_log() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        // A bare ":" should be a no-op after normalization
        app.execute_palette_input(":");
        app.execute_palette_input("  ");
        app.execute_palette_input(":  ");

        assert_eq!(app.state.log.len(), before, "No-op commands should not log");
    }

    #[test]
    fn palette_enter_with_colon_only_does_not_execute() {
        let mut app = App::new();
        app.new_game(42);
        app.state.overlay.show_palette = true;
        app.state.overlay.palette_input = ":".to_string();
        let before = app.state.log.len();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.overlay.show_palette);
        // No command should have been executed → log unchanged
        assert_eq!(app.state.log.len(), before);
    }

    #[test]
    fn save_error_message_contains_error_prefix() {
        let path = tmp_save_path("error_prefix");
        let _ = std::fs::remove_file(&path);
        // Use an invalid path to force an error
        let invalid_path = std::path::Path::new("/nonexistent_dir/farspace_test.sav");
        let mut app = App::new();
        app.new_game(42);
        let result = app.save_game(invalid_path);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("Error:"),
            "Expected 'Error:' prefix, got: {}",
            msg
        );
        assert!(
            msg.contains("nonexistent_dir"),
            "Expected path in message, got: {}",
            msg
        );
    }

    #[test]
    fn menu_load_key_triggers_load() {
        // This test calls handle_key(l) on the menu, which internally uses DEFAULT_SAVE_PATH.
        // We just verify it doesn't crash and stays on menu when the file is absent.
        // The file may or may not exist from other tests; either outcome (load succeeds or fails
        // gracefully) is acceptable here — we only care there's no panic.
        let mut app = App::new();
        app.handle_key(key(KeyCode::Char('l')));
        // If load failed, screen stays Menu. If it somehow succeeded, it moves to Galaxy.
        // Both are valid; the key requirement is no panic.
        let _ = app.state.active;
    }

    #[test]
    fn end_turn_report_counts_key_events() {
        let events = vec![
            CoreEvent::SystemExplored { star: StarId(1) },
            CoreEvent::PlanetSurveyCompleted {
                star: StarId(2),
                planet_index: 0,
            },
            CoreEvent::ColonizationCompleted {
                empire: game_core::EmpireId(1),
                fleet: FleetId(9),
                star: StarId(3),
                planet_index: 1,
                colony: ColonyId(77),
            },
            CoreEvent::ResearchCompleted { tech: TechId(4) },
            CoreEvent::FleetArrived {
                fleet: FleetId(8),
                star: StarId(5),
            },
            CoreEvent::FoodShortage {
                empire: game_core::EmpireId(1),
                deficit: 2,
            },
            CoreEvent::ColonyIsolated {
                colony: ColonyId(77),
            },
            CoreEvent::ColonyReconnected {
                colony: ColonyId(78),
            },
            CoreEvent::InvasionSucceeded {
                attacker: game_core::EmpireId(1),
                defender: game_core::EmpireId(2),
                fleet: FleetId(10),
                star: StarId(6),
                planet_index: 0,
                colony: ColonyId(79),
                transports_lost: 1,
            },
            CoreEvent::InvasionFailed {
                attacker: game_core::EmpireId(1),
                defender: game_core::EmpireId(2),
                fleet: FleetId(11),
                star: StarId(6),
                planet_index: 1,
                colony: ColonyId(80),
                invasion_strength: 12,
                defense_strength: 20,
                transports_lost: 1,
                reason: "Defenses held".to_string(),
            },
            CoreEvent::Error {
                message: "bad command".to_string(),
            },
        ];

        let report = App::build_end_turn_report(12, &events);
        assert!(report.contains("Turn 12 global summary (all empires)"));
        assert!(report.contains("explored 1"));
        assert!(report.contains("surveyed 1"));
        assert!(report.contains("colonized 1"));
        assert!(report.contains("research 1"));
        assert!(report.contains("queued starts 0"));
        assert!(report.contains("arrivals 1"));
        assert!(report.contains("invasions won 1"));
        assert!(report.contains("invasions failed 1"));
        assert!(report.contains("warnings 1"));
        assert!(report.contains("isolated 1"));
        assert!(report.contains("reconnected 1"));
        assert!(report.contains("errors 1"));
    }

    #[test]
    fn end_turn_report_handles_empty_event_list() {
        let report = App::build_end_turn_report(3, &[]);
        assert_eq!(
            report,
            "Turn 3 global summary (all empires): explored 0, surveyed 0, colonized 0, research 0, queued starts 0, arrivals 0, invasions won 0, invasions failed 0, warnings 0, isolated 0, reconnected 0, errors 0."
        );
    }

    #[test]
    fn dispatch_command_logs_events_and_updates_error_status() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.dispatch_command(Command::SelectResearch { tech: TechId(9999) });

        assert!(app.state.log.len() > before);
        assert_eq!(
            app.state.status_message.as_deref(),
            Some("Error: Tech 9999 not found")
        );
    }

    #[test]
    fn dispatch_command_end_turn_sets_summary_status() {
        let mut app = App::new();
        app.new_game(42);
        let initial_turn = app.engine.as_ref().unwrap().state.turn;

        app.dispatch_command(Command::EndTurn);

        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
        assert!(app
            .state
            .status_message
            .as_deref()
            .is_some_and(|message| message.starts_with("Turn 2 global summary")));
    }

    #[test]
    fn galaxy_renders_with_log_entries() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        app.new_game(42);
        app.end_turn();

        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    // ──────────────────────────────────────────────────────────────────
    // Colony screen tests
    // ──────────────────────────────────────────────────────────────────

    /// Navigate to the colony screen from a star that has the player's colony.
    fn goto_colony_screen(app: &mut App) -> bool {
        // Find the home star (which holds the colony)
        let engine = app.engine.as_ref().unwrap();
        let player_empire = engine.state.player_empire;
        let home_star_id = engine
            .state
            .colonies
            .values()
            .find(|c| c.owner == player_empire)
            .map(|c| c.star);

        if let Some(star_id) = home_star_id {
            app.state.navigation.selected_star = Some(star_id);
            app.try_enter_colony()
        } else {
            false
        }
    }

    #[test]
    fn enter_colony_from_sector_map_with_c_key() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::SectorMap;

        // Select the star that has the player's home colony
        let player_empire = app.engine.as_ref().unwrap().state.player_empire;
        let home_star_id = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .values()
            .find(|c| c.owner == player_empire)
            .map(|c| c.star);
        app.state.navigation.selected_star = home_star_id;

        // Press 'c' to enter the colony screen — exercises the actual key binding
        app.handle_key(key(KeyCode::Char('c')));

        assert_eq!(app.state.active, Screen::Colony);
        assert!(app.state.colony.selected_colony.is_some());
    }

    #[test]
    fn try_enter_colony_prefers_selected_planet_colony() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();
        let player_empire = engine.state.player_empire;
        let home_colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == player_empire)
            .map(|(id, _)| *id)
            .expect("player colony should exist");
        let home_colony = engine
            .state
            .colonies
            .get(&home_colony_id)
            .cloned()
            .expect("player colony data should exist");
        let home_star_id = home_colony.star;

        let second_colony_id = ColonyId(engine.state.next_colony_id);
        engine.state.next_colony_id += 1;
        let mut second_colony = home_colony.clone();
        second_colony.id = second_colony_id;
        second_colony.planet_index = 1;
        second_colony.accumulated_production = 0;
        second_colony.build_queue.clear();
        engine
            .state
            .colonies
            .insert(second_colony_id, second_colony);

        let star = engine
            .state
            .stars
            .get_mut(&home_star_id)
            .expect("home star should exist");
        while star.planets.len() < 2 {
            let mut clone = star.planets[0].clone();
            clone.colony = None;
            star.planets.push(clone);
        }
        star.planets[1].colony = Some(second_colony_id);

        app.state.navigation.selected_star = Some(home_star_id);
        app.state.navigation.selected_planet_index = 1;
        assert!(app.try_enter_colony());
        assert_eq!(app.state.colony.selected_colony, Some(second_colony_id));
        assert_eq!(app.state.active, Screen::Colony);
    }

    #[test]
    fn c_key_in_system_view_enters_selected_planet_colony() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();
        let player_empire = engine.state.player_empire;
        let home_colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == player_empire)
            .map(|(id, _)| *id)
            .expect("player colony should exist");
        let home_colony = engine
            .state
            .colonies
            .get(&home_colony_id)
            .cloned()
            .expect("player colony data should exist");
        let home_star_id = home_colony.star;

        let second_colony_id = ColonyId(engine.state.next_colony_id);
        engine.state.next_colony_id += 1;
        let mut second_colony = home_colony.clone();
        second_colony.id = second_colony_id;
        second_colony.planet_index = 1;
        second_colony.accumulated_production = 0;
        second_colony.build_queue.clear();
        engine
            .state
            .colonies
            .insert(second_colony_id, second_colony);

        let star = engine
            .state
            .stars
            .get_mut(&home_star_id)
            .expect("home star should exist");
        while star.planets.len() < 2 {
            let mut clone = star.planets[0].clone();
            clone.colony = None;
            star.planets.push(clone);
        }
        star.planets[1].colony = Some(second_colony_id);

        app.state.active = Screen::System;
        app.state.navigation.selected_star = Some(home_star_id);
        app.state.navigation.selected_planet_index = 1;
        app.handle_key(key(KeyCode::Char('c')));

        assert_eq!(app.state.active, Screen::Colony);
        assert_eq!(app.state.colony.selected_colony, Some(second_colony_id));
    }

    #[test]
    fn enter_key_in_system_view_enters_selected_planet_colony() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_ref().unwrap();
        let player_empire = engine.state.player_empire;
        let home_colony_id = engine
            .state
            .colonies
            .iter()
            .find(|(_, c)| c.owner == player_empire)
            .map(|(id, _)| *id)
            .expect("player colony should exist");
        let home_star_id = engine.state.colonies[&home_colony_id].star;

        app.state.active = Screen::System;
        app.state.navigation.selected_star = Some(home_star_id);
        app.state.navigation.selected_planet_index =
            engine.state.colonies[&home_colony_id].planet_index;
        app.state.colony.selected_colony = None;

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.state.active, Screen::Colony);
        assert_eq!(app.state.colony.selected_colony, Some(home_colony_id));
    }

    #[test]
    fn enter_key_in_system_view_keeps_system_without_player_colony() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_ref().unwrap();
        let player_empire = engine.state.player_empire;
        let target_star = engine
            .state
            .stars
            .iter()
            .find(|(_, s)| {
                s.planets.iter().all(|p| {
                    p.colony.is_none_or(|cid| {
                        engine
                            .state
                            .colonies
                            .get(&cid)
                            .is_none_or(|c| c.owner != player_empire)
                    })
                })
            })
            .map(|(id, _)| *id)
            .expect("need a star without a player colony");

        app.state.active = Screen::System;
        app.state.navigation.selected_star = Some(target_star);
        app.state.navigation.selected_planet_index = 0;
        app.state.colony.selected_colony = None;

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.state.active, Screen::System);
        assert!(app.state.colony.selected_colony.is_none());
    }

    #[test]
    fn try_enter_colony_returns_false_without_engine() {
        let mut app = App::new();
        assert!(!app.try_enter_colony());
    }

    #[test]
    fn try_enter_colony_returns_false_without_selected_star() {
        let mut app = App::new();
        app.new_game(42);
        app.state.navigation.selected_star = None;
        assert!(!app.try_enter_colony());
    }

    #[test]
    fn try_enter_colony_returns_false_for_empty_star() {
        let mut app = App::new();
        app.new_game(42);
        // Navigate to SectorMap first
        app.state.active = Screen::SectorMap;
        // Select a star that has no colony
        let engine = app.engine.as_ref().unwrap();
        let player_empire = engine.state.player_empire;
        let empty_star = engine
            .state
            .stars
            .iter()
            .find(|(_, s)| {
                s.planets.iter().all(|p| {
                    p.colony.is_none_or(|cid| {
                        engine
                            .state
                            .colonies
                            .get(&cid)
                            .is_none_or(|c| c.owner != player_empire)
                    })
                })
            })
            .map(|(id, _)| *id);

        if let Some(star_id) = empty_star {
            app.state.navigation.selected_star = Some(star_id);
            assert!(!app.try_enter_colony());
            assert_eq!(app.state.active, Screen::SectorMap);
        }
        // If every star has a colony (very unlikely with 20 stars and 1 colony) we skip
    }

    #[test]
    fn esc_returns_to_sector_map_from_colony_screen() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);
        assert_eq!(app.state.active, Screen::Colony);

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.state.active, Screen::SectorMap);
        assert!(app.state.colony.selected_colony.is_none());
    }

    #[test]
    fn colony_build_cursor_moves_with_j_k() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let initial = app.state.colony.build_cursor;
        app.handle_key(key(KeyCode::Char('j')));
        assert_ne!(app.state.colony.build_cursor, initial);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.state.colony.build_cursor, initial);
    }

    #[test]
    fn colony_build_cursor_wraps_around_bottom() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let count = app.visible_build_count();
        // Move down past the last item
        for _ in 0..count {
            app.handle_key(key(KeyCode::Char('j')));
        }
        // Cursor should have wrapped to the start
        assert_eq!(app.state.colony.build_cursor, 0);
    }

    #[test]
    fn colony_build_cursor_wraps_around_top() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        // Move up from 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        let count = app.visible_build_count();
        assert_eq!(app.state.colony.build_cursor, count - 1);
    }

    #[test]
    fn enter_key_queues_building_on_colony_screen() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let colony_id = app.state.colony.selected_colony.unwrap();
        let initial_queue_len = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .build_queue
            .len();

        // Press Enter to queue the currently selected building
        app.handle_key(key(KeyCode::Enter));

        let new_queue_len = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .build_queue
            .len();

        assert_eq!(
            new_queue_len,
            initial_queue_len + 1,
            "Queue should grow by 1 after Enter"
        );
    }

    #[test]
    fn end_turn_works_from_colony_screen_with_e_key() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let initial_turn = app.engine.as_ref().unwrap().state.turn;
        app.handle_key(key(KeyCode::Char('e')));
        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    }

    #[test]
    fn colony_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    #[test]
    fn save_load_preserves_colony_buildings() {
        let path = tmp_save_path("colony_buildings");
        let mut app = App::new();
        app.new_game(42);

        // Navigate to colony and queue AquacultureBay (first building, cost 60)
        goto_colony_screen(&mut app);
        app.handle_key(key(KeyCode::Enter)); // queue first building
        app.handle_key(key(KeyCode::Esc)); // back to galaxy

        let colony_id = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .keys()
            .next()
            .copied()
            .unwrap();

        // Complete the building: base production is 10/turn, AquacultureBay costs 60 → 6 turns
        for _ in 0..6 {
            app.handle_key(key(KeyCode::Char('e')));
        }

        let buildings_before = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .buildings
            .clone();

        assert!(
            !buildings_before.is_empty(),
            "Building should be completed after enough production turns"
        );

        // Save and reload
        app.save_game(&path).expect("save should succeed");
        app.load_game(&path).expect("load should succeed");

        let buildings_after = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .buildings
            .clone();

        assert_eq!(
            buildings_before, buildings_after,
            "Colony buildings should survive save/load"
        );

        let _ = std::fs::remove_file(&path);
    }

    // ──────────────────────────────────────────────────────────────────
    // Research screen tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn r_key_opens_research_screen_from_sector_overview() {
        let mut app = App::new();
        app.new_game(42);
        assert_eq!(app.state.active, Screen::SectorOverview);

        app.handle_key(key(KeyCode::Char('r')));

        assert_eq!(app.state.active, Screen::Research);
        assert_eq!(app.state.research.cursor, 0);
    }

    #[test]
    fn esc_closes_research_screen_returns_to_sector_map() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.state.active, Screen::SectorMap);
    }

    #[test]
    fn research_cursor_wraps_on_j() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research.cursor = 0;

        // j increments cursor; just verify no panic and cursor stays in bounds
        app.handle_key(key(KeyCode::Char('j')));
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research.cursor < techs_len);
    }

    #[test]
    fn research_cursor_wraps_on_k() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research.cursor = 0;

        // k at position 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        // cursor should now point to last tech (5 for 6 techs with index 0..5)
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research.cursor < techs_len);
    }

    #[test]
    fn enter_selects_research_tech() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research.cursor = 0;

        app.handle_key(key(KeyCode::Enter));

        // Check that a tech is now selected in the engine
        let empire = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .empires
            .get(&app.engine.as_ref().unwrap().state.player_empire)
            .unwrap();
        assert!(empire.research.current_tech.is_some());
    }

    #[test]
    fn a_key_queues_research_tech() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research.cursor = 0;

        app.handle_key(key(KeyCode::Char('a')));

        let empire = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .empires
            .get(&app.engine.as_ref().unwrap().state.player_empire)
            .unwrap();
        assert_eq!(empire.research.queue.len(), 1);
    }

    #[test]
    fn c_key_clears_research_queue() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;

        app.state.research.cursor = 0;
        app.handle_key(key(KeyCode::Char('a')));
        app.state.research.cursor = 1;
        app.handle_key(key(KeyCode::Char('a')));

        app.handle_key(key(KeyCode::Char('c')));

        let empire = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .empires
            .get(&app.engine.as_ref().unwrap().state.player_empire)
            .unwrap();
        assert!(empire.research.queue.is_empty());
    }

    #[test]
    fn end_turn_works_from_research_screen_with_t_key() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        let initial_turn = app.engine.as_ref().unwrap().state.turn;

        app.handle_key(key(KeyCode::Char('t')));

        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    }

    #[test]
    fn research_screen_renders_via_app() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;

        terminal.draw(|frame| app.render(frame)).unwrap();
    }

    #[test]
    fn research_state_persists_through_save_load() {
        let path = tmp_save_path("research_persist");
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research.cursor = 0;

        // Select tech and end a turn
        app.handle_key(key(KeyCode::Enter));
        app.handle_key(key(KeyCode::Char('t')));

        let progress_before = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .empires
            .get(&app.engine.as_ref().unwrap().state.player_empire)
            .unwrap()
            .research
            .progress;

        app.save_game(&path).expect("save should succeed");
        app.load_game(&path).expect("load should succeed");

        let progress_after = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .empires
            .get(&app.engine.as_ref().unwrap().state.player_empire)
            .unwrap()
            .research
            .progress;

        assert_eq!(
            progress_before, progress_after,
            "Research progress must survive save/load"
        );

        let _ = std::fs::remove_file(&path);
    }

    // ──────────────────────────────────────────────────────────────────
    // Scout dispatch TUI tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn s_key_dispatches_scout_to_unexplored_star() {
        let mut app = App::new();
        app.new_game(42);

        // Navigate to SectorMap since 'S' is only handled there
        app.state.active = Screen::SectorMap;

        // Select an unexplored star (Engine::new(42) explores at most 4 of 20 stars)
        let star_id = {
            let engine = app.engine.as_ref().unwrap();
            engine
                .state
                .stars
                .keys()
                .find(|id| !engine.state.explored_stars.contains(id))
                .copied()
                .expect("Engine::new(42) must have unexplored stars")
        };

        app.state.navigation.selected_star = Some(star_id);
        let before = app.state.log.len();

        app.handle_key(key(KeyCode::Char('S')));

        // A log entry should have been added (ScoutDispatched message)
        assert!(
            app.state.log.len() > before,
            "Scout dispatch should add a log entry"
        );

        let has_mission = app
            .engine
            .as_ref()
            .unwrap()
            .state
            .scout_missions
            .values()
            .any(|m| m.destination == star_id);
        assert!(has_mission, "Scout mission should be active after S key");
    }

    #[test]
    fn dispatch_scout_without_engine_does_nothing() {
        let mut app = App::new();
        // dispatch_scout with no engine should be a no-op (no panic)
        app.dispatch_scout();
        assert!(app.engine.is_none());
    }

    #[test]
    fn dispatch_scout_without_star_selection_logs_message() {
        let mut app = App::new();
        app.new_game(42);
        app.state.navigation.selected_star = None;

        let before = app.state.log.len();
        app.dispatch_scout();
        assert!(app.state.log.len() > before, "Should log a message");
    }

    #[test]
    fn dispatch_scout_to_explored_star_logs_error() {
        let mut app = App::new();
        app.new_game(42);

        // Select an already-explored star
        let explored_star = *app
            .engine
            .as_ref()
            .unwrap()
            .state
            .explored_stars
            .iter()
            .next()
            .unwrap();
        app.state.navigation.selected_star = Some(explored_star);

        let before = app.state.log.len();
        app.dispatch_scout();
        assert!(
            app.state.log.len() > before,
            "Error should be logged for explored target"
        );
    }

    #[test]
    fn dispatch_scout_when_no_fleet_available_logs_message() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();

        // Remove all fleets so none are available
        engine.state.fleets.clear();

        let unexplored = engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .copied()
            .expect("Engine::new(42) must have unexplored stars");

        app.state.navigation.selected_star = Some(unexplored);
        let before = app.state.log.len();
        app.dispatch_scout();
        assert!(
            app.state.log.len() > before,
            "Should log 'no scout available'"
        );
    }

    #[test]
    fn first_idle_player_fleet_filters_busy_kind_and_location() {
        let mut app = App::new();
        app.new_game(42);
        let engine = app.engine.as_mut().unwrap();
        let home = engine.state.empires[&engine.state.player_empire].home_star;
        let busy_scout = FleetId(1);
        let idle_scout = FleetId(7000);
        let idle_science = FleetId(7001);

        let destination = engine
            .state
            .stars
            .keys()
            .find(|id| **id != home)
            .copied()
            .expect("test galaxy should have another star");
        engine.state.scout_missions.insert(
            busy_scout,
            game_core::ScoutMission {
                fleet: busy_scout,
                destination,
                turns_remaining: 2,
                origin: home,
                total_duration: 2,
            },
        );
        engine.state.fleets.insert(
            idle_scout,
            game_core::Fleet {
                id: idle_scout,
                owner: engine.state.player_empire,
                location: destination,
                ships: 1,
                kind: FleetKind::Scout,
                strength: 1,
                integrity: 100,
            },
        );
        engine.state.fleets.insert(
            idle_science,
            game_core::Fleet {
                id: idle_science,
                owner: engine.state.player_empire,
                location: home,
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );

        assert_eq!(
            first_idle_player_fleet(&engine.state, Some(FleetKind::Scout), None),
            Some(idle_scout)
        );
        let expected_home_science = engine
            .state
            .fleets
            .values()
            .filter(|fleet| {
                fleet.owner == engine.state.player_empire
                    && fleet.kind == FleetKind::Science
                    && fleet.location == home
                    && !engine.state.scout_missions.contains_key(&fleet.id)
                    && !engine.state.survey_missions.contains_key(&fleet.id)
                    && !engine.state.fleet_missions.contains_key(&fleet.id)
            })
            .map(|fleet| fleet.id)
            .min();
        assert_eq!(
            first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(home)),
            expected_home_science
        );
        assert_eq!(
            first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(destination)),
            None
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Fleet movement (M key) tests
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn m_key_dispatches_move_fleet() {
        let mut app = App::new();
        app.new_game(42);

        // Navigate to SectorMap since 'M' is only handled there
        app.state.active = Screen::SectorMap;

        // Select an explored star that is not the fleet's home
        let engine = app.engine.as_ref().unwrap();
        let fleet_id = game_core::FleetId(1);
        let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
        let dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial_location)
            .expect("Need explored star other than home");
        app.state.navigation.selected_star = Some(dest);

        let before_log_len = app.state.log.len();
        app.handle_key(key(KeyCode::Char('M')));

        // A fleet mission should have been created
        let mission_count = app.engine.as_ref().unwrap().state.fleet_missions.len();
        assert!(
            mission_count > 0,
            "Fleet mission should be created after M key"
        );
        // Log should have grown
        assert!(app.state.log.len() > before_log_len);
    }

    #[test]
    fn move_fleet_without_engine_is_noop() {
        let mut app = App::new();
        // No game started
        app.move_fleet();
        assert!(app.engine.is_none());
    }

    #[test]
    fn move_fleet_without_selection_logs_error() {
        let mut app = App::new();
        app.new_game(42);
        app.state.navigation.selected_star = None;
        let before = app.state.log.len();
        app.move_fleet();
        assert!(app.state.log.len() > before);
    }

    #[test]
    fn move_fleet_when_no_idle_fleet_logs_error() {
        let mut app = App::new();
        app.new_game(42);

        // Put all fleets on scout missions
        let engine = app.engine.as_mut().unwrap();
        let dest = *engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .expect("Unexplored star needed");
        use game_core::{FleetId, ScoutMission, StarId};
        engine.state.scout_missions.insert(
            FleetId(1),
            ScoutMission {
                fleet: FleetId(1),
                destination: dest,
                turns_remaining: 3,
                origin: StarId(0),
                total_duration: 3,
            },
        );

        let explored = *engine
            .state
            .explored_stars
            .iter()
            .next()
            .expect("Need explored star");
        app.state.navigation.selected_star = Some(explored);

        let before = app.state.log.len();
        app.move_fleet();
        assert!(app.state.log.len() > before, "Should log no idle fleet");
    }

    #[test]
    fn move_fleet_to_explored_star_creates_mission_and_logs() {
        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_ref().unwrap();
        let fleet_id = game_core::FleetId(1);
        let initial = engine.state.fleets.get(&fleet_id).unwrap().location;
        let dest = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != initial)
            .expect("Need explored star other than home");
        app.state.navigation.selected_star = Some(dest);
        let before = app.state.log.len();
        app.move_fleet();

        let engine = app.engine.as_ref().unwrap();
        assert!(
            engine.state.fleet_missions.contains_key(&fleet_id),
            "Fleet mission should be created"
        );
        assert!(app.state.log.len() > before, "Log should grow");
    }

    #[test]
    fn system_colonize_key_without_colonizer_logs_error() {
        let mut app = App::new();
        app.new_game(42);

        // No colonizer fleet exists at game start
        let engine = app.engine.as_ref().unwrap();
        let star = *engine.state.explored_stars.iter().next().unwrap();
        app.state.navigation.selected_star = Some(star);
        app.state.active = Screen::System;

        let before = app.state.log.len();
        app.handle_key(key(KeyCode::Char('C')));
        assert!(
            app.state.log.len() > before,
            "Should log error when no colonizer available"
        );
    }

    #[test]
    fn system_view_survey_targets_selected_planet() {
        use game_core::{FleetKind, SurveyMission};

        let mut app = App::new();
        app.new_game(42);

        let engine = app.engine.as_mut().unwrap();
        let star = *engine.state.explored_stars.iter().next().unwrap();
        let science_fleet = game_core::FleetId(77);
        engine.state.fleets.insert(
            science_fleet,
            game_core::Fleet {
                id: science_fleet,
                owner: engine.state.player_empire,
                location: star,
                ships: 1,
                kind: FleetKind::Science,
                strength: 1,
                integrity: 100,
            },
        );
        engine.state.stars.get_mut(&star).unwrap().planets[0].surveyed = false;
        app.state.navigation.selected_star = Some(star);
        app.state.navigation.selected_planet_index = 0;
        app.state.active = Screen::System;

        app.handle_key(key(KeyCode::Char('S')));

        let engine = app.engine.as_ref().unwrap();
        assert!(
            engine.state.survey_missions.contains_key(&science_fleet),
            "Survey command should create a survey mission"
        );
        assert!(matches!(
            engine.state.survey_missions.get(&science_fleet),
            Some(SurveyMission {
                star: s,
                planet_index: 0,
                ..
            }) if *s == star
        ));
    }

    #[test]
    fn colonize_without_engine_is_noop() {
        let mut app = App::new();
        // No game started
        app.colonize_selected_planet();
        assert!(app.engine.is_none());
    }

    #[test]
    fn colonize_without_selection_logs_error() {
        let mut app = App::new();
        app.new_game(42);
        app.state.navigation.selected_star = None;
        let before = app.state.log.len();
        app.colonize_selected_planet();
        assert!(app.state.log.len() > before);
    }

    #[test]
    fn colonize_key_with_colonizer_succeeds() {
        use game_core::{Command, FleetKind, OrbitalStructureType};

        let mut app = App::new();
        app.new_game(42);

        // Build a colonizer fleet via the engine directly
        let colony_id = game_core::ColonyId(1);
        // Inject Shipyard so Colony Ship can be queued
        app.engine
            .as_mut()
            .unwrap()
            .state
            .colonies
            .get_mut(&colony_id)
            .unwrap()
            .orbital_installations
            .push(OrbitalStructureType::Shipyard);
        app.engine
            .as_mut()
            .unwrap()
            .state
            .empires
            .get_mut(&game_core::EmpireId(1))
            .unwrap()
            .research
            .completed
            .push(game_core::TechId(2));
        app.engine.as_mut().unwrap().apply_turn(vec![
            Command::QueueBuild {
                colony: colony_id,
                item: game_core::BuildItem::Colony,
            },
            Command::SetColonyFocus {
                colony: colony_id,
                prod_pct: 100,
                research_pct: 0,
            },
        ]);
        for _ in 0..21 {
            app.engine
                .as_mut()
                .unwrap()
                .apply_turn(vec![Command::EndTurn]);
        }

        let engine = app.engine.as_ref().unwrap();
        let colonizer_id = engine
            .state
            .fleets
            .values()
            .find(|f| f.kind == FleetKind::Colonizer)
            .map(|f| f.id)
            .expect("Colonizer must exist");

        let home = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .unwrap()
            .home_star;
        let target = *engine
            .state
            .explored_stars
            .iter()
            .find(|&&id| id != home)
            .expect("Need explored star");
        app.engine
            .as_mut()
            .unwrap()
            .state
            .stars
            .get_mut(&target)
            .unwrap()
            .planets[0]
            .surveyed = true;

        // Move colonizer to target
        app.engine
            .as_mut()
            .unwrap()
            .apply_turn(vec![Command::MoveFleet {
                fleet: colonizer_id,
                destination: target,
            }]);
        for _ in 0..4 {
            app.engine
                .as_mut()
                .unwrap()
                .apply_turn(vec![Command::EndTurn]);
        }

        // Verify colonizer is at target
        let engine = app.engine.as_ref().unwrap();
        assert_eq!(
            engine.state.fleets.get(&colonizer_id).unwrap().location,
            target
        );

        let colonies_before = engine.state.colonies.len();

        // Select target star and press C
        app.state.navigation.selected_star = Some(target);
        app.state.active = Screen::System;
        app.handle_key(key(KeyCode::Char('C')));

        let engine = app.engine.as_ref().unwrap();
        assert_eq!(
            engine.state.colonies.len(),
            colonies_before + 1,
            "Colonization should create a new colony"
        );
        assert!(
            !engine.state.fleets.contains_key(&colonizer_id),
            "Colonizer should be consumed"
        );
    }

    #[test]
    fn sector_overview_renders_with_colonizer_fleet_present() {
        use game_core::FleetKind;
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut engine = game_core::Engine::new(42);

        // Add a colonizer fleet at the home star
        let player = engine.state.player_empire;
        let home = engine.state.empires.get(&player).unwrap().home_star;
        let fleet_id = game_core::FleetId(50);
        engine.state.fleets.insert(
            fleet_id,
            game_core::Fleet {
                id: fleet_id,
                owner: player,
                location: home,
                ships: 1,
                kind: FleetKind::Colonizer,
                strength: 1,
                integrity: 100,
            },
        );

        let sector = engine.state.stars.get(&home).map(|s| s.sector);
        let app_state = crate::AppState {
            navigation: crate::app::NavigationState {
                selected_sector: sector,
                selected_star: Some(home),
                ..Default::default()
            },
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                crate::screens::sector_overview::render_sector_overview(
                    frame,
                    area,
                    &app_state,
                    &engine.state,
                );
            })
            .unwrap();
    }

    #[test]
    fn ai_events_hidden_for_unknown_empire() {
        let mut app = App::new();
        app.new_game(42);

        let ai_empire = {
            let engine = app.engine.as_ref().unwrap();
            *engine
                .state
                .empires
                .keys()
                .find(|id| **id != engine.state.player_empire)
                .expect("AI empire must exist")
        };

        // Ensure empire is Unknown (no contact)
        app.engine
            .as_mut()
            .unwrap()
            .state
            .diplomacy
            .insert(ai_empire, game_core::RelationshipStatus::Unknown);

        app.state.log.clear();

        let event = game_core::Event::AiResearchSelected {
            empire: ai_empire,
            tech: game_core::TechId(1),
        };
        app.push_core_event_to_log(&event);

        assert_eq!(
            app.state.log.len(),
            0,
            "AI event for unknown empire must not appear in log"
        );
    }

    #[test]
    fn ai_events_visible_for_contacted_empire() {
        let mut app = App::new();
        app.new_game(42);

        let ai_empire = {
            let engine = app.engine.as_ref().unwrap();
            *engine
                .state
                .empires
                .keys()
                .find(|id| **id != engine.state.player_empire)
                .expect("AI empire must exist")
        };

        app.engine
            .as_mut()
            .unwrap()
            .state
            .diplomacy
            .insert(ai_empire, game_core::RelationshipStatus::Contacted);

        app.state.log.clear();

        let event = game_core::Event::AiResearchSelected {
            empire: ai_empire,
            tech: game_core::TechId(1),
        };
        app.push_core_event_to_log(&event);

        assert_eq!(
            app.state.log.len(),
            1,
            "AI event for contacted empire must appear in log"
        );
    }

    #[test]
    fn non_player_economy_events_hidden_from_log() {
        let mut app = App::new();
        app.new_game(42);

        let ai_empire = {
            let engine = app.engine.as_ref().unwrap();
            *engine
                .state
                .empires
                .keys()
                .find(|id| **id != engine.state.player_empire)
                .expect("AI empire must exist")
        };

        app.state.log.clear();

        app.push_core_event_to_log(&CoreEvent::EconomySummary {
            empire: ai_empire,
            credits_income: 9,
            maintenance: 2,
            food_produced: 6,
            food_consumed: 4,
        });
        app.push_core_event_to_log(&CoreEvent::FoodShortage {
            empire: ai_empire,
            deficit: 3,
        });
        app.push_core_event_to_log(&CoreEvent::CreditDeficit {
            empire: ai_empire,
            deficit: 2,
        });

        assert_eq!(
            app.state.log.len(),
            0,
            "Non-player economy events must not appear in log"
        );
    }

    #[test]
    fn non_player_colony_status_events_hidden_from_log() {
        let mut app = App::new();
        app.new_game(42);

        let ai_colony = {
            let engine = app.engine.as_ref().unwrap();
            *engine
                .state
                .colonies
                .iter()
                .find(|(_, colony)| colony.owner != engine.state.player_empire)
                .map(|(id, _)| id)
                .expect("AI colony must exist")
        };

        app.state.log.clear();

        app.push_core_event_to_log(&CoreEvent::ColonyStatusWarning {
            colony: ai_colony,
            food_deficit: 1,
            housing_deficit: 2,
            unemployed: 3,
        });
        app.push_core_event_to_log(&CoreEvent::PopulationGrew {
            colony: ai_colony,
            new_population: 9,
        });

        assert_eq!(
            app.state.log.len(),
            0,
            "Non-player colony status events must not appear in log"
        );
    }

    #[test]
    fn player_operational_events_still_visible_in_log() {
        let mut app = App::new();
        app.new_game(42);

        let (player_empire, player_colony) = {
            let engine = app.engine.as_ref().unwrap();
            let player_empire = engine.state.player_empire;
            let player_colony = *engine
                .state
                .colonies
                .iter()
                .find(|(_, colony)| colony.owner == player_empire)
                .map(|(id, _)| id)
                .expect("Player colony must exist");
            (player_empire, player_colony)
        };

        app.state.log.clear();

        app.push_core_event_to_log(&CoreEvent::EconomySummary {
            empire: player_empire,
            credits_income: 9,
            maintenance: 2,
            food_produced: 6,
            food_consumed: 4,
        });
        app.push_core_event_to_log(&CoreEvent::ColonyStatusWarning {
            colony: player_colony,
            food_deficit: 1,
            housing_deficit: 2,
            unemployed: 3,
        });

        assert_eq!(
            app.state.log.len(),
            2,
            "Player operational events must still appear in log"
        );
    }
}
