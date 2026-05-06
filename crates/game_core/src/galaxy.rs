//! Galaxy generation

use crate::state::{Planet, PlanetSize, SpectralClass, Star, StarId};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::collections::BTreeSet;

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

/// Generate a galaxy with the given seed and star count
pub fn generate_galaxy(seed: u64, star_count: usize) -> Vec<Star> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let star_count = star_count.clamp(10, 100);

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
                Planet {
                    name: planet_name,
                    size,
                    colony: None,
                    habitable: true,
                }
            })
            .collect();

        stars.push(Star {
            id: StarId(id as u64),
            name,
            x,
            y,
            spectral_class,
            planets,
        });
    }

    stars
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
        let stars1 = generate_galaxy(42, 20);
        let stars2 = generate_galaxy(42, 20);

        assert_eq!(stars1.len(), stars2.len());
        for (s1, s2) in stars1.iter().zip(stars2.iter()) {
            assert_eq!(s1.id, s2.id);
            assert_eq!(s1.name, s2.name);
            assert_eq!(s1.x, s2.x);
            assert_eq!(s1.y, s2.y);
            assert_eq!(s1.spectral_class, s2.spectral_class);
            assert_eq!(s1.planets.len(), s2.planets.len());
        }
    }

    #[test]
    fn different_seeds_produce_different_galaxies() {
        let stars1 = generate_galaxy(42, 20);
        let stars2 = generate_galaxy(43, 20);

        // At least some stars should have different names or positions
        let same_count = stars1
            .iter()
            .zip(stars2.iter())
            .filter(|(s1, s2)| s1.name == s2.name && s1.x == s2.x && s1.y == s2.y)
            .count();

        assert!(same_count < stars1.len() / 2, "Galaxies should differ");
    }

    #[test]
    fn galaxy_star_count_within_bounds() {
        // Test minimum clamping
        let stars = generate_galaxy(0, 5);
        assert_eq!(stars.len(), 10);

        // Test maximum clamping
        let stars = generate_galaxy(1, 150);
        assert_eq!(stars.len(), 100);

        // Test normal range
        let stars = generate_galaxy(42, 30);
        assert_eq!(stars.len(), 30);

        // Test edge cases
        let stars = generate_galaxy(u64::MAX, 20);
        assert_eq!(stars.len(), 20);
    }

    #[test]
    fn no_duplicate_coordinates() {
        let stars = generate_galaxy(42, 50);
        let mut coords: BTreeSet<(i32, i32)> = BTreeSet::new();
        for star in &stars {
            assert!(
                coords.insert((star.x, star.y)),
                "Duplicate coordinates found"
            );
        }
    }

    #[test]
    fn no_duplicate_names() {
        let stars = generate_galaxy(42, 50);
        let mut names: BTreeSet<String> = BTreeSet::new();
        for star in &stars {
            assert!(names.insert(star.name.clone()), "Duplicate name found");
        }
    }

    #[test]
    fn all_stars_have_planets() {
        let stars = generate_galaxy(42, 30);
        for star in &stars {
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
        let stars = generate_galaxy(42, 20);
        let home = find_home_star(&stars);
        assert!(home.is_some());
        assert!(!home.unwrap().planets.is_empty());
    }

    #[test]
    fn sequential_star_ids() {
        let stars = generate_galaxy(42, 20);
        for (i, star) in stars.iter().enumerate() {
            assert_eq!(star.id.0 as usize, i);
        }
    }
}
