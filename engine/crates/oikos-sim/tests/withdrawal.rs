//! Stores and herds weaken sharing (#13). People with buffers of their own lean on
//! reciprocity less: among the Basarwa, households with stored crops or livestock took
//! less part in exchange networks than those without (Cashdan 1985), and the household
//! stores of early farmers were kept from demands to share (Testart 1982; Bogaard et al.
//! 2009). A group withholds `sharing_withdrawal` of its sharing as far as its store, up
//! to `storage_years` of need, and its herds insure it: it then neither gives in its
//! neighbors' bad years nor draws in its own.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::{SHARE_ONE, Traits};
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

/// Farming spreading by population pressure from a few founding bands at 400 mm,
/// harvests swinging (`harvest_cv` 0.3, the region sharing a third of each year's
/// shock), farmers storing up to `storage` years of need, `sharing` of each year's
/// harvest pooled with the neighbors' and `withdrawal` of it withheld by groups with full
/// stores. Nobody herds.
/// Demography and cultivation are fixed at values of their own, so calibration never
/// moves the test.
fn world(seed: u64, storage: f64, sharing: f64, withdrawal: f64) -> (World, Vec<u8>, Vec<u8>) {
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
        herd_pressure_weight: 0.0,
        harvest_cv: 0.3,
        forage_risk_ratio: 0.5,
        harvest_correlation: 0.3,
        storage_years: storage,
        storage_loss: 0.1,
        bad_year_moves: 0.0,
        harvest_sharing: sharing,
        sharing_withdrawal: withdrawal,
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
fn groups_insured_by_stores_or_herds_share_less() {
    let g = |withdrawal| {
        Growth::from_params(&Params {
            harvest_sharing: 0.4,
            sharing_withdrawal: withdrawal,
            storage_years: 0.5,
            ..Params::default()
        })
    };
    let (none, full) = (Traits::from_years(0.0), Traits::from_years(0.5));
    assert_eq!(g(0.5).sharing_of(none, 0), 0.4, "nothing stored, no herds: all of it");
    assert_eq!(g(0.0).sharing_of(full, SHARE_ONE), 0.4, "no withdrawal");
    assert!((g(0.5).sharing_of(full, 0) - 0.2).abs() < 1e-9, "a full store");
    assert!((g(1.0).sharing_of(full, 0)).abs() < 1e-9);
    assert!((g(0.5).sharing_of(Traits::from_years(1.0), 0) - 0.2).abs() < 1e-9, "fuller counts as full");
    assert!((g(1.0).sharing_of(Traits::from_years(0.25), 0) - 0.2).abs() < 1e-9, "half full");
    assert!((g(1.0).sharing_of(none, SHARE_ONE / 2) - 0.2).abs() < 1e-9, "half its effort herding");
    // Store and herds insure together: 1 - (1 - 1/2)(1 - 1/2) of it.
    assert!((g(1.0).sharing_of(Traits::from_years(0.25), SHARE_ONE / 2) - 0.1).abs() < 1e-9);
}

#[test]
fn farmers_with_full_stores_withdraw_from_sharing() {
    let run = |sharing, withdrawal| {
        let (mut w, ..) = world(3, 1.0, sharing, withdrawal);
        mean(&people(&mut w, 2500, 200))
    };
    let (alone, sharing, withdrawn) = (run(0.0, 0.0), run(0.5, 0.0), run(0.5, 1.0));
    // Stores empty in bad years, and groups share again: some of the gain stays (seeds
    // 3 to 6 keep 46-78% of it).
    assert!(
        alone < withdrawn && withdrawn < sharing - 0.15 * (sharing - alone),
        "{alone} alone, {sharing} sharing, {withdrawn} sharing until stores fill"
    );
}

#[test]
fn groups_without_stores_or_herds_withdraw_nothing() {
    let run = |withdrawal| {
        let (mut w, ..) = world(3, 0.0, 0.5, withdrawal);
        // The state hash covers the parameters, so compare what people did.
        w.run_years(1500, 2, 1).iter().map(|s| s.population).collect::<Vec<_>>()
    };
    let (withdrawn, sharing) = (run(1.0), run(0.0));
    assert!(sharing.last() > Some(&0), "farmers live there");
    assert_eq!(withdrawn, sharing);
}

#[test]
fn withdrawal_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, 1.0, 0.5, 0.5);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, 1.0, 0.5, 0.5);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_without_withdrawal() {
    let (w, ..) = world(9, 1.0, 0.5, 0.5);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("sharing_withdrawal").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.sharing_withdrawal, 0.0, "groups shared whatever they held");
}
