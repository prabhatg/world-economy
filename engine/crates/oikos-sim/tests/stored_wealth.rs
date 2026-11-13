//! Stores are wealth (#13). Where norms let a household keep its store against demands to
//! share, what it puts by is its own and passes to its heirs: storage made sedentary
//! foragers unequal (Testart 1982), and at Catalhoyuk households kept private pantries
//! beside celebrated surplus (Bogaard et al. 2009). So a full store makes part of a
//! group's wealth owned and inherited like land closed in by farms (ADR-0074), even on
//! an open frontier: `stored_wealth` of the open land's share, times how full the store
//! is.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::{Inequality, store_fill};
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

/// Farming spreads from a few founding bands by population pressure, so the land fills
/// with farmers over a few thousand years; farmers store up to `storage` years of need,
/// losing `loss` of it a year,
/// and `stored_wealth` of a full store is owned like closed-in land. Harvests do not
/// swing, so stores fill and stay full.
fn farming_world(seed: u64, storage: f64, loss: f64, stored_wealth: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        fission_probability: 0.0,
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
        disease_mortality: 0.0,
        // the village world the test was written for (the round-8 defaults, ADR-0066)
        village_fission_size: 127,
        aggregation_rate: 0.05,
        success_bias: 0.0,
        wild_yield_ratio: 0.3877,
        // settlements of one group, as when the test was written (ADR-0082)
        settlement_integration: 1.0,
        settlement_hierarchy: 0.0,
        harvest_cv: 0.0,
        storage_years: storage,
        storage_loss: loss,
        stored_wealth,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// Settled-people-weighted mean variance of log wealth.
fn settled_inequality(w: &World) -> f64 {
    let (mut settled, mut sum) = (0.0, 0.0);
    for a in w.agent_slices() {
        for i in 0..a.len() {
            let (n, e) = (a.weight[i] as f64, a.cultivation[i] as f64 / SHARE_ONE as f64);
            settled += n * e;
            sum += n * e * a.inequality[i] as f64 / SHARE_ONE as f64;
        }
    }
    sum / settled.max(1e-300)
}

#[test]
fn a_full_store_is_owned_like_closed_in_land() {
    assert_eq!(store_fill(0, 0.5), 0.0);
    assert!((store_fill(SHARE_ONE / 4, 0.5) - 0.5).abs() < 1e-9, "a quarter year of half a year's room");
    assert_eq!(store_fill(SHARE_ONE, 0.5), 1.0, "fuller counts as full");
    assert_eq!(store_fill(SHARE_ONE, 0.0), 0.0, "nobody stores");
    // Owned share of a group's wealth: effort times closure, plus stored_wealth of the
    // open rest times how full the store is.
    assert_eq!(Inequality::owned(0.8, 0.5, 0.0, 1.0), 0.4, "an empty store owns nothing more");
    assert_eq!(Inequality::owned(0.8, 0.5, 1.0, 0.0), 0.4, "stores not owned");
    assert!((Inequality::owned(0.8, 0.5, 1.0, 1.0) - 0.8).abs() < 1e-12, "a full store owns it all");
    assert!((Inequality::owned(0.8, 0.0, 0.5, 0.5) - 0.2).abs() < 1e-12, "the open frontier");
    // A group that has left farming lives by foragers' levelling norms, which share out
    // what it still holds (Woodburn 1982), so its leftover store is not owned.
    assert_eq!(Inequality::owned(0.0, 1.0, 1.0, 1.0), 0.0, "foragers own no land and keep no store");
}

#[test]
fn frontier_farmers_with_stores_grow_unequal() {
    // On the open frontier, four centuries in (some 5,000 farmers, their stores about
    // four-fifths full), storing farmers whose stores are owned grow unequal; without
    // owned stores they stay nearer the forager level until farmland closes in (by 1,500
    // years both are near the landed level).
    let run = |stored_wealth| {
        let (mut w, ..) = farming_world(3, 0.5, 0.1, stored_wealth);
        w.run_years(400, 2, 400);
        settled_inequality(&w)
    };
    let m = Inequality::from_params(&Params::default());
    let (forager, landed) = (m.stationary((0.0, 0.0)), m.stationary((1.0, 0.0)));
    let (shared, owned) = (run(0.0), run(1.0));
    assert!(owned > shared + 0.25 * (landed - forager), "owned stores {owned}, shared {shared}");
}

#[test]
fn without_stores_nothing_is_owned_beyond_land() {
    // Where nobody stores, and where stores are kept but rot within the year, so every
    // group's store is empty when inequality is updated: what a group owns follows its
    // own store, not whether storing is possible.
    for (storage, loss) in [(0.0, 0.1), (0.5, 1.0)] {
        let run = |stored_wealth| {
            let (mut w, ..) = farming_world(3, storage, loss, stored_wealth);
            w.run_years(400, 2, 400);
            w.agent_slices().flat_map(|a| a.inequality.to_vec()).collect::<Vec<_>>()
        };
        assert_eq!(run(1.0), run(0.0), "storage {storage}, loss {loss}");
    }
}

#[test]
fn stored_wealth_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = farming_world(9, 0.5, 0.1, 0.5);
        w.run_years(1500, parts, 1500);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(9, 0.5, 0.1, 0.5);
    w.run_years(777, 3, 777);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(723, 2, 723);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_with_stores_unowned() {
    let (w, ..) = farming_world(9, 0.5, 0.1, 0.5);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("stored_wealth").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.stored_wealth, 0.0, "only closed-in land was owned");
}
