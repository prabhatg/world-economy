//! Parameters come from the registry (engine/params/m0-toy.toml), never ad hoc.

use oikos_params::Value;
use oikos_sim::params::registry;
use oikos_sim::workunit::Start;
use oikos_sim::{Params, WorkUnit, WorldConfig, run_work_unit};
use std::collections::BTreeMap;

#[test]
fn defaults_from_registry_match_the_m0_values() {
    let p = Params::default();
    assert_eq!(p.period_ticks, 365);
    assert_eq!(p.growth_rate, 0.01);
    assert_eq!((p.capacity_min, p.capacity_max), (20, 400));
    assert_eq!(p.fission_threshold, 60);
    assert_eq!(p.fission_probability, 0.2);
    assert_eq!(p.exploration_probability, 0.3);
    assert_eq!(p.merge_threshold, 10);
    assert_eq!((p.initial_bands, p.initial_weight), (12, 30));
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
    };
    let e = run_work_unit(&wu, |p| Err(p.into()), |_, _| Ok(())).unwrap_err();
    assert!(e.contains("growth_rate"), "{e}");
}
