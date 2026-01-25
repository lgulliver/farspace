package ui

import (
	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
)

type KeyMap struct {
	Quit       key.Binding
	EndTurn    key.Binding
	Help       key.Binding
	Palette    key.Binding
	Galaxy     key.Binding
	Planets    key.Binding
	Fleets     key.Binding
	Tech       key.Binding
	Diplo      key.Binding
	Reports    key.Binding
	TurnReport key.Binding
}

func DefaultKeyMap() KeyMap {
	return KeyMap{
		Quit:       key.NewBinding(key.WithKeys("q"), key.WithHelp("q", "quit")),
		EndTurn:    key.NewBinding(key.WithKeys("enter", "e"), key.WithHelp("enter", "end turn")),
		Help:       key.NewBinding(key.WithKeys("?"), key.WithHelp("?", "help")),
		Palette:    key.NewBinding(key.WithKeys(":"), key.WithHelp(":", "palette")),
		Galaxy:     key.NewBinding(key.WithKeys("g"), key.WithHelp("g", "galaxy")),
		Planets:    key.NewBinding(key.WithKeys("p"), key.WithHelp("p", "planets")),
		Fleets:     key.NewBinding(key.WithKeys("f"), key.WithHelp("f", "fleets")),
		Tech:       key.NewBinding(key.WithKeys("t"), key.WithHelp("t", "tech")),
		Diplo:      key.NewBinding(key.WithKeys("d"), key.WithHelp("d", "diplo")),
		Reports:    key.NewBinding(key.WithKeys("r"), key.WithHelp("r", "reports")),
		TurnReport: key.NewBinding(key.WithKeys("0", "home"), key.WithHelp("0", "turn report")),
	}
}

func (k KeyMap) Hints() string {
	return "g p f t d r 0/home  enter: end turn  ?: help  : palette  q: quit"
}

func keyMatches(msg tea.KeyMsg, binding key.Binding) bool {
	return key.Matches(msg, binding)
}
