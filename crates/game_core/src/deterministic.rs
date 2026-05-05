//! Deterministic helpers for ordered iteration

use crate::state::{ColonyId, EmpireId, FleetId, StarId};
use std::collections::BTreeMap;

/// Get sorted star IDs from a BTreeMap (already sorted by Ord)
pub fn sorted_star_ids<T>(map: &BTreeMap<StarId, T>) -> Vec<StarId> {
    map.keys().copied().collect()
}

/// Get sorted empire IDs from a BTreeMap (already sorted by Ord)
pub fn sorted_empire_ids<T>(map: &BTreeMap<EmpireId, T>) -> Vec<EmpireId> {
    map.keys().copied().collect()
}

/// Get sorted colony IDs from a BTreeMap (already sorted by Ord)
pub fn sorted_colony_ids<T>(map: &BTreeMap<ColonyId, T>) -> Vec<ColonyId> {
    map.keys().copied().collect()
}

/// Get sorted fleet IDs from a BTreeMap (already sorted by Ord)
pub fn sorted_fleet_ids<T>(map: &BTreeMap<FleetId, T>) -> Vec<FleetId> {
    map.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_star_ids_returns_ordered() {
        let mut map: BTreeMap<StarId, ()> = BTreeMap::new();
        map.insert(StarId(5), ());
        map.insert(StarId(1), ());
        map.insert(StarId(10), ());
        map.insert(StarId(3), ());

        let ids = sorted_star_ids(&map);
        assert_eq!(ids, vec![StarId(1), StarId(3), StarId(5), StarId(10)]);
    }

    #[test]
    fn sorted_colony_ids_returns_ordered() {
        let mut map: BTreeMap<ColonyId, ()> = BTreeMap::new();
        map.insert(ColonyId(100), ());
        map.insert(ColonyId(1), ());
        map.insert(ColonyId(50), ());

        let ids = sorted_colony_ids(&map);
        assert_eq!(ids, vec![ColonyId(1), ColonyId(50), ColonyId(100)]);
    }

    #[test]
    fn empty_maps_return_empty_vecs() {
        let map: BTreeMap<StarId, ()> = BTreeMap::new();
        assert!(sorted_star_ids(&map).is_empty());

        let map: BTreeMap<EmpireId, ()> = BTreeMap::new();
        assert!(sorted_empire_ids(&map).is_empty());

        let map: BTreeMap<FleetId, ()> = BTreeMap::new();
        assert!(sorted_fleet_ids(&map).is_empty());
    }
}
