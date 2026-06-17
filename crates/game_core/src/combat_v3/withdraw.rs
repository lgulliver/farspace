//! Withdraw semantics: the `Warp Retreat` card and the free `r`-command
//! retreat.  Card-based withdraw preserves the turn; the free command
//! burns it.

use super::card::CardId;

/// Apply a `Warp Retreat` card play.  Returns the new integrity for the
/// retreating side (50% of pre-retreat).
pub fn apply_withdraw_card(current_integrity: u32) -> u32 {
    (current_integrity * 50) / 100
}

/// Compute the integrity for a free retreat command (25% of pre-retreat).
pub fn free_retreat(current_integrity: u32) -> u32 {
    (current_integrity * 25) / 100
}

/// Stable id for the `Warp Retreat` card.  Re-exported so callers don't
/// have to import the card module directly.
pub const WARP_RETREAT_CARD_ID: CardId = CardId(12);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdraw_card_preserves_half() {
        assert_eq!(apply_withdraw_card(100), 50);
        assert_eq!(apply_withdraw_card(60), 30);
        assert_eq!(apply_withdraw_card(0), 0);
    }

    #[test]
    fn free_retreat_preserves_quarter() {
        assert_eq!(free_retreat(100), 25);
        assert_eq!(free_retreat(80), 20);
        assert_eq!(free_retreat(0), 0);
    }
}
