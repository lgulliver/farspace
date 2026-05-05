//! Key bindings

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Key binding configuration
pub struct KeyMap;

impl KeyMap {
    /// Check if key is quit
    pub fn is_quit(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('Q'),
                modifiers: KeyModifiers::SHIFT,
                ..
            } | KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }
        )
    }

    /// Check if key toggles help
    pub fn is_help(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('?'),
                ..
            }
        )
    }

    /// Check if key toggles command palette
    pub fn is_palette(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char(':'),
                ..
            }
        )
    }

    /// Check if key ends turn
    pub fn is_end_turn(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('t'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('T'),
                modifiers: KeyModifiers::SHIFT,
                ..
            }
        )
    }

    /// Check if key toggles search
    pub fn is_search(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('/'),
                ..
            }
        )
    }

    /// Check for movement keys, returns (dx, dy)
    pub fn movement(key: KeyEvent) -> Option<(i32, i32)> {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => Some((-1, 0)),
            KeyCode::Char('j') | KeyCode::Down => Some((0, 1)),
            KeyCode::Char('k') | KeyCode::Up => Some((0, -1)),
            KeyCode::Char('l') | KeyCode::Right => Some((1, 0)),
            _ => None,
        }
    }

    /// Check for new game key
    pub fn is_new_game(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('N'),
                modifiers: KeyModifiers::SHIFT,
                ..
            }
        )
    }

    /// Check for load game key
    pub fn is_load_game(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            } | KeyEvent {
                code: KeyCode::Char('L'),
                modifiers: KeyModifiers::SHIFT,
                ..
            }
        )
    }

    /// Check for escape key
    pub fn is_escape(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Esc,
                ..
            }
        )
    }

    /// Check for confirm key
    pub fn is_confirm(key: KeyEvent) -> bool {
        matches!(
            key,
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } | KeyEvent {
                code: KeyCode::Char('y'),
                ..
            } | KeyEvent {
                code: KeyCode::Char('Y'),
                ..
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_with_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        }
    }

    #[test]
    fn quit_keys() {
        assert!(KeyMap::is_quit(key(KeyCode::Char('q'))));
        assert!(KeyMap::is_quit(key_with_mod(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
        assert!(!KeyMap::is_quit(key(KeyCode::Char('x'))));
    }

    #[test]
    fn help_key() {
        assert!(KeyMap::is_help(key(KeyCode::Char('?'))));
        assert!(!KeyMap::is_help(key(KeyCode::Char('h'))));
    }

    #[test]
    fn movement_keys() {
        assert_eq!(KeyMap::movement(key(KeyCode::Char('h'))), Some((-1, 0)));
        assert_eq!(KeyMap::movement(key(KeyCode::Char('j'))), Some((0, 1)));
        assert_eq!(KeyMap::movement(key(KeyCode::Char('k'))), Some((0, -1)));
        assert_eq!(KeyMap::movement(key(KeyCode::Char('l'))), Some((1, 0)));
        assert_eq!(KeyMap::movement(key(KeyCode::Left)), Some((-1, 0)));
        assert_eq!(KeyMap::movement(key(KeyCode::Char('x'))), None);
    }

    #[test]
    fn end_turn_keys() {
        assert!(KeyMap::is_end_turn(key(KeyCode::Enter)));
        assert!(KeyMap::is_end_turn(key(KeyCode::Char('t'))));
    }
}
