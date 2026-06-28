//! Galaxy generation

use crate::state::SeededRng;
use crate::state::{
    DiscoveryRarity, HyperspaceLane, Planet, PlanetAnomaly, PlanetClass, PlanetSize, PlanetSpecial,
    Sector, SectorId, SpectralClass, Star, StarId, StrategicResource,
};
use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use rand::rngs::ChaCha8Rng;
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
const FRONTIER_DISTANCE_DIVISOR: i32 = 240;
const SYNTHETIC_CLASS_STRIDE: usize = 7;
const SYNTHETIC_SPECTRAL_STRIDE: usize = 3;
const SYNTHETIC_SECTOR_DIVISOR: u64 = 5;
const SYNTHETIC_SECTOR_MODULUS: u64 = 8;
const SYNTHETIC_COORD_MODULUS: i32 = 1001;
const SYNTHETIC_COORD_OFFSET: i32 = 500;
const SYNTHETIC_X_STAR_MULT: i32 = 97;
const SYNTHETIC_X_PLANET_MULT: i32 = 31;
const SYNTHETIC_Y_STAR_MULT: i32 = 53;
const SYNTHETIC_Y_PLANET_MULT: i32 = 17;

/// Result of galaxy generation containing sectors and stars
pub struct GeneratedGalaxy {
    pub sectors: Vec<Sector>,
    pub stars: Vec<Star>,
}

/// Deterministic environment context for one planet resource roll.
#[derive(Debug, Clone, Copy)]
pub struct ResourceGenerationContext {
    pub planet_class: PlanetClass,
    pub spectral_class: SpectralClass,
    pub sector_id: SectorId,
    pub star_x: i32,
    pub star_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetDiscoveries {
    pub specials: Vec<PlanetSpecial>,
    pub anomalies: Vec<PlanetAnomaly>,
    pub resources: Vec<StrategicResource>,
}

fn frontier_distance_bonus(x: i32, y: i32, max: i32) -> u32 {
    ((x.abs() + y.abs()) / FRONTIER_DISTANCE_DIVISOR).clamp(0, max) as u32
}

fn planet_special_weight(
    special: PlanetSpecial,
    context: ResourceGenerationContext,
    is_hazardous: bool,
    has_precursor_signature: bool,
    in_nebula_band: bool,
) -> u32 {
    let base = match special.rarity() {
        DiscoveryRarity::Common => 26,
        DiscoveryRarity::Uncommon => 14,
        DiscoveryRarity::Rare => 6,
        DiscoveryRarity::Legendary => 2,
    };
    let class_bias = match (special, context.planet_class) {
        (
            PlanetSpecial::MineralRich | PlanetSpecial::SubterraneanMegacaverns,
            PlanetClass::Barren | PlanetClass::Volcanic,
        ) => 14,
        (
            PlanetSpecial::FertileBiosphere | PlanetSpecial::BioluminescentJungles,
            PlanetClass::Terran | PlanetClass::Oceanic,
        ) => 12,
        (
            PlanetSpecial::CrystalFormations | PlanetSpecial::CrystalForests,
            PlanetClass::Frozen | PlanetClass::Desert,
        ) => 10,
        (PlanetSpecial::HyperconductiveOceans, PlanetClass::Oceanic) => 16,
        (PlanetSpecial::VolatileCoreInstability, PlanetClass::Volcanic) => 15,
        (PlanetSpecial::FrozenDataVault, PlanetClass::Frozen) => 12,
        (PlanetSpecial::LowGravity, PlanetClass::Barren | PlanetClass::Desert) => 8,
        (PlanetSpecial::NaniteScarfields, PlanetClass::Volcanic | PlanetClass::Barren) => 9,
        _ => 0,
    };
    let spectral_bias = match (special, context.spectral_class) {
        (PlanetSpecial::HyperconductiveOceans, SpectralClass::A | SpectralClass::F) => 7,
        (PlanetSpecial::GravitationalFractureZone, SpectralClass::O | SpectralClass::B) => 8,
        (
            PlanetSpecial::CrystalFormations | PlanetSpecial::CrystalForests,
            SpectralClass::A | SpectralClass::F,
        ) => 6,
        (PlanetSpecial::OrbitalGraveyard, SpectralClass::G | SpectralClass::K) => 4,
        _ => 0,
    };
    let frontier_bonus = frontier_distance_bonus(context.star_x, context.star_y, 5);
    let hazard_bias = if is_hazardous {
        match special {
            PlanetSpecial::HostileWeather
            | PlanetSpecial::VolatileCoreInstability
            | PlanetSpecial::GravitationalFractureZone
            | PlanetSpecial::NaniteScarfields => 12,
            _ => 2,
        }
    } else {
        0
    };
    let precursor_bias = if has_precursor_signature {
        match special {
            PlanetSpecial::AncientRuins
            | PlanetSpecial::PrecursorBeacon
            | PlanetSpecial::AncientDefenseGrid
            | PlanetSpecial::FrozenDataVault
            | PlanetSpecial::OrbitalGraveyard => 15,
            _ => 0,
        }
    } else {
        0
    };
    let nebula_bias = if in_nebula_band {
        match special {
            PlanetSpecial::BioluminescentJungles
            | PlanetSpecial::CrystalForests
            | PlanetSpecial::GravitationalFractureZone => 5,
            _ => 0,
        }
    } else {
        0
    };
    base + class_bias + spectral_bias + frontier_bonus + hazard_bias + precursor_bias + nebula_bias
}

fn anomaly_weight(
    anomaly: PlanetAnomaly,
    context: ResourceGenerationContext,
    is_hazardous: bool,
    has_precursor_signature: bool,
    in_nebula_band: bool,
) -> u32 {
    let base = match anomaly.rarity() {
        DiscoveryRarity::Common => 0,
        DiscoveryRarity::Uncommon => 15,
        DiscoveryRarity::Rare => 8,
        DiscoveryRarity::Legendary => 3,
    };
    let class_bias = match (anomaly, context.planet_class) {
        (PlanetAnomaly::FrozenColonyVessel, PlanetClass::Frozen | PlanetClass::Terran) => 10,
        (PlanetAnomaly::RogueNaniteSwarm, PlanetClass::Volcanic | PlanetClass::Barren) => 12,
        (PlanetAnomaly::GraviticStormFront, PlanetClass::Volcanic | PlanetClass::Frozen) => 10,
        (PlanetAnomaly::CollapsedJumpCorridor, PlanetClass::Barren | PlanetClass::Desert) => 8,
        _ => 0,
    };
    let spectral_bias = match (anomaly, context.spectral_class) {
        (PlanetAnomaly::TemporalEchoField, SpectralClass::A | SpectralClass::F) => 8,
        (PlanetAnomaly::GraviticStormFront, SpectralClass::O | SpectralClass::B) => 10,
        (PlanetAnomaly::QuantumReflectionZone, SpectralClass::B | SpectralClass::A) => 8,
        _ => 0,
    };
    let frontier_bonus = frontier_distance_bonus(context.star_x, context.star_y, 6);
    let hazard_bias = if is_hazardous {
        match anomaly {
            PlanetAnomaly::RogueNaniteSwarm
            | PlanetAnomaly::GraviticStormFront
            | PlanetAnomaly::DerelictBattleSphere => 14,
            _ => 3,
        }
    } else {
        0
    };
    let precursor_bias = if has_precursor_signature {
        match anomaly {
            PlanetAnomaly::SilentRelayNetwork
            | PlanetAnomaly::PrecursorListeningPost
            | PlanetAnomaly::VoidSignalArray => 18,
            _ => 2,
        }
    } else {
        0
    };
    let nebula_bias = if in_nebula_band {
        match anomaly {
            PlanetAnomaly::QuantumReflectionZone
            | PlanetAnomaly::TemporalEchoField
            | PlanetAnomaly::SilentRelayNetwork => 6,
            _ => 0,
        }
    } else {
        0
    };
    base + class_bias + spectral_bias + frontier_bonus + hazard_bias + precursor_bias + nebula_bias
}

fn strategic_resource_weight(
    resource: StrategicResource,
    context: ResourceGenerationContext,
    is_hazardous: bool,
    has_precursor_signature: bool,
    in_nebula_band: bool,
) -> u32 {
    let base = match resource.rarity() {
        crate::state::StrategicResourceRarity::Common => 22,
        crate::state::StrategicResourceRarity::Uncommon => 12,
        crate::state::StrategicResourceRarity::Rare => 6,
        crate::state::StrategicResourceRarity::Legendary => 2,
    };
    let class_bias = match (resource, context.planet_class) {
        (StrategicResource::Helium3, PlanetClass::Barren | PlanetClass::Frozen) => 14,
        (StrategicResource::ReactiveIsotopes, PlanetClass::Volcanic) => 10,
        (StrategicResource::NeutroniumDeposits, PlanetClass::Barren | PlanetClass::Volcanic) => 9,
        (StrategicResource::HyperfiberOrganics, PlanetClass::Oceanic | PlanetClass::Terran) => 10,
        (StrategicResource::PsionicSpores, PlanetClass::Oceanic | PlanetClass::Frozen) => 8,
        (StrategicResource::QuantumCrystals, PlanetClass::Frozen | PlanetClass::Desert) => 8,
        (StrategicResource::DarkMatter, PlanetClass::Barren | PlanetClass::Frozen) => 8,
        (StrategicResource::LivingAlloy, PlanetClass::Volcanic | PlanetClass::Terran) => 8,
        (StrategicResource::AntimatterResidue, PlanetClass::Volcanic) => 8,
        (StrategicResource::PrecursorDatacores, PlanetClass::Barren | PlanetClass::Desert) => 8,
        _ => 0,
    };
    let spectral_bias = match (resource, context.spectral_class) {
        (StrategicResource::Helium3, SpectralClass::O | SpectralClass::B) => 10,
        (StrategicResource::DarkMatter, SpectralClass::O | SpectralClass::A) => 8,
        (StrategicResource::QuantumCrystals, SpectralClass::F | SpectralClass::A) => 6,
        (StrategicResource::AntimatterResidue, SpectralClass::B) => 8,
        _ => 0,
    };
    let frontier_bonus = ((context.star_x.abs() + context.star_y.abs()) / FRONTIER_DISTANCE_DIVISOR)
        .clamp(0, 4) as u32;
    let sector_wave = ((context.sector_id.0 as i32 % 5) - 2).unsigned_abs();
    let sector_bias = (4u32).saturating_sub(sector_wave);
    let hazard_bias = if is_hazardous {
        match resource {
            StrategicResource::AntimatterResidue | StrategicResource::DarkMatter => 10,
            _ => 2,
        }
    } else {
        0
    };
    let precursor_bias = if has_precursor_signature {
        match resource {
            StrategicResource::PrecursorDatacores => 18,
            StrategicResource::LivingAlloy | StrategicResource::QuantumCrystals => 5,
            _ => 0,
        }
    } else {
        0
    };
    let nebula_bias = if in_nebula_band {
        match resource {
            StrategicResource::DarkMatter | StrategicResource::PsionicSpores => 6,
            _ => 0,
        }
    } else {
        0
    };
    base + class_bias
        + spectral_bias
        + frontier_bonus
        + sector_bias
        + hazard_bias
        + precursor_bias
        + nebula_bias
}

/// Context-aware deterministic discovery generation.
pub fn generate_planet_discoveries_for_context(
    galaxy_seed: u64,
    star_id: StarId,
    planet_index: usize,
    context: ResourceGenerationContext,
) -> PlanetDiscoveries {
    let planet_seed = galaxy_seed
        .wrapping_add(star_id.0.wrapping_mul(1_000_003))
        .wrapping_add(planet_index as u64 * 999_983)
        .wrapping_add(context.sector_id.0.wrapping_mul(17_719))
        .wrapping_add(
            ((context.star_x as i64).unsigned_abs() + (context.star_y as i64).unsigned_abs()) * 131,
        );
    let mut planet_rng = SeededRng::new(planet_seed);
    // Separate anomaly RNG stream keeps anomaly rolls independent from legacy
    // special/resource RNG consumption while preserving deterministic placement.
    let mut anomaly_rng = SeededRng::new(planet_seed ^ 0xA11A_D15C_0FFE_51E5);

    let is_hazardous = planet_rng.random::<u8>() < 28;
    let has_precursor_signature = planet_rng.random::<u8>() < 10;
    let in_nebula_band = planet_rng.random::<u8>() < 22;
    let hotspot_bias = planet_rng.random::<u8>() < 12;
    let poor_bias = !hotspot_bias && planet_rng.random::<u8>() < 16;

    let mut specials = Vec::new();
    let special_roll_threshold = if has_precursor_signature {
        146u8
    } else if is_hazardous {
        124u8
    } else {
        92u8
    };
    if planet_rng.random::<u8>() < special_roll_threshold {
        let all = PlanetSpecial::all();
        let weights: Vec<u32> = all
            .iter()
            .map(|special| {
                planet_special_weight(
                    *special,
                    context,
                    is_hazardous,
                    has_precursor_signature,
                    in_nebula_band,
                )
            })
            .collect();
        if let Ok(dist) = WeightedIndex::new(&weights) {
            specials.push(all[dist.sample(&mut planet_rng)]);
        }
    }

    let mut anomalies = Vec::new();
    let anomaly_roll_threshold = if has_precursor_signature {
        68u8
    } else if is_hazardous || hotspot_bias {
        54u8
    } else {
        34u8
    };
    if anomaly_rng.random::<u8>() < anomaly_roll_threshold {
        let all = PlanetAnomaly::all();
        let weights: Vec<u32> = all
            .iter()
            .map(|anomaly| {
                anomaly_weight(
                    *anomaly,
                    context,
                    is_hazardous,
                    has_precursor_signature,
                    in_nebula_band,
                )
            })
            .collect();
        if let Ok(dist) = WeightedIndex::new(&weights) {
            anomalies.push(all[dist.sample(&mut anomaly_rng)]);
        }
    }

    let base_resource_roll = if hotspot_bias {
        140u8
    } else if poor_bias {
        42u8
    } else {
        88u8
    };
    let mut resources = Vec::new();
    if planet_rng.random::<u8>() < base_resource_roll {
        let all = StrategicResource::all();
        let weights: Vec<u32> = all
            .iter()
            .map(|resource| {
                strategic_resource_weight(
                    *resource,
                    context,
                    is_hazardous,
                    has_precursor_signature,
                    in_nebula_band,
                )
            })
            .collect();
        if let Ok(dist) = WeightedIndex::new(&weights) {
            let selected = all[dist.sample(&mut planet_rng)];
            resources.push(selected);
            if hotspot_bias && planet_rng.random::<u8>() < 20 {
                let alt = all[dist.sample(&mut planet_rng)];
                if !resources.contains(&alt) {
                    resources.push(alt);
                }
            }
        }
    }

    PlanetDiscoveries {
        specials,
        anomalies,
        resources,
    }
}

/// Context-aware deterministic resource generation.
pub fn generate_planet_specials_and_resources_for_context(
    galaxy_seed: u64,
    star_id: StarId,
    planet_index: usize,
    context: ResourceGenerationContext,
) -> (Vec<PlanetSpecial>, Vec<StrategicResource>) {
    let discoveries =
        generate_planet_discoveries_for_context(galaxy_seed, star_id, planet_index, context);
    (discoveries.specials, discoveries.resources)
}

/// Backward-compatible API used by older migration/tests.
///
/// Uses deterministic synthetic context derived from inputs only.
pub fn generate_planet_specials_and_resources(
    galaxy_seed: u64,
    star_id: StarId,
    planet_index: usize,
) -> (Vec<PlanetSpecial>, Vec<StrategicResource>) {
    // Synthetic context uses fixed coprime strides for deterministic compatibility generation.
    let class = PlanetClass::all()
        [(star_id.0 as usize + planet_index * SYNTHETIC_CLASS_STRIDE) % PlanetClass::all().len()];
    let spectral = SpectralClass::all()[(star_id.0 as usize
        + planet_index * SYNTHETIC_SPECTRAL_STRIDE)
        % SpectralClass::all().len()];
    let sector = SectorId((star_id.0 / SYNTHETIC_SECTOR_DIVISOR) % SYNTHETIC_SECTOR_MODULUS);
    let x = ((star_id.0 as i32 * SYNTHETIC_X_STAR_MULT
        + planet_index as i32 * SYNTHETIC_X_PLANET_MULT)
        % SYNTHETIC_COORD_MODULUS)
        - SYNTHETIC_COORD_OFFSET;
    let y = ((star_id.0 as i32 * SYNTHETIC_Y_STAR_MULT
        + planet_index as i32 * SYNTHETIC_Y_PLANET_MULT)
        % SYNTHETIC_COORD_MODULUS)
        - SYNTHETIC_COORD_OFFSET;
    let discoveries = generate_planet_discoveries_for_context(
        galaxy_seed,
        star_id,
        planet_index,
        ResourceGenerationContext {
            planet_class: class,
            spectral_class: spectral,
            sector_id: sector,
            star_x: x,
            star_y: y,
        },
    );
    (discoveries.specials, discoveries.resources)
}

/// Generate a galaxy with the given seed and star count
pub fn generate_galaxy(seed: u64, star_count: usize) -> GeneratedGalaxy {
    let star_count = star_count.clamp(10, 100);
    // Derive sector count from star count (roughly 1 sector per 10 stars, min 2, max 8)
    let sector_count = ((star_count as f64 / 10.0).ceil() as usize).clamp(2, 8);
    generate_galaxy_with_config(seed, star_count, sector_count)
}

/// Generate a galaxy with explicit star count and sector count.
///
/// `star_count` is clamped to `10..=100`; `sector_count` to `2..=8`.
pub fn generate_galaxy_with_config(
    seed: u64,
    star_count: usize,
    sector_count: usize,
) -> GeneratedGalaxy {
    let mut rng = SeededRng::new(seed);
    let star_count = star_count.clamp(10, 2000);
    let sector_count = sector_count.clamp(2, 20);

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

    // Scale the coordinate range with galaxy size so that larger galaxies
    // have room to place stars without excessive collisions.  Tiny (40)
    // gets the base 500 range; each additional star adds ~0.02 units.
    let coord_range: i32 = 250 + (star_count as i32).saturating_mul(2).min(3000);
    for id in 0..star_count {
        // Generate unique coordinates
        let (x, y) = loop {
            let x = rng.random_range(-coord_range..=coord_range);
            let y = rng.random_range(-coord_range..=coord_range);
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
        let planet_count = rng.random_range(1..=4);
        let planets: Vec<Planet> = (0..planet_count)
            .map(|i| {
                let planet_name = format!("{} {}", name, ROMAN_NUMERALS[i]);
                let size = *PlanetSize::all().choose(&mut rng).unwrap();
                // Assign class deterministically based on star_id + planet index
                // to avoid consuming extra RNG calls that would break fixed-seed tests
                let class_idx = (id * 37 + i * 11) % PlanetClass::all().len();
                let class = PlanetClass::all()[class_idx];
                let discoveries = generate_planet_discoveries_for_context(
                    seed,
                    StarId(id as u64),
                    i,
                    ResourceGenerationContext {
                        planet_class: class,
                        spectral_class,
                        sector_id: SectorId(sector_id as u64),
                        star_x: x,
                        star_y: y,
                    },
                );
                Planet {
                    name: planet_name,
                    size,
                    class,
                    colony: None,
                    habitable: true,
                    surveyed: false,
                    specials: discoveries.specials,
                    resources: discoveries.resources,
                    anomalies: discoveries.anomalies,
                    ancient_ruins_collected: false,
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
const MAX_LANE_COMPARE_STARS: usize = 30;

/// Sort stars by squared distance to a reference coordinate.
fn stars_nearby_sorted<'a>(
    stars: &[&'a Star],
    cx: i32,
    cy: i32,
) -> Vec<&'a Star> {
    let mut sorted: Vec<_> = stars.to_vec();
    sorted.sort_by_key(|s| {
        let dx = (s.x - cx) as i64;
        let dy = (s.y - cy) as i64;
        dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy))
    });
    sorted.truncate(MAX_LANE_COMPARE_STARS);
    sorted
}

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

    // One closest pair per sector.  For large sectors (≥30 stars)
    // we only examine the 30 closest stars to the sector center so
    // the O(n²) comparison cost stays bounded.  This preserves the
    // existing connectivity pattern for small galaxies while keeping
    // 700-star Epics fast.
    for (&sector_id, stars_in_sector) in &stars_by_sector {
        let center = sectors.iter().find(|s| s.id == sector_id);
        let (cx, cy) = center.map(|s| (s.x, s.y)).unwrap_or((0, 0));
        let candidates = stars_nearby_sorted(stars_in_sector, cx, cy);

        let mut best: Option<(i64, StarId, StarId)> = None;
        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                let a = candidates[i];
                let b = candidates[j];
                let dx = (a.x - b.x) as i64;
                let dy = (a.y - b.y) as i64;
                let sq = dx * dx + dy * dy;
                let candidate = (sq, a.id.min(b.id), a.id.max(b.id));
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }
        if let Some((_, a, b)) = best
            && let Some(lane) = HyperspaceLane::new(a, b)
        {
            lanes.insert(lane);
        }
    }

    // One closest pair per adjacent sector pair.
    // Each side is also limited to the 30 stars nearest its sector
    // center to avoid O(n·m) blowup on large galaxies.
    for (sa, sb) in adjacent_sector_pairs(sectors) {
        let Some(a_stars) = stars_by_sector.get(&sa) else {
            continue;
        };
        let Some(b_stars) = stars_by_sector.get(&sb) else {
            continue;
        };

        let a_center = sectors.iter().find(|s| s.id == sa);
        let b_center = sectors.iter().find(|s| s.id == sb);
        let (acx, acy) = a_center.map(|s| (s.x, s.y)).unwrap_or((0, 0));
        let (bcx, bcy) = b_center.map(|s| (s.x, s.y)).unwrap_or((0, 0));
        let a_pool = stars_nearby_sorted(a_stars, acx, acy);
        let b_pool = stars_nearby_sorted(b_stars, bcx, bcy);

        let mut best: Option<(i64, StarId, StarId)> = None;
        for a in &a_pool {
            for b in &b_pool {
                let dx = (a.x - b.x) as i64;
                let dy = (a.y - b.y) as i64;
                let sq = dx * dx + dy * dy;
                let base = HyperspaceLane::new(a.id, b.id).expect("distinct stars");
                let tie_break = ((seed ^ ((base.a().0 << 32) | base.b().0)) & 0xFFFF) as i64;
                let candidate = (sq * 65_536 + tie_break, base.a(), base.b());
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }

        if let Some((_, a, b)) = best
            && let Some(lane) = HyperspaceLane::new(a, b)
        {
            lanes.insert(lane);
        }
    }

    lanes.into_iter().collect()
}

fn adjacent_sector_pairs(sectors: &[Sector]) -> Vec<(SectorId, SectorId)> {
    // Sector centers are generated on a deterministic sparse grid where
    // immediate neighbors are roughly 400-600 units apart and non-neighbors are larger.
    // Treating <=600 as adjacent keeps links local without over-connecting distant sectors.
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
            assert!(star_ids.contains(&lane.a()));
            assert!(star_ids.contains(&lane.b()));
            assert_ne!(lane.a(), lane.b());
        }
    }

    // ── Planet specials and strategic resources ─────────────────────────────

    #[test]
    fn planet_specials_generation_is_reproducible() {
        // Same seed always produces the same specials and resources for every planet.
        let (specials_a, resources_a) = generate_planet_specials_and_resources(42, StarId(3), 0);
        let (specials_b, resources_b) = generate_planet_specials_and_resources(42, StarId(3), 0);
        assert_eq!(specials_a, specials_b, "specials must be deterministic");
        assert_eq!(resources_a, resources_b, "resources must be deterministic");
        let discoveries_a = generate_planet_discoveries_for_context(
            42,
            StarId(3),
            0,
            ResourceGenerationContext {
                planet_class: PlanetClass::Frozen,
                spectral_class: SpectralClass::A,
                sector_id: SectorId(1),
                star_x: 200,
                star_y: -160,
            },
        );
        let discoveries_b = generate_planet_discoveries_for_context(
            42,
            StarId(3),
            0,
            ResourceGenerationContext {
                planet_class: PlanetClass::Frozen,
                spectral_class: SpectralClass::A,
                sector_id: SectorId(1),
                star_x: 200,
                star_y: -160,
            },
        );
        assert_eq!(
            discoveries_a, discoveries_b,
            "discoveries must be deterministic"
        );
    }

    #[test]
    fn planet_specials_can_differ_by_seed() {
        // Different seeds should produce at least some variation across all planets.
        let gal1 = generate_galaxy(42, 30);
        let gal2 = generate_galaxy(9999, 30);
        let same_count = gal1
            .stars
            .iter()
            .zip(gal2.stars.iter())
            .flat_map(|(s1, s2)| s1.planets.iter().zip(s2.planets.iter()))
            .filter(|(p1, p2)| p1.specials == p2.specials && p1.resources == p2.resources)
            .count();
        let total = gal1.stars.iter().map(|s| s.planets.len()).sum::<usize>();
        // Not all planets may differ, but at least half should
        assert!(
            same_count < total,
            "different seeds should produce different specials on at least some planets"
        );
    }

    #[test]
    fn planet_specials_differ_by_planet_index() {
        // Verify that planet_index is meaningfully incorporated into the seed by checking
        // that across a range of indices at least two distinct (specials, resources) pairs
        // are produced.  If all 10 outputs were identical it would mean planet_index has no
        // effect on the sub-RNG seed.
        let seed = 42u64;
        let star_id = StarId(5);
        let outputs: Vec<_> = (0..10)
            .map(|i| generate_planet_specials_and_resources(seed, star_id, i))
            .collect();
        // Count how many outputs differ from the first one.
        let first = &outputs[0];
        let distinct_count = outputs.iter().filter(|o| *o != first).count();
        assert!(
            distinct_count > 0,
            "planet_index must produce variation: all 10 planet indices for \
             seed={}, star_id={} produced identical (specials, resources) output",
            seed,
            star_id.0
        );
    }

    #[test]
    fn galaxy_generation_includes_planet_specials() {
        // All generated planets must have specials/resources fields (even if empty).
        let gal = generate_galaxy(42, 20);
        let mut any_specials = false;
        let mut any_anomalies = false;
        let mut any_resources = false;
        for star in &gal.stars {
            for planet in &star.planets {
                if !planet.specials.is_empty() {
                    any_specials = true;
                }
                if !planet.anomalies.is_empty() {
                    any_anomalies = true;
                }
                if !planet.resources.is_empty() {
                    any_resources = true;
                }
                // Fields must exist and ancient_ruins_collected always starts false
                assert!(!planet.ancient_ruins_collected);
            }
        }
        // With a 40% special rate over many planets, at least some should appear.
        assert!(any_specials, "at least some planets should have specials");
        assert!(any_anomalies, "at least some planets should have anomalies");
        // With a 30% resource rate over many planets, at least some should appear.
        assert!(any_resources, "at least some planets should have resources");
    }

    #[test]
    fn common_specials_and_uncommon_anomalies_outnumber_legendary_findings() {
        let gal = generate_galaxy(1337, 100);
        let mut common_specials = 0usize;
        let mut legendary_specials = 0usize;
        let mut uncommon_anomalies = 0usize;
        let mut legendary_anomalies = 0usize;

        for star in &gal.stars {
            for planet in &star.planets {
                for special in &planet.specials {
                    match special.rarity() {
                        DiscoveryRarity::Common => common_specials += 1,
                        DiscoveryRarity::Legendary => legendary_specials += 1,
                        _ => {}
                    }
                }
                for anomaly in &planet.anomalies {
                    match anomaly.rarity() {
                        DiscoveryRarity::Uncommon => uncommon_anomalies += 1,
                        DiscoveryRarity::Legendary => legendary_anomalies += 1,
                        _ => {}
                    }
                }
            }
        }

        assert!(common_specials > legendary_specials);
        assert!(uncommon_anomalies >= legendary_anomalies);
    }

    #[test]
    fn common_resources_are_more_frequent_than_rare_tiers() {
        let gal = generate_galaxy(4242, 80);
        let mut common = 0usize;
        let mut legendary = 0usize;

        for star in &gal.stars {
            for planet in &star.planets {
                for resource in &planet.resources {
                    match resource.rarity() {
                        crate::state::StrategicResourceRarity::Common => common += 1,
                        crate::state::StrategicResourceRarity::Uncommon => {}
                        crate::state::StrategicResourceRarity::Rare => {}
                        crate::state::StrategicResourceRarity::Legendary => legendary += 1,
                    }
                }
            }
        }

        assert!(
            common > 0,
            "common resources should appear in generated galaxies"
        );
        assert!(
            common > legendary,
            "common resources should appear more often than legendary resources"
        );
    }

    #[test]
    fn hostile_weather_is_a_valid_special() {
        // Verify HostileWeather (a negative special) is included in the valid set.
        assert!(
            PlanetSpecial::all().contains(&PlanetSpecial::HostileWeather),
            "HostileWeather must be in all()"
        );
        let effect = PlanetSpecial::HostileWeather.yield_effect();
        assert!(
            effect.food < 0,
            "HostileWeather should impose a food penalty"
        );
        assert!(
            effect.industry < 0,
            "HostileWeather should impose an industry penalty"
        );
    }
}
