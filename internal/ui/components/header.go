package components

import "fmt"

func RenderHeader(width int, turn int, active string) string {
	content := fmt.Sprintf("FARSPACE • Turn %d • %s", turn, active)
	return HeaderStyle.Width(width).Render(content)
}
