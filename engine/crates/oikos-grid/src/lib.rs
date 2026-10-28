//! Grid topology on the H3 geodesic hexagon grid.
//!
//! Only integer topology (cell ids, neighbors, hierarchy) is used at simulation time.
//! Geometry (areas, centroids) involves platform trigonometry, so it is computed
//! offline into forcing packs or via `oikos-math`, never from `h3o` inside a run.

use h3o::{BaseCell, CellIndex, Resolution};
use std::ops::Range;

#[derive(Debug)]
pub enum GridError {
    InvalidBaseCell(u8),
    InvalidResolution(u8),
}

impl std::fmt::Display for GridError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridError::InvalidBaseCell(b) => write!(f, "invalid H3 base cell {b}"),
            GridError::InvalidResolution(r) => write!(f, "invalid H3 resolution {r}"),
        }
    }
}

impl std::error::Error for GridError {}

/// A region of H3 cells at one resolution with neighbor lists restricted to the region.
///
/// Cells are stored in ascending H3 index order, which follows the H3 hierarchy and
/// keeps nearby cells close together; local indices (`u32`) are positions in that order.
pub struct Grid {
    resolution: Resolution,
    cells: Vec<CellIndex>,
    neighbor_offsets: Vec<u32>,
    neighbor_targets: Vec<u32>,
}

impl Grid {
    /// All descendants at `resolution` of one of the 122 H3 base cells.
    pub fn from_base_cell(base_cell: u8, resolution: u8) -> Result<Grid, GridError> {
        let base = BaseCell::try_from(base_cell).map_err(|_| GridError::InvalidBaseCell(base_cell))?;
        let res = Resolution::try_from(resolution).map_err(|_| GridError::InvalidResolution(resolution))?;
        let root = CellIndex::base_cells()
            .find(|c| c.base_cell() == base)
            .ok_or(GridError::InvalidBaseCell(base_cell))?;
        Ok(Grid::from_cells(res, root.children(res).collect()))
    }

    /// Builds a grid from arbitrary cells (all at `resolution`).
    pub fn from_cells(resolution: Resolution, mut cells: Vec<CellIndex>) -> Grid {
        cells.sort_unstable_by_key(|c| u64::from(*c));
        cells.dedup();
        // Cells are sorted, so a binary search finds local indices: no hashing, whose
        // per-process random seed would make instruction counts vary between runs.
        let index = |c: &CellIndex| {
            cells.binary_search_by_key(&u64::from(*c), |x| u64::from(*x)).ok().map(|i| i as u32)
        };

        let mut neighbor_offsets = Vec::with_capacity(cells.len() + 1);
        let mut neighbor_targets = Vec::with_capacity(cells.len() * 6);
        neighbor_offsets.push(0);
        for cell in &cells {
            let mut ns: Vec<u32> = cell
                .grid_disk::<Vec<_>>(1)
                .into_iter()
                .filter(|n| n != cell)
                .filter_map(|n| index(&n))
                .collect();
            ns.sort_unstable();
            neighbor_targets.extend(ns);
            neighbor_offsets.push(neighbor_targets.len() as u32);
        }
        Grid { resolution, cells, neighbor_offsets, neighbor_targets }
    }

    pub fn resolution(&self) -> u8 {
        u8::from(self.resolution)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn h3(&self, cell: u32) -> u64 {
        u64::from(self.cells[cell as usize])
    }

    /// Neighbors of `cell` inside the region, in ascending local-index order.
    pub fn neighbors(&self, cell: u32) -> &[u32] {
        let a = self.neighbor_offsets[cell as usize] as usize;
        let b = self.neighbor_offsets[cell as usize + 1] as usize;
        &self.neighbor_targets[a..b]
    }

    /// Splits cells into `parts` contiguous ranges of near-equal size.
    pub fn partition(&self, parts: usize) -> Vec<Range<u32>> {
        partition_ranges(self.len() as u32, parts)
    }
}

/// `n` items into `parts` contiguous ranges whose sizes differ by at most one.
pub fn partition_ranges(n: u32, parts: usize) -> Vec<Range<u32>> {
    assert!(parts > 0);
    let parts = parts as u32;
    (0..parts).map(|p| (n * p / parts)..(n * (p + 1) / parts)).collect()
}

/// Cells into `parts` contiguous ranges of near-equal total cost: each range ends at the
/// first cell where the running cost reaches its share, so no range costs more than its
/// share plus about one cell. Every range gets at least one cell while cells last.
/// Contiguity keeps partitions spatially compact (cells are in H3 hierarchy order).
pub fn partition_weighted(costs: &[u64], parts: usize) -> Vec<Range<u32>> {
    assert!(parts > 0);
    let n = costs.len();
    let total: u128 = costs.iter().map(|&c| c as u128).sum();
    if total == 0 {
        return partition_ranges(n as u32, parts);
    }
    let mut bounds = Vec::with_capacity(parts + 1);
    bounds.push(0usize);
    let (mut i, mut prefix) = (0usize, 0u128);
    for p in 1..parts {
        while i < n && prefix * (parts as u128) < total * p as u128 {
            prefix += costs[i] as u128;
            i += 1;
        }
        let lo = (bounds[p - 1] + 1).min(n);
        let hi = n.saturating_sub(parts - p).max(lo);
        bounds.push(i.clamp(lo, hi));
    }
    bounds.push(n);
    bounds.windows(2).map(|b| b[0] as u32..b[1] as u32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn southwest_asia_region() {
        // Base cell 22 covers the Fertile Crescent (36°N, 40°E).
        let g = Grid::from_base_cell(22, 4).unwrap();
        assert_eq!(g.len(), 2401);
        for c in 0..g.len() as u32 {
            let ns = g.neighbors(c);
            assert!(ns.len() <= 6 && !ns.is_empty());
            for &n in ns {
                assert!(g.neighbors(n).contains(&c), "neighbor relation must be symmetric");
            }
        }
        // Interior cells have six neighbors; only the region boundary has fewer.
        let interior = (0..g.len() as u32).filter(|&c| g.neighbors(c).len() == 6).count();
        assert!(interior > g.len() * 9 / 10, "interior = {interior}");
    }

    #[test]
    fn partitions_cover() {
        let r = partition_ranges(10, 3);
        assert_eq!(r, vec![0..3, 3..6, 6..10]);
    }

    fn cost(costs: &[u64], r: &Range<u32>) -> u64 {
        costs[r.start as usize..r.end as usize].iter().sum()
    }

    #[test]
    fn weighted_partitions_follow_cost_not_cell_count() {
        // Three quarters of the work sits in the first eighth of the cells.
        let costs: Vec<u64> = (0..800).map(|c| if c < 100 { 30 } else { 1 }).collect();
        let r = partition_weighted(&costs, 4);
        assert_eq!(r.len(), 4);
        assert!(r[0].end < 100 && r[1].end < 100 && r[2].end <= 101, "{r:?}");
        let total: u64 = costs.iter().sum();
        for p in &r {
            assert!(cost(&costs, p) <= total / 4 + 30, "{p:?}: {}", cost(&costs, p));
        }
    }

    #[test]
    fn weighted_partitions_isolate_a_hot_cell() {
        let r = partition_weighted(&[1, 1, 100, 1, 1], 4);
        assert_eq!(r, vec![0..2, 2..3, 3..4, 4..5]);
    }

    #[test]
    fn weighted_partitions_handle_degenerate_inputs() {
        assert_eq!(partition_weighted(&[0, 0, 0, 0], 2), vec![0..2, 2..4]);
        assert_eq!(partition_weighted(&[5, 5], 4), vec![0..1, 1..2, 2..2, 2..2]);
        assert_eq!(partition_weighted(&[], 2), vec![0..0, 0..0]);
        assert_eq!(partition_weighted(&[7, 1, 1], 1), vec![0..3]);
    }

    proptest::proptest! {
        #[test]
        fn weighted_partitions_cover_and_bound_cost(
            costs in proptest::collection::vec(0u64..1000, 0..300),
            parts in 1usize..17,
        ) {
            let r = partition_weighted(&costs, parts);
            let n = costs.len() as u32;
            proptest::prop_assert_eq!(r.len(), parts);
            proptest::prop_assert_eq!(r[0].start, 0);
            proptest::prop_assert_eq!(r[parts - 1].end, n);
            for w in r.windows(2) {
                proptest::prop_assert_eq!(w[0].end, w[1].start);
            }
            // Every partition gets a cell while there are cells to go around.
            for p in &r[..parts.min(costs.len())] {
                proptest::prop_assert!(!p.is_empty(), "{:?}", r);
            }
            let total: u64 = costs.iter().sum();
            let max = costs.iter().copied().max().unwrap_or(0);
            for p in &r {
                proptest::prop_assert!(cost(&costs, p) <= total / parts as u64 + 2 * max, "{:?} {:?}", r, costs);
            }
        }

        #[test]
        fn uniform_costs_give_near_equal_cell_counts(n in 0u32..2000, parts in 1usize..33) {
            let r = partition_weighted(&vec![3; n as usize], parts);
            let sizes: Vec<u32> = r.iter().map(|p| p.end - p.start).collect();
            let (lo, hi) = (sizes.iter().min().unwrap(), sizes.iter().max().unwrap());
            proptest::prop_assert!(hi - lo <= 1, "{:?}", sizes);
        }
    }
}
