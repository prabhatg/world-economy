//! Rank built by giving (#13). Households that run short in a bad year are fed by those
//! with food to spare, and a gift binds the one who cannot return it (Mauss 1925; the
//! norm of reciprocity, Gouldner 1960): in years of real shortage the needy repay with
//! labour, land and deference, and social storage turns into rank (Halstead and O'Shea
//! 1982). Settled farmers, tied to their fields and stores, cannot walk away from what
//! they owe as mobile foragers can (Woodburn 1982). So a year's unmet need adds
//! `giving_rank` times a group's cultivation effort times the need squared to its
//! variance of log wealth.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
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

/// Farming spreading by population pressure from a few founding bands, harvests swinging
/// by `cv` (the region sharing a third of each year's shock), farmers keeping half a
/// year of food, `giving` of a year's unmet need squared turning into rank. Demography
/// and cultivation are fixed at values of their own, so calibration never moves the test.
fn world(seed: u64, cv: f64, giving: f64) -> (World, Vec<u8>, Vec<u8>) {
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
        harvest_cv: cv,
        forage_risk_ratio: 0.5,
        harvest_correlation: 0.3,
        harvest_persistence: 0.0,
        storage_years: 0.5,
        storage_loss: 0.1,
        harvest_sharing: 0.0,
        stored_wealth: 0.0,
        bad_year_moves: 0.0,
        giving_rank: giving,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// Foragers only, their harvests swinging by 0.4, `giving` as in [`world`].
fn foragers(seed: u64, giving: f64) -> (World, Vec<u8>, Vec<u8>) {
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
        disease_mortality: 0.0,
        aggregation_rate: 0.0,
        harvest_cv: 0.4,
        forage_risk_ratio: 1.0,
        storage_years: 0.0,
        giving_rank: giving,
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
fn a_years_unmet_need_becomes_debt() {
    let growth = |giving_rank| Growth::from_params(&Params { giving_rank, ..Params::default() });
    let g = growth(0.5);
    assert!((g.debts(1.0, 0.2) - 0.02).abs() < 1e-15, "half of a fifth's square");
    assert!((g.debts(0.5, 0.2) - 0.01).abs() < 1e-15, "half-settled, half the debt");
    assert_eq!(g.debts(0.0, 0.5), 0.0, "foragers walk away from what they owe");
    assert_eq!(g.debts(1.0, 0.0), 0.0, "a year that feeds everyone makes no debts");
    assert_eq!(growth(0.0).debts(1.0, 0.5), 0.0, "gifts bind nobody");
    let fifth = Growth::from_params(&Params { giving_rank: 0.5, period_ticks: 73, ..Params::default() });
    assert!((fifth.debts(1.0, 0.2) - 0.004).abs() < 1e-15, "a fifth of a year owes a fifth as much");
}

#[test]
fn a_shortfall_binds_farmers_alike() {
    // One year of growth alone from the same farming world, with and without rank from
    // giving. A group's shortfall follows its cell's harvest, its own effort (farmers
    // feel more of a bad year) and its own store, so groups alike in all three are bound
    // alike; no shortfall exceeds the whole need; groups that farm nothing owe nothing.
    let (mut w, g, c) = world(3, 0.3, 0.0);
    w.run_years(600, 2, 600);
    let groups = |w: &World| {
        w.agent_slices()
            .flat_map(|a| {
                (0..a.len())
                    .map(|i| ((a.cell[i], a.cultivation[i], a.store[i]), a.inequality[i]))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let start = groups(&w);
    let step = |giving_rank| {
        let mut w = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
        let p = Params { giving_rank, ..w.params.clone() };
        w.set_mechanisms(vec![Box::new(Growth::from_params(&p))]).unwrap();
        w.step(2);
        groups(&w).into_iter().map(|(_, v)| v).collect::<Vec<_>>()
    };
    let (unbound, bound) = (step(0.0), step(1.0));
    assert_eq!((start.len(), unbound.len()), (bound.len(), bound.len()), "growth alone keeps every group");
    let mut alike: std::collections::BTreeMap<(u32, u32, u32), (f64, usize)> = Default::default();
    for ((&(key, _), &v0), &v1) in start.iter().zip(&unbound).zip(&bound) {
        let (cell, effort, _) = key;
        let added = (v1 as f64 - v0 as f64) / SHARE_ONE as f64;
        if effort == 0 {
            assert_eq!(added, 0.0, "a forager in cell {cell}");
            continue;
        }
        assert!(added >= 0.0 && added <= effort as f64 / SHARE_ONE as f64 + 1e-9, "cell {cell}: {added}");
        let (first, n) = alike.entry(key).or_insert((added, 0));
        assert!((added - *first).abs() < 1e-8, "cell {cell}: {added} against {first}");
        *n += 1;
    }
    let shared = alike.values().filter(|&&(added, n)| added > 0.0 && n > 1).count();
    assert!(shared > 0, "no two groups alike fell short");
}

#[test]
fn farmers_in_swinging_years_grow_ranked() {
    // Debts come from bad years: where harvests swing, giving ranks farmers well above
    // farmers who owe nothing (by 0.11 at 800 years, from 0.79), and where years are
    // alike, growth hardly ever outruns the land, so it ranks them barely at all (0).
    let run = |cv, giving| {
        let (mut w, ..) = world(3, cv, giving);
        w.run_years(800, 2, 800);
        settled_inequality(&w)
    };
    let (swinging, steady) = (run(0.3, 1.0) - run(0.3, 0.0), run(0.0, 1.0) - run(0.0, 0.0));
    assert!(swinging > 0.05, "giving adds {swinging} where years swing");
    assert!(steady.abs() < 0.2 * swinging, "swinging {swinging}, steady {steady}");
}

#[test]
fn foragers_owe_nothing() {
    let run = |giving| {
        let (mut w, ..) = foragers(5, giving);
        w.run_years(300, 2, 300);
        w.agent_slices().flat_map(|a| a.inequality.to_vec()).collect::<Vec<_>>()
    };
    assert_eq!(run(1.0), run(0.0));
}

#[test]
fn giving_is_partition_independent_and_restores() {
    let run = |parts| {
        let (mut w, ..) = world(9, 0.3, 1.0);
        w.run_years(1000, parts, 1000);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = world(9, 0.3, 1.0);
    w.run_years(577, 3, 577);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(423, 2, 423);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn parameters_from_before_restore_without_rank_from_giving() {
    let (w, ..) = world(9, 0.3, 1.0);
    let mut json: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&serde_json::to_vec(&w.params).unwrap()).unwrap();
    assert!(json.remove("giving_rank").is_some());
    let p = Params::from_checkpoint(&serde_json::to_vec(&json).unwrap()).unwrap();
    assert_eq!(p.giving_rank, 0.0, "gifts bound nobody");
}
