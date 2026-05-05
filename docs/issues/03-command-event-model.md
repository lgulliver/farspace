# Issue: Command / Event Model

**Labels:** `core`, `copilot-ready`

## Goal

Establish the full initial `Command` and `Event` enums in `game_core`, implement `Engine::apply_turn`, and write comprehensive tests. This is the contract all future features build on.

## Commands (initial set)

- `EndTurn`
- `SetBudget { empire: EmpireId, research_pct: u8, industry_pct: u8, civics_pct: u8 }`

## Events (initial set)

- `TurnAdvanced { new_turn: u32 }`
- `Error { message: String }`

## Validation Rules

- `SetBudget`: percentages must sum to exactly 100; empire must exist
- Unknown empire → `Event::Error`
- Invalid sum → `Event::Error`
- Commands are processed in order; a failing command does not abort subsequent commands

## Acceptance Criteria

- [ ] `Command` enum covers the initial set
- [ ] `Event` enum covers the initial set
- [ ] `Engine::apply_turn(&mut self, commands: &[Command]) -> Vec<Event>` implemented
- [ ] `EndTurn` increments `state.turn` and emits `TurnAdvanced`
- [ ] `SetBudget` validates sum and empire existence before mutating
- [ ] Invalid commands produce `Event::Error` with descriptive message
- [ ] State is unchanged after an invalid command

## Tests Required

- `end_turn_advances_turn` (positive)
- `set_budget_valid_updates_empire` (positive)
- `set_budget_sum_not_100_emits_error` (negative)
- `set_budget_unknown_empire_emits_error` (negative)
- `empty_command_list_produces_no_events` (edge case)
- `multiple_commands_processed_in_order` (positive)
