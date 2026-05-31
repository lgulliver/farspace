//! UI-only animation utilities.
//!
//! All motion here is driven by the deterministic `AppState::tick_count` render
//! counter — never by wall-clock time, and never touching simulation state. This
//! keeps the game core reproducible while letting the TUI feel alive.

/// Lightweight screen-transition descriptor.
///
/// This is scaffolding: the state machine and timing are deterministic now, so
/// richer visual application (fade/slide compositing) can be layered on later
/// without revisiting navigation call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScreenTransition {
    #[default]
    None,
    Fade,
    SlideLeft,
    SlideRight,
}

/// Deterministic, tick-driven transition progress.
///
/// A transition is inert (immediately inactive) when its kind is
/// [`ScreenTransition::None`] or its duration is zero, which makes it
/// reduced-motion safe by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransitionState {
    kind: ScreenTransition,
    elapsed: u16,
    duration: u16,
}

impl TransitionState {
    /// Begin a transition lasting `duration` render ticks.
    pub fn start(&mut self, kind: ScreenTransition, duration: u16) {
        if kind == ScreenTransition::None || duration == 0 {
            *self = Self::default();
        } else {
            *self = Self {
                kind,
                elapsed: 0,
                duration,
            };
        }
    }

    /// Advance one render tick. Saturates at completion and resets to inert.
    pub fn advance(&mut self) {
        if !self.is_active() {
            return;
        }
        self.elapsed = self.elapsed.saturating_add(1);
        if self.elapsed >= self.duration {
            *self = Self::default();
        }
    }

    /// Whether a transition is currently playing.
    pub fn is_active(&self) -> bool {
        self.kind != ScreenTransition::None && self.elapsed < self.duration
    }

    /// The active transition kind (`None` when inert).
    pub fn kind(&self) -> ScreenTransition {
        self.kind
    }

    /// Progress in `0.0..=1.0`. Returns `1.0` when inert.
    pub fn progress(&self) -> f32 {
        if self.duration == 0 {
            return 1.0;
        }
        (self.elapsed as f32 / self.duration as f32).clamp(0.0, 1.0)
    }
}

/// Two-phase low-frequency pulse for selection/transit shimmer.
///
/// Deterministic on the UI tick: the value flips every `period` ticks. A
/// `period` of zero disables the pulse (always `false`), which callers use to
/// honour reduced-motion.
pub fn pulse_on(tick: u64, period: u64) -> bool {
    if period == 0 {
        return false;
    }
    (tick / period).is_multiple_of(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_with_none_or_zero_duration_is_inert() {
        let mut t = TransitionState::default();
        t.start(ScreenTransition::None, 10);
        assert!(!t.is_active());
        t.start(ScreenTransition::Fade, 0);
        assert!(!t.is_active());
    }

    #[test]
    fn transition_advances_deterministically_and_terminates() {
        let mut t = TransitionState::default();
        t.start(ScreenTransition::Fade, 4);
        assert!(t.is_active());
        assert_eq!(t.kind(), ScreenTransition::Fade);

        let mut active_ticks = 0;
        for _ in 0..100 {
            if !t.is_active() {
                break;
            }
            active_ticks += 1;
            t.advance();
        }
        // Exactly `duration` ticks of activity, then permanently inert.
        assert_eq!(active_ticks, 4);
        assert!(!t.is_active());
        assert_eq!(t.kind(), ScreenTransition::None);
    }

    #[test]
    fn progress_is_monotonic_within_unit_range() {
        let mut t = TransitionState::default();
        t.start(ScreenTransition::SlideLeft, 5);
        let mut last = -1.0_f32;
        while t.is_active() {
            let p = t.progress();
            assert!((0.0..=1.0).contains(&p));
            assert!(p >= last);
            last = p;
            t.advance();
        }
        // Inert transition reports complete.
        assert_eq!(t.progress(), 1.0);
    }

    #[test]
    fn pulse_is_deterministic_and_alternates() {
        // period 0 disables motion (reduced-motion path).
        assert!(!pulse_on(0, 0));
        assert!(!pulse_on(999, 0));

        // Same tick always yields the same phase.
        assert_eq!(pulse_on(7, 3), pulse_on(7, 3));

        // Phase flips every `period` ticks.
        assert!(pulse_on(0, 2));
        assert!(pulse_on(1, 2));
        assert!(!pulse_on(2, 2));
        assert!(!pulse_on(3, 2));
        assert!(pulse_on(4, 2));
    }
}
