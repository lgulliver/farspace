package game

import "sort"

// SortedStarIDs ensures deterministic iteration order for maps.
func SortedStarIDs(m map[StarID]*Star) []StarID {
	ids := make([]StarID, 0, len(m))
	for id := range m {
		ids = append(ids, id)
	}
	sort.Slice(ids, func(i, j int) bool { return ids[i] < ids[j] })
	return ids
}

// SortedEmpireIDs ensures deterministic iteration order for maps.
func SortedEmpireIDs(m map[EmpireID]*Empire) []EmpireID {
	ids := make([]EmpireID, 0, len(m))
	for id := range m {
		ids = append(ids, id)
	}
	sort.Slice(ids, func(i, j int) bool { return ids[i] < ids[j] })
	return ids
}

// SortedFleetIDs ensures deterministic iteration order for maps.
func SortedFleetIDs(m map[FleetID]*Fleet) []FleetID {
	ids := make([]FleetID, 0, len(m))
	for id := range m {
		ids = append(ids, id)
	}
	sort.Slice(ids, func(i, j int) bool { return ids[i] < ids[j] })
	return ids
}
