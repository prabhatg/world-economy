//! Harvest risk (#13): years differ. Rain varies from year to year, more where it is
//! scarce (Golodets et al. 2013), and dry-farmed yields swing further still (FAO 2008:
//! from nothing to three times the mean), most at the dry limit of farming (Wilkinson
//! 1994). Foragers, drawing on many wild foods and free to move, feel less of it
//! (Winterhalder 1986). A bad year kills where it leaves people beyond what the harvest
//! feeds, never more than it cannot feed.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::environment::{harvest_cv, harvest_cv_of};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{Growth, harvest_shock};
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

/// Warm country with winter rain, `precip` mm a year everywhere.
fn climate_pack(h3: &[u64], precip: u16) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-12_000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![precip; h3.len()])).unwrap();
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

/// Foragers, or farming spreading by population pressure from a few founding bands, at
/// `precip` mm, the harvest's year-to-year spread `cv` at full rain. Demography and
/// cultivation are fixed at values of their own, so calibration never moves the test.
fn world(seed: u64, farming: bool, precip: u16, cv: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, precip));
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
        harvest_cv: cv,
        forage_risk_ratio: 0.5,
        harvest_correlation: 0.3,
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

/// Mean absolute change from one year to the next, relative to the mean.
fn swing(x: &[f64]) -> f64 {
    x.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (x.len() - 1) as f64 / mean(x)
}

#[test]
fn harvests_swing_more_where_rain_is_scarce_and_level_off_where_it_no_longer_limits_cereals() {
    let p = Params { harvest_cv: 0.3, ..Params::default() };
    let full = p.cereal_precip_full;
    assert!((harvest_cv(full, &p) - 0.3).abs() < 1e-12);
    assert!((harvest_cv(4.0 * full, &p) - 0.3).abs() < 1e-12, "rain no longer limits");
    assert!((harvest_cv(full / 4.0, &p) - 0.6).abs() < 1e-12, "as the square root of the rain's shortfall");
    assert_eq!(harvest_cv(5.0, &p), 1.0, "at most the mean itself");
    assert_eq!(harvest_cv(0.0, &p), 1.0);
    assert_eq!(harvest_cv(full / 4.0, &Params { harvest_cv: 0.0, ..p.clone() }), 0.0, "no risk");
}

#[test]
fn floodplain_fields_swing_as_fields_at_full_rain() {
    // Floods water the fields whatever the rain (ADR-0085), so they swing with the flood,
    // taken to be as a rain-fed harvest's where rain no longer limits it.
    let p = Params { harvest_cv: 0.3, ..Params::default() };
    let dry = harvest_cv(10.0, &p);
    assert_eq!(harvest_cv_of(10.0, 0.0, 0.0, &p), dry, "no fields: the rain's own");
    assert_eq!(harvest_cv_of(10.0, 5.0, 0.0, &p), dry);
    assert!((harvest_cv_of(10.0, 0.0, 5.0, &p) - 0.3).abs() < 1e-12);
    assert!((harvest_cv_of(10.0, 1.0, 3.0, &p) - (dry + 3.0 * 0.3) / 4.0).abs() < 1e-12);
}

#[test]
fn harvest_shocks_are_standard_and_share_the_regions_part() {
    // Mean 0 and variance 1, each cell's year the region's shock and its own.
    let n = 40_000u64;
    let (rho, seed) = (0.3, 7);
    let a: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 0, t, rho)).collect();
    let b: Vec<f64> = (0..n).map(|t| harvest_shock(seed, 1, t, rho)).collect();
    let m = mean(&a);
    let var = a.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / n as f64;
    assert!(m.abs() < 0.02, "mean {m}");
    assert!((var - 1.0).abs() < 0.03, "variance {var}");
    let cov = a.iter().zip(&b).map(|(x, y)| x * y).sum::<f64>() / n as f64;
    assert!((cov - rho).abs() < 0.03, "correlation {cov}");
    assert_eq!(harvest_shock(seed, 5, 9, rho), harvest_shock(seed, 5, 9, rho), "deterministic");
    let alone: Vec<f64> =
        (0..2000).map(|t| harvest_shock(seed, 1, t, 0.0) * harvest_shock(seed, 2, t, 0.0)).collect();
    assert!(mean(&alone).abs() < 0.06, "independent cells");
}

#[test]
fn a_bad_year_kills_at_most_those_its_harvest_cannot_feed() {
    let g = Growth::from_params(&Params { fertility: 0.035, growth_rate: 0.02, ..Params::default() });
    // At capacity in an average year: deaths offset births.
    let (b, d) = g.vital_rates(0.0, 100.0, 100.0);
    assert!((b - d - 0.0).abs() < 1e-12);
    // A harvest that feeds a tenth: logistic deaths would be b + r * 9, more than the 90%
    // the harvest cannot feed is not; a total failure, every one of them.
    let (_, d) = g.vital_rates(0.0, 100.0, 10.0);
    assert!(d <= 0.9 + 1e-12 && d > 0.2, "deaths {d}");
    let (_, d) = g.vital_rates(0.0, 100.0, 0.0);
    assert!((0.999..=1.0).contains(&d), "deaths {d}");
}

#[test]
fn harvest_risk_thins_farmers_more_than_foragers() {
    // Bad years kill, good years do not make up for them: a population near capacity
    // loses more in a year that feeds half than it gains in one that feeds half again.
    let run = |farming, cv| {
        let (mut w, ..) = world(3, farming, 400, cv);
        mean(&people(&mut w, 2500, 200))
    };
    let (farmers, farmers_risk) = (run(true, 0.0), run(true, 0.3));
    let (foragers, foragers_risk) = (run(false, 0.0), run(false, 0.3));
    assert!(farmers > 1.5 * foragers, "the test needs farmers: {farmers} against {foragers}");
    let (lost_farming, lost_foraging) = (1.0 - farmers_risk / farmers, 1.0 - foragers_risk / foragers);
    assert!(lost_farming > 0.01, "farmers lose {lost_farming}");
    assert!(lost_farming > lost_foraging, "farmers lose {lost_farming}, foragers {lost_foraging}");
}

#[test]
fn farmers_numbers_swing_more_at_the_dry_margin() {
    let run = |precip| {
        let (mut w, ..) = world(5, true, precip, 0.3);
        swing(&people(&mut w, 2500, 200))
    };
    let (dry, wet) = (run(250), run(600));
    assert!(dry > wet, "year-to-year swings {dry} at 250 mm, {wet} at 600 mm");
    let (mut calm, ..) = world(5, true, 250, 0.0);
    assert!(swing(&people(&mut calm, 2500, 200)) < dry, "without harvest risk");
}

#[test]
fn harvest_risk_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, true, 300, 0.3);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, true, 300, 0.3);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn checkpoints_from_before_harvest_risk_restore_without_it() {
    let (w, ..) = world(9, true, 300, 0.3);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    for k in ["harvest_cv", "forage_risk_ratio", "harvest_correlation"] {
        assert!(json.remove(k).is_some(), "{k}");
    }
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.harvest_cv, 0.0, "years were alike");
}

#[test]
fn harvest_risk_is_on_by_default() {
    let p = Params::default();
    assert!(p.harvest_cv > 0.0 && p.forage_risk_ratio > 0.0 && p.forage_risk_ratio < 1.0);
    assert!((0.0..1.0).contains(&p.harvest_correlation));
}
