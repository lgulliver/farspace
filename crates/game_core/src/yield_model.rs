//! Colony yield model — deterministic economic output calculations
//!
//! # Colony Yield Model v2
//!
//! Each turn a colony produces the following resources, derived from its
//! population, installed buildings, orbital structures, colony stability, and
//! the class of the planet it occupies:
//!
//! * **Industry** — the effective production base:
//!   `population + FabricationYard × 2 + stability_modifier`
//!   where `stability_modifier = (stability − 100) / 10` (0 at neutral 100).
//!
//! * **Credits** — `industry × prod_pct / 100`
//!
//! * **Science** — `industry × research_pct / 100 + ScienceNexus × population
//!   + planet_science_bonus`
//!
//! * **Food** — `population + planet_food_bonus + AquacultureBay × population`
//!
//! * **Food consumed** — `population` (always equals base population)
//!
//! * **Maintenance** — sum of per-building and per-orbital-structure costs

use crate::state::{planet_yield_effect, BuildingType, Colony, Planet}; // ColonyRole applied via colony.role.modifiers()

/// Computed economic yields for a colony in a single turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColonyYield {
    /// Effective industrial output this turn (population + building bonuses + stability modifier + role modifier).
    pub industry: i64,
    /// Credits generated this turn (`industry × prod_pct / 100` + role flat modifier).
    pub credits: i64,
    /// Science generated this turn (`industry × research_pct / 100 + ScienceNexus bonus + planet bonus` + role flat modifier).
    pub science: i64,
    /// Food produced this turn (`population + planet bonus + AquacultureBay × population` + role flat modifier).
    pub food: i64,
    /// Food consumed this turn (= population).
    pub food_consumed: i64,
    /// Total credit maintenance cost this turn (surface buildings + orbital structures + role surcharge).
    pub maintenance: i64,
}

/// Calculate colony yields for a single turn.
///
/// `planet` should be the planet this colony occupies.  When `None` is passed
/// all planet-class bonuses default to zero, which is correct both in isolated
/// unit tests and in any production path where a planet lookup fails (graceful
/// degradation rather than a panic).
///
/// Planet specials and strategic resources are applied automatically when the
/// planet has been surveyed (visibility gate) and a colony exists on it.
pub fn calculate_yield(colony: &Colony, planet: Option<&Planet>) -> ColonyYield {
    let pop = colony.population as i64;
    let buildings = &colony.buildings;
    let orbitals = &colony.orbital_installations;
    let role_mod = colony.role.modifiers();

    // Compute total special/resource yield effect (zero when planet is unsurveyed or absent).
    let special_effect = planet.map(planet_yield_effect).unwrap_or_default();

    // Industry = population + FabricationYard × 2 + stability modifier + role modifier + specials.
    // stability_modifier = (stability − 100) / 10; clamped at 0 to prevent
    // negative industry on heavily destabilised colonies.
    let fabrication_bonus: i64 = buildings
        .iter()
        .filter(|b| **b == BuildingType::FabricationYard)
        .count() as i64
        * 2;
    let stability_mod = (colony.stability as i64 - 100) / 10;
    let industry =
        (pop + fabrication_bonus + stability_mod + role_mod.industry + special_effect.industry)
            .max(0);

    // Credits = industry × prod_pct / 100 + role flat modifier + special flat modifier.
    let credits =
        ((industry * colony.prod_pct as i64) / 100 + role_mod.credits + special_effect.credits)
            .max(0);

    // Science = industry × research_pct / 100 + ScienceNexus × population + planet bonus
    //         + role flat modifier + special flat modifier.
    let nexus_count = buildings
        .iter()
        .filter(|b| **b == BuildingType::ScienceNexus)
        .count() as i64;
    let planet_science_bonus = planet.map(|p| p.class.science_bonus()).unwrap_or(0);
    let science = ((industry * colony.research_pct as i64) / 100
        + nexus_count * pop
        + planet_science_bonus
        + role_mod.science
        + special_effect.science)
        .max(0);

    // Food = population + planet food bonus + AquacultureBay × population
    //      + role flat modifier + special flat modifier.
    let aqua_count = buildings
        .iter()
        .filter(|b| **b == BuildingType::AquacultureBay)
        .count() as i64;
    let planet_food_bonus = planet.map(|p| p.class.food_bonus()).unwrap_or(0);
    let food = pop + planet_food_bonus + aqua_count * pop + role_mod.food + special_effect.food;
    let food_consumed = pop;

    // Maintenance = sum of building costs + sum of orbital structure costs
    //             + role surcharge + special delta (negative delta reduces maintenance).
    let building_maint: i64 = buildings.iter().map(|b| b.maintenance_cost()).sum();
    let orbital_maint: i64 = orbitals.iter().map(|o| o.maintenance_cost()).sum();
    let maintenance =
        (building_maint + orbital_maint + role_mod.maintenance + special_effect.maintenance).max(0);

    ColonyYield {
        industry,
        credits,
        science,
        food,
        food_consumed,
        maintenance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        BuildingType, Colony, ColonyId, EmpireId, OrbitalStructureType, Planet, PlanetClass,
        PlanetSize, StarId,
    };

    fn base_colony() -> Colony {
        Colony {
            id: ColonyId(1),
            star: StarId(1),
            planet_index: 0,
            owner: EmpireId(1),
            population: 10,
            production: 10,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 100,
            role: crate::state::ColonyRole::Balanced,
        }
    }

    fn terran_planet() -> Planet {
        Planet {
            name: "Test".to_string(),
            size: PlanetSize::Medium,
            class: PlanetClass::Terran,
            colony: Some(ColonyId(1)),
            habitable: true,
            surveyed: true,
            specials: vec![],
            resources: vec![],
            ancient_ruins_collected: false,
        }
    }

    // ── Positive paths ─────────────────────────────────────────────────────

    #[test]
    fn base_colony_yield_no_buildings() {
        // population=10, no buildings, stability=100, 50/50 focus, Terran planet
        let colony = base_colony();
        let planet = terran_planet();
        let y = calculate_yield(&colony, Some(&planet));

        assert_eq!(
            y.industry, 10,
            "industry should equal population at neutral stability"
        );
        assert_eq!(y.credits, 5, "credits = 10 * 50 / 100 = 5");
        assert_eq!(y.science, 5, "science = 10 * 50 / 100 = 5");
        assert_eq!(y.food, 10, "food = population with no bonuses on Terran");
        assert_eq!(
            y.food_consumed, 10,
            "food_consumed always equals population"
        );
        assert_eq!(y.maintenance, 0, "no buildings = no maintenance");
    }

    #[test]
    fn fabrication_yard_boosts_industry() {
        let mut colony = base_colony();
        colony.buildings = vec![BuildingType::FabricationYard];
        let y = calculate_yield(&colony, None);

        // industry = 10 + 2 = 12; credits = 12 * 50 / 100 = 6
        assert_eq!(y.industry, 12);
        assert_eq!(y.credits, 6);
        assert_eq!(y.maintenance, 1, "FabricationYard costs 1 cr/turn");
    }

    #[test]
    fn science_nexus_adds_population_to_science() {
        let mut colony = base_colony();
        colony.buildings = vec![BuildingType::ScienceNexus];
        let planet = terran_planet();
        let y = calculate_yield(&colony, Some(&planet));

        // science = 10 * 50 / 100 + 1 * 10 = 5 + 10 = 15
        assert_eq!(y.science, 15);
        assert_eq!(y.maintenance, 1, "ScienceNexus costs 1 cr/turn");
    }

    #[test]
    fn aquaculture_bay_doubles_food() {
        let mut colony = base_colony();
        colony.buildings = vec![BuildingType::AquacultureBay];
        let y = calculate_yield(&colony, None);

        // food = 10 + 0 + 1 * 10 = 20
        assert_eq!(y.food, 20);
        assert_eq!(y.maintenance, 0, "AquacultureBay has no maintenance cost");
    }

    #[test]
    fn stability_above_100_boosts_industry() {
        let mut colony = base_colony();
        colony.stability = 110; // stability_mod = (110 - 100) / 10 = 1
        let y = calculate_yield(&colony, None);

        assert_eq!(y.industry, 11, "industry = 10 + 0 + 1 = 11");
        assert_eq!(
            y.credits, 5,
            "credits = 11 * 50 / 100 = 5 (integer division)"
        );
    }

    #[test]
    fn oceanic_planet_adds_food_bonus() {
        let colony = base_colony();
        let planet = Planet {
            name: "Wet World".to_string(),
            size: PlanetSize::Medium,
            class: PlanetClass::Oceanic,
            colony: Some(ColonyId(1)),
            habitable: true,
            surveyed: true,
            specials: vec![],
            resources: vec![],
            ancient_ruins_collected: false,
        };
        let y = calculate_yield(&colony, Some(&planet));

        assert_eq!(y.food, 12, "food = 10 + 2 (Oceanic bonus) = 12");
    }

    #[test]
    fn frozen_planet_adds_science_bonus() {
        let colony = base_colony();
        let planet = Planet {
            name: "Ice World".to_string(),
            size: PlanetSize::Medium,
            class: PlanetClass::Frozen,
            colony: Some(ColonyId(1)),
            habitable: true,
            surveyed: true,
            specials: vec![],
            resources: vec![],
            ancient_ruins_collected: false,
        };
        let y = calculate_yield(&colony, Some(&planet));

        assert_eq!(y.science, 6, "science = 5 + 0 + 1 (Frozen bonus) = 6");
        assert_eq!(y.food, 9, "food = 10 - 1 (Frozen penalty) = 9");
    }

    #[test]
    fn shipyard_orbital_adds_maintenance() {
        let mut colony = base_colony();
        colony.orbital_installations = vec![OrbitalStructureType::Shipyard];
        let y = calculate_yield(&colony, None);

        assert_eq!(y.maintenance, 2, "Shipyard costs 2 cr/turn");
    }

    #[test]
    fn combined_buildings_and_orbitals_maintenance() {
        let mut colony = base_colony();
        colony.buildings = vec![BuildingType::FabricationYard, BuildingType::ScienceNexus];
        colony.orbital_installations = vec![OrbitalStructureType::Shipyard];
        let y = calculate_yield(&colony, None);

        // maintenance = FabricationYard(1) + ScienceNexus(1) + Shipyard(2) = 4
        assert_eq!(y.maintenance, 4);
        // industry = 10 + 2 = 12 (FabricationYard adds +2, stability neutral)
        assert_eq!(y.industry, 12);
    }

    #[test]
    fn no_planet_uses_zero_bonuses() {
        let colony = base_colony();
        let y_no_planet = calculate_yield(&colony, None);
        let y_terran = calculate_yield(&colony, Some(&terran_planet()));

        // Terran has 0 bonuses, so both should match
        assert_eq!(y_no_planet, y_terran);
    }

    // ── Negative / edge-case paths ──────────────────────────────────────────

    #[test]
    fn stability_below_100_reduces_industry_clamped_at_zero() {
        let mut colony = base_colony();
        colony.population = 1; // very small colony
        colony.stability = 80; // stability_mod = (80 - 100) / 10 = -2
        let y = calculate_yield(&colony, None);

        // industry = 1 + 0 + (-2) = -1, clamped to 0
        assert_eq!(y.industry, 0, "industry cannot go below 0");
        assert_eq!(y.credits, 0);
        assert_eq!(y.science, 0);
    }

    #[test]
    fn barren_planet_penalises_food() {
        let colony = base_colony();
        let planet = Planet {
            name: "Rock".to_string(),
            size: PlanetSize::Medium,
            class: PlanetClass::Barren,
            colony: Some(ColonyId(1)),
            habitable: false,
            surveyed: true,
            specials: vec![],
            resources: vec![],
            ancient_ruins_collected: false,
        };
        let y = calculate_yield(&colony, Some(&planet));

        // food = 10 - 2 (Barren) = 8
        assert_eq!(y.food, 8);
    }

    #[test]
    fn hundred_percent_production_focus_gives_zero_science() {
        let mut colony = base_colony();
        colony.prod_pct = 100;
        colony.research_pct = 0;
        let y = calculate_yield(&colony, None);

        assert_eq!(y.credits, 10, "all industry converted to credits");
        assert_eq!(y.science, 0, "no research focus → no base science");
    }

    #[test]
    fn hundred_percent_research_focus_gives_zero_credits() {
        let mut colony = base_colony();
        colony.prod_pct = 0;
        colony.research_pct = 100;
        let y = calculate_yield(&colony, None);

        assert_eq!(y.credits, 0, "no production focus → no credits");
        assert_eq!(y.science, 10, "all industry converted to science");
    }

    #[test]
    fn food_consumed_always_equals_population() {
        let mut colony = base_colony();
        colony.population = 7;
        colony.buildings = vec![BuildingType::AquacultureBay, BuildingType::AquacultureBay];
        let y = calculate_yield(&colony, None);

        assert_eq!(
            y.food_consumed, 7,
            "consumed equals population regardless of buildings"
        );
        assert_eq!(y.food, 7 + 7 + 7, "food = pop + 0 + 2*AquacultureBay*pop");
    }

    // ── Colony role modifier tests ──────────────────────────────────────────

    #[test]
    fn balanced_role_applies_no_modifiers() {
        use crate::state::ColonyRole;
        let mut colony_balanced = base_colony();
        colony_balanced.role = ColonyRole::Balanced;
        let mut colony_default = base_colony();
        colony_default.role = ColonyRole::Balanced;

        let y_balanced = calculate_yield(&colony_balanced, None);
        let y_default = calculate_yield(&colony_default, None);

        // Both must be identical and match the expected base yield
        assert_eq!(y_balanced, y_default, "Balanced must not alter base yields");
        assert_eq!(y_balanced.food, 10, "Balanced: food = pop");
        assert_eq!(y_balanced.industry, 10, "Balanced: industry = pop");
        assert_eq!(y_balanced.credits, 5, "Balanced: credits = 10*50/100");
        assert_eq!(y_balanced.science, 5, "Balanced: science = 10*50/100");
        assert_eq!(y_balanced.maintenance, 0, "Balanced: no maintenance");
    }

    #[test]
    fn agricultural_role_boosts_food_reduces_industry() {
        use crate::state::ColonyRole;
        let mut colony = base_colony();
        colony.role = ColonyRole::Agricultural;
        let y = calculate_yield(&colony, None);

        // industry = 10 - 1 = 9; food = 10 + 2 = 12
        assert_eq!(y.industry, 9, "Agricultural: industry = pop - 1");
        assert_eq!(y.food, 12, "Agricultural: food = pop + 2");
        // credits and science are scaled from reduced industry
        assert_eq!(y.credits, 4, "Agricultural: credits = 9*50/100 = 4");
        assert_eq!(y.science, 4, "Agricultural: science = 9*50/100 = 4");
        assert_eq!(y.maintenance, 0, "Agricultural: no maintenance surcharge");
    }

    #[test]
    fn industrial_role_boosts_industry_reduces_science() {
        use crate::state::ColonyRole;
        let mut colony = base_colony();
        colony.role = ColonyRole::Industrial;
        let y = calculate_yield(&colony, None);

        // industry = 10 + 2 = 12; science = 12*50/100 - 1 = 6 - 1 = 5
        assert_eq!(y.industry, 12, "Industrial: industry = pop + 2");
        assert_eq!(y.credits, 6, "Industrial: credits = 12*50/100 = 6");
        assert_eq!(y.science, 5, "Industrial: science = 12*50/100 - 1 = 5");
        assert_eq!(y.food, 10, "Industrial: food unchanged");
        assert_eq!(y.maintenance, 0, "Industrial: no maintenance surcharge");
    }

    #[test]
    fn scientific_role_boosts_science_reduces_credits() {
        use crate::state::ColonyRole;
        let mut colony = base_colony();
        colony.role = ColonyRole::Scientific;
        let y = calculate_yield(&colony, None);

        // industry = 10; science = 10*50/100 + 2 = 7; credits = 10*50/100 - 1 = 4
        assert_eq!(y.industry, 10, "Scientific: industry unchanged");
        assert_eq!(y.science, 7, "Scientific: science = 5 + 2 = 7");
        assert_eq!(y.credits, 4, "Scientific: credits = 5 - 1 = 4");
        assert_eq!(y.food, 10, "Scientific: food unchanged");
        assert_eq!(y.maintenance, 0, "Scientific: no maintenance surcharge");
    }

    #[test]
    fn financial_role_boosts_credits_reduces_industry() {
        use crate::state::ColonyRole;
        let mut colony = base_colony();
        colony.role = ColonyRole::Financial;
        let y = calculate_yield(&colony, None);

        // industry = 10 - 1 = 9; credits = 9*50/100 + 2 = 4 + 2 = 6
        assert_eq!(y.industry, 9, "Financial: industry = pop - 1");
        assert_eq!(y.credits, 6, "Financial: credits = 4 + 2 = 6");
        assert_eq!(y.science, 4, "Financial: science = 9*50/100 = 4");
        assert_eq!(y.food, 10, "Financial: food unchanged");
        assert_eq!(y.maintenance, 0, "Financial: no maintenance surcharge");
    }

    #[test]
    fn military_role_increases_maintenance_no_yield_change() {
        use crate::state::ColonyRole;
        let mut colony = base_colony();
        colony.role = ColonyRole::Military;
        let y = calculate_yield(&colony, None);

        // Military does not modify food/industry/credits/science
        assert_eq!(y.industry, 10, "Military: industry unchanged");
        assert_eq!(y.credits, 5, "Military: credits unchanged");
        assert_eq!(y.science, 5, "Military: science unchanged");
        assert_eq!(y.food, 10, "Military: food unchanged");
        assert_eq!(y.maintenance, 1, "Military: maintenance +1 surcharge");
    }

    #[test]
    fn role_modifiers_are_deterministic() {
        use crate::state::ColonyRole;
        // Running calculate_yield twice with the same state must produce identical results.
        let mut colony = base_colony();
        colony.role = ColonyRole::Industrial;
        let y1 = calculate_yield(&colony, None);
        let y2 = calculate_yield(&colony, None);
        assert_eq!(y1, y2, "yield calculation must be deterministic");
    }

    #[test]
    fn credits_clamped_at_zero_for_scientific_role_with_no_production() {
        use crate::state::ColonyRole;
        // Edge case: colony with minimal industry and Scientific role getting -1 credits
        let mut colony = base_colony();
        colony.population = 1;
        colony.prod_pct = 100;
        colony.research_pct = 0;
        colony.role = ColonyRole::Scientific;
        let y = calculate_yield(&colony, None);
        // credits = 1*100/100 - 1 = 0 (clamped at 0)
        assert_eq!(y.credits, 0, "credits must not go below 0");
        // science = 1*0/100 + 2 = 2
        assert_eq!(y.science, 2);
    }

    // ── Planet specials and strategic resources ──────────────────────────────

    fn planet_with_specials(
        specials: Vec<crate::state::PlanetSpecial>,
        resources: Vec<crate::state::StrategicResource>,
        surveyed: bool,
    ) -> Planet {
        Planet {
            name: "Test".to_string(),
            size: PlanetSize::Medium,
            class: PlanetClass::Terran,
            colony: Some(ColonyId(1)),
            habitable: true,
            surveyed,
            specials,
            resources,
            ancient_ruins_collected: false,
        }
    }

    #[test]
    fn unsurveyed_planet_hides_special_effects() {
        use crate::state::PlanetSpecial;
        // Mineral Rich normally gives +2 industry; must be hidden when not surveyed.
        let colony = base_colony();
        let unsurveyed = planet_with_specials(vec![PlanetSpecial::MineralRich], vec![], false);
        let y = calculate_yield(&colony, Some(&unsurveyed));
        assert_eq!(
            y.industry, 10,
            "MineralRich must not apply when planet is unsurveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_mineral_rich_bonus() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::MineralRich], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // industry = 10 + 2 (MineralRich) = 12
        assert_eq!(
            y.industry, 12,
            "MineralRich should add +2 industry when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_fertile_biosphere_bonus() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::FertileBiosphere], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // food = 10 + 2 (FertileBiosphere) = 12
        assert_eq!(
            y.food, 12,
            "FertileBiosphere should add +2 food when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_ancient_ruins_science_bonus() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::AncientRuins], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // science = 5 + 2 (AncientRuins) = 7
        assert_eq!(
            y.science, 7,
            "AncientRuins should add +2 science when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_crystal_formations_bonus() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::CrystalFormations], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // credits = 5 + 1 (Crystal) = 6; science = 5 + 1 (Crystal) = 6
        assert_eq!(
            y.credits, 6,
            "CrystalFormations should add +1 credits when surveyed"
        );
        assert_eq!(
            y.science, 6,
            "CrystalFormations should add +1 science when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_hostile_weather_penalties() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::HostileWeather], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // industry = 10 - 1 = 9; food = 10 - 1 = 9
        assert_eq!(
            y.industry, 9,
            "HostileWeather should apply -1 industry penalty"
        );
        assert_eq!(y.food, 9, "HostileWeather should apply -1 food penalty");
    }

    #[test]
    fn surveyed_planet_applies_low_gravity_bonus() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![PlanetSpecial::LowGravity], vec![], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // industry = 10 + 2 = 12
        assert_eq!(
            y.industry, 12,
            "LowGravity should add +2 industry when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_helium3_maintenance_reduction() {
        use crate::state::{BuildingType, StrategicResource};
        let mut colony = base_colony();
        // Add a building so maintenance > 0 before the reduction
        colony.buildings = vec![BuildingType::FabricationYard]; // maintenance = 1
        let surveyed = planet_with_specials(vec![], vec![StrategicResource::Helium3], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        // maintenance = 1 (FabricationYard) - 1 (Helium3) = 0 (clamped at 0)
        assert_eq!(
            y.maintenance, 0,
            "Helium3 should reduce maintenance by 1 (clamped at 0)"
        );
    }

    #[test]
    fn surveyed_planet_applies_rare_metals_industry_bonus() {
        use crate::state::StrategicResource;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![], vec![StrategicResource::RareMetals], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        assert_eq!(
            y.industry, 11,
            "RareMetals should add +1 industry when surveyed"
        );
    }

    #[test]
    fn surveyed_planet_applies_bio_cultures_food_bonus() {
        use crate::state::StrategicResource;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![], vec![StrategicResource::BioCultures], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        assert_eq!(y.food, 12, "BioCultures should add +2 food when surveyed");
    }

    #[test]
    fn surveyed_planet_applies_quantum_crystals_science_bonus() {
        use crate::state::StrategicResource;
        let colony = base_colony();
        let surveyed = planet_with_specials(vec![], vec![StrategicResource::QuantumCrystals], true);
        let y = calculate_yield(&colony, Some(&surveyed));
        assert_eq!(
            y.science, 7,
            "QuantumCrystals should add +2 science when surveyed"
        );
    }

    #[test]
    fn special_and_resource_effects_stack() {
        use crate::state::{PlanetSpecial, StrategicResource};
        let colony = base_colony();
        let surveyed = planet_with_specials(
            vec![PlanetSpecial::MineralRich],
            vec![StrategicResource::RareMetals],
            true,
        );
        let y = calculate_yield(&colony, Some(&surveyed));
        // industry = 10 + 2 (MineralRich) + 1 (RareMetals) = 13
        assert_eq!(
            y.industry, 13,
            "specials and resources should stack additively"
        );
    }

    #[test]
    fn special_effects_are_deterministic() {
        use crate::state::PlanetSpecial;
        let colony = base_colony();
        let planet = planet_with_specials(vec![PlanetSpecial::AncientRuins], vec![], true);
        let y1 = calculate_yield(&colony, Some(&planet));
        let y2 = calculate_yield(&colony, Some(&planet));
        assert_eq!(y1, y2, "yield with specials must be deterministic");
    }
}
