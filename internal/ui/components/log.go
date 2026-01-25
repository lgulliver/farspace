package components

import "strings"

const maxLogLines = 5

func AppendLog(lines []string, entry string) []string {
	lines = append(lines, entry)
	if len(lines) > maxLogLines {
		lines = lines[len(lines)-maxLogLines:]
	}
	return lines
}

func RenderLog(lines []string) string {
	if len(lines) == 0 {
		return "log: (empty)"
	}
	start := 0
	if len(lines) > 2 {
		start = len(lines) - 2
	}
	return "log: " + strings.Join(lines[start:], " | ")
}
