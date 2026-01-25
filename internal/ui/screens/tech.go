package screens

import "github.com/charmbracelet/lipgloss"

func TechView(width, height int) string {
	content := "Technology\n\nResearch tree placeholder."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
