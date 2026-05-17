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

pub fn user_config_path() -> Option<PathBuf> {
    if let Some(base) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(base).join("farspace").join("ui.conf"));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config").join("farspace").join("ui.conf"))
}

pub fn map_char_for_mode(mode: VisualMode, ch: char) -> char {
    match mode {
        VisualMode::Ascii => ascii_fallback(ch),
        VisualMode::Unicode => unicode_fallback(ch),
        VisualMode::NerdFont => ch,
    }
}

pub fn map_symbol_for_mode(mode: VisualMode, symbol: &str) -> String {
    symbol.chars().map(|ch| map_char_for_mode(mode, ch)).collect()
}

fn unicode_fallback(ch: char) -> char {
    if ('\u{e000}'..='\u{f8ff}').contains(&ch) {
        '?'
    } else {
        ch
    }
}

fn ascii_fallback(ch: char) -> char {
    match ch {
        '│' | '┃' | '║' | '┆' | '┊' | '╎' => '|',
        '─' | '━' | '═' | '┄' | '┈' | '╌' => '-',
        '┌' | '┐' | '└' | '┘' | '╔' | '╗' | '╚' | '╝' | '├' | '┤' | '┬' | '┴' | '┼' | '╠'
        | '╣' | '╦' | '╩' | '╬' | '╱' | '╲' => '+',
        '◌' | '○' | '◉' | '◍' | '◐' | '◒' | '◎' | '●' | '◈' | '▪' | '▦' => 'o',
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
        assert_eq!(
            VisualMode::Ascii.next().next().next(),
            VisualMode::Ascii
        );
    }

    #[test]
    fn ascii_mode_falls_back_unicode_glyphs() {
        assert_eq!(map_char_for_mode(VisualMode::Ascii, '►'), '>');
        assert_eq!(map_char_for_mode(VisualMode::Ascii, '⚠'), '!');
    }

    #[test]
    fn unicode_mode_rejects_private_use_glyphs() {
        assert_eq!(map_char_for_mode(VisualMode::Unicode, '\u{e0b0}'), '?');
    }
}
