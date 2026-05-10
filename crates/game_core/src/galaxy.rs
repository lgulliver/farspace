//! Galaxy generation

use crate::state::{
    HyperspaceLane, Planet, PlanetClass, PlanetSize, Sector, SectorId, SpectralClass, Star, StarId,
};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::collections::{BTreeMap, BTreeSet};

/// Sector name prefixes (original IP)
const SECTOR_NAME_PREFIXES: &[&str] = &[
    "Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta",
];

/// Sector name suffixes (original IP)
const SECTOR_NAME_SUFFIXES: &[&str] = &[
    "Reach", "Frontier", "Core", "Void", "Passage", "Expanse", "Cluster", "Haven",
];

/// Star name prefixes (original IP)
const STAR_NAME_PREFIXES: &[&str] = &[
    "Vel", "Keth", "Sorn", "Tar", "Avon", "Drel", "Yeth", "Forn", "Stel", "Bran", "Ceth", "Gal",
    "Hes", "Idor", "Jov",
];

/// Star name suffixes (original IP)
const STAR_NAME_SUFFIXES: &[&str] = &[
    "Soris", "Andara", "Mora", "Veth", "Kelus", "Thrax", "Ulim", "Preth", "Solas", "Neven", "Xar",
    "Yoth", "Zavar", "Elun", "Drosa",
];

/// Roman numerals for planet naming
const ROMAN_NUMERALS: &[&str] = &["I", "II", "III", "IV", "V", "VI", "VII", "VIII"];

/// Result of galaxy generation containing sectors and stars
pub struct GeneratedGalaxy {
    pub sectors: Vec<Sector>,
    pub stars: Vec<Star>,
}

/// Generate a galaxy with the given seed and star count
pub fn generate_galaxy(seed: u64, star_count: usize) -> GeneratedGalaxy {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let star_count = star_count.clamp(10, 100);

    // Determine sector count based on star count (roughly 1 sector per 10 stars, min 2, max 8)
    let sector_count = ((star_count as f64 / 10.0).ceil() as usize).clamp(2, 8);

    // Generate sector positions in a grid-like pattern across the galaxy
    let sector_positions = generate_sector_positions(sector_count, &mut rng);

    // Generate deterministic sector names without consuming RNG.
    let sector_names: Vec<String> = (0..sector_count)
        .map(|i| format!("{} {}", SECTOR_NAME_PREFIXES[i], SECTOR_NAME_SUFFIXES[i]))
        .collect();

    let sectors: Vec<Sector> = (0..sector_count)
        .map(|id| Sector {
            id: SectorId(id as u64),
            name: sector_names[id].clone(),
            x: sector_positions[id].0,
            y: sector_positions[id].1,
        })
        .collect();

    // Generate stars and assign them to sectors
    let mut stars = Vec::with_capacity(star_count);
    let mut used_coords: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut used_names: BTreeSet<String> = BTreeSet::new();

    for id in 0..star_count {
        // Generate unique coordinates
        let (x, y) = loop {
            let x = rng.gen_range(-500..=500);
            let y = rng.gen_range(-500..=500);
            if !used_coords.contains(&(x, y)) {
                used_coords.insert((x, y));
                break (x, y);
            }
        };

        // Assign to sector based on nearest sector center (deterministic)
        let sector_id = find_nearest_sector(x, y, &sector_positions);

        // Generate unique name
        let name = loop {
            let prefix = STAR_NAME_PREFIXES.choose(&mut rng).unwrap();
            let suffix = STAR_NAME_SUFFIXES.choose(&mut rng).unwrap();
            let name = format!("{} {}", prefix, suffix);
            if !used_names.contains(&name) {
                used_names.insert(name.clone());
                break name;
            }
        };

        // Random spectral class
        let spectral_class = *SpectralClass::all().choose(&mut rng).unwrap();

        // Generate 1-4 planets
        let planet_count = rng.gen_range(1..=4);
        let planets: Vec<Planet> = (0..planet_count)
            .map(|i| {
                let planet_name = format!("{} {}", name, ROMAN_NUMERALS[i]);
                let size = *PlanetSize::all().choose(&mut rng).unwrap();
                // Assign class deterministically based on star_id + planet index
                // to avoid consuming extra RNG calls that would break fixed-seed tests
                let class_idx = (id * 37 + i * 11) % PlanetClass::all().len();
                let class = PlanetClass::all()[class_idx];
                Planet {
                    name: planet_name,
                    size,
                    class,
                    colony: None,
                    habitable: true,
                    surveyed: false,
                }
            })
            .collect();

        stars.push(Star {
            id: StarId(id as u64),
            sector: SectorId(sector_id as u64),
            name,
            x,
            y,
            spectral_class,
            planets,
        });
    }

    GeneratedGalaxy { sectors, stars }
}

/// Generate sparse deterministic hyperspace lanes for the provided galaxy.
///
/// v1 model:
/// - at most one intra-sector lane per sector (closest pair)
/// - at most one inter-sector lane per adjacent sector pair (closest cross-pair)
pub fn generate_hyperspace_lanes(
    seed: u64,
    sectors: &[Sector],
    stars: &[Star],
) -> Vec<HyperspaceLane> {
    let mut lanes = BTreeSet::new();
    if stars.len() < 2 {
        return Vec::new();
    }

    let mut stars_by_sector: BTreeMap<SectorId, Vec<&Star>> = BTreeMap::new();
    for star in stars {
        stars_by_sector.entry(star.sector).or_default().push(star);
    }
    for stars_in_sector in stars_by_sector.values_mut() {
        stars_in_sector.sort_by_key(|s| s.id);
    }

    // One closest pair per sector.
    for stars_in_sector in stars_by_sector.values() {
        let mut best: Option<(i64, StarId, StarId)> = None;
        for i in 0..stars_in_sector.len() {
            for j in (i + 1)..stars_in_sector.len() {
                let a = stars_in_sector[i];
                let b = stars_in_sector[j];
                let dx = (a.x - b.x) as i64;
                let dy = (a.y - b.y) as i64;
                let sq = dx * dx + dy * dy;
                let candidate = (sq, a.id.min(b.id), a.id.max(b.id));
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }
        if let Some((_, a, b)) = best {
            if let Some(lane) = HyperspaceLane::new(a, b) {
                lanes.insert(lane);
            }
        }
    }

    // One closest pair per adjacent sector pair.
    for (sa, sb) in adjacent_sector_pairs(sectors) {
        let Some(a_stars) = stars_by_sector.get(&sa) else {
            continue;
        };
        let Some(b_stars) = stars_by_sector.get(&sb) else {
            continue;
        };

        let mut best: Option<(i64, StarId, StarId)> = None;
        for a in a_stars {
            for b in b_stars {
                let dx = (a.x - b.x) as i64;
                let dy = (a.y - b.y) as i64;
                let sq = dx * dx + dy * dy;
                let base = HyperspaceLane::new(a.id, b.id).expect("distinct stars");
                // Deterministic seed-based tie-break for equal-distance pairs.
                let tie_break = ((seed ^ ((base.a.0 << 32) | base.b.0)) & 0xFFFF) as i64;
                let candidate = (sq * 65_536 + tie_break, base.a, base.b);
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }

        if let Some((_, a, b)) = best {
            if let Some(lane) = HyperspaceLane::new(a, b) {
                lanes.insert(lane);
            }
        }
    }

    lanes.into_iter().collect()
}

fn adjacent_sector_pairs(sectors: &[Sector]) -> Vec<(SectorId, SectorId)> {
    // Sector centers are generated on a deterministic sparse grid. Treat sectors
    // with center distance <= 600 units as adjacent.
    const ADJACENT_SECTOR_MAX_SQ_DIST: i64 = 600 * 600;
    let mut pairs = Vec::new();
    for i in 0..sectors.len() {
        for j in (i + 1)..sectors.len() {
            let a = &sectors[i];
            let b = &sectors[j];
            let dx = (a.x - b.x) as i64;
            let dy = (a.y - b.y) as i64;
            let sq = dx * dx + dy * dy;
            if sq <= ADJACENT_SECTOR_MAX_SQ_DIST {
                pairs.push((a.id.min(b.id), a.id.max(b.id)));
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

/// Generate sector center positions in a grid pattern
fn generate_sector_positions(count: usize, _rng: &mut ChaCha8Rng) -> Vec<(i32, i32)> {
    if count == 0 {
        return vec![];
    }

    // For 2-4 sectors, use a simple 2x2 grid
    // For 5-8 sectors, use a 3x3 grid (with some sectors unused)
    // Note: Positions are deterministic to maintain RNG consistency for star generation
    match count {
        1 => vec![(0, 0)],
        2 => vec![(-250, 0), (250, 0)],
        3 => vec![(-250, -200), (250, -200), (0, 200)],
        4 => vec![(-250, -200), (250, -200), (-250, 200), (250, 200)],
        5 => vec![(-250, -200), (250, -200), (-250, 200), (250, 200), (0, 0)],
        6 => vec![
            (-300, -200),
            (0, -200),
            (300, -200),
            (-200, 200),
            (0, 200),
            (200, 200),
        ],
        7 => vec![
            (-300, -200),
            (0, -200),
            (300, -200),
            (-200, 200),
            (0, 200),
            (200, 200),
            (0, 0),
        ],
        _ => vec![
            (-300, -200),
            (0, -200),
            (300, -200),
            (-300, 200),
            (0, 200),
            (300, 200),
            (-150, 0),
            (150, 0),
        ],
    }
}

/// Find the nearest sector for a star at position (x, y)
fn find_nearest_sector(x: i32, y: i32, sector_positions: &[(i32, i32)]) -> usize {
    let mut nearest = 0;
    let mut nearest_dist = i32::MAX;

    for (idx, &(sx, sy)) in sector_positions.iter().enumerate() {
        let dx = x - sx;
        let dy = y - sy;
        let dist = dx * dx + dy * dy;
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest = idx;
        }
    }

    nearest
}

/// Find a suitable home star (prefer M-class or G-class)
pub fn find_home_star(stars: &[Star]) -> Option<&Star> {
    // First try to find an M-class star
    if let Some(star) = stars
        .iter()
        .find(|s| s.spectral_class == SpectralClass::M && !s.planets.is_empty())
    {
        return Some(star);
    }
    // Then try G-class
    if let Some(star) = stars
        .iter()
        .find(|s| s.spectral_class == SpectralClass::G && !s.planets.is_empty())
    {
        return Some(star);
    }
    // Fall back to any star with planets
    stars.iter().find(|s| !s.planets.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galaxy_generation_is_reproducible() {
        let gal1 = generate_galaxy(42, 20);
        let gal2 = generate_galaxy(42, 20);

        assert_eq!(gal1.stars.len(), gal2.stars.len());
        for (s1, s2) in gal1.stars.iter().zip(gal2.stars.iter()) {
            assert_eq!(s1.id, s2.id);
            assert_eq!(s1.name, s2.name);
            assert_eq!(s1.sector, s2.sector);
            assert_eq!(s1.x, s2.x);
            assert_eq!(s1.y, s2.y);
            assert_eq!(s1.spectral_class, s2.spectral_class);
            assert_eq!(s1.planets.len(), s2.planets.len());
        }
        assert_eq!(gal1.sectors.len(), gal2.sectors.len());
    }

    #[test]
    fn different_seeds_produce_different_galaxies() {
        let gal1 = generate_galaxy(42, 20);
        let gal2 = generate_galaxy(43, 20);

        // At least some stars should have different names or positions
        let same_count = gal1
            .stars
            .iter()
            .zip(gal2.stars.iter())
            .filter(|(s1, s2)| s1.name == s2.name && s1.x == s2.x && s1.y == s2.y)
            .count();

        assert!(same_count < gal1.stars.len() / 2, "Galaxies should differ");
    }

    #[test]
    fn galaxy_star_count_within_bounds() {
        // Test minimum clamping
        let gal = generate_galaxy(0, 5);
        assert_eq!(gal.stars.len(), 10);

        // Test maximum clamping
        let gal = generate_galaxy(1, 150);
        assert_eq!(gal.stars.len(), 100);

        // Test normal range
        let gal = generate_galaxy(42, 30);
        assert_eq!(gal.stars.len(), 30);

        // Test edge cases
        let gal = generate_galaxy(u64::MAX, 20);
        assert_eq!(gal.stars.len(), 20);
    }

    #[test]
    fn no_duplicate_coordinates() {
        let gal = generate_galaxy(42, 50);
        let mut coords: BTreeSet<(i32, i32)> = BTreeSet::new();
        for star in &gal.stars {
            assert!(
                coords.insert((star.x, star.y)),
                "Duplicate coordinates found"
            );
        }
    }

    #[test]
    fn no_duplicate_names() {
        let gal = generate_galaxy(42, 50);
        let mut names: BTreeSet<String> = BTreeSet::new();
        for star in &gal.stars {
            assert!(names.insert(star.name.clone()), "Duplicate name found");
        }
    }

    #[test]
    fn all_stars_have_planets() {
        let gal = generate_galaxy(42, 30);
        for star in &gal.stars {
            assert!(
                !star.planets.is_empty(),
                "Star should have at least 1 planet"
            );
            assert!(
                star.planets.len() <= 4,
                "Star should have at most 4 planets"
            );
        }
    }

    #[test]
    fn find_home_star_returns_some() {
        let gal = generate_galaxy(42, 20);
        let home = find_home_star(&gal.stars);
        assert!(home.is_some());
        assert!(!home.unwrap().planets.is_empty());
    }

    #[test]
    fn sequential_star_ids() {
        let gal = generate_galaxy(42, 20);
        for (i, star) in gal.stars.iter().enumerate() {
            assert_eq!(star.id.0 as usize, i);
        }
    }

    // Sector tests

    #[test]
    fn sector_generation_is_reproducible() {
        let gal1 = generate_galaxy(42, 20);
        let gal2 = generate_galaxy(42, 20);

        assert_eq!(gal1.sectors.len(), gal2.sectors.len());
        for (s1, s2) in gal1.sectors.iter().zip(gal2.sectors.iter()) {
            assert_eq!(s1.id, s2.id);
            assert_eq!(s1.name, s2.name);
            assert_eq!(s1.x, s2.x);
            assert_eq!(s1.y, s2.y);
        }
    }

    #[test]
    fn same_seed_assigns_systems_to_same_sectors() {
        let gal1 = generate_galaxy(42, 20);
        let gal2 = generate_galaxy(42, 20);

        for (s1, s2) in gal1.stars.iter().zip(gal2.stars.iter()) {
            assert_eq!(
                s1.sector, s2.sector,
                "Star {} should be in same sector",
                s1.id.0
            );
        }
    }

    #[test]
    fn every_system_belongs_to_exactly_one_sector() {
        let gal = generate_galaxy(42, 20);

        for star in &gal.stars {
            // Check that the sector ID is valid
            assert!(
                gal.sectors.iter().any(|s| s.id == star.sector),
                "Star {} has invalid sector {:?}",
                star.id.0,
                star.sector
            );
        }

        // Each star should have exactly one sector
        let mut sector_counts: std::collections::BTreeMap<SectorId, usize> =
            std::collections::BTreeMap::new();
        for star in &gal.stars {
            *sector_counts.entry(star.sector).or_insert(0) += 1;
        }

        // Total should equal number of stars
        let total: usize = sector_counts.values().sum();
        assert_eq!(total, gal.stars.len());
    }

    #[test]
    fn sector_system_positions_are_deterministic() {
        let gal1 = generate_galaxy(42, 20);
        let gal2 = generate_galaxy(42, 20);

        // For each star, both galaxies should have the same position
        for (s1, s2) in gal1.stars.iter().zip(gal2.stars.iter()) {
            assert_eq!(s1.x, s2.x, "Star {} x position should be same", s1.id.0);
            assert_eq!(s1.y, s2.y, "Star {} y position should be same", s1.id.0);
        }
    }

    #[test]
    fn sectors_have_unique_names() {
        let gal = generate_galaxy(42, 20);
        let mut names: BTreeSet<String> = BTreeSet::new();
        for sector in &gal.sectors {
            assert!(
                names.insert(sector.name.clone()),
                "Duplicate sector name found: {}",
                sector.name
            );
        }
    }

    #[test]
    fn sectors_have_unique_ids() {
        let gal = generate_galaxy(42, 20);
        let mut ids: Vec<SectorId> = gal.sectors.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), gal.sectors.len(), "Sector IDs should be unique");
    }

    #[test]
    fn sector_count_based_on_star_count() {
        // Small galaxy: 10-19 stars -> 2 sectors
        let gal = generate_galaxy(42, 10);
        assert_eq!(gal.sectors.len(), 2);

        // Medium galaxy: 20-29 stars -> 2 sectors
        let gal = generate_galaxy(42, 20);
        assert_eq!(gal.sectors.len(), 2);

        // Larger galaxy: 30-39 stars -> 3 sectors
        let gal = generate_galaxy(42, 30);
        assert_eq!(gal.sectors.len(), 3);

        // Large galaxy: 50-59 stars -> 5 sectors
        let gal = generate_galaxy(42, 50);
        assert_eq!(gal.sectors.len(), 5);

        // Very large: 80+ stars -> 8 sectors (max)
        let gal = generate_galaxy(42, 80);
        assert_eq!(gal.sectors.len(), 8);
    }

    #[test]
    fn no_duplicate_sector_coordinates() {
        let gal = generate_galaxy(42, 50);
        let mut coords: BTreeSet<(i32, i32)> = BTreeSet::new();
        for sector in &gal.sectors {
            assert!(
                coords.insert((sector.x, sector.y)),
                "Duplicate sector coordinates found"
            );
        }
    }

    #[test]
    fn hyperspace_lane_generation_is_reproducible() {
        let gal = generate_galaxy(42, 30);
        let lanes_a = generate_hyperspace_lanes(42, &gal.sectors, &gal.stars);
        let lanes_b = generate_hyperspace_lanes(42, &gal.sectors, &gal.stars);
        assert_eq!(lanes_a, lanes_b);
    }

    #[test]
    fn hyperspace_lanes_connect_valid_distinct_systems() {
        let gal = generate_galaxy(8, 30);
        let lanes = generate_hyperspace_lanes(8, &gal.sectors, &gal.stars);
        let star_ids: BTreeSet<StarId> = gal.stars.iter().map(|s| s.id).collect();
        for lane in lanes {
            assert!(star_ids.contains(&lane.a));
            assert!(star_ids.contains(&lane.b));
            assert_ne!(lane.a, lane.b);
        }
    }
}
