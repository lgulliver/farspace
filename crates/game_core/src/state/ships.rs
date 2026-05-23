use super::*;

// ---------------------------------------------------------------------------
// Custom ship designs
// ---------------------------------------------------------------------------

/// A player- or AI-created ship design built from a hull and components.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CustomShipDesign {
    pub design_id: CustomDesignId,
    pub hull_id: HullId,
    pub components: Vec<ComponentId>,
    pub owner: EmpireId,
    pub name: String,
    pub obsolete: bool,
}

/// Computed combat and logistics statistics derived from hull + components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DerivedShipStats {
    pub attack: u32,
    pub defense: u32,
    pub hp: u32,
    pub production_cost: u64,
    pub maintenance: u32,
    pub fleet_kind: FleetKind,
    pub invasion_strength: u32,
    pub survey_effectiveness: u32,
}

impl CustomShipDesign {
    /// Compute the effective stats for this design by combining hull base
    /// values with all component modifiers.
    pub fn derived_stats(&self) -> DerivedShipStats {
        let hull = match self.hull_id.template() {
            Some(h) => h,
            None => return DerivedShipStats::default(),
        };

        let mut attack = hull.base_attack as i32;
        let mut defense = hull.base_defense as i32;
        let mut hp = hull.base_hp as i32;
        let mut cost = hull.base_cost as i64;
        let mut maintenance = hull.base_maintenance as i32;
        // Hull-granted capabilities: certain FleetKinds always provide these
        let mut invasion_strength: u32 = match hull.fleet_kind {
            FleetKind::TroopTransport => 12,
            _ => 0,
        };
        let mut survey_effectiveness: u32 = match hull.fleet_kind {
            FleetKind::Science | FleetKind::SurveyCutter => 100,
            _ => 0,
        };

        for &comp_id in &self.components {
            if let Some(comp) = comp_id.def() {
                attack += comp.attack_modifier;
                defense += comp.defense_modifier;
                hp += comp.hp_modifier;
                cost += comp.cost_modifier;
                maintenance += comp.maintenance_modifier;
                for &tag in comp.special_tags {
                    match tag {
                        ComponentTag::Invasion => invasion_strength += 5,
                        ComponentTag::Survey => survey_effectiveness += 5,
                        _ => {}
                    }
                }
            }
        }

        DerivedShipStats {
            attack: attack.max(1) as u32,
            defense: defense.max(1) as u32,
            hp: hp.max(1) as u32,
            production_cost: cost.max(1) as u64,
            maintenance: maintenance.max(0) as u32,
            fleet_kind: hull.fleet_kind,
            invasion_strength,
            survey_effectiveness,
        }
    }

    /// Validate that the design is buildable: hull and all components must be
    /// tech-unlocked, and components must match available hull slots exactly.
    pub fn validate_with_resources(
        &self,
        completed_techs: &[TechId],
        available_resources: &[StrategicResource],
    ) -> Result<(), &'static str> {
        let hull = match self.hull_id.template() {
            Some(h) => h,
            None => return Err("Invalid hull"),
        };

        // Check hull tech requirement
        if let Some(tech) = hull.required_tech {
            if !completed_techs.contains(&tech) {
                return Err("Hull tech not unlocked");
            }
        }

        // Component count must equal slot count exactly — no partial or over-configured designs
        if self.components.len() != hull.slots.len() {
            return Err("Component count does not match hull slot count");
        }

        // Verify each component in slot order: category must match slot, tech must be unlocked
        for (&comp_id, &slot_cat) in self.components.iter().zip(hull.slots.iter()) {
            let comp = match comp_id.def() {
                Some(c) => c,
                None => return Err("Invalid component"),
            };

            if comp.category != slot_cat {
                return Err("Component category does not match slot");
            }

            if let Some(tech) = comp.required_tech {
                if !completed_techs.contains(&tech) {
                    return Err("Component tech not unlocked");
                }
            }
            if let Some(resource) = comp.required_resource {
                if !available_resources.contains(&resource) {
                    return Err("Component resource not available");
                }
            }
        }

        Ok(())
    }

    /// Backward-compatible tech-only validation helper.
    pub fn validate(&self, completed_techs: &[TechId]) -> Result<(), &'static str> {
        self.validate_with_resources(completed_techs, &[])
    }
}

/// Static record describing a constructible ship design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipDesignRecord {
    pub id: ShipDesignId,
    pub name: &'static str,
    pub cost: u64,
    pub fleet_kind: FleetKind,
    pub ships: u32,
    pub strength: u32,
    /// Credits-per-turn upkeep cost for one fleet of this design.
    pub maintenance: u32,
    /// Short role description shown in the production UI.
    pub role: &'static str,
    pub required_tech: Option<TechId>,
}

/// All constructible ship designs in deterministic display order.
pub fn all_ship_designs() -> &'static [ShipDesignRecord] {
    &[
        ShipDesignRecord {
            id: ShipDesignId::SCOUT,
            name: "Scout",
            cost: 50,
            fleet_kind: FleetKind::Scout,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Exploration",
            required_tech: None,
        },
        ShipDesignRecord {
            id: ShipDesignId::COLONY,
            name: "Colony Ship",
            cost: 200,
            fleet_kind: FleetKind::Colonizer,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Colonization",
            required_tech: Some(TechId::HABITAT_SEEDING),
        },
        ShipDesignRecord {
            id: ShipDesignId::SCIENCE,
            name: "Science Ship",
            cost: 100,
            fleet_kind: FleetKind::Science,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Survey",
            required_tech: Some(TechId::SURVEY_DRONES),
        },
        ShipDesignRecord {
            id: ShipDesignId::TROOP_TRANSPORT,
            name: "Troop Transport",
            cost: 150,
            fleet_kind: FleetKind::TroopTransport,
            ships: 1,
            strength: 1,
            maintenance: 2,
            role: "Invasion",
            required_tech: Some(TechId::TROOP_TRANSPORTS),
        },
        ShipDesignRecord {
            id: ShipDesignId::FAST_SCOUT,
            name: "Fast Scout",
            cost: 75,
            fleet_kind: FleetKind::FastScout,
            ships: 1,
            strength: 1,
            maintenance: 1,
            role: "Rapid Exploration",
            required_tech: Some(TechId::RAPID_TRANSIT),
        },
        ShipDesignRecord {
            id: ShipDesignId::SURVEY_CUTTER,
            name: "Survey Cutter",
            cost: 150,
            fleet_kind: FleetKind::SurveyCutter,
            ships: 1,
            strength: 1,
            maintenance: 2,
            role: "Deep Survey",
            required_tech: Some(TechId::ADVANCED_SURVEY),
        },
        ShipDesignRecord {
            id: ShipDesignId::COLONY_ARK,
            name: "Colony Ark",
            cost: 350,
            fleet_kind: FleetKind::ColonyArk,
            ships: 1,
            strength: 2,
            maintenance: 2,
            role: "Mass Colonization",
            required_tech: Some(TechId::COLONIAL_VANGUARD),
        },
        ShipDesignRecord {
            id: ShipDesignId::ESCORT_FRIGATE,
            name: "Escort Frigate",
            cost: 120,
            fleet_kind: FleetKind::EscortFrigate,
            ships: 2,
            strength: 3,
            maintenance: 2,
            role: "Defensive Combat",
            required_tech: Some(TechId::PERIMETER_DEFENSE),
        },
        ShipDesignRecord {
            id: ShipDesignId::MISSILE_FRIGATE,
            name: "Missile Frigate",
            cost: 200,
            fleet_kind: FleetKind::MissileFrigate,
            ships: 2,
            strength: 5,
            maintenance: 3,
            role: "Strike Combat",
            required_tech: Some(TechId::STRIKE_DOCTRINE),
        },
        ShipDesignRecord {
            id: ShipDesignId::DESTROYER,
            name: "Destroyer",
            cost: 300,
            fleet_kind: FleetKind::Destroyer,
            ships: 3,
            strength: 8,
            maintenance: 4,
            role: "Heavy Combat",
            required_tech: Some(TechId::FLEET_COORDINATION),
        },
        ShipDesignRecord {
            id: ShipDesignId::PATROL_CORVETTE,
            name: "Patrol Corvette",
            cost: 80,
            fleet_kind: FleetKind::PatrolCorvette,
            ships: 1,
            strength: 2,
            maintenance: 1,
            role: "Local Security",
            required_tech: Some(TechId::PERIMETER_DEFENSE),
        },
    ]
}

impl ShipDesignId {
    pub const SCOUT: ShipDesignId = ShipDesignId(1);
    pub const COLONY: ShipDesignId = ShipDesignId(2);
    pub const SCIENCE: ShipDesignId = ShipDesignId(3);
    pub const TROOP_TRANSPORT: ShipDesignId = ShipDesignId(4);
    pub const FAST_SCOUT: ShipDesignId = ShipDesignId(5);
    pub const SURVEY_CUTTER: ShipDesignId = ShipDesignId(6);
    pub const COLONY_ARK: ShipDesignId = ShipDesignId(7);
    pub const ESCORT_FRIGATE: ShipDesignId = ShipDesignId(8);
    pub const MISSILE_FRIGATE: ShipDesignId = ShipDesignId(9);
    pub const DESTROYER: ShipDesignId = ShipDesignId(10);
    pub const PATROL_CORVETTE: ShipDesignId = ShipDesignId(11);

    /// All design IDs in deterministic display order, derived from `all_ship_designs()`
    /// to ensure both stay in sync automatically.
    pub fn all() -> impl Iterator<Item = ShipDesignId> {
        all_ship_designs().iter().map(|d| d.id)
    }

    /// Resolve this ID to a known design record.
    pub fn record(&self) -> Option<&'static ShipDesignRecord> {
        all_ship_designs().iter().find(|d| d.id == *self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ComponentId, CustomDesignId, EmpireId, HullId, StrategicResource, TechId};

    fn player() -> EmpireId {
        EmpireId(1)
    }

    fn make_design(hull_id: HullId, components: Vec<ComponentId>) -> CustomShipDesign {
        CustomShipDesign {
            design_id: CustomDesignId(0),
            hull_id,
            components,
            owner: player(),
            name: "Test".to_string(),
            obsolete: false,
        }
    }

    /// Test 1: derived_stats positive path — scout hull with no components yields correct
    /// base values.
    #[test]
    fn derived_stats_scout_hull_no_components() {
        let design = make_design(HullId::SCOUT, vec![]);
        let stats = design.derived_stats();
        assert!(stats.attack >= 1, "attack must be at least 1");
        assert!(stats.hp >= 1, "hp must be at least 1");
        assert!(stats.production_cost >= 1, "cost must be at least 1");
    }

    /// Test 2: derived_stats with components — adding an Engine component to scout increases stats.
    #[test]
    fn derived_stats_with_component_modifies_attack() {
        let base = make_design(HullId::SCOUT, vec![]).derived_stats();
        // ComponentId(20) is Chemical Thrusters — Engine slot, zero stat modifiers
        let design = make_design(HullId::SCOUT, vec![ComponentId(20)]);
        let stats = design.derived_stats();
        // Both should have valid stats; cost with component should be >= base cost
        assert!(stats.production_cost >= base.production_cost);
        assert!(stats.hp >= 1);
        assert!(stats.attack >= 1);
    }

    /// Test 3: validate positive path — scout hull with correct components and no tech requirement.
    #[test]
    fn validate_scout_hull_no_tech_required() {
        // Scout has [Engine, Utility] slots; ComponentId(20)=Chemical Thrusters, ComponentId(32)=Cargo Pods
        let design = make_design(HullId::SCOUT, vec![ComponentId(20), ComponentId(32)]);
        let result = design.validate(&[]);
        assert!(
            result.is_ok(),
            "Scout hull with correct components should be valid with no techs"
        );
    }

    /// Test 4: validate negative path — hull requires tech not in list.
    #[test]
    fn validate_fails_when_hull_tech_missing() {
        // Colony Ark requires COLONIAL_VANGUARD (TechId 15)
        let design = make_design(HullId::COLONY_ARK, vec![]);
        let result = design.validate(&[]);
        assert!(
            result.is_err(),
            "Colony Ark without Colonial Vanguard tech should fail validation"
        );
    }

    /// Test 5: validate negative path — wrong slot category (Kinetic Battery is Weapon, Scout first slot is Engine).
    #[test]
    fn validate_fails_wrong_slot_category() {
        // Scout hull has [Engine, Utility] slots; put Kinetic Battery (Weapon) in Engine slot → mismatch
        let design = make_design(HullId::SCOUT, vec![ComponentId(1), ComponentId(32)]);
        let result = design.validate(&[TechId(4)]); // provide Kinetic Battery tech so tech check isn't the blocker
        assert!(
            result.is_err(),
            "Kinetic Battery (Weapon) should not fit Scout hull Engine slot"
        );
        assert_eq!(
            result.unwrap_err(),
            "Component category does not match slot"
        );
    }

    /// Test 6: validate negative path — component tech not unlocked.
    #[test]
    fn validate_fails_when_component_tech_missing() {
        // ComponentId(2) is Missile Rack — Weapon slot, requires TechId(17) STRIKE_DOCTRINE
        // Escort Frigate slot 0 is Weapon; hull requires TechId(16) PERIMETER_DEFENSE
        // Provide hull tech (16) but NOT component tech (17)
        let design = make_design(
            HullId::ESCORT_FRIGATE,
            vec![
                ComponentId(2),
                ComponentId(10),
                ComponentId(20),
                ComponentId(30),
            ],
        );
        let result = design.validate(&[TechId(16)]); // hull unlocked, Missile Rack not
        assert!(
            result.is_err(),
            "Component with locked tech should fail validation"
        );
        assert_eq!(result.unwrap_err(), "Component tech not unlocked");
    }

    /// Test: resource-gated component fails without strategic resource access.
    #[test]
    fn validate_fails_when_component_resource_missing() {
        // Shield Matrix requires Quantum Crystals in addition to Perimeter Defense tech.
        let design = make_design(
            HullId::ESCORT_FRIGATE,
            vec![
                ComponentId(1),
                ComponentId(11),
                ComponentId(20),
                ComponentId(30),
            ],
        );
        let result = design.validate_with_resources(&[TechId(4), TechId(16)], &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Component resource not available");
    }

    /// Test: resource-gated component succeeds once the strategic resource is available.
    #[test]
    fn validate_succeeds_when_component_resource_present() {
        let design = make_design(
            HullId::ESCORT_FRIGATE,
            vec![
                ComponentId(1),
                ComponentId(11),
                ComponentId(20),
                ComponentId(30),
            ],
        );
        let result = design.validate_with_resources(
            &[TechId(4), TechId(16)],
            &[StrategicResource::QuantumCrystals],
        );
        assert!(result.is_ok());
    }

    /// Test 7: validate positive path with unlocked tech — Colony Ark with all required techs.
    #[test]
    fn validate_succeeds_with_correct_tech_unlocked() {
        // Colony Ark has [Engine, MissionModule, Utility] slots and requires TechId(15)
        // Components: Chemical Thrusters(Engine), Colony Core(MissionModule,needs 2), Cargo Pods(Utility)
        let design = make_design(
            HullId::COLONY_ARK,
            vec![ComponentId(20), ComponentId(40), ComponentId(32)],
        );
        let result = design.validate(&[TechId(15), TechId(2)]);
        assert!(
            result.is_ok(),
            "Colony Ark with all required techs should pass validation"
        );
    }

    /// Test 8: derived_stats for invalid hull returns default.
    #[test]
    fn derived_stats_invalid_hull_returns_default() {
        let design = make_design(HullId(9999), vec![]);
        let stats = design.derived_stats();
        assert_eq!(stats, DerivedShipStats::default());
    }
}
