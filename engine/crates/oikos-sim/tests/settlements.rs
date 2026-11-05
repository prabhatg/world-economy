//! Settlements of several groups (ADR-0082): a group keeps the size at which decisions
//! among its households overload it (scalar stress, Johnson 1982), but a settlement holds
//! several groups when institutions bind households across them: ritual, shared buildings
//! and descent (Bandy 2004; Hodder 2014), and leaders whose decisions others accept, which
//! the model lets wealth inequality allow. So farmers' settlements grow beyond the size of
//! a village of one group, and surveys count settlements, not groups.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::{ALONE, SHARE_ONE};
use oikos_sim::inputs::Inputs;
use oikos_sim::mechanisms::Disease;
use oikos_sim::{Params, World, WorldConfig, checkpoint};
use std::collections::BTreeMap;

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

/// Wild-cereal country everywhere: 15 degC, 500 mm.
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

/// Farming spreads from a few founding bands, and cells fill with some 350 farmers who
/// gather into villages (the world of `farming_groups_gather_into_villages`, with land
/// for several villages a cell), with settlements of `integration` groups' worth and
/// `hierarchy`.
fn farming_world(seed: u64, integration: f64, hierarchy: f64) -> (World, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (g, c) = (geometry_pack(&h3), climate_pack(&h3));
    let p = Params {
        initial_occupancy: 0.0,
        fission_probability: 0.2,
        cultivation_pressure_weight: 0.0,
        cultivation_stress_weight: 0.0,
        cultivation_abundance_weight: 0.8,
        cultivation_variability_weight: 0.0,
        soil_erosion_rate: 0.0,
        growth_rate: 0.03,
        sedentary_growth_rate: 0.03,
        dispersal_probability: 0.1,
        forager_density_ref: 0.01,
        farm_density: 0.2,
        selection_coefficient: 0.005,
        adoption_rate: 0.02,
        innovation_share: 1.0,
        fission_threshold: 60,
        village_fission_size: 127,
        aggregation_rate: 0.1,
        disease_mortality: 0.0,
        neighbor_learning: 0.0,
        success_bias: 0.0,
        settlement_integration: integration,
        settlement_hierarchy: hierarchy,
        ..Params::default()
    };
    let id = blake3::hash(&g).to_hex().to_string();
    (World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(&[&g, &c])).unwrap(), g, c)
}

/// Each settlement: its cell, people, groups, people-weighted effort (fixed point) and
/// largest group.
#[derive(Debug, Default)]
struct Town {
    cell: u32,
    people: u64,
    groups: u64,
    effort: u128,
    largest: u64,
}

/// Settlements by cell and label (and a lone group's id).
fn towns(w: &World) -> BTreeMap<(u32, u32, u64), Town> {
    let mut out: BTreeMap<(u32, u32, u64), Town> = BTreeMap::new();
    for a in w.agent_slices() {
        for i in 0..a.len() {
            let alone = if a.settlement[i] == ALONE { a.id[i].0 } else { 0 };
            let t = out.entry((a.cell[i], a.settlement[i], alone)).or_default();
            t.cell = a.cell[i];
            t.people += a.weight[i];
            t.groups += 1;
            t.effort += a.weight[i] as u128 * a.cultivation[i] as u128;
            t.largest = t.largest.max(a.weight[i]);
        }
    }
    out
}

fn mean_size(w: &World) -> f64 {
    let t = towns(w);
    let settled: Vec<_> = t.values().filter(|t| t.effort > 0).collect();
    settled.iter().map(|t| t.people as f64).sum::<f64>() / settled.len() as f64
}

#[test]
fn farmers_live_in_settlements_larger_than_any_of_their_groups() {
    let (mut w, ..) = farming_world(6, 3.0, 0.0);
    w.run_years(800, 2, 800);
    let t = towns(&w);
    let largest_group = t.values().map(|t| t.largest).max().unwrap();
    let largest = t.values().map(|t| t.people).max().unwrap();
    assert!(
        largest_group < 150,
        "groups keep their limit (127 at full effort), give or take a year's growth"
    );
    assert!(largest > 2 * largest_group, "a town of {largest}, groups of up to {largest_group}");
    assert!(t.values().any(|t| t.groups >= 3));

    let (mut alone, ..) = farming_world(6, 1.0, 0.0);
    alone.run_years(800, 2, 800);
    for a in alone.agent_slices() {
        assert!(a.alone(0..a.len()), "integration 1: every group lives by itself");
    }
    assert!(mean_size(&w) > 1.5 * mean_size(&alone), "{} against {}", mean_size(&w), mean_size(&alone));
}

#[test]
fn hierarchy_holds_larger_settlements_together_where_wealth_is_unequal() {
    let run = |hierarchy| {
        let (mut w, ..) = farming_world(6, 1.5, hierarchy);
        w.run_years(800, 2, 800);
        mean_size(&w)
    };
    let (flat, ranked) = (run(0.0), run(4.0));
    assert!(ranked > 1.2 * flat, "{ranked} with hierarchy, {flat} without");
}

#[test]
fn surveys_count_each_settlement_once() {
    // A settlement of several groups is one site (ADR-0072): counted as far as it is
    // settled, its people-weighted effort, and sized by the log of all its people
    // (ADR-0078).
    let (mut w, ..) = farming_world(6, 3.0, 0.0);
    w.run_years(800, 2, 800);
    let (mut count, mut logs) = (vec![0.0f64; w.grid.len()], vec![0.0f64; w.grid.len()]);
    for t in towns(&w).values().filter(|t| t.people > 0) {
        let effort = (t.effort / t.people as u128) as f64 / SHARE_ONE as f64;
        count[t.cell as usize] += effort;
        if effort > 0.0 {
            logs[t.cell as usize] += effort * oikos_math::ln(t.people as f64);
        }
    }
    let (got_count, got_logs) = (w.settlements(), w.village_sizes());
    for c in 0..count.len() {
        assert!((got_count[c] - count[c]).abs() < 1e-6, "cell {c}: {} against {}", got_count[c], count[c]);
        assert!((got_logs[c] - logs[c]).abs() < 1e-6, "cell {c}: {} against {}", got_logs[c], logs[c]);
    }
    let (mut four, ..) = farming_world(6, 3.0, 0.0);
    four.run_years(800, 4, 800);
    assert_eq!(
        (four.settlements(), four.village_sizes()),
        (got_count, got_logs),
        "the same at any partition count"
    );
}

#[test]
fn skeletons_count_the_infected_of_each_settlement() {
    // A settlement's infection persists as in one group of all its people (ADR-0075,
    // ADR-0082): its infected are its people times the prevalence at its size and pooled
    // practice, what lesions on its dead would show (#42).
    let (mut w, ..) = farming_world(6, 3.0, 0.0);
    w.run_years(800, 2, 800);
    let disease = Disease::from_params(&w.params);
    let mut want = vec![0.0f64; w.grid.len()];
    for t in towns(&w).values().filter(|t| t.people > 0) {
        let effort = (t.effort / t.people as u128) as f64 / SHARE_ONE as f64;
        want[t.cell as usize] += t.people as f64 * disease.prevalence(t.people as f64, effort);
    }
    let got = w.infected();
    assert!(want.iter().sum::<f64>() > 100.0, "some settlements carry the infection");
    for c in 0..want.len() {
        assert!((got[c] - want[c]).abs() < 1e-6, "cell {c}: {} against {}", got[c], want[c]);
    }
    let (mut four, ..) = farming_world(6, 3.0, 0.0);
    four.run_years(800, 4, 800);
    assert_eq!(four.infected(), got, "the same at any partition count");
}

#[test]
fn settlements_are_partition_independent_and_restore() {
    let run = |parts| {
        let (mut w, ..) = farming_world(9, 3.0, 2.0);
        w.run_years(800, parts, 800);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let (mut w, g, c) = farming_world(9, 3.0, 2.0);
    w.run_years(417, 3, 417);
    assert!(w.agent_slices().any(|a| !a.alone(0..a.len())), "the checkpoint holds towns");
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c])).unwrap();
    restored.run_years(383, 2, 383);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn checkpoints_from_before_settlements_restore_every_group_alone() {
    // OIKOSCK5 had no settlement labels (and no settlement parameters): every group was a
    // settlement of its own, which is what it restores as.
    let (mut w, g, c) = farming_world(9, 1.0, 0.0);
    w.run_years(300, 2, 300);
    let now = checkpoint::encode(&w);
    let n = w.agent_count();
    let agents_at = now.len() - 44 * n;
    let mut old = b"OIKOSCK5".to_vec();
    let at = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8;
    old.extend_from_slice(&now[8..at]);
    let len = u32::from_le_bytes(now[at..at + 4].try_into().unwrap()) as usize;
    let mut params: serde_json::Map<String, serde_json::Value> =
        serde_json::from_slice(&now[at + 4..at + 4 + len]).unwrap();
    for k in ["settlement_integration", "settlement_hierarchy"] {
        assert!(params.remove(k).is_some(), "{k}");
    }
    let json = serde_json::to_vec(&params).unwrap();
    old.extend_from_slice(&(json.len() as u32).to_le_bytes());
    old.extend_from_slice(&json);
    old.extend_from_slice(&now[at + 4 + len..agents_at]);
    for k in 0..n {
        old.extend_from_slice(&now[agents_at + 44 * k..agents_at + 44 * k + 32]);
    }
    let restored = checkpoint::decode_with(&old, inputs(&[&g, &c])).unwrap();
    assert_eq!(checkpoint::encode(&restored), now);
}

#[test]
fn checkpoints_from_before_herding_restore_with_no_herds() {
    // OIKOSCK6 had no herding (#37): no group herded and no herd was domestic, which is
    // what it restores as.
    let (mut w, g, c) = farming_world(9, 3.0, 0.0);
    w.run_years(300, 2, 300);
    let now = checkpoint::encode(&w);
    let n = w.agent_count();
    let agents_at = now.len() - 44 * n;
    let mut old = b"OIKOSCK6".to_vec();
    old.extend_from_slice(&now[8..agents_at]);
    for k in 0..n {
        old.extend_from_slice(&now[agents_at + 44 * k..agents_at + 44 * k + 36]);
    }
    let restored = checkpoint::decode_with(&old, inputs(&[&g, &c])).unwrap();
    assert!(restored.agent_slices().all(|a| a.herding.iter().chain(&a.herd_domestication).all(|&x| x == 0)));
    assert_eq!(checkpoint::encode(&restored), now, "no world herds yet, so nothing is lost");
}
