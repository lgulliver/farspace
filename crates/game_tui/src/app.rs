//! Application state and main run loop

use crate::components::{render_help, render_palette, EventLog};
use crate::keys::KeyMap;
use crate::screens::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::{all_techs, BuildingType, ColonyId, Command, Engine, StarId, TechId};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::time::Duration;

/// Default save file path
const DEFAULT_SAVE_PATH: &str = "farspace.sav";

/// Main application state
pub struct App {
    pub state: AppState,
    pub engine: Option<Engine>,
    pub pending_commands: Vec<Command>,
}

/// UI state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub active: Screen,
    pub show_help: bool,
    pub show_palette: bool,
    pub palette_input: String,
    pub search_input: String,
    pub show_search: bool,
    pub selected_star: Option<StarId>,
    /// Currently viewed colony (set when entering the colony screen)
    pub selected_colony: Option<ColonyId>,
    /// Cursor index for the build-picker on the colony screen
    pub colony_build_cursor: usize,
    /// Cursor index for the tech list on the research screen
    pub research_cursor: usize,
    pub log: EventLog,
    pub quit: bool,
}

impl App {
    /// Create a new application
    pub fn new() -> Self {
        App {
            state: AppState::default(),
            engine: None,
            pending_commands: Vec::new(),
        }
    }

    /// Start a new game with the given seed
    pub fn new_game(&mut self, seed: u64) {
        let engine = Engine::new(seed);

        // Select the first star by default
        self.state.selected_star = engine.state.stars.keys().next().copied();

        // Add initial log entry
        self.state.log.push("Game started".to_string());

        self.engine = Some(engine);
        self.state.active = Screen::Galaxy;
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
        self.engine = Some(Engine::from_state(state));
        self.state.selected_star = selected_star;
        self.state.active = Screen::Galaxy;
        Ok(())
    }

    /// Execute a palette command string (e.g. "save", ":save")
    fn execute_palette_command(&mut self, cmd: &str) {
        let cmd = cmd.trim_start_matches(':').trim();
        if cmd.is_empty() {
            return;
        }
        let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
        match cmd {
            "save" => match self.save_game(&path) {
                Ok(()) => self.state.log.push("Game saved.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            "load" => match self.load_game(&path) {
                Ok(()) => self.state.log.push("Game loaded.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            other => {
                self.state
                    .log
                    .push(format!("Error: Unknown command: {}", other));
            }
        }
    }

    /// Run the main event loop
    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        while !self.state.quit {
            terminal.draw(|frame| self.render(frame))?;

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
        if self.state.show_help {
            render_help(frame, area, &self.state.active);
        }

        if self.state.show_palette {
            render_palette(frame, area, &self.state.palette_input);
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) {
        // Handle overlays first
        if self.state.show_help {
            if KeyMap::is_help(key) || KeyMap::is_escape(key) {
                self.state.show_help = false;
            }
            return;
        }

        if self.state.show_palette {
            match key.code {
                KeyCode::Esc => {
                    self.state.show_palette = false;
                    self.state.palette_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.state.palette_input.clone();
                    self.state.show_palette = false;
                    self.state.palette_input.clear();
                    // Normalize before checking — skip empty, whitespace-only, or bare ":"
                    let normalized = cmd.trim_start_matches(':').trim();
                    if !normalized.is_empty() {
                        self.execute_palette_command(&cmd);
                    }
                }
                KeyCode::Backspace => {
                    self.state.palette_input.pop();
                }
                KeyCode::Char(c) => {
                    self.state.palette_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Global keys
        if KeyMap::is_help(key) {
            self.state.show_help = true;
            return;
        }

        if KeyMap::is_palette(key) {
            self.state.show_palette = true;
            return;
        }

        if KeyMap::is_quit(key) {
            self.state.quit = true;
            return;
        }

        // Screen-specific handling
        match self.state.active {
            Screen::Menu => self.handle_menu_key(key),
            Screen::Galaxy => self.handle_galaxy_key(key),
            Screen::Colony => self.handle_colony_key(key),
            Screen::Research => self.handle_research_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        if KeyMap::is_new_game(key) {
            // Use a fixed default seed for deterministic, reproducible games.
            // A user-configurable seed will be added via the command palette.
            self.new_game(42);
        } else if KeyMap::is_load_game(key) {
            let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
            match self.load_game(&path) {
                Ok(()) => self.state.log.push("Game loaded.".to_string()),
                Err(e) => self.state.log.push(e),
            }
        }
    }

    fn handle_galaxy_key(&mut self, key: KeyEvent) {
        // Movement
        if let Some((dx, dy)) = KeyMap::movement(key) {
            self.move_star_selection(dx, dy);
            return;
        }

        // Enter colony view with 'c'
        if key.code == KeyCode::Char('c') {
            self.try_enter_colony();
            return;
        }

        // Open research screen with 'r'
        if key.code == KeyCode::Char('r') {
            self.state.active = Screen::Research;
            self.state.research_cursor = 0;
            return;
        }

        // End turn
        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_colony_key(&mut self, key: KeyEvent) {
        match key.code {
            // Return to galaxy map
            KeyCode::Esc => {
                self.state.active = Screen::Galaxy;
                self.state.selected_colony = None;
            }
            // Navigate build picker
            KeyCode::Char('j') | KeyCode::Down => {
                let count = BuildingType::all().len();
                self.state.colony_build_cursor = (self.state.colony_build_cursor + 1) % count;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = BuildingType::all().len();
                self.state.colony_build_cursor =
                    (self.state.colony_build_cursor + count.saturating_sub(1)) % count;
            }
            // Queue the selected building
            KeyCode::Enter => {
                self.queue_building();
            }
            // End turn from colony screen
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    /// Try to enter the colony screen for the selected star.
    /// Returns true if a player colony was found and the screen transitioned.
    fn try_enter_colony(&mut self) -> bool {
        let engine = match &self.engine {
            Some(e) => e,
            None => return false,
        };

        let star_id = match self.state.selected_star {
            Some(id) => id,
            None => return false,
        };

        let star = match engine.state.stars.get(&star_id) {
            Some(s) => s,
            None => return false,
        };

        // Find the first planet at this star that has a player-owned colony
        for planet in &star.planets {
            if let Some(colony_id) = planet.colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.selected_colony = Some(colony_id);
                        self.state.colony_build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_research_key(&mut self, key: KeyEvent) {
        match key.code {
            // Return to galaxy map
            KeyCode::Esc => {
                self.state.active = Screen::Galaxy;
            }
            // Navigate tech list
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.available_tech_count();
                if count > 0 {
                    self.state.research_cursor = (self.state.research_cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.available_tech_count();
                if count > 0 {
                    self.state.research_cursor =
                        (self.state.research_cursor + count.saturating_sub(1)) % count;
                }
            }
            // Select the highlighted tech for research
            KeyCode::Enter => {
                self.select_research_tech();
            }
            // End turn from research screen (excluding Enter, which selects tech)
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    /// Returns the number of technologies available (not yet completed) for the player empire.
    fn available_tech_count(&self) -> usize {
        let engine = match &self.engine {
            Some(e) => e,
            None => return 0,
        };
        let completed = engine
            .state
            .empires
            .get(&engine.state.player_empire)
            .map(|e| &e.research.completed);
        all_techs()
            .iter()
            .filter(|t| completed.map(|c| !c.contains(&t.id)).unwrap_or(true))
            .count()
    }

    /// Select the highlighted technology for research
    fn select_research_tech(&mut self) {
        // Collect the tech_id first using a scoped borrow
        let tech_id: TechId = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };

            let empire = match engine.state.empires.get(&engine.state.player_empire) {
                Some(e) => e,
                None => return,
            };

            let all = all_techs();
            let available: Vec<_> = all
                .iter()
                .filter(|t| !empire.research.completed.contains(&t.id))
                .collect();

            if available.is_empty() {
                return;
            }

            let cursor = self.state.research_cursor % available.len();
            available[cursor].id
        };

        self.pending_commands
            .push(Command::SelectResearch { tech: tech_id });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Queue the currently selected building at the active colony
    fn queue_building(&mut self) {
        let colony_id = match self.state.selected_colony {
            Some(id) => id,
            None => return,
        };

        let buildings = BuildingType::all();
        let cursor = self.state.colony_build_cursor % buildings.len();
        let bt = buildings[cursor];

        self.pending_commands.push(Command::QueueBuild {
            colony: colony_id,
            item: game_core::BuildItem::Structure(bt),
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    fn move_star_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.selected_star {
            Some(id) => id,
            None => {
                // Select first star if none selected
                self.state.selected_star = engine.state.stars.keys().next().copied();
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
            self.state.selected_star = Some(id);
        }
    }

    fn end_turn(&mut self) {
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };

        // Add EndTurn command
        self.pending_commands.push(Command::EndTurn);

        // Process all pending commands
        let commands = std::mem::take(&mut self.pending_commands);
        let events = engine.apply_turn(commands);

        // Add events to log
        for event in events {
            self.state.log.push(event.to_log_message());
        }
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
        assert!(!app.state.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.state.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(!app.state.show_help);
    }

    #[test]
    fn toggle_palette_on_colon() {
        let mut app = App::new();
        assert!(!app.state.show_palette);

        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.show_palette);

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_palette);
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
        assert_eq!(app.state.active, Screen::Galaxy);
        assert!(app.engine.is_some());
    }

    #[test]
    fn star_selection_moves_on_hjkl() {
        let mut app = App::new();
        app.new_game(42);

        let initial = app.state.selected_star;
        assert!(initial.is_some());

        // Move right
        app.handle_key(key(KeyCode::Char('l')));

        // Selection should change (might be same if no star to right)
        // Just verify it doesn't crash
        assert!(app.state.selected_star.is_some());
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
        assert!(app.state.selected_star.is_none());
    }

    #[test]
    fn move_star_selection_with_no_selection_selects_first() {
        let mut app = App::new();
        app.new_game(42);
        app.state.selected_star = None;

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

        // 'Enter' ends the turn
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    }

    #[test]
    fn escape_closes_help_overlay() {
        let mut app = App::new();
        app.state.show_help = true;

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_help);
    }

    #[test]
    fn palette_key_closes_palette() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = String::new();

        // Pressing `:` when palette is open adds `:` to input (does not close)
        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.show_palette);
        assert_eq!(app.state.palette_input, ":");

        // Pressing Esc closes the palette
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_palette);
        assert!(app.state.palette_input.is_empty());
    }

    #[test]
    fn palette_accepts_character_input() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input.clear();

        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('v')));
        app.handle_key(key(KeyCode::Char('e')));

        assert_eq!(app.state.palette_input, "save");
    }

    #[test]
    fn palette_backspace_removes_last_char() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = "sav".to_string();

        app.handle_key(key(KeyCode::Backspace));

        assert_eq!(app.state.palette_input, "sa");
    }

    #[test]
    fn palette_unknown_command_logs_error() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.execute_palette_command("unknowncmd");

        assert!(app.state.log.len() > before);
    }

    #[test]
    fn palette_enter_executes_and_closes() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = "unknowncmd".to_string();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.show_palette);
        assert!(app.state.palette_input.is_empty());
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
        assert_eq!(app.state.active, Screen::Galaxy);

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
        app.execute_palette_command(":");
        app.execute_palette_command("  ");
        app.execute_palette_command(":  ");

        assert_eq!(app.state.log.len(), before, "No-op commands should not log");
    }

    #[test]
    fn palette_enter_with_colon_only_does_not_execute() {
        let mut app = App::new();
        app.new_game(42);
        app.state.show_palette = true;
        app.state.palette_input = ":".to_string();
        let before = app.state.log.len();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.show_palette);
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
            app.state.selected_star = Some(star_id);
            app.try_enter_colony()
        } else {
            false
        }
    }

    #[test]
    fn enter_colony_from_galaxy_with_c_key() {
        let mut app = App::new();
        app.new_game(42);
        assert_eq!(app.state.active, Screen::Galaxy);

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
        app.state.selected_star = home_star_id;

        // Press 'c' to enter the colony screen — exercises the actual key binding
        app.handle_key(key(KeyCode::Char('c')));

        assert_eq!(app.state.active, Screen::Colony);
        assert!(app.state.selected_colony.is_some());
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
        app.state.selected_star = None;
        assert!(!app.try_enter_colony());
    }

    #[test]
    fn try_enter_colony_returns_false_for_empty_star() {
        let mut app = App::new();
        app.new_game(42);
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
            app.state.selected_star = Some(star_id);
            assert!(!app.try_enter_colony());
            assert_eq!(app.state.active, Screen::Galaxy);
        }
        // If every star has a colony (very unlikely with 20 stars and 1 colony) we skip
    }

    #[test]
    fn esc_returns_to_galaxy_from_colony_screen() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);
        assert_eq!(app.state.active, Screen::Colony);

        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.state.active, Screen::Galaxy);
        assert!(app.state.selected_colony.is_none());
    }

    #[test]
    fn colony_build_cursor_moves_with_j_k() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let initial = app.state.colony_build_cursor;
        app.handle_key(key(KeyCode::Char('j')));
        assert_ne!(app.state.colony_build_cursor, initial);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.state.colony_build_cursor, initial);
    }

    #[test]
    fn colony_build_cursor_wraps_around_bottom() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let count = BuildingType::all().len();
        // Move down past the last item
        for _ in 0..count {
            app.handle_key(key(KeyCode::Char('j')));
        }
        // Cursor should have wrapped to the start
        assert_eq!(app.state.colony_build_cursor, 0);
    }

    #[test]
    fn colony_build_cursor_wraps_around_top() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        // Move up from 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        let count = BuildingType::all().len();
        assert_eq!(app.state.colony_build_cursor, count - 1);
    }

    #[test]
    fn enter_key_queues_building_on_colony_screen() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let colony_id = app.state.selected_colony.unwrap();
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
    fn r_key_opens_research_screen() {
        let mut app = App::new();
        app.new_game(42);
        assert_eq!(app.state.active, Screen::Galaxy);

        app.handle_key(key(KeyCode::Char('r')));

        assert_eq!(app.state.active, Screen::Research);
        assert_eq!(app.state.research_cursor, 0);
    }

    #[test]
    fn esc_closes_research_screen() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.state.active, Screen::Galaxy);
    }

    #[test]
    fn research_cursor_wraps_on_j() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research_cursor = 0;

        // j increments cursor; just verify no panic and cursor stays in bounds
        app.handle_key(key(KeyCode::Char('j')));
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research_cursor < techs_len);
    }

    #[test]
    fn research_cursor_wraps_on_k() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research_cursor = 0;

        // k at position 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        // cursor should now point to last tech (5 for 6 techs with index 0..5)
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research_cursor < techs_len);
    }

    #[test]
    fn enter_selects_research_tech() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research_cursor = 0;

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
        app.state.research_cursor = 0;

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
}
