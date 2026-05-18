use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualMode {
    Ascii,
    #[default]
    Unicode,
    NerdFont,
}

impl VisualMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ascii => "ASCII",
            Self::Unicode => "Unicode",
            Self::NerdFont => "NerdFont",
        }
    }

    pub const fn config_value(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Unicode => "unicode",
            Self::NerdFont => "nerdfont",
        }
    }

    pub fn from_config_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "ascii" => Some(Self::Ascii),
            "unicode" => Some(Self::Unicode),
            "nerdfont" | "nerd_font" | "nerd-font" => Some(Self::NerdFont),
            _ => None,
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Ascii => Self::Unicode,
            Self::Unicode => Self::NerdFont,
            Self::NerdFont => Self::Ascii,
        }
    }

    pub const fn preview_sample(self) -> &'static str {
        match self {
            Self::Ascii => "* . > o !",
            Self::Unicode => "✦ ◌ ► ◉ ⚠",
            Self::NerdFont => "\u{e0b0} \u{e0b1} \u{f0a9} \u{f111} \u{f071}",
        }
    }
}

/// Resolve FARSPACE UI config path.
///
/// Empty `XDG_CONFIG_HOME` / `HOME` env vars are treated as unset to avoid
/// producing relative paths like `farspace/ui.conf` in current working directory.
pub fn user_config_path() -> Option<PathBuf> {
    user_config_path_from_env(
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

fn user_config_path_from_env(
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(base) = xdg_config_home.filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(base).join("farspace").join("ui.conf"));
    }
    home.filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map(|dir| dir.join(".config").join("farspace").join("ui.conf"))
}

pub fn map_char_for_mode(mode: VisualMode, ch: char) -> char {
    match mode {
        VisualMode::Ascii => ascii_fallback(ch),
        VisualMode::Unicode => unicode_fallback(ch),
        VisualMode::NerdFont => ch,
    }
}

pub fn map_symbol_for_mode<'a>(mode: VisualMode, symbol: &'a str) -> Cow<'a, str> {
    if mode == VisualMode::NerdFont {
        return Cow::Borrowed(symbol);
    }

    let mut chars = symbol.char_indices();
    while let Some((idx, ch)) = chars.next() {
        let mapped = map_char_for_mode(mode, ch);
        if mapped != ch {
            let mut owned = String::with_capacity(symbol.len());
            owned.push_str(&symbol[..idx]);
            owned.push(mapped);
            for (_, rest) in chars {
                owned.push(map_char_for_mode(mode, rest));
            }
            return Cow::Owned(owned);
        }
    }
    Cow::Borrowed(symbol)
}

fn unicode_fallback(ch: char) -> char {
    // Filter all Unicode private-use ranges so Unicode mode never renders
    // NerdFont/private glyphs that are font-dependent.
    let is_private_use = ('\u{e000}'..='\u{f8ff}').contains(&ch)
        || ('\u{f0000}'..='\u{ffffd}').contains(&ch)
        || ('\u{100000}'..='\u{10fffd}').contains(&ch);
    if is_private_use {
        '?'
    } else {
        ch
    }
}

fn ascii_fallback(ch: char) -> char {
    match ch {
        '│' | '┃' | '║' | '┆' | '┊' | '╎' => '|',
        '─' | '━' | '═' | '┄' | '┈' | '╌' => '-',
        '┌' | '┐' | '└' | '┘' | '╔' | '╗' | '╚' | '╝' | '├' | '┤' | '┬' | '┴' | '┼' | '╠' | '╣'
        | '╦' | '╩' | '╬' | '╱' | '╲' => '+',
        '◌' | '○' | '◉' | '◍' | '◐' | '◒' | '◎' | '●' | '◈' | '▪' | '▦' => {
            'o'
        }
        '·' | '•' | '∙' => '.',
        '►' | '▶' | '▸' | '›' | '➤' | '⏵' => '>',
        '◄' | '◀' | '‹' | '◂' => '<',
        '▲' | '△' => '^',
        '▼' | '▽' => 'v',
        '✦' | '✶' | '✷' | '★' | '☆' | '☼' | '☉' => '*',
        '⚠' => '!',
        '✖' | '✗' | '✘' | '⚔' => 'x',
        '✓' | '✔' => 'v',
        '📊' => '#',
        '💾' => 's',
        '░' | '▒' | '▓' | '█' | '▮' | '▌' => '#',
        '→' => '>',
        '←' => '<',
        '↑' => '^',
        '↓' => 'v',
        _ if ch.is_ascii() => ch,
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_cycle_roundtrip() {
        assert_eq!(VisualMode::Ascii.next().next().next(), VisualMode::Ascii);
    }

    #[test]
    fn ascii_mode_falls_back_unicode_glyphs() {
        assert_eq!(map_char_for_mode(VisualMode::Ascii, '►'), '>');
        assert_eq!(map_char_for_mode(VisualMode::Ascii, '⚠'), '!');
    }

    #[test]
    fn unicode_mode_rejects_private_use_glyphs() {
        assert_eq!(map_char_for_mode(VisualMode::Unicode, '\u{e0b0}'), '?');
        assert_eq!(map_char_for_mode(VisualMode::Unicode, '\u{f0001}'), '?');
    }

    #[test]
    fn map_symbol_returns_borrowed_when_unchanged() {
        let mapped = map_symbol_for_mode(VisualMode::Unicode, "abc");
        assert!(matches!(mapped, Cow::Borrowed("abc")));
    }

    #[test]
    fn empty_env_vars_do_not_create_relative_config_path() {
        let path = user_config_path_from_env(Some(OsStr::new("")), Some(OsStr::new("")));
        assert!(path.is_none());
    }
}
