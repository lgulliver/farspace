package game

type Command interface {
	Kind() string
}

type EndTurn struct{}

func (EndTurn) Kind() string { return "EndTurn" }

type SetBudget struct {
	Empire      EmpireID
	ResearchPct int
	IndustryPct int
	CivicsPct   int
}

func (SetBudget) Kind() string { return "SetBudget" }
