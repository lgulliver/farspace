# Engine Contract

This document defines the minimal core engine pipeline and the command/event interface.

## Pipeline

1. UI collects user input and emits Commands.
2. Core validates and applies Commands deterministically.
3. Core emits Events that describe what happened.
4. UI renders Events (e.g., Turn Report).

## Commands

- EndTurn
- SetBudget (ResearchPct, IndustryPct, CivicsPct)

## Events

- TurnAdvanced (NewTurn)
- ErrorEvent (Message)

## Determinism

- The core uses a seeded RNG.
- Iteration over maps must be sorted before use.
