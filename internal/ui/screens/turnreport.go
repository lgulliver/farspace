package screens

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"

	"github.com/farspace/farspace/internal/game"
)

func TurnReportView(width, height int, events []game.Event) string {
	lines := []string{"Turn Report", ""}
	if len(events) == 0 {
		lines = append(lines, "No events yet. Press Enter to end the turn.")
	} else {
		for _, ev := range events {
			lines = append(lines, "- "+eventLine(ev))
		}
	}

	return lipgloss.NewStyle().Padding(1, 2).Render(strings.Join(lines, "\n"))
}

func eventLine(ev game.Event) string {
	switch e := ev.(type) {
	case game.TurnAdvanced:
		return fmt.Sprintf("Turn advanced to %d", e.NewTurn)
	case game.ErrorEvent:
		return fmt.Sprintf("Error: %s", e.Message)
	default:
		return "Event: " + ev.Kind()
	}
}
