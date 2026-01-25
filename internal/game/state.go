package game

type GameState struct {
	Turn    int
	Seed    uint64
	Stars   map[StarID]*Star
	Empires map[EmpireID]*Empire
	Fleets  map[FleetID]*Fleet
}

type Star struct {
	Name string
}

type Empire struct {
	Name   string
	Budget Budget
}

type Fleet struct {
	Name string
}

type Budget struct {
	ResearchPct int
	IndustryPct int
	CivicsPct   int
}
