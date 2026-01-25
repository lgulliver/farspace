package main

import (
	"flag"
	"fmt"
	"time"

	tea "github.com/charmbracelet/bubbletea"

	"github.com/farspace/farspace/internal/game"
	"github.com/farspace/farspace/internal/ui"
)

func main() {
	seedFlag := flag.Uint64("seed", 0, "seed for deterministic RNG")
	flag.Parse()

	seed := *seedFlag
	if seed == 0 {
		seed = uint64(time.Now().UnixNano())
	}

	engine := game.NewEngine(seed)
	model := ui.NewAppModel(engine)

	p := tea.NewProgram(model, tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Println("error running app:", err)
	}
}
