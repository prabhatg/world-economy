//! Cost-weighted partitioning: partitions follow where the agents are, re-balance as
//! population moves, and never change results.

use oikos_sim::world::{AGENT_COST, CELL_COST};
use oikos_sim::{Params, World, WorldConfig, checkpoint};
use std::ops::Range;

const RES4: WorldConfig = WorldConfig::SOUTHWEST_ASIA;

/// Agents only in the first quarter of the cells.
fn hotspot() -> World {
    World::filled_cells(RES4, 3, Params::default(), 4, 0..600).unwrap()
}

/// Max partition cost over the mean, in percent.
fn imbalance_pct(costs: &[u64]) -> u64 {
    let total: u64 = costs.iter().sum();
    costs.iter().max().unwrap() * costs.len() as u64 * 100 / total
}

/// What equal cell-count partitions would cost for the same agents.
fn equal_split_costs(w: &World, parts: usize) -> Vec<u64> {
    let ranges = w.grid.partition(parts);
    let mut agents = vec![0u64; parts];
    for a in w.agent_slices() {
        for &c in &a.cell {
            agents[ranges.iter().position(|r| r.contains(&c)).unwrap()] += 1;
        }
    }
    ranges.iter().zip(agents).map(|(r, a)| CELL_COST * (r.end - r.start) as u64 + AGENT_COST * a).collect()
}

#[test]
fn partitions_follow_the_agents_in_a_hotspot() {
    let mut w = hotspot();
    w.step(4);
    let ranges = w.partition_ranges();
    assert_eq!(ranges.len(), 4);
    assert!(ranges[2].end < 600, "three partitions share the hotspot: {ranges:?}");
    assert!(imbalance_pct(&w.partition_costs()) <= 105, "{:?}", w.partition_costs());
    assert!(imbalance_pct(&equal_split_costs(&w, 4)) > 250, "the test world must be skewed");
}

#[test]
fn partitions_rebalance_as_population_spreads() {
    let mut w = World::new(RES4, 20261025, Params::default()).unwrap();
    let mut seen: Vec<Vec<Range<u32>>> = Vec::new();
    for _ in 0..2000 {
        w.step(4);
        if seen.last() != Some(&w.partition_ranges()) {
            seen.push(w.partition_ranges());
        }
    }
    assert!(seen.len() > 3, "partitions should move with the population: {} layouts", seen.len());
    let now = imbalance_pct(&w.partition_costs());
    assert!(now <= 125, "imbalance {now}% (costs {:?})", w.partition_costs());
}

#[test]
fn rebalancing_never_changes_results() {
    let run = |parts: usize| {
        let mut w = hotspot();
        for _ in 0..60 {
            w.step(parts);
        }
        checkpoint::state_hash(&w)
    };
    let one = run(1);
    for parts in [2, 4, 7, 16] {
        assert_eq!(run(parts), one, "{parts} partitions");
    }
}

#[test]
fn changing_the_partition_count_mid_run_keeps_results() {
    let mut a = hotspot();
    let mut b = hotspot();
    for (i, parts) in [3, 8, 1, 5, 5, 2].into_iter().cycle().take(40).enumerate() {
        a.step(parts);
        b.step(1);
        assert_eq!(a.total_population(), b.total_population(), "step {i}");
    }
    assert_eq!(checkpoint::state_hash(&a), checkpoint::state_hash(&b));
    let covered: Vec<u32> = a.partition_ranges().iter().flat_map(|r| r.clone()).collect();
    assert_eq!(covered, (0..a.grid.len() as u32).collect::<Vec<_>>());
}
