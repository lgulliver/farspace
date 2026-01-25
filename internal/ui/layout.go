package ui

import "github.com/charmbracelet/lipgloss"

func ComposeLayout(width, height int, header, main, footer string) string {
	header = lipgloss.NewStyle().Width(width).Render(header)
	footer = lipgloss.NewStyle().Width(width).Render(footer)

	mainHeight := height - lipgloss.Height(header) - lipgloss.Height(footer)
	if mainHeight < 0 {
		mainHeight = 0
	}
	main = lipgloss.NewStyle().Width(width).Height(mainHeight).Render(main)

	return lipgloss.JoinVertical(lipgloss.Left, header, main, footer)
}
