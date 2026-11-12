//! Storage (#13): the first answer to a bad year (Halstead and O'Shea 1989). A group puts
//! a good year's surplus by and eats from it when a harvest falls short. Storing takes a
//! delayed-return economy and norms that let a household keep its store against demands
//! to share (Woodburn 1982; Testart 1982): settled cultivators have them, as granaries
//! and household stores show (Kuijt and Finlayson 2009; Bogaard et al. 2009), mobile
//! foragers mostly do not, so what a group can keep moves with its cultivation effort.
//! Stores rot and are eaten by pests.

use oikos_core::EntityId;
use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::{Agents, SHARE_ONE, Traits};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::Growth;
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

/// Warm country with winter rain, 400 mm a year everywhere.
fn climate_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![400; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

/// Foragers, or farming spreading by population pressure from a few founding bands,
/// harvests swinging (`harvest_cv` 0.3), keeping `years` of food at full cultivation.
fn world(seed: u64, farming: bool, years: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        cultivation_pressure_weight: if farming { 0.8 } else { 0.0 },
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: if farming { 0.2 } else { 0.0 },
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        growth_rate: 0.02,
        sedentary_growth_rate: 0.02,
        fertility: 0.035,
        sedentary_fertility: 0.035,
        dispersal_probability: 0.1,
        forager_density_ref: 0.1,
        farm_density: 2.0,
        selection_coefficient: 0.02,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        disease_mortality: 0.0,
        aggregation_rate: 0.0,
        success_bias: 0.0,
        settlement_integration: 1.0,
        settlement_hierarchy: 0.0,
        herding_weight: 0.0,
        harvest_cv: 0.3,
        forage_risk_ratio: 0.5,
        harvest_correlation: 0.3,
        storage_years: years,
        storage_loss: 0.1,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// The world's people, once a year over `years` after `burn_in` years.
fn people(w: &mut World, burn_in: u64, years: u64) -> Vec<f64> {
    w.run_years(burn_in, 2, burn_in);
    (0..years)
        .map(|_| {
            w.run_years(1, 2, 1);
            w.agent_slices().flat_map(|a| a.weight.iter()).map(|&n| n as f64).sum()
        })
        .collect()
}

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

fn stored_food(w: &World) -> f64 {
    w.agent_slices()
        .flat_map(|a| a.weight.iter().zip(&a.store))
        .map(|(&n, &s)| n as f64 * s as f64 / SHARE_ONE as f64)
        .sum()
}

#[test]
fn a_store_fills_in_good_years_and_feeds_in_bad_ones() {
    let g = Growth::from_params(&Params { storage_years: 0.5, storage_loss: 0.1, ..Params::default() });
    // A harvest that feeds a fifth more than the people: the surplus goes into the store,
    // up to half a year's need for a group cultivating fully, a quarter at half effort.
    let (fed, store) = g.stored(1.0, 0.0, 1.2);
    assert_eq!(fed, 1.2);
    assert!((store - 0.2 * 0.9).abs() < 1e-12, "a tenth lost in the year: {store}");
    let (_, store) = g.stored(0.5, 0.2, 1.2);
    assert!((store - 0.25 * 0.9).abs() < 1e-12, "{store}");
    let (_, store) = g.stored(1.0, 0.45, 1.5);
    assert!((store - 0.5 * 0.9).abs() < 1e-12, "full: {store}");
    // A harvest that feeds seven in ten: the store makes up the shortfall while it lasts.
    let (fed, store) = g.stored(1.0, 0.5, 0.7);
    assert!((fed - 1.0).abs() < 1e-12 && (store - 0.2 * 0.9).abs() < 1e-12, "{fed} {store}");
    let (fed, store) = g.stored(1.0, 0.1, 0.7);
    assert!((fed - 0.8).abs() < 1e-12 && store == 0.0, "{fed} {store}");
    // Foragers keep nothing; a store above what the group can keep is not refilled.
    assert_eq!(g.stored(0.0, 0.0, 1.5), (1.5, 0.0));
    let (_, store) = g.stored(0.2, 0.3, 1.5);
    assert!((store - 0.27).abs() < 1e-12, "{store}");
}

#[test]
fn births_share_the_store_and_the_dead_leave_theirs() {
    // The food stays with the group: 100 people holding 0.45 years each who become 105
    // hold 45 person-years still, and so do 90 who survive a year.
    let one = SHARE_ONE as f64;
    let store = (0.45 * one) as u32;
    let per = |w_new: u64| Growth::shared(store, 100, w_new) as f64 / one;
    assert!((per(105) * 105.0 - 45.0).abs() < 1e-6, "{}", per(105) * 105.0);
    assert!((per(90) * 90.0 - 45.0).abs() < 1e-6, "{}", per(90) * 90.0);
    assert_eq!(Growth::shared(store, 100, 100), store);
    assert_eq!(Growth::shared(store, 100, 0), 0, "nobody left");
}

#[test]
fn a_store_keeps_its_yearly_pace_however_often_growth_runs() {
    // Growth five times a year: a store at a harvest that just feeds its people loses a
    // tenth over the year, not five tenths, and a surplus fills it at a year's pace.
    let p = Params { storage_years: 0.5, storage_loss: 0.1, ..Params::default() };
    let fifth = Growth::from_params(&Params { period_ticks: 73, ..p.clone() });
    let year = Growth::from_params(&p);
    let mut store = 0.5;
    for _ in 0..5 {
        store = fifth.stored(1.0, store, 1.0).1;
    }
    assert!((store - year.stored(1.0, 0.5, 1.0).1).abs() < 1e-9, "{store}");
    let (mut a, b) = (0.0, year.stored(1.0, 0.0, 1.2).1);
    for _ in 0..5 {
        a = fifth.stored(1.0, a, 1.2).1;
    }
    assert!((a - b).abs() < 0.02, "a fifth of the surplus each time: {a} against {b}");
    let (mut fed, mut left) = (0.0, 0.5);
    for _ in 0..5 {
        let (eaten, kept) = fifth.stored(1.0, left, 0.7);
        fed += eaten / 5.0;
        left = kept;
    }
    assert!((fed - 1.0).abs() < 1e-9, "a year's shortfall, a fifth at a time: {fed}");
}

#[test]
fn storage_spares_farmers_the_worst_of_bad_years() {
    let run = |years| {
        let (mut w, ..) = world(3, true, years);
        let p = people(&mut w, 2500, 200);
        (mean(&p), p.windows(2).map(|x| (x[1] - x[0]).min(0.0)).sum::<f64>() / mean(&p), stored_food(&w))
    };
    let (without, lost_without, none) = run(0.0);
    let (with, lost_with, kept) = run(0.5);
    assert_eq!(none, 0.0);
    assert!(kept > 0.0, "farmers store food");
    assert!(with > without, "people {with} with stores, {without} without");
    assert!(lost_with > lost_without, "falls {lost_with} with stores, {lost_without} without");
}

#[test]
fn foragers_keep_no_store() {
    let (mut w, ..) = world(3, false, 0.5);
    w.run_years(1000, 2, 1000);
    assert!(w.agent_count() > 0);
    assert_eq!(stored_food(&w), 0.0);
}

#[test]
fn merged_groups_keep_their_stored_food() {
    let t = |store: f64| Traits {
        cultivation: SHARE_ONE / 2,
        store: (store * SHARE_ONE as f64) as u32,
        ..Traits::FORAGER
    };
    let mut a = Agents::default();
    a.push_with(EntityId::new(0, 1), 0, 30, t(0.4));
    a.push_with(EntityId::new(0, 2), 0, 10, t(0.0));
    a.push_with(EntityId::new(0, 3), 0, 60, Traits::FORAGER);
    a.merge_small(1000);
    assert_eq!(a.len(), 1);
    let kept = a.weight[0] as f64 * a.store[0] as f64 / SHARE_ONE as f64;
    assert!((kept - 12.0).abs() < 1e-6, "food stored {kept}, before 12");
    // Pooled sums that pass u64: five billion people each holding two years.
    let big = Traits { store: 2 * SHARE_ONE, ..t(0.0) };
    let t = Traits::pooled((5_000_000_000, big), (5_000_000_000, big));
    assert_eq!(t.store, 2 * SHARE_ONE);
}

#[test]
fn storage_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, true, 0.5);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, true, 0.5);
    w.run_years(777, 3, 777);
    assert!(stored_food(&w) > 0.0);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn checkpoints_from_before_storage_restore_with_empty_stores() {
    let (mut w, g, c) = world(9, true, 0.5);
    w.run_years(700, 2, 700);
    assert!(stored_food(&w) > 0.0);
    let now = checkpoint::encode(&w);
    let n = w.agent_count();
    let agents_at = now.len() - 56 * n;
    let mut old = b"OIKOSCK8".to_vec();
    old.extend_from_slice(&now[8..agents_at]);
    for k in 0..n {
        old.extend_from_slice(&now[agents_at + 56 * k..agents_at + 56 * k + 52]);
    }
    let restored = checkpoint::decode_with(&old, inputs(&[&g, &c])).unwrap();
    assert_eq!(restored.agent_count(), n);
    assert_eq!(stored_food(&restored), 0.0);
}

#[test]
fn checkpoint_parameters_from_before_storage_restore_without_it() {
    let (w, ..) = world(9, true, 0.5);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    for k in ["storage_years", "storage_loss"] {
        assert!(json.remove(k).is_some(), "{k}");
    }
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.storage_years, 0.0, "nobody stored");
}

#[test]
fn storage_is_on_by_default() {
    let p = Params::default();
    assert!(p.storage_years > 0.0 && p.storage_loss > 0.0 && p.storage_loss < 1.0);
}
