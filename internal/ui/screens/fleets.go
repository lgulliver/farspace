package screens

import "github.com/charmbracelet/lipgloss"

func FleetsView(width, height int) string {
	content := "Fleets\n\nFleet overview placeholder."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
