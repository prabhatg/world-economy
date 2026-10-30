//! Down-the-line exchange of obsidian (#15, ADR-0060): bands within reach of a source
//! have its obsidian; every other band holds a share of what itself and its neighbors
//! held the step before, less what is used up or kept, so obsidian falls off with
//! distance along the settlement network (Renfrew, Dixon and Cann 1968) and never
//! crosses empty land. It is a tracer: it never changes what people do.

use oikos_grid::Grid;
use oikos_io::pack::{Data, Dims, FieldSpec, Pack, PackBuilder, Region, TimeRule, cells_hash};
use oikos_sim::agents::SHARE_ONE;
use oikos_sim::exchange::{OBSIDIAN, pass_down_the_line};
use oikos_sim::inputs::Inputs;
use oikos_sim::{Params, World, WorldConfig, checkpoint};

fn grid() -> Grid {
    Grid::from_base_cell(22, 3).unwrap()
}

fn h3_cells() -> Vec<u64> {
    let g = grid();
    (0..g.len() as u32).map(|c| g.h3(c)).collect()
}

/// Steps from `from` to every cell over the grid's neighbors.
fn distances(g: &Grid, from: u32) -> Vec<u32> {
    let mut d = vec![u32::MAX; g.len()];
    d[from as usize] = 0;
    let mut frontier = vec![from];
    while !frontier.is_empty() {
        let mut next = Vec::new();
        for c in frontier {
            for &n in g.neighbors(c) {
                if d[n as usize] == u32::MAX {
                    d[n as usize] = d[c as usize] + 1;
                    next.push(n);
                }
            }
        }
        frontier = next;
    }
    d
}

/// The cell farthest from the region's edge (largest distance to its nearest
/// incomplete-neighborhood cell), so rings around it are whole.
fn center(g: &Grid) -> u32 {
    let edge: Vec<u32> = (0..g.len() as u32).filter(|&c| g.neighbors(c).len() < 6).collect();
    let depth: Vec<u32> = (0..g.len()).map(|_| u32::MAX).collect();
    let depth = edge.iter().fold(depth, |acc, &e| {
        let d = distances(g, e);
        acc.iter().zip(&d).map(|(&a, &b)| a.min(b)).collect()
    });
    (0..g.len() as u32).max_by_key(|&c| (depth[c as usize], std::cmp::Reverse(c))).unwrap()
}

/// Mean availability (share) in each ring around `from`, up to `rings`.
fn ring_means(x: &[u32], d: &[u32], rings: u32) -> Vec<f64> {
    (0..=rings)
        .map(|r| {
            let v: Vec<f64> = d
                .iter()
                .zip(x)
                .filter(|&(&k, _)| k == r)
                .map(|(_, &v)| v as f64 / SHARE_ONE as f64)
                .collect();
            v.iter().sum::<f64>() / v.len() as f64
        })
        .collect()
}

fn run_steps(g: &Grid, population: &[u64], supplied: &[bool], loss: f64, steps: usize) -> Vec<u32> {
    let (mut x, mut next) = (vec![0; g.len()], vec![0; g.len()]);
    for _ in 0..steps {
        pass_down_the_line(g, population, supplied, loss, &x, &mut next);
        std::mem::swap(&mut x, &mut next);
    }
    x
}

#[test]
fn obsidian_falls_off_with_distance_from_its_source() {
    let g = grid();
    let s = center(&g);
    let d = distances(&g, s);
    let supplied: Vec<bool> = (0..g.len() as u32).map(|c| c == s).collect();
    let everyone = vec![25; g.len()];
    let x = run_steps(&g, &everyone, &supplied, 0.05, 400);
    assert_eq!(x[s as usize], SHARE_ONE, "bands at the source have all they want");
    let m = ring_means(&x, &d, 6);
    assert!(m.windows(2).all(|w| w[1] < w[0]), "falls with every step: {m:?}");
    // Down-the-line exchange falls off about exponentially (Renfrew 1977): each step
    // keeps a similar share of the last.
    let ratios: Vec<f64> = m.windows(2).skip(2).map(|w| w[1] / w[0]).collect();
    assert!(ratios.iter().all(|&r| (0.3..0.95).contains(&r)), "{ratios:?}");
    let spread =
        ratios.iter().cloned().fold(f64::MIN, f64::max) - ratios.iter().cloned().fold(f64::MAX, f64::min);
    assert!(spread < 0.15, "a steady fall-off: {ratios:?}");
    // More is used up or kept along the way, so less travels far.
    let thrifty = ring_means(&run_steps(&g, &everyone, &supplied, 0.01, 400), &d, 6);
    assert!(thrifty[6] > 2.0 * m[6], "{} vs {}", thrifty[6], m[6]);
}

#[test]
fn obsidian_never_crosses_empty_land_and_fades_where_the_chain_breaks() {
    let g = grid();
    let s = center(&g);
    let d = distances(&g, s);
    let supplied: Vec<bool> = (0..g.len() as u32).map(|c| c == s).collect();
    let gap: Vec<u64> = d.iter().map(|&k| if k == 3 { 0 } else { 25 }).collect();
    let x = run_steps(&g, &gap, &supplied, 0.02, 300);
    let m = ring_means(&x, &d, 5);
    assert!(m[2] > 0.0 && m[3] == 0.0 && m[4] == 0.0 && m[5] == 0.0, "{m:?}");
    // Settled again, the chain carries obsidian beyond.
    let everyone = vec![25; g.len()];
    let mut x2 = vec![0; g.len()];
    pass_down_the_line(&g, &everyone, &supplied, 0.02, &x, &mut x2);
    let mut x = x2.clone();
    for _ in 0..50 {
        pass_down_the_line(&g, &everyone, &supplied, 0.02, &x, &mut x2);
        std::mem::swap(&mut x, &mut x2);
    }
    assert!(ring_means(&x, &d, 5)[5] > 0.0);
    // Cut off from the source, the bands' obsidian fades step by step.
    let none = vec![false; g.len()];
    let faded = (0..200).fold(x.clone(), |x, _| {
        let mut next = vec![0; g.len()];
        pass_down_the_line(&g, &everyone, &none, 0.02, &x, &mut next);
        next
    });
    assert!(faded.iter().zip(&x).all(|(&f, &v)| f < v || v == 0), "fades everywhere");
    assert!(faded.iter().all(|&f| (f as f64) < 0.05 * SHARE_ONE as f64));
    // Nobody lives at the source: nothing to pass on.
    let mut empty_source = everyone.clone();
    empty_source[s as usize] = 0;
    assert!(run_steps(&g, &empty_source, &supplied, 0.02, 100).iter().all(|&v| v == 0));
}

// --- worlds with an obsidian pack ---

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

/// Wild-cereal country everywhere (15 degC, 500 mm), so foragers live in every cell.
fn climate_pack(h3: &[u64]) -> Vec<u8> {
    let mut b = PackBuilder::new("climate", region(h3));
    b.time(vec![-13000, 3000], TimeRule::Linear);
    b.field(spec("temp_mean", Dims::TimeCell, 0.01), Data::I16(vec![1500; 2 * h3.len()])).unwrap();
    b.field(spec("precip", Dims::TimeCell, 1.0), Data::U16(vec![500; 2 * h3.len()])).unwrap();
    b.build()
}

/// One field per source group: 1 in the cells within reach of one of its sources.
fn obsidian_pack(h3: &[u64], sources: &[(&str, u32)]) -> Vec<u8> {
    let mut b = PackBuilder::new(OBSIDIAN, region(h3));
    for &(name, cell) in sources {
        let v: Vec<u8> = (0..h3.len() as u32).map(|c| (c == cell) as u8).collect();
        b.field(spec(name, Dims::Cell, 1.0), Data::U8(v)).unwrap();
    }
    b.build()
}

fn inputs(packs: &[&[u8]]) -> Inputs {
    Inputs::from_packs(packs.iter().map(|b| Pack::parse(b.to_vec()).unwrap()).collect()).unwrap()
}

fn world(packs: &[&[u8]], seed: u64) -> World {
    let id = blake3::hash(packs[0]).to_hex().to_string();
    let p = Params { initial_occupancy: 0.5, cultivation_pressure_weight: 0.0, ..Params::default() };
    World::with_inputs(WorldConfig::geometry(&id), seed, p, inputs(packs)).unwrap()
}

/// Two sources at opposite ends of the region.
fn west_and_east() -> (u32, u32) {
    let g = grid();
    let s = center(&g);
    let d = distances(&g, s);
    let far: Vec<u32> = (0..g.len() as u32).filter(|&c| d[c as usize] == 5).collect();
    let west = far[0];
    let dw = distances(&g, west);
    let east = *far.iter().max_by_key(|&&c| (dw[c as usize], std::cmp::Reverse(c))).unwrap();
    (west, east)
}

fn packs() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let h3 = h3_cells();
    let (west, east) = west_and_east();
    (geometry_pack(&h3), climate_pack(&h3), obsidian_pack(&h3, &[("west", west), ("east", east)]))
}

#[test]
fn each_source_supplies_its_own_side_of_the_region() {
    let (g, c, o) = packs();
    let mut w = world(&[&g, &c, &o], 3);
    assert_eq!(w.obsidian_sources(), ["west", "east"]);
    w.run_years(500, 2, 500);
    let (west, east) = west_and_east();
    let (dw, de) = (distances(&w.grid, west), distances(&w.grid, east));
    let (xw, xe) = (w.obsidian(0), w.obsidian(1));
    assert_eq!((xw[west as usize], xe[east as usize]), (1.0, 1.0));
    let mut checked = 0;
    for cell in 0..w.grid.len() {
        if w.population[cell] == 0 || xw[cell] + xe[cell] == 0.0 {
            continue;
        }
        let share = xw[cell] / (xw[cell] + xe[cell]);
        if dw[cell] + 2 <= de[cell] {
            assert!(share > 0.5, "cell {cell} nearer the west source: {share}");
            checked += 1;
        } else if de[cell] + 2 <= dw[cell] {
            assert!(share < 0.5, "cell {cell} nearer the east source: {share}");
            checked += 1;
        }
    }
    assert!(checked > 100, "{checked}");
}

#[test]
fn obsidian_is_a_tracer_that_never_changes_history() {
    let (g, c, o) = packs();
    let mut with = world(&[&g, &c, &o], 5);
    let mut without = world(&[&g, &c], 5);
    with.run_years(400, 2, 400);
    without.run_years(400, 2, 400);
    assert_eq!(with.population, without.population);
    assert!(without.obsidian_sources().is_empty());
    assert!(with.obsidian(0).iter().any(|&x| x > 0.0 && x < 1.0));
}

#[test]
fn obsidian_is_partition_independent_and_restores() {
    let (g, c, o) = packs();
    let run = |parts| {
        let mut w = world(&[&g, &c, &o], 7);
        w.run_years(600, parts, 600);
        checkpoint::state_hash(&w)
    };
    assert_eq!(run(1), run(4));
    let mut w = world(&[&g, &c, &o], 7);
    w.run_years(333, 3, 333);
    let mut restored = checkpoint::decode_with(&checkpoint::encode(&w), inputs(&[&g, &c, &o])).unwrap();
    assert_eq!(restored.obsidian(1), w.obsidian(1));
    restored.run_years(267, 2, 267);
    assert_eq!(checkpoint::state_hash(&restored), run(1));
}

#[test]
fn checkpoints_from_before_exchange_restore_without_obsidian() {
    let (g, c, _) = packs();
    let mut w = world(&[&g, &c], 9);
    w.run_years(300, 2, 300);
    let now = checkpoint::encode(&w);
    // The previous layout: magic OIKOSCK3 and no source count after the soil.
    let cells = w.population.len();
    let params_len = serde_json::to_vec(&w.params).unwrap().len();
    let header = 8 + 4 + 32 + 4 + 32 * w.inputs.ids().len() + 8 + 8 + 4 + params_len + 4;
    let sources_at = header + 20 * cells;
    let mut before = now.clone();
    before[..8].copy_from_slice(b"OIKOSCK3");
    before.drain(sources_at..sources_at + 4);
    let restored = checkpoint::decode_with(&before, inputs(&[&g, &c])).unwrap();
    assert_eq!(checkpoint::encode(&restored), now);
    // A checkpoint whose source count does not match its packs is refused.
    let mut bad = now.clone();
    bad[sources_at..sources_at + 4].copy_from_slice(&2u32.to_le_bytes());
    let err = checkpoint::decode_with(&bad, inputs(&[&g, &c])).err().expect("refused");
    assert!(err.contains("obsidian"), "{err}");
}
