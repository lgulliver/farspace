package game

type Event interface {
	Kind() string
}

type TurnAdvanced struct {
	NewTurn int
}

func (TurnAdvanced) Kind() string { return "TurnAdvanced" }

type ErrorEvent struct {
	Message string
}

func (ErrorEvent) Kind() string { return "Error" }
