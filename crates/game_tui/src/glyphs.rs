use crate::visual_mode::VisualMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphSet {
    pub star: char,
    pub star_unexplored: char,
    pub star_selected: char,
    pub fleet_route: char,
    pub scout_route: char,
    pub fleet_stationary_single: char,
    pub fleet_stationary_multi: char,
    pub lane: char,
    pub lane_highlight: char,
    pub transit: char,
    pub capital_marker: char,
    pub list_selected: char,
    pub planet_colonized: char,
    pub planet_uncolonized: char,
    pub blockade: char,
    pub orbit_selected: char,
    pub orbit: char,
    pub selector_left: char,
    pub selector_right: char,
    pub selector_up: char,
    pub selector_down: char,
    pub warning: char,
    pub special: char,
    pub anomaly: char,
    pub resource: char,
    pub bullet: char,
    pub separator_dot: char,
    pub horizontal_rule: char,
    pub arrow_right: char,
    pub palette_cursor: char,
    pub sector_selected: char,
    pub status_error: char,
    pub status_progress: char,
    pub status_done: char,
    pub status_save: char,
    pub severity_historic: char,
    pub severity_urgent: char,
}

pub const fn glyphs_for_mode(mode: VisualMode) -> GlyphSet {
    match mode {
        VisualMode::Ascii => GlyphSet {
            star: '*',
            star_unexplored: 'o',
            star_selected: '@',
            fleet_route: '~',
            scout_route: '+',
            fleet_stationary_single: '>',
            fleet_stationary_multi: '+',
            lane: '.',
            lane_highlight: '*',
            transit: '>',
            capital_marker: '^',
            list_selected: '>',
            planet_colonized: 'o',
            planet_uncolonized: 'o',
            blockade: 'x',
            orbit_selected: '*',
            orbit: '.',
            selector_left: '<',
            selector_right: '>',
            selector_up: '^',
            selector_down: 'v',
            warning: '!',
            special: '+',
            anomaly: '?',
            resource: '#',
            bullet: '*',
            separator_dot: '.',
            horizontal_rule: '-',
            arrow_right: '>',
            palette_cursor: '|',
            sector_selected: '@',
            status_error: 'x',
            status_progress: '>',
            status_done: 'v',
            status_save: 's',
            severity_historic: '*',
            severity_urgent: '!',
        },
        VisualMode::Unicode => GlyphSet {
            star: '★',
            star_unexplored: '◌',
            star_selected: '@',
            fleet_route: '~',
            scout_route: '+',
            fleet_stationary_single: '›',
            fleet_stationary_multi: '+',
            lane: '·',
            lane_highlight: '•',
            transit: '►',
            capital_marker: '^',
            list_selected: '▶',
            planet_colonized: '◉',
            planet_uncolonized: '○',
            blockade: '⚔',
            orbit_selected: '•',
            orbit: '·',
            selector_left: '⟨',
            selector_right: '⟩',
            selector_up: '⌃',
            selector_down: '⌄',
            warning: '⚠',
            special: '✦',
            anomaly: '◈',
            resource: '◆',
            bullet: '•',
            separator_dot: '·',
            horizontal_rule: '─',
            arrow_right: '→',
            palette_cursor: '▌',
            sector_selected: '◆',
            status_error: '✖',
            status_progress: '▸',
            status_done: '✓',
            status_save: '◈',
            severity_historic: '★',
            severity_urgent: '⚠',
        },
        VisualMode::NerdFont => GlyphSet {
            star: '\u{f005}',
            star_unexplored: '\u{f10c}',
            star_selected: '@',
            fleet_route: '~',
            scout_route: '+',
            fleet_stationary_single: '\u{f105}',
            fleet_stationary_multi: '\u{f0c8}',
            lane: '\u{e0b1}',
            lane_highlight: '\u{e0b0}',
            transit: '\u{f0a9}',
            capital_marker: '^',
            list_selected: '\u{f054}',
            planet_colonized: '\u{f015}',
            planet_uncolonized: '\u{f10c}',
            blockade: '\u{f00d}',
            orbit_selected: '\u{f111}',
            orbit: '\u{f10c}',
            selector_left: '\u{f104}',
            selector_right: '\u{f105}',
            selector_up: '\u{f106}',
            selector_down: '\u{f107}',
            warning: '\u{f071}',
            special: '\u{f005}',
            anomaly: '\u{f0c8}',
            resource: '\u{f0a0}',
            bullet: '\u{f111}',
            separator_dot: '\u{f111}',
            horizontal_rule: '─',
            arrow_right: '\u{f061}',
            palette_cursor: '\u{f0d7}',
            sector_selected: '\u{f0a4}',
            status_error: '\u{f057}',
            status_progress: '\u{e0b1}',
            status_done: '\u{f00c}',
            status_save: '\u{f0c7}',
            severity_historic: '\u{f005}',
            severity_urgent: '\u{f071}',
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_mode_is_ascii_safe() {
        let glyphs = glyphs_for_mode(VisualMode::Ascii);
        let chars = [
            glyphs.star,
            glyphs.star_unexplored,
            glyphs.transit,
            glyphs.warning,
            glyphs.special,
            glyphs.anomaly,
            glyphs.resource,
            glyphs.bullet,
            glyphs.separator_dot,
            glyphs.horizontal_rule,
            glyphs.palette_cursor,
            glyphs.status_error,
            glyphs.status_progress,
            glyphs.status_done,
            glyphs.status_save,
            glyphs.severity_historic,
            glyphs.severity_urgent,
        ];
        assert!(chars.iter().all(|ch| ch.is_ascii()));
    }

    #[test]
    fn unicode_mode_uses_non_private_unicode_symbols() {
        let glyphs = glyphs_for_mode(VisualMode::Unicode);
        let chars = [
            glyphs.star,
            glyphs.star_unexplored,
            glyphs.transit,
            glyphs.warning,
            glyphs.special,
            glyphs.anomaly,
            glyphs.resource,
            glyphs.separator_dot,
            glyphs.status_done,
            glyphs.severity_historic,
            glyphs.severity_urgent,
        ];
        assert!(chars.iter().all(|ch| {
            !('\u{e000}'..='\u{f8ff}').contains(ch)
                && !('\u{f0000}'..='\u{ffffd}').contains(ch)
                && !('\u{100000}'..='\u{10fffd}').contains(ch)
        }));
    }

    #[test]
    fn nerdfont_mode_uses_richer_private_use_icons() {
        let glyphs = glyphs_for_mode(VisualMode::NerdFont);
        assert!(('\u{e000}'..='\u{f8ff}').contains(&glyphs.transit));
        assert!(('\u{e000}'..='\u{f8ff}').contains(&glyphs.status_done));
        assert!(('\u{e000}'..='\u{f8ff}').contains(&glyphs.palette_cursor));
    }
}
