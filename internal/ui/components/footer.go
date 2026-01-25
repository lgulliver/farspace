package components

func RenderFooter(width int, hints string, logLines []string) string {
	content := hints + "\n" + RenderLog(logLines)
	return FooterStyle.Width(width).Render(content)
}
