//! Parameters come from the registry (engine/params/m0-toy.toml), never ad hoc.

use oikos_params::Value;
use oikos_sim::params;
use oikos_sim::params::registry;
use oikos_sim::workunit::Start;
use oikos_sim::{Params, WorkUnit, WorldConfig, run_work_unit};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn defaults_from_registry_match_the_m0_values() {
    let p = Params::default();
    assert_eq!(p.period_ticks, 365);
    assert_eq!((p.growth_rate, p.sedentary_growth_rate), (0.02784, 0.02648));
    assert_eq!((p.fertility, p.sedentary_fertility), (0.0325, 0.0325));
    assert_eq!((p.capacity_min, p.capacity_max), (20, 400));
    assert_eq!(p.fission_threshold, 60);
    assert_eq!(p.fission_probability, 0.2);
    assert_eq!(p.exploration_probability, 0.3);
    assert_eq!(p.merge_threshold, 10);
    assert_eq!((p.initial_bands, p.initial_weight), (12, 30));
    assert_eq!(p.capacity_period_ticks, 3650);
    assert_eq!(
        (p.miami_npp_max, p.miami_temp_offset, p.miami_temp_slope, p.miami_precip_slope),
        (3000.0, 1.315, 0.119, 0.000664)
    );
    assert_eq!((p.npp_reference, p.forager_density_ref, p.npp_elasticity), (1000.0, 0.183, 1.219));
    assert_eq!((p.aquatic_bonus, p.perennial_discharge), (0.5, 1.0));
    assert_eq!((p.dispersal_crowding, p.dispersal_probability, p.dispersal_fraction), (0.5007, 0.02443, 0.5));
}

#[test]
fn registry_declares_exactly_the_engine_parameters() {
    let fields: Vec<String> = match serde_json::to_value(Params::default()).unwrap() {
        serde_json::Value::Object(m) => m.keys().cloned().collect(),
        _ => unreachable!(),
    };
    let keys: Vec<String> = registry().keys().map(String::from).collect();
    assert_eq!(fields, keys);
}

#[test]
fn overrides_are_checked_against_the_registry() {
    let mut o = BTreeMap::new();
    o.insert("growth_rate".to_string(), Value::Real(0.02));
    assert_eq!(Params::from_registry(registry(), &o).unwrap().growth_rate, 0.02);

    o.insert("growth_rate".to_string(), Value::Real(5.0));
    let e = Params::from_registry(registry(), &o).unwrap_err();
    assert!(e.contains("growth_rate"), "{e}");
}

#[test]
fn out_of_range_params_are_rejected_by_validate() {
    let p = Params { fission_probability: 1.5, ..Params::default() };
    assert!(p.validate().unwrap_err().contains("fission_probability"));
    let p = Params { capacity_min: 500, capacity_max: 400, ..Params::default() };
    assert!(p.validate().unwrap_err().contains("capacity_min"));
}

#[test]
fn work_unit_with_out_of_range_params_fails() {
    let wu = WorkUnit {
        format: 1,
        id: None,
        world: WorldConfig::SOUTHWEST_ASIA,
        seed: 1,
        params: Some(Params { growth_rate: 0.5, ..Params::default() }),
        start: Start::Initial,
        years: 10,
        partitions: 1,
        summary_every_years: 10,
        checkpoint_out: None,
        outputs: None,
        activity: None,
        inputs: vec![],
    };
    let e = run_work_unit(&wu, |p| Err(p.into()), |_, _| Ok(())).unwrap_err();
    assert!(e.contains("growth_rate"), "{e}");
}

#[test]
fn every_parameter_is_in_the_first_checkpoints_or_restores_by_a_rule() {
    // A checkpoint stores every parameter its engine had, so one lacking a parameter
    // predates it: each parameter added since the oldest restorable format needs a rule.
    let keys: BTreeSet<&str> = registry().keys().collect();
    let first: BTreeSet<&str> = params::IN_FIRST_CHECKPOINTS.iter().copied().collect();
    let added: BTreeSet<&str> = params::ADDED_SINCE_CHECKPOINTS.iter().map(|(k, _)| *k).collect();
    assert!(first.is_disjoint(&added));
    let covered: BTreeSet<&str> = first.union(&added).copied().collect();
    let missing: Vec<_> = keys.difference(&covered).collect();
    assert!(missing.is_empty(), "add a Restore rule to ADDED_SINCE_CHECKPOINTS for {missing:?}");
    assert_eq!(covered, keys, "rules for parameters the registry no longer has");
}

#[test]
fn checkpoints_from_before_each_added_parameter_restore_the_model_they_were_written_with() {
    // The parameters of the first checkpoints (OIKOSCK2, ADR-0049), with a forager growth
    // rate of their own.
    let first: serde_json::Map<String, serde_json::Value> =
        match serde_json::to_value(Params::default()).unwrap() {
            serde_json::Value::Object(m) => {
                m.into_iter().filter(|(k, _)| params::IN_FIRST_CHECKPOINTS.contains(&k.as_str())).collect()
            }
            _ => unreachable!(),
        };
    let mut old = first.clone();
    old.insert("growth_rate".into(), serde_json::json!(0.017));
    let p = Params::from_checkpoint(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert_eq!(p.growth_rate, 0.017);
    assert_eq!(p.sedentary_growth_rate, 0.017, "before ADR-0053 every band grew at the forager rate");
    assert_eq!(p.cultivation_variability_weight, 0.0, "ADR-0054 switched off");
    assert_eq!(p.soil_erosion_rate, 0.0, "ADR-0055: no erosion, soil stays full");
    assert_eq!(p.innovation_share, 1.0, "ADR-0065: individual learning");
    assert_eq!((p.fertility, p.sedentary_fertility), (0.0, 0.0), "ADR-0068: births and deaths not split");
    assert_eq!(p.village_fission_size, 0, "ADR-0070: no villages");
    assert_eq!(
        (p.pastoral_wealth_transmission, p.pastoral_wealth_shock),
        (p.wealth_transmission, p.wealth_shock),
        "#45: herds pass on what foragers' wealth does"
    );
    assert_eq!((p.herd_pressure_weight, p.wadi_share), (0.0, 0.0), "#44: herding by land alone, no wadis");
    // Parameters a checkpoint stored keep its values, whatever the rules.
    let now = serde_json::to_vec(&Params {
        sedentary_growth_rate: 0.03,
        soil_erosion_rate: 1e-4,
        ..Params::default()
    })
    .unwrap();
    let kept = Params::from_checkpoint(&now).unwrap();
    assert_eq!((kept.sedentary_growth_rate, kept.soil_erosion_rate), (0.03, 1e-4));
}
