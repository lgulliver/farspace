package screens

import "github.com/charmbracelet/lipgloss"

func DiploView(width, height int) string {
	content := "Diplomacy\n\nDiplomatic view placeholder."
	return lipgloss.NewStyle().Padding(1, 2).Render(content)
}
