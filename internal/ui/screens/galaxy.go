package screens

import "github.com/charmbracelet/lipgloss"

func GalaxyView(width, height int) string {
	content := "Galaxy\n\nA star map will appear here."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
