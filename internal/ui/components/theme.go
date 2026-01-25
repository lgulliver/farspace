package components

import "github.com/charmbracelet/lipgloss"

var (
	HeaderStyle = lipgloss.NewStyle().
			Background(lipgloss.Color("#2B2B2B")).
			Foreground(lipgloss.Color("#F2F2F2")).
			Padding(0, 1)

	FooterStyle = lipgloss.NewStyle().
			Background(lipgloss.Color("#1C1C1C")).
			Foreground(lipgloss.Color("#E0E0E0")).
			Padding(0, 1)

	TitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#F6C177"))

	PanelStyle = lipgloss.NewStyle().
			Border(lipgloss.NormalBorder()).
			BorderForeground(lipgloss.Color("#4A4A4A")).
			Padding(1, 2)
)
