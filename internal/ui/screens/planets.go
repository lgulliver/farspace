package screens

import "github.com/charmbracelet/lipgloss"

func PlanetsView(width, height int) string {
	content := "Planets\n\nPlanet management placeholder."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
