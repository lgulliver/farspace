//! FARSPACE - A deterministic, turn-based 4X space strategy game
//!
//! This is the main binary entrypoint.

mod update;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use game_tui::App;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, panic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TerminalRestoreStep {
    ShowCursor,
    LeaveAlternateScreen,
    DisableRawMode,
}

#[derive(Debug, Default)]
struct TerminalRestoreState {
    raw_mode_enabled: bool,
    alternate_screen_enabled: bool,
    cursor_hidden: bool,
}

impl TerminalRestoreState {
    fn restore_steps(&self) -> Vec<TerminalRestoreStep> {
        let mut steps = Vec::new();
        if self.cursor_hidden {
            steps.push(TerminalRestoreStep::ShowCursor);
        }
        if self.alternate_screen_enabled {
            steps.push(TerminalRestoreStep::LeaveAlternateScreen);
        }
        if self.raw_mode_enabled {
            steps.push(TerminalRestoreStep::DisableRawMode);
        }
        steps
    }

    fn mark_restored(&mut self, step: TerminalRestoreStep) {
        match step {
            TerminalRestoreStep::ShowCursor => self.cursor_hidden = false,
            TerminalRestoreStep::LeaveAlternateScreen => self.alternate_screen_enabled = false,
            TerminalRestoreStep::DisableRawMode => self.raw_mode_enabled = false,
        }
    }
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restore_state: TerminalRestoreState,
}

impl TerminalSession {
    fn enter() -> io::Result<Self> {
        let mut restore_state = TerminalRestoreState::default();
        enable_raw_mode()?;
        restore_state.raw_mode_enabled = true;

        let mut stdout = io::stdout();
        if let Err(err) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        restore_state.alternate_screen_enabled = true;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(err) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                return Err(err);
            }
        };
        if let Err(err) = terminal.hide_cursor() {
            let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(err);
        }
        restore_state.cursor_hidden = true;

        Ok(Self {
            terminal,
            restore_state,
        })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut first_error = None;
        for step in self.restore_state.restore_steps() {
            let result = match step {
                TerminalRestoreStep::ShowCursor => self.terminal.show_cursor(),
                TerminalRestoreStep::LeaveAlternateScreen => {
                    execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
                }
                TerminalRestoreStep::DisableRawMode => disable_raw_mode(),
            };

            match result {
                Ok(()) => self.restore_state.mark_restored(step),
                Err(err) if first_error.is_none() => first_error = Some(err),
                Err(_) => {}
            }
        }

        if let Some(err) = first_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn main() -> Result<()> {
    // Apply any staged update before the TUI starts.
    update::check_and_apply_staged();

    let mut terminal = TerminalSession::enter()?;
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let mut app = App::new();
        let (check_rx, download_tx, download_rx) =
            update::setup_update_system(app.update_channel());
        app.set_update_channels(check_rx, download_tx, download_rx);
        app.run(terminal.terminal_mut())
    }));

    let restore_result = terminal.restore();

    match result {
        Ok(Ok(true)) => {
            // User confirmed "apply update and restart".
            restore_result?;
            update::check_and_apply_staged();
            restart_process();
        }
        Ok(Ok(false)) => restore_result?,
        Ok(Err(e)) => {
            restore_result?;
            eprintln!("Application error: {}", e);
            std::process::exit(1);
        }
        Err(payload) => {
            let _ = restore_result;
            panic::resume_unwind(payload);
        }
    }

    Ok(())
}

/// Re-execute the current binary in place (replacing the current process on Unix,
/// spawning a new process and exiting on Windows).
fn restart_process() -> ! {
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("farspace"));

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).exec();
        eprintln!("Failed to restart: {err}");
        std::process::exit(1);
    }

    #[cfg(windows)]
    {
        let _ = std::process::Command::new(&exe).spawn();
        std::process::exit(0);
    }

    #[cfg(not(any(unix, windows)))]
    {
        eprintln!("Auto-restart not supported on this platform. Please relaunch manually.");
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_restore_steps_are_ordered_for_terminal_cleanup() {
        let mut state = TerminalRestoreState {
            raw_mode_enabled: true,
            alternate_screen_enabled: true,
            cursor_hidden: true,
        };

        assert_eq!(
            state.restore_steps(),
            vec![
                TerminalRestoreStep::ShowCursor,
                TerminalRestoreStep::LeaveAlternateScreen,
                TerminalRestoreStep::DisableRawMode,
            ]
        );
        state.mark_restored(TerminalRestoreStep::ShowCursor);
        state.mark_restored(TerminalRestoreStep::LeaveAlternateScreen);
        state.mark_restored(TerminalRestoreStep::DisableRawMode);
        assert!(state.restore_steps().is_empty());
    }

    #[test]
    fn terminal_restore_steps_skip_inactive_modes() {
        let state = TerminalRestoreState {
            raw_mode_enabled: true,
            alternate_screen_enabled: false,
            cursor_hidden: false,
        };

        assert_eq!(
            state.restore_steps(),
            vec![TerminalRestoreStep::DisableRawMode]
        );
    }
}
