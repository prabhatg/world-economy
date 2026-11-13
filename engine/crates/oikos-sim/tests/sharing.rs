//! Sharing between groups (#13): a group in a bad year draws on neighbors the year
//! favoured, and gives in turn when they are short. Among foragers, partners in other
//! camps share food and access to land in bad years (hxaro, Wiessner 1982); among early
//! farmers, households and villages exchange food and obligations against future need
//! (social storage, Halstead 1989). Transfers follow need rather than accounts, which
//! pools risk best where harvests are volatile (Aktipis et al. 2016). Pooling with
//! neighbors helps only as far as their years differ from one's own.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::pooled_harvest;
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

/// Farming spreading by population pressure from a few founding bands at 400 mm,
/// harvests swinging (`harvest_cv` 0.3, the region sharing `correlation` of each year's
/// shock), nobody storing, `sharing` of each year's harvest pooled with the neighbors'.
/// Demography and cultivation are fixed at values of their own, so calibration never
/// moves the test.
fn world(seed: u64, sharing: f64, correlation: f64) -> (World, Vec<u8>, Vec<u8>) {
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
        harvest_correlation: correlation,
        storage_years: 0.0,
        bad_year_moves: 0.0,
        harvest_sharing: sharing,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// The world's people, once a year over `years` after `burn_in` years.
fn people(w: &mut World, burn_in: u64, years: u64) -> Vec<f64> {
    w.run_years(burn_in, 2, burn_in);
    (0..years).map(|_| w.run_years(1, 2, 1).last().unwrap().population as f64).collect()
}

fn mean(x: &[f64]) -> f64 {
    x.iter().sum::<f64>() / x.len() as f64
}

/// Mean fall from one year to the next, where numbers fall, relative to the mean.
fn falls(x: &[f64]) -> f64 {
    x.windows(2).map(|w| (w[0] - w[1]).max(0.0)).sum::<f64>() / (x.len() - 1) as f64 / mean(x)
}

#[test]
fn a_group_lives_on_its_own_harvest_and_its_share_of_the_neighbors() {
    assert_eq!(pooled_harvest(0.4, &[1.2, 1.6], 0.0), 0.4, "no sharing");
    assert!((pooled_harvest(0.4, &[1.2, 1.6], 0.5) - 0.9).abs() < 1e-12, "half its own, half theirs");
    assert!((pooled_harvest(0.4, &[1.2, 1.6], 1.0) - 1.4).abs() < 1e-12);
    assert_eq!(pooled_harvest(0.4, &[], 0.5), 0.4, "nobody near to share with");
}

#[test]
fn sharing_keeps_more_farmers_alive_through_bad_years() {
    let run = |sharing| {
        let (mut w, ..) = world(3, sharing, 0.3);
        people(&mut w, 2500, 200)
    };
    let (alone, shared) = (run(0.0), run(0.5));
    assert!(mean(&shared) > 1.03 * mean(&alone), "{} sharing, {} alone", mean(&shared), mean(&alone));
    assert!(
        falls(&shared) < falls(&alone),
        "year-to-year falls {} sharing, {} alone",
        falls(&shared),
        falls(&alone)
    );
}

#[test]
fn sharing_pools_nothing_where_every_cell_has_the_same_year() {
    let run = |sharing| {
        let (mut w, ..) = world(3, sharing, 1.0);
        mean(&people(&mut w, 2500, 100))
    };
    let (alone, shared) = (run(0.0), run(0.5));
    assert!((shared / alone - 1.0).abs() < 0.005, "{shared} sharing, {alone} alone");
}

#[test]
fn sharing_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, 0.5, 0.3);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, 0.5, 0.3);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_without_sharing() {
    let (w, ..) = world(9, 0.5, 0.3);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("harvest_sharing").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.harvest_sharing, 0.0, "groups lived on their own harvests");
}
