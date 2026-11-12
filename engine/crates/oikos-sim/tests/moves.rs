//! Moving in bad years (#13): people judge the land by the year they are living, not by
//! its long-run mean. Foragers leave a camp whose returns have fallen for one where they
//! are better (Kelly 2013) and herders take their flocks to where the rain fell, so a band
//! in a cell its harvest has failed buds off toward a neighbor the year favoured
//! (Halstead and O'Shea 1989: mobility, the first of the answers to a bad year). People
//! weigh recent experience heavily (Hertwig et al. 2004). A group that still has food
//! stored stays: it has no need to go, and its fields and stores tie it down.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::Dispersal;
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

/// Foragers at 250 mm with harvests swinging (`harvest_cv` 0.4 at full rain, mostly each
/// cell's own), `moves` of a bad year weighing in where they go. Demography and dispersal
/// are fixed at values of their own, so calibration never moves the test.
fn world(seed: u64, moves: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3, 250));
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
        storage_years: 0.0,
        bad_year_moves: moves,
        harvest_sharing: 0.0,
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

#[test]
fn the_land_is_judged_by_the_year_as_far_as_bad_years_weigh() {
    let judge = |moves| Dispersal::from_params(&Params { bad_year_moves: moves, ..Params::default() });
    assert_eq!(judge(0.0).this_year(1000, 0.4), 1000, "by the mean");
    assert_eq!(judge(1.0).this_year(1000, 0.4), 400, "by the year");
    assert_eq!(judge(0.5).this_year(1000, 0.4), 700);
    assert_eq!(judge(1.0).this_year(1000, 1.5), 1500, "a good year feeds more");
    assert_eq!(judge(1.0).this_year(1000, 0.0), 0, "a failed harvest feeds nobody");
}

#[test]
fn a_group_with_food_stored_stays_through_a_bad_year() {
    assert!(Dispersal::stays(1, 0.5), "it eats from its store");
    assert!(!Dispersal::stays(0, 0.5), "nothing left: it may go");
    assert!(!Dispersal::stays(1, 1.0), "not a bad year: as any other");
    assert!(!Dispersal::stays(1, 1.2));
}

#[test]
fn moving_in_bad_years_keeps_foragers_on_a_margin_bad_years_would_empty() {
    // Foragers feeling all of a harvest's swings at 250 mm: bad years empty cells, and
    // judging the land by its mean, a thinned cell is never crowded enough to send anyone
    // to refill them, so the margin empties. Judging by the year, a bad one crowds even a
    // thinned cell and sends a group to a neighbor the year favoured: the margin stays
    // peopled (the rescue effect, Brown and Kodric-Brown 1977).
    let run = |moves| {
        let (mut w, ..) = world(3, moves);
        people(&mut w, 1500, 300)
    };
    let (stay, go) = (run(0.0), run(1.0));
    assert_eq!(stay.last(), Some(&0.0), "judging by the mean, the margin empties");
    assert!(go.iter().all(|&n| n > 500.0), "judging by the year it stays peopled: {}", mean(&go));
}

#[test]
fn moving_in_bad_years_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, 1.0);
        w.run_years(1200, parts, 1200);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, 1.0);
    w.run_years(577, 3, 577);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(623, 2, 623);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_judging_by_the_mean() {
    let (w, ..) = world(9, 1.0);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("bad_year_moves").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.bad_year_moves, 0.0, "people judged the land by its mean");
}
