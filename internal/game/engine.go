package game

import "fmt"

type Engine struct {
	State GameState
}

func NewEngine(seed uint64) *Engine {
	state := GameState{
		Turn:    1,
		Seed:    seed,
		Stars:   map[StarID]*Star{},
		Empires: map[EmpireID]*Empire{},
		Fleets:  map[FleetID]*Fleet{},
	}
	state.Empires[EmpireID(1)] = &Empire{Name: "Terran Union"}
	return &Engine{State: state}
}

func (e *Engine) ApplyTurn(cmds []Command) ([]Event, error) {
	events := make([]Event, 0)
	advance := false

	for _, cmd := range cmds {
		switch c := cmd.(type) {
		case SetBudget:
			if c.ResearchPct < 0 || c.IndustryPct < 0 || c.CivicsPct < 0 {
				events = append(events, ErrorEvent{Message: "budget percentages must be non-negative"})
				continue
			}
			sum := c.ResearchPct + c.IndustryPct + c.CivicsPct
			if sum != 100 {
				events = append(events, ErrorEvent{Message: fmt.Sprintf("budget must sum to 100, got %d", sum)})
				continue
			}
			empire, ok := e.State.Empires[c.Empire]
			if !ok {
				events = append(events, ErrorEvent{Message: "empire not found"})
				continue
			}
			empire.Budget = Budget{
				ResearchPct: c.ResearchPct,
				IndustryPct: c.IndustryPct,
				CivicsPct:   c.CivicsPct,
			}
		case EndTurn:
			advance = true
		default:
			events = append(events, ErrorEvent{Message: "unknown command"})
		}
	}

	if advance {
		e.State.Turn++
		events = append(events, TurnAdvanced{NewTurn: e.State.Turn})
	}

	return events, nil
}
