//! Disease (ADR-0075): the infection settlement adds, which persists only in groups above
//! a critical size (Bartlett 1957), smaller for settled villagers, who live among their
//! own waste and cannot walk away from it, than for mobile foragers, who move camp. So
//! farming villages carry a disease burden that grows with their size, and foragers' camps
//! add none (Armelagos et al. 1991; Eshed et al. 2010).

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{self, Disease};
use oikos_sim::schedule::Schedule;
use oikos_sim::{Params, World, WorldConfig, checkpoint};

fn h3_cells() -> Vec<u64> {
    let g = Grid::from_base_cell(22, 3).unwrap();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

fn region(h3: &[u64]) -> Region {
    Region { name: "test".into(), resolution: 3, cells: h3.len() as u32, cells_hash: cells_hash(h3) }
}

fn spec(name: &str, dims: Dims, scale: f64) -> FieldSpec {
    FieldSpec {
        name: name.into(),
        unit: "".into(),
        description: name.into(),
        dims,
        scale,
        offset: 0.0,
        nodata: None,
    }
}

fn geometry_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("geometry", region(h3));
    b.field(spec("h3", Dims::Cell, 1.0), Data::U64(h3.to_vec())).unwrap();
    b.field(spec("area_km2", Dims::Cell, 1e-6), Data::U32(vec![4_000_000_000; h3.len()])).unwrap();
    b.build()
}

/// Warm country with winter rain, where wild cereals grow everywhere.
fn climate_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![500; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

/// Farming spreads from a few founding bands by population pressure, and farmers gather
/// into villages, so the land fills with villages over a few thousand years.
fn farming_world(seed: u64, mortality: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        cultivation_pressure_weight: 0.8,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.2,
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        growth_rate: 0.01,
        sedentary_growth_rate: 0.01,
        dispersal_probability: 0.1,
        forager_density_ref: 0.1,
        farm_density: 2.0,
        selection_coefficient: 0.02,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        disease_mortality: mortality,
        // the village world the test was written for (the round-8 defaults, ADR-0066)
        disease_threshold: 50.0,
        village_fission_size: 127,
        aggregation_rate: 0.05,
        success_bias: 0.0,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// People in groups farming more than half their effort, and the number of such groups.
fn villages(w: &World) -> (u64, u64) {
    w.agent_slices()
        .flat_map(|a| a.weight.iter().zip(&a.cultivation))
        .filter(|&(&n, &c)| n > 0 && c > oikos_sim::agents::SHARE_ONE / 2)
        .fold((0, 0), |(people, groups), (&n, _)| (people + n, groups + 1))
}

#[test]
fn the_infection_persists_only_above_the_critical_group_size() {
    // Density-dependent transmission: R0 is the group's size over the critical size, and
    // the endemic prevalence 1 - 1/R0 (Anderson and May 1991), so a group above the
    // critical size keeps exactly that many people susceptible.
    let d = Disease::from_params(&Params { disease_threshold: 50.0, ..Params::default() });
    assert_eq!(d.critical_size(1.0), 50.0);
    assert_eq!(d.prevalence(40.0, 1.0), 0.0);
    assert_eq!(d.prevalence(50.0, 1.0), 0.0);
    assert!((d.prevalence(100.0, 1.0) - 0.5).abs() < 1e-12);
    assert!((d.prevalence(200.0, 1.0) - 0.75).abs() < 1e-12);
}

#[test]
fn mobile_foragers_need_a_larger_group_to_keep_it() {
    // Moving camp leaves waste, vermin and contaminated water behind (Kelly 2013;
    // Armelagos et al. 1991): mobility cuts transmission, so the critical size rises.
    let p = Params { disease_threshold: 50.0, mobile_contact: 0.4, ..Params::default() };
    let d = Disease::from_params(&p);
    assert!((d.critical_size(0.0) - 125.0).abs() < 1e-9);
    assert!((d.critical_size(0.5) - 50.0 / 0.7).abs() < 1e-9);
    assert!(d.prevalence(100.0, 0.0) == 0.0 && d.prevalence(100.0, 1.0) > 0.0);
}

#[test]
fn foragers_camps_add_no_disease_burden_by_default() {
    // Bands escape the crowd infections that need many hosts (Black 1975; their own
    // persistent infections are in their baseline mortality): at the defaults a camp at
    // its size limit is below the critical size.
    let p = Params::default();
    let d = Disease::from_params(&p);
    assert!(d.critical_size(0.0) > 1.5 * p.fission_threshold as f64, "{}", d.critical_size(0.0));
    assert_eq!(d.deaths(p.fission_threshold as f64, 0.0), 0.0);
}

#[test]
fn larger_villages_lose_a_larger_share_of_their_people() {
    // The cost of crowding that the social pull of large villages (ADR-0071) pays.
    let p = Params { disease_threshold: 50.0, ..Params::default() };
    let d = Disease::from_params(&p);
    let share = |n: f64| d.deaths(n, 1.0) / n;
    assert!(share(60.0) < share(90.0) && share(90.0) < share(127.0));
    assert!(share(127.0) < p.disease_mortality, "deaths are among the infected only");
    assert_eq!(Disease::from_params(&Params { disease_mortality: 0.0, ..p }).deaths(127.0, 1.0), 0.0);
}

#[test]
fn disease_thins_farming_villages() {
    // Villages lose people to infection, the largest most: fewer villagers, in smaller
    // villages, than in the same world without disease.
    let run = |mortality| {
        let (mut w, ..) = farming_world(3, mortality);
        w.run_years(3000, 2, 3000);
        villages(&w)
    };
    let (healthy, healthy_villages) = run(0.0);
    let (sick, sick_villages) = run(0.02);
    assert!(healthy > 10_000, "the test needs villages: {healthy}");
    assert!((sick as f64) < 0.97 * healthy as f64, "villagers {sick} with disease, {healthy} without");
    let (healthy_size, sick_size) =
        (healthy as f64 / healthy_villages as f64, sick as f64 / sick_villages as f64);
    assert!(sick_size < healthy_size, "villages of {sick_size} with disease, {healthy_size} without");
}

#[test]
fn disease_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = farming_world(9, 0.02);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(9, 0.02);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn disease_follows_growth() {
    let specs: Vec<_> = mechanisms::defaults(&Params::default()).iter().map(|m| m.spec()).collect();
    let order = Schedule::build(&specs).unwrap();
    let names: Vec<&str> = order.order().iter().map(|&i| specs[i].name).collect();
    assert_eq!(names[..2], ["growth", "disease"]);
}

#[test]
fn checkpoints_from_before_disease_restore_without_it() {
    let (mut w, g, c) = farming_world(9, 0.0);
    w.run_years(100, 2, 100);
    let now = checkpoint::encode(&w);
    // The parameters before ADR-0075 had no disease: drop them from the JSON.
    let at = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8;
    let len = u32::from_le_bytes(now[at..at + 4].try_into().unwrap()) as usize;
    let mut params: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&now[at + 4..at + 4 + len]).unwrap();
    for k in ["disease_threshold", "mobile_contact", "disease_mortality"] {
        assert!(params.remove(k).is_some(), "{k}");
    }
    let old = serde_json::to_vec(&params).unwrap();
    let mut before = now[..at].to_vec();
    before.extend_from_slice(&(old.len() as u32).to_le_bytes());
    before.extend_from_slice(&old);
    before.extend_from_slice(&now[at + 4 + len..]);
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    assert_eq!(restored.params.disease_mortality, 0.0, "whatever the registry default");
}
