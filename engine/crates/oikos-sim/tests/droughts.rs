//! Droughts that last (#13, open question 44). Dry years come in runs: a year's harvest
//! shock carries over into the next by `harvest_persistence` (a first-order
//! autoregression), so a run of bad years outlasts a store of half a year's need, which
//! buffers one bad year but not a series (Halstead and O'Shea 1989), and this year's
//! harvest says something about next year's, so judging where to go by it (ADR-0125)
//! stops costing. Each year's shock stays standard (mean 0, variance 1), and cells share
//! the region's part of it as before.

use oikos_core::{Philox4x32, Stream, Tick};
use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{harvest_at, harvest_shock};
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

/// Farming spreading by population pressure from a few founding bands, harvests swinging
/// (`harvest_cv` 0.3, the region sharing a third of each year's shock) and carrying
/// over `persistence` of it from year to year, farmers keeping `years` of food.
/// Demography and cultivation are fixed at values of their own, so calibration never
/// moves the test.
fn world(seed: u64, persistence: f64, years: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        cultivation_pressure_weight: 0.8,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.2,
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
        harvest_persistence: persistence,
        storage_years: years,
        storage_loss: 0.1,
        harvest_sharing: 0.0,
        bad_year_moves: 0.0,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// Foragers at 400 mm feeling all of a harvest's swings (`harvest_cv` 0.4 at full rain,
/// mostly each cell's own), carrying over `persistence` from year to year, `moves` of a
/// bad year weighing in where they go (ADR-0125's test world, with more rain), pooling
/// `sharing` of each year's harvest with the neighbors'.
fn foragers(seed: u64, moves: f64, persistence: f64, sharing: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        cultivation_pressure_weight: 0.0,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.0,
        cultivation_variability_weight: 0.0,
        herding_weight: 0.0,
        growth_rate: 0.02,
        sedentary_growth_rate: 0.02,
        fertility: 0.035,
        sedentary_fertility: 0.035,
        dispersal_crowding: 0.5,
        dispersal_probability: 0.5,
        dispersal_fraction: 0.5,
        forager_density_ref: 0.1,
        disease_mortality: 0.0,
        aggregation_rate: 0.0,
        harvest_cv: 0.4,
        forage_risk_ratio: 1.0,
        harvest_correlation: 0.1,
        harvest_persistence: persistence,
        storage_years: 0.0,
        bad_year_moves: moves,
        harvest_sharing: sharing,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

/// Correlation of `x` with itself `lag` years later.
fn autocorrelation(x: &[f64], lag: usize) -> f64 {
    let m = mean(x);
    let var = x.iter().map(|v| (v - m) * (v - m)).sum::<f64>();
    x.windows(lag + 1).map(|w| (w[0] - m) * (w[lag] - m)).sum::<f64>() / var
}

#[test]
fn without_persistence_years_are_independent_as_before() {
    // The shock of ADR-0122, drawn the same way: persistence 0 changes no bit.
    let (seed, rho) = (7, 0.3_f64);
    let rng = Philox4x32::new(seed, Stream::Harvest);
    let standard = |subject: u64, year: u64| {
        let sum: f64 = rng.block(subject, year, 0).iter().map(|&u| (u as f64 + 0.5) / 4_294_967_296.0).sum();
        (sum - 2.0) * 3f64.sqrt()
    };
    for (cell, year) in [(0u32, 0u64), (5, 9), (12, 4000)] {
        let want = rho.sqrt() * standard(u64::MAX, year) + (1.0 - rho).sqrt() * standard(cell as u64, year);
        assert_eq!(harvest_shock(seed, cell, year, rho, 0.0), want);
        assert_eq!(harvest_shock(seed, cell, year, 0.0, 0.0), standard(cell as u64, year));
    }
}

#[test]
fn persistent_shocks_stay_standard_and_carry_over() {
    let n = 40_000u64;
    let (rho, phi, seed) = (0.3, 0.6, 7);
    let a: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 0, t, rho, phi)).collect();
    let b: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 1, t, rho, phi)).collect();
    let m = mean(&a);
    let var = a.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64;
    assert!(m.abs() < 0.04, "mean {m}");
    assert!((var - 1.0).abs() < 0.05, "variance {var}");
    let cov = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>() / n as f64;
    assert!((cov - rho).abs() < 0.04, "the region's share {cov}");
    assert!((autocorrelation(&a, 1) - phi).abs() < 0.03, "a year carries over {}", autocorrelation(&a, 1));
    assert!((autocorrelation(&a, 2) - phi * phi).abs() < 0.03, "and fades {}", autocorrelation(&a, 2));
    let iid: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 0, t, rho, 0.0)).collect();
    assert!(autocorrelation(&iid, 1).abs() < 0.02);
    let weak: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 0, t, rho, 0.3)).collect();
    assert!((autocorrelation(&weak, 1) - 0.3).abs() < 0.03, "{}", autocorrelation(&weak, 1));
    // One harvest a year, however often growth runs, persistent or not.
    assert_eq!(harvest_at(seed, 3, Tick(365 * 40 + 30), rho, phi), harvest_shock(seed, 3, 40, rho, phi));
    assert_eq!(
        harvest_at(seed, 3, Tick(365 * 40), rho, phi),
        harvest_at(seed, 3, Tick(365 * 41 - 1), rho, phi)
    );
}

#[test]
fn a_store_buffers_lasting_droughts_less() {
    // Half a year's food buffers a bad year but not a run of them: storing gains farmers
    // less where droughts last (seeds 3 to 5: 13-15% with independent years, 7-8% with
    // persistence 0.6).
    let gain = |persistence| {
        let run = |years| {
            let (mut w, ..) = world(3, persistence, years);
            w.run_years(2500, 2, 2500);
            mean(
                &(0..200).map(|_| w.run_years(1, 2, 1).last().unwrap().population as f64).collect::<Vec<_>>(),
            )
        };
        run(0.5) / run(0.0) - 1.0
    };
    let (independent, lasting) = (gain(0.0), gain(0.6));
    assert!(independent > 0.0 && lasting > 0.0, "a store helps: {independent}, {lasting}");
    assert!(lasting < 0.8 * independent, "{lasting} with lasting droughts, {independent} without");
}

#[test]
fn moving_in_bad_years_pays_where_droughts_last() {
    // Judging where to go by this year's harvest (ADR-0125) costs foragers numbers where
    // years are independent: a bad year says nothing of the next, and moving away from
    // it leaves land the next year may favour. Where droughts last, this year foretells
    // the next, and judging by it no longer costs (seeds 3 and 4: 0.969 of staying's
    // numbers with independent years, 1.002-1.003 with persistence 0.6).
    let ratio = |persistence| {
        let run = |moves| {
            let (mut w, ..) = foragers(3, moves, persistence, 0.0);
            w.run_years(1500, 2, 1500);
            mean(
                &(0..300).map(|_| w.run_years(1, 2, 1).last().unwrap().population as f64).collect::<Vec<_>>(),
            )
        };
        run(1.0) / run(0.0)
    };
    let (independent, lasting) = (ratio(0.0), ratio(0.6));
    assert!(independent < 0.99, "moving costs where years are independent: {independent}");
    assert!(lasting > 0.995, "no longer costs where droughts last: {lasting}");
}

#[test]
fn droughts_are_partition_independent_and_restore() {
    let run = |parts| {
        let (mut w, ..) = world(9, 0.6, 0.5);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    // Through moving in bad years and sharing, which judge neighbors' years too.
    let moving = |parts| {
        let (mut w, ..) = foragers(9, 1.0, 0.6, 0.3);
        w.run_years(1000, parts, 1000);
        checkpoint::state_hash(&w)
    };
    assert_eq!(moving(1), moving(4));
    let (mut w, g, c) = world(9, 0.6, 0.5);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_with_independent_years() {
    let (w, ..) = world(9, 0.6, 0.5);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("harvest_persistence").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.harvest_persistence, 0.0, "every year's harvest was its own");
}
