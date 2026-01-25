package screens

import "github.com/charmbracelet/lipgloss"

func ReportsView(width, height int) string {
	content := "Reports\n\nStrategic reports placeholder."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
