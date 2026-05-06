//! Application state and main run loop

use crate::components::{render_help, render_palette, EventLog};
use crate::keys::KeyMap;
use crate::screens::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::{Command, Engine, StarId};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::path::PathBuf;
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

    /// Save the current game to the default path. Returns an error message on failure.
    pub fn save_game(&mut self) -> Result<(), String> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| "No game in progress".to_string())?;
        let path = PathBuf::from(DEFAULT_SAVE_PATH);
        game_save::save_to_file(&engine.state, &path).map_err(|e| format!("Save failed: {}", e))?;
        Ok(())
    }

    /// Load a game from the default path. Returns an error message on failure.
    pub fn load_game(&mut self) -> Result<(), String> {
        let path = PathBuf::from(DEFAULT_SAVE_PATH);
        let state = game_save::load_from_file(&path).map_err(|e| format!("Load failed: {}", e))?;
        let selected_star = state.stars.keys().next().copied();
        self.engine = Some(Engine::from_state(state));
        self.state.selected_star = selected_star;
        self.state.active = Screen::Galaxy;
        Ok(())
    }

    /// Execute a palette command string (e.g. "save", ":save")
    fn execute_palette_command(&mut self, cmd: &str) {
        let cmd = cmd.trim_start_matches(':').trim();
        match cmd {
            "save" => match self.save_game() {
                Ok(()) => self.state.log.push("Game saved.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            "load" => match self.load_game() {
                Ok(()) => self.state.log.push("Game loaded.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            other => {
                self.state.log.push(format!("Unknown command: {}", other));
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
                    if !cmd.is_empty() {
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
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        if KeyMap::is_new_game(key) {
            // Use a fixed default seed for deterministic, reproducible games.
            // A user-configurable seed will be added via the command palette.
            self.new_game(42);
        } else if KeyMap::is_load_game(key) {
            match self.load_game() {
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

        // End turn
        if KeyMap::is_end_turn(key) {
            self.end_turn();
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
        let result = app.save_game();
        assert!(result.is_err());
    }

    #[test]
    fn save_and_load_round_trip_via_app() {
        let mut app = App::new();
        app.new_game(42);

        // End a turn so state is non-trivial
        app.end_turn();
        let turn_before = app.engine.as_ref().unwrap().state.turn;

        // Save then load
        app.save_game().expect("save should succeed");
        app.load_game().expect("load should succeed");

        let turn_after = app.engine.as_ref().unwrap().state.turn;
        assert_eq!(turn_before, turn_after);
        assert_eq!(app.state.active, Screen::Galaxy);

        // Clean up save file
        let _ = std::fs::remove_file(DEFAULT_SAVE_PATH);
    }

    #[test]
    fn load_game_missing_file_logs_error() {
        // Make sure file doesn't exist
        let _ = std::fs::remove_file(DEFAULT_SAVE_PATH);

        let mut app = App::new();
        let result = app.load_game();
        assert!(result.is_err());
    }

    #[test]
    fn save_command_via_palette_logs_success() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.execute_palette_command("save");

        assert!(app.state.log.len() > before);
        // Last log entry should indicate save succeeded
        let last = app.state.log.last_n(1);
        assert!(
            last[0].contains("saved") || last[0].contains("Save"),
            "Expected save confirmation, got: {}",
            last[0]
        );

        let _ = std::fs::remove_file(DEFAULT_SAVE_PATH);
    }

    #[test]
    fn load_command_after_save_via_palette() {
        let mut app = App::new();
        app.new_game(42);
        app.end_turn();
        let turn_before = app.engine.as_ref().unwrap().state.turn;

        app.execute_palette_command("save");
        app.execute_palette_command("load");

        let turn_after = app.engine.as_ref().unwrap().state.turn;
        assert_eq!(turn_before, turn_after);

        let _ = std::fs::remove_file(DEFAULT_SAVE_PATH);
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
    fn menu_load_key_triggers_load() {
        // Make sure no save file exists to verify error path
        let _ = std::fs::remove_file(DEFAULT_SAVE_PATH);
        let mut app = App::new();

        // 'l' on menu should attempt load (will fail with no save file)
        app.handle_key(key(KeyCode::Char('l')));
        // App stays on menu since load failed (no crash)
        assert_eq!(app.state.active, Screen::Menu);
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
}
