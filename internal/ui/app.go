package ui

import (
	"fmt"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"

	"github.com/farspace/farspace/internal/game"
	"github.com/farspace/farspace/internal/ui/components"
	"github.com/farspace/farspace/internal/ui/screens"
)

type ScreenID int

const (
	ScreenGalaxy ScreenID = iota
	ScreenPlanets
	ScreenFleets
	ScreenTech
	ScreenDiplo
	ScreenReports
	ScreenTurnReport
)

type AppModel struct {
	engine      *game.Engine
	active      ScreenID
	lastEvents  []game.Event
	width       int
	height      int
	log         []string
	showHelp    bool
	showPalette bool
	keys        KeyMap
}

func NewAppModel(engine *game.Engine) *AppModel {
	m := &AppModel{
		engine: engine,
		active: ScreenGalaxy,
		keys:   DefaultKeyMap(),
	}
	m.log = components.AppendLog(m.log, "booted FARSPACE")
	return m
}

func (m *AppModel) Init() tea.Cmd {
	return nil
}

func (m *AppModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		return m, nil
	case tea.KeyMsg:
		switch {
		case keyMatches(msg, m.keys.Quit):
			return m, tea.Quit
		case keyMatches(msg, m.keys.Galaxy):
			m.active = ScreenGalaxy
		case keyMatches(msg, m.keys.Planets):
			m.active = ScreenPlanets
		case keyMatches(msg, m.keys.Fleets):
			m.active = ScreenFleets
		case keyMatches(msg, m.keys.Tech):
			m.active = ScreenTech
		case keyMatches(msg, m.keys.Diplo):
			m.active = ScreenDiplo
		case keyMatches(msg, m.keys.Reports):
			m.active = ScreenReports
		case keyMatches(msg, m.keys.TurnReport):
			m.active = ScreenTurnReport
		case keyMatches(msg, m.keys.Help):
			m.showHelp = !m.showHelp
			if m.showHelp {
				m.showPalette = false
			}
		case keyMatches(msg, m.keys.Palette):
			m.showPalette = !m.showPalette
			if m.showPalette {
				m.showHelp = false
				m.log = components.AppendLog(m.log, "palette opened (placeholder)")
			}
		case keyMatches(msg, m.keys.EndTurn):
			events, err := m.engine.ApplyTurn([]game.Command{game.EndTurn{}})
			if err != nil {
				m.log = components.AppendLog(m.log, fmt.Sprintf("engine error: %v", err))
			} else {
				m.lastEvents = events
				for _, ev := range events {
					m.log = components.AppendLog(m.log, eventToString(ev))
				}
				m.active = ScreenTurnReport
			}
		}
	}

	return m, nil
}

func (m *AppModel) View() string {
	if m.width == 0 || m.height == 0 {
		return ""
	}

	header := components.RenderHeader(m.width, m.engine.State.Turn, screenName(m.active))

	main := ""
	switch {
	case m.showHelp:
		main = lipgloss.NewStyle().Padding(1).Render("Help (placeholder)\n\nKeys: g p f t d r 0/home\nEnter: end turn\n?: toggle help\n: palette")
	case m.showPalette:
		main = lipgloss.NewStyle().Padding(1).Render("Command Palette (placeholder)\n\nType a command... (not implemented)\n\nExamples:\n- end turn\n- go galaxy\n- go reports\n\nPress : to close.")
	default:
		switch m.active {
		case ScreenGalaxy:
			main = screens.GalaxyView(m.width, m.height)
		case ScreenPlanets:
			main = screens.PlanetsView(m.width, m.height)
		case ScreenFleets:
			main = screens.FleetsView(m.width, m.height)
		case ScreenTech:
			main = screens.TechView(m.width, m.height)
		case ScreenDiplo:
			main = screens.DiploView(m.width, m.height)
		case ScreenReports:
			main = screens.ReportsView(m.width, m.height)
		case ScreenTurnReport:
			main = screens.TurnReportView(m.width, m.height, m.lastEvents)
		}
	}

	footer := components.RenderFooter(m.width, m.keys.Hints(), m.log)
	return ComposeLayout(m.width, m.height, header, main, footer)
}

func screenName(id ScreenID) string {
	switch id {
	case ScreenGalaxy:
		return "Galaxy"
	case ScreenPlanets:
		return "Planets"
	case ScreenFleets:
		return "Fleets"
	case ScreenTech:
		return "Tech"
	case ScreenDiplo:
		return "Diplomacy"
	case ScreenReports:
		return "Reports"
	case ScreenTurnReport:
		return "Turn Report"
	default:
		return "Unknown"
	}
}

func eventToString(ev game.Event) string {
	switch e := ev.(type) {
	case game.TurnAdvanced:
		return fmt.Sprintf("turn advanced to %d", e.NewTurn)
	case game.ErrorEvent:
		return fmt.Sprintf("error: %s", e.Message)
	default:
		return "event: " + ev.Kind()
	}
}
