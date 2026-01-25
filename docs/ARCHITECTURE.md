# Architecture

FARSPACE separates the headless game core from the terminal UI.

- `/internal/game`: deterministic, headless engine. No terminal dependencies.
- `/internal/ui`: Bubble Tea TUI client. Translates input into Commands and renders Events.
- `/cmd/farspace`: application entrypoint.

## Flow

UI -> Commands -> Engine.ApplyTurn -> Events -> UI

The UI does not mutate state directly. It only submits commands and renders the resulting events.
